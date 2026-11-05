//! Stream mode: an endless song, composed eight bars at a time ([`crate::compose::stream`]) and rendered stage by
//! stage ahead of the playhead, joined without a seam.
//!
//! **Seamless joins.** The renderer keeps every voice's state from one stage to the next, by the voice's name: the
//! oscillator's phase, the envelope and where it stands (attack, decay, release), the gate's last value (so a note
//! held across the seam is not struck again), the filter's state, the note's start for `note_age`, and every drum
//! hit still ringing. The send effects (the reverb with its predelay, the ping-pong delay, the tape wow's history,
//! the warmth's low-pass) are one instance for the whole stream. A voice that leaves is released from where it
//! stood with its last stage's functions, and one that returns picks up its own state. So a stage played after the
//! one before is, sample for sample, what one long render of both would be (`tests/stream.rs` checks exactly that).
//!
//! **Level.** The offline renderer scales a song to -1 dBFS by its peak; a stream has no end to measure. Each stage's
//! peak is known before it plays, so each gets a gain that keeps it 1 dB under the ceiling (rising by at most 1 dB a
//! stage, never above 1), and the gain moves between stages over half a second: lowered at the end of the louder
//! stage's predecessor, raised at the start of the next. A gain between two safe gains is safe, so nothing clips and
//! nothing steps.
//!
//! **What differs from the offline render**, all documented: `note_left` sees only to the end of the stage; warmth
//! saturates against a fixed reference (full scale) instead of the song's own peak; wow reads only behind (a stream
//! cannot read ahead), so it delays the mix by up to its depth.

use crate::compose::stream::{Composer, Seed, Stage, STAGE_BEATS};
use crate::effects::{Freeverb, PingPong};
use crate::error::Error;
use crate::expr::At;
use crate::render::{drum_sample, not_finite, pan_gains, splitmix64, tone, DrumHit, Svf, TonalFrame, VoiceTrace, CEILING};
use crate::song::{Drum, Song, Tonal, Voice, VoiceKind};
use std::collections::{HashMap, VecDeque};
use std::f64::consts::{LN_2, PI, TAU};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;

/// The first sample of stage `k` at this tempo and rate: the first sample whose time is at or after its first beat.
pub fn stage_start(k: u64, bpm: f64, rate: u32) -> u64 {
    let x = k as f64 * STAGE_BEATS * 60.0 * rate as f64 / bpm;
    let s = (x - 1e-6).ceil();
    if s < 0.0 {
        0
    } else {
        s as u64
    }
}

/// The stage containing absolute sample `frame`.
pub fn stage_of(frame: u64, bpm: f64, rate: u32) -> u64 {
    let mut k = (frame as f64 * bpm / (60.0 * rate as f64) / STAGE_BEATS).floor().max(0.0) as u64;
    while k > 0 && stage_start(k, bpm, rate) > frame {
        k -= 1;
    }
    while stage_start(k + 1, bpm, rate) <= frame {
        k += 1;
    }
    k
}

// ------------------------------------------------------------------------------------------------ voice state

#[derive(Clone, Default)]
struct TonalState {
    env: f64,
    attacking: bool,
    prev_gate: f64,
    fenv: f64,
    filter: Svf,
    phase: f64,
    phase2: f64,
    /// The note's first sample and the sample its gate closes, relative to this stage's start.
    note_start: i64,
    note_end: i64,
}

#[derive(Clone)]
struct Ring {
    /// Relative to this stage's start (negative: it began in an earlier stage).
    start: i64,
    amp: f64,
    seed: u64,
}

#[derive(Clone, Default)]
struct DrumState {
    hits: Vec<Ring>,
    count: u64,
    prev: f64,
}

#[derive(Clone)]
enum State {
    Tonal(TonalState),
    Drum(DrumState),
}

/// A voice that has left: its last stage's song (for its functions), and how many beats after that stage's start
/// this stage begins.
struct Leaving {
    song: Song,
    index: usize,
    offset_beats: f64,
    state: State,
}

/// What one voice rendered over one stage: its samples, and its trace (frames at the absolute multiples of `step`).
struct VoiceOut {
    x: Vec<f64>,
    written: Vec<bool>,
    trace: VoiceTrace,
    state: State,
    /// The voice has fallen silent and holds nothing ringing (a leaving voice can then be forgotten).
    silent: bool,
}

/// The renderer's view of one stage: its song, where it starts, its length.
struct Span {
    start: u64,
    n: usize,
    /// The beat (in the song's own time) of sample 0 of the span.
    t0: f64,
    bpm: f64,
    rate: f64,
    step: usize,
}

impl Span {
    #[inline]
    fn t(&self, i: usize) -> f64 {
        // The absolute sample's time, less the stage's first beat: the same number the offline renderer computes for
        // a song that starts at this stage.
        (self.start + i as u64) as f64 * self.bpm / (60.0 * self.rate) - self.t0
    }
    #[inline]
    fn records(&self, i: usize) -> bool {
        (self.start + i as u64) % self.step as u64 == 0
    }
}

/// One tonal voice over a span: the offline renderer's `render_tonal`, with its state carried in and out. With
/// `off`, the gate is held closed (a voice that has left, releasing).
fn render_tonal(song: &Song, voice: &Voice, tv: &Tonal, st: &mut TonalState, sp: &Span, off: bool) -> Result<VoiceOut, Error> {
    let p = &song.program;
    let rate = sp.rate;
    let nyquist = rate / 2.0;
    let attack_step = 1.0 / (tv.attack_ms * 0.001 * rate);
    let decay_step = (1.0 - tv.sustain) / (tv.decay_ms * 0.001 * rate);
    let release_step = 1.0 / (tv.release_ms * 0.001 * rate);
    let fenv_fall = (-1.0 / (tv.filter_decay_ms * 0.001 * rate)).exp();
    let k = std::f64::consts::SQRT_2 * (1.0 - tv.resonance);
    let drive_gain = 1.0 + tv.drive;
    let drive_norm = 1.0 / drive_gain.tanh();
    let detune = if tv.detune_cents > 0.0 { Some((tv.detune_cents / 2400.0).exp2()) } else { None };
    let roll = tv.rolloff_hz / rate;
    let n = sp.n;
    let mut out = VoiceOut { x: vec![0.0; n], written: vec![false; n], trace: VoiceTrace::Tonal(Vec::new()), state: State::Tonal(TonalState::default()), silent: false };
    let mut frames = Vec::with_capacity(n / sp.step + 2);
    let mut gates = Vec::with_capacity(n);
    for i in 0..n {
        let g = if off { 0.0 } else { tv.gate.eval(p, sp.t(i)) };
        if !g.is_finite() {
            return Err(not_finite(voice, "gate", g, sp.t(i)));
        }
        gates.push(g);
    }
    for i in 0..n {
        let t = sp.t(i);
        let g = gates[i];
        let onset = g > 0.0 && st.prev_gate <= 0.0;
        if onset {
            st.note_start = i as i64;
            st.note_end = (i..n).find(|&j| gates[j] <= 0.0).unwrap_or(n) as i64;
        }
        let at = At { t, age: (i as i64 - st.note_start) as f64 / rate, left: (st.note_end - i as i64) as f64 / rate };
        st.prev_gate = g;
        let record = sp.records(i);
        if g > 0.0 {
            if onset {
                st.attacking = true;
                st.fenv = 1.0;
            }
            if st.attacking {
                st.env = (st.env + attack_step).min(1.0);
                if st.env >= 1.0 {
                    st.attacking = false;
                }
            } else {
                st.env = (st.env - decay_step).max(tv.sustain);
            }
        } else {
            st.attacking = false;
            st.env = (st.env - release_step).max(0.0);
            if st.env == 0.0 {
                if record {
                    frames.push(TonalFrame { gate: g, pitch: f64::NAN, amp: f64::NAN, level: 0.0 });
                }
                st.phase = 0.0;
                st.phase2 = 0.0;
                st.filter = Svf::default();
                continue;
            }
        }
        let semis = tv.pitch.eval_at(p, &at);
        if !semis.is_finite() {
            return Err(not_finite(voice, "pitch", semis, t));
        }
        let freq = song.reference_hz * (semis / 12.0).exp2();
        if freq * detune.unwrap_or(1.0) >= nyquist {
            return Err(Error::line(
                voice.line,
                format!("{}: pitch {semis} semitones ({freq:.1} Hz) at beat {t:.6} is above the Nyquist limit ({nyquist} Hz)", voice.label()),
            ));
        }
        let amp = match &tv.amp {
            Some(a) => {
                let v = a.eval_at(p, &at);
                if !v.is_finite() {
                    return Err(not_finite(voice, "amp", v, t));
                }
                v
            }
            None => 1.0,
        };
        let dt = freq / rate;
        let mut s = match detune {
            None => {
                let s = tone(tv, roll, st.phase, dt);
                st.phase += dt;
                s
            }
            Some(r) => {
                let (d1, d2) = (dt / r, dt * r);
                let s = 0.5 * (tone(tv, roll, st.phase, d1) + tone(tv, roll, st.phase2, d2));
                st.phase += d1;
                st.phase2 += d2;
                st.phase2 -= st.phase2.floor();
                s
            }
        };
        st.phase -= st.phase.floor();
        if let Some(c) = &tv.cutoff {
            let hz = c.eval_at(p, &at);
            if !hz.is_finite() {
                return Err(not_finite(voice, "cutoff", hz, t));
            }
            let hz = hz * (tv.filter_env * st.fenv).exp2();
            s = st.filter.lowpass(s, (hz.max(10.0).min(0.45 * rate) / rate * PI).tan(), k);
        }
        st.fenv *= fenv_fall;
        if tv.drive > 0.0 {
            s = (drive_gain * s).tanh() * drive_norm;
        }
        if record {
            frames.push(TonalFrame { gate: g, pitch: semis, amp, level: st.env * amp * voice.gain });
        }
        out.x[i] = s * st.env * amp * voice.gain;
        out.written[i] = true;
    }
    // Carry the note's place into the next stage's sample numbers.
    st.note_start -= n as i64;
    st.note_end -= n as i64;
    out.silent = st.env == 0.0 && st.prev_gate <= 0.0;
    out.trace = VoiceTrace::Tonal(frames);
    out.state = State::Tonal(st.clone());
    Ok(out)
}

/// One drum over a span: the offline `render_drum` with its ringing hits carried. `off`: no new hits.
fn render_drum(song: &Song, voice: &Voice, drum: &Drum, seed: u64, st: &mut DrumState, sp: &Span, off: bool) -> Result<VoiceOut, Error> {
    let p = &song.program;
    let rate = sp.rate;
    let secs = match &drum.dash {
        Some(d) => {
            if d.from_hz.max(d.to_hz) >= rate / 2.0 {
                return Err(Error::line(
                    voice.line,
                    format!("{}: a dash from {} Hz to {} Hz reaches the Nyquist limit ({} Hz)", voice.label(), d.from_hz, d.to_hz, rate / 2.0),
                ));
            }
            d.length_ms * 0.001
        }
        None => drum.noise.as_ref().map_or(drum.kind.length(), |n| n.length_ms * 0.001),
    };
    let len = (secs * rate).ceil() as i64;
    let n = sp.n;
    let mut out = VoiceOut { x: vec![0.0; n], written: vec![false; n], trace: VoiceTrace::Drum(Vec::new()), state: State::Drum(DrumState::default()), silent: false };
    let mut onsets = Vec::new();
    for i in 0..n {
        if !off {
            let t = sp.t(i);
            let g = drum.trigger.eval(p, t);
            if !g.is_finite() {
                return Err(not_finite(voice, "trigger", g, t));
            }
            if g > 0.0 && st.prev <= 0.0 {
                let amp = match &drum.amp {
                    Some(a) => {
                        let v = a.eval(p, t);
                        if !v.is_finite() {
                            return Err(not_finite(voice, "amp", v, t));
                        }
                        v
                    }
                    None => 1.0,
                };
                st.hits.push(Ring { start: i as i64, amp: amp * voice.gain, seed: splitmix64(seed ^ st.count) });
                onsets.push(DrumHit { sample: sp.start + i as u64, amp });
                st.count += 1;
            }
            st.prev = g;
        } else {
            st.prev = 0.0;
        }
        let i64i = i as i64;
        st.hits.retain(|h| i64i - h.start < len);
        if st.hits.is_empty() {
            continue;
        }
        let s: f64 = st.hits.iter().map(|h| h.amp * drum_sample(drum.kind, drum.dash.as_ref(), drum.noise.as_ref(), (i64i - h.start) as usize, rate, h.seed)).sum();
        out.x[i] = s;
        out.written[i] = true;
    }
    for h in &mut st.hits {
        h.start -= n as i64;
    }
    out.silent = st.hits.is_empty();
    out.trace = VoiceTrace::Drum(onsets);
    out.state = State::Drum(st.clone());
    Ok(out)
}

fn render_voice(song: &Song, index: usize, seed: u64, state: State, sp: &Span, off: bool) -> Result<VoiceOut, Error> {
    let v = &song.voices[index];
    match (&v.kind, state) {
        (VoiceKind::Tonal(tv), State::Tonal(mut st)) => render_tonal(song, v, tv, &mut st, sp, off),
        (VoiceKind::Drum(d), State::Drum(mut st)) => render_drum(song, v, d, seed, &mut st, sp, off),
        (VoiceKind::Tonal(tv), State::Drum(_)) => render_tonal(song, v, tv, &mut TonalState::default(), sp, off),
        (VoiceKind::Drum(d), State::Tonal(_)) => render_drum(song, v, d, seed, &mut DrumState::default(), sp, off),
    }
}

/// Whether two versions of a voice are the same instrument: everything but the time functions (pitch, gate, trigger,
/// cutoff, and an amp that is there in both) is equal.
fn same_instrument(a: &Voice, b: &Voice) -> bool {
    if (a.gain, a.pan, a.reverb, a.delay) != (b.gain, b.pan, b.reverb, b.delay) {
        return false;
    }
    match (&a.kind, &b.kind) {
        (VoiceKind::Tonal(x), VoiceKind::Tonal(y)) => {
            x.wave == y.wave
                && x.partials == y.partials
                && x.rolloff_hz == y.rolloff_hz
                && (x.attack_ms, x.decay_ms, x.sustain, x.release_ms) == (y.attack_ms, y.decay_ms, y.sustain, y.release_ms)
                && (x.resonance, x.filter_env, x.filter_decay_ms, x.drive, x.detune_cents) == (y.resonance, y.filter_env, y.filter_decay_ms, y.drive, y.detune_cents)
                && x.amp.is_some() == y.amp.is_some()
                && x.cutoff.is_some() == y.cutoff.is_some()
        }
        (VoiceKind::Drum(x), VoiceKind::Drum(y)) => {
            format!("{:?}{:?}", x.kind, x.dash) == format!("{:?}{:?}", y.kind, y.dash) && x.amp.is_some() == y.amp.is_some()
        }
        _ => false,
    }
}

fn fresh_state(v: &Voice) -> State {
    match v.kind {
        VoiceKind::Tonal(_) => State::Tonal(TonalState::default()),
        VoiceKind::Drum(_) => State::Drum(DrumState::default()),
    }
}

// ------------------------------------------------------------------------------------------------ the renderer

/// Warmth's reference level in a stream (the offline renderer uses the song's own peak).
pub const WARMTH_REFERENCE: f64 = 1.0;
/// How long the gain takes to move between two stages' levels, in seconds.
pub const GAIN_RAMP_SECS: f64 = 0.5;
/// A stage's gain keeps its peak this far under the ceiling, and rises by at most this much a stage (as a factor).
const HEADROOM: f64 = 0.891_250_938_133_745_5; // -1 dB
const MAX_RISE: f64 = 1.122_018_454_301_963_3; // +1 dB

/// One rendered stage.
pub struct Rendered {
    pub index: u64,
    /// The absolute sample of its first frame.
    pub start: u64,
    /// Interleaved stereo, before the stage's gain.
    pub audio: Vec<f32>,
    /// The stage's gain (see the module header).
    pub gain: f64,
    /// Per roster voice: frames at the absolute multiples of the trace step that fall in this stage (all closed for a
    /// voice that is absent), or every drum hit with its absolute sample.
    pub traces: Vec<VoiceTrace>,
}

impl Rendered {
    pub fn frames(&self) -> usize {
        self.audio.len() / 2
    }
}

/// The state that runs through the whole stream: every voice, the effects, the level.
pub struct Renderer {
    rate: u32,
    bpm: f64,
    step: usize,
    roster: Vec<String>,
    /// Which roster voices are drums (an absent voice's empty trace has its kind).
    roster_drum: Vec<bool>,
    live: HashMap<String, (State, Song, usize)>,
    leaving: Vec<(String, Leaving)>,
    verb: Freeverb,
    predelay: VecDeque<f64>,
    delay: PingPong,
    wow_hist: [VecDeque<f64>; 2],
    warm_lp: [f64; 2],
    stage: u64,
    last_gain: Option<f64>,
    /// Diagnostic: start every stage from rest, as if each were its own song (to show what a seam would be without
    /// the carried state). Off in a real stream.
    pub reset_at_seams: bool,
}

impl Renderer {
    /// A renderer for a stream whose first stage is `first` (its tempo, rate and effects hold for the whole stream).
    pub fn new(first: &Song, roster: Vec<String>, roster_drum: Vec<bool>, step: usize) -> Renderer {
        let fx = &first.effects;
        let rate = first.rate as f64;
        let pre = (fx.reverb_predelay_ms * 0.001 * rate).round() as usize;
        let dl = (fx.delay_beats * 60.0 / first.bpm * rate).round() as usize;
        Renderer {
            rate: first.rate,
            bpm: first.bpm,
            step: step.max(1),
            roster,
            roster_drum,
            live: HashMap::new(),
            leaving: Vec::new(),
            verb: Freeverb::new(rate, fx.reverb_size, fx.reverb_damp, fx.reverb_width),
            predelay: std::iter::repeat(0.0).take(pre).collect(),
            delay: PingPong::new(dl, fx.delay_feedback, fx.delay_tone_hz, rate),
            wow_hist: [VecDeque::new(), VecDeque::new()],
            warm_lp: [0.0; 2],
            stage: 0,
            last_gain: None,
            reset_at_seams: false,
        }
    }

    fn roster_index(&self, name: &str) -> usize {
        self.roster.iter().position(|r| r == name).unwrap_or(usize::MAX)
    }

    /// Render the next stage. Its song must keep the first stage's tempo and rate.
    pub fn render(&mut self, song: &Song) -> Result<Rendered, Error> {
        if song.bpm != self.bpm || song.rate != self.rate {
            return Err(Error::msg("a stream keeps one tempo and rate"));
        }
        let k = self.stage;
        let start = stage_start(k, self.bpm, self.rate);
        let end = stage_start(k + 1, self.bpm, self.rate);
        let n = (end - start) as usize;
        let sp = Span { start, n, t0: k as f64 * STAGE_BEATS, bpm: self.bpm, rate: self.rate as f64, step: self.step };
        if self.reset_at_seams {
            self.live.clear();
            self.leaving.clear();
            let fresh = Renderer::new(song, self.roster.clone(), self.roster_drum.clone(), self.step);
            self.verb = fresh.verb;
            self.predelay = fresh.predelay;
            self.delay = fresh.delay;
            self.wow_hist = fresh.wow_hist;
            self.warm_lp = fresh.warm_lp;
        }

        // Voices present now take their state from the last stage, or from their release if they had left.
        let mut jobs: Vec<(String, Song, usize, State, f64, bool)> = Vec::new();
        let mut live_prev = std::mem::take(&mut self.live);
        for (i, v) in song.voices.iter().enumerate() {
            let state = match live_prev.remove(&v.name) {
                Some((s, old, idx)) if same_instrument(&old.voices[idx], v) => s,
                // Its instrument changed (a new module from the move book): the old one is released with its own
                // settings and the new one starts from rest, as a voice leaving and another entering would. Swapping
                // the spectrum, gain or amp of a note while it sounds is a step in the waveform: a click.
                Some((s, old, idx)) => {
                    self.leaving.push((v.name.clone(), Leaving { song: old, index: idx, offset_beats: 0.0, state: s }));
                    fresh_state(v)
                }
                None => match self.leaving.iter().position(|(n, _)| *n == v.name) {
                    Some(at) => self.leaving.remove(at).1.state,
                    None => fresh_state(v),
                },
            };
            jobs.push((v.name.clone(), song.clone(), i, state, 0.0, false));
        }
        // Those that were here and are not: released with their own functions.
        for (name, (state, old, idx)) in live_prev {
            self.leaving.push((name, Leaving { song: old, index: idx, offset_beats: 0.0, state }));
        }
        let leaving = std::mem::take(&mut self.leaving);
        for (name, l) in leaving {
            jobs.push((name, l.song, l.index, l.state, l.offset_beats + STAGE_BEATS, true));
        }

        // Render every voice; each is independent until the sum, so they run on several threads, each with its own
        // copy of its song (definitions keep a result cache, so a song is not shared between threads).
        let seeds: Vec<u64> = jobs.iter().map(|j| splitmix64(0x4C55_4D49_4E45_5343 ^ self.roster_index(&j.0) as u64)).collect();
        let outs: Vec<(String, Song, usize, f64, bool, Result<VoiceOut, Error>)> = std::thread::scope(|scope| {
            let handles: Vec<_> = jobs
                .into_iter()
                .zip(seeds)
                .map(|((name, s, idx, state, offset, off), seed)| {
                    let sp = Span { start: sp.start, n: sp.n, t0: sp.t0 - offset, bpm: sp.bpm, rate: sp.rate, step: sp.step };
                    scope.spawn(move || {
                        let r = render_voice(&s, idx, seed, state, &sp, off);
                        (name, s, idx, offset, off, r)
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().expect("a voice thread panicked")).collect()
        });

        let mut left = vec![0.0; n];
        let mut right = vec![0.0; n];
        let mut rev = vec![0.0; n];
        let mut del = vec![0.0; n];
        let mut traces: Vec<Option<VoiceTrace>> = (0..self.roster.len()).map(|_| None).collect();
        for (name, s, idx, offset, off, r) in outs {
            let out = r?;
            let v = &s.voices[idx];
            let (gl, gr) = pan_gains(v.pan);
            for i in 0..n {
                if !out.written[i] {
                    continue;
                }
                let x = out.x[i];
                left[i] += x * gl;
                right[i] += x * gr;
                if v.reverb > 0.0 {
                    rev[i] += x * v.reverb;
                }
                if v.delay > 0.0 {
                    del[i] += x * v.delay;
                }
            }
            let ri = self.roster_index(&name);
            // The playing voice's trace wins over a release of the same name (an instrument that was replaced).
            if ri < traces.len() && (!off || traces[ri].is_none()) {
                traces[ri] = Some(out.trace);
            }
            if off {
                if !out.silent {
                    self.leaving.push((name, Leaving { song: s, index: idx, offset_beats: offset, state: out.state }));
                }
            } else {
                self.live.insert(name, (out.state, s, idx));
            }
        }
        // Absent voices: closed frames on the same grid, no hits.
        let closed = (start..end).filter(|i| i % self.step as u64 == 0).count();
        let traces: Vec<VoiceTrace> = traces
            .into_iter()
            .enumerate()
            .map(|(ri, t)| {
                t.unwrap_or_else(|| {
                    if self.roster_drum.get(ri).copied().unwrap_or(false) {
                        VoiceTrace::Drum(Vec::new())
                    } else {
                        VoiceTrace::Tonal(vec![TonalFrame { gate: 0.0, pitch: f64::NAN, amp: f64::NAN, level: 0.0 }; closed])
                    }
                })
            })
            .collect();

        self.effects(song, &mut left, &mut right, &rev, &del);

        let mut peak = 0.0f64;
        for (i, &x) in left.iter().chain(&right).enumerate() {
            if !x.is_finite() {
                return Err(Error::msg(format!("the stream's mix is not finite at stage {} sample {} ({x})", k + 1, i % n.max(1))));
            }
            peak = peak.max(x.abs());
        }
        let safe = if peak > 0.0 { CEILING * HEADROOM / peak } else { 1.0 };
        let gain = match self.last_gain {
            None => safe.min(1.0),
            Some(g) => safe.min(g * MAX_RISE).min(1.0),
        };
        self.last_gain = Some(gain);
        let mut audio = Vec::with_capacity(2 * n);
        for i in 0..n {
            audio.push(left[i] as f32);
            audio.push(right[i] as f32);
        }
        self.stage += 1;
        Ok(Rendered { index: k, start, audio, gain, traces })
    }

    /// The send effects, wow and warmth, carried across stages (see the module header for what differs offline).
    fn effects(&mut self, song: &Song, left: &mut [f64], right: &mut [f64], rev: &[f64], del: &[f64]) {
        let fx = &song.effects;
        let n = left.len();
        if !self.predelay.is_empty() {
            for (i, &x) in rev.iter().enumerate() {
                self.predelay.push_back(x);
                let y = self.predelay.pop_front().unwrap_or(0.0);
                let (l, r) = self.verb.tick(y);
                left[i] += l;
                right[i] += r;
            }
        } else {
            for (i, &x) in rev.iter().enumerate() {
                let (l, r) = self.verb.tick(x);
                left[i] += l;
                right[i] += r;
            }
        }
        for (i, &x) in del.iter().enumerate() {
            let (l, r) = self.delay.tick(x);
            left[i] += l;
            right[i] += r;
        }
        if fx.wow_cents > 0.0 {
            let rate = self.rate as f64;
            let f = fx.wow_rate_hz;
            let depth = fx.wow_cents / 1200.0 * LN_2 * rate / (TAU * f);
            let start = stage_start(self.stage, self.bpm, self.rate);
            let keep = (2.0 * depth).ceil() as usize + 4;
            for (ch, x) in [&mut *left, &mut *right].into_iter().enumerate() {
                let hist = &mut self.wow_hist[ch];
                for i in 0..n {
                    hist.push_back(x[i]);
                    while hist.len() > keep {
                        hist.pop_front();
                    }
                    // The playhead runs `d` samples behind the newest, d swinging between 0 and twice the depth, so
                    // its speed swings by the wow's cents (Catmull-Rom between samples, as offline).
                    let d = depth * (1.0 + (TAU * f * (start + i as u64) as f64 / rate).sin());
                    let last = hist.len() as f64 - 1.0;
                    let pos = (last - d).max(0.0);
                    let k0 = pos.floor() as i64;
                    let u = pos - k0 as f64;
                    let at = |j: i64| hist[j.clamp(0, last as i64) as usize];
                    let (p0, p1, p2, p3) = (at(k0 - 1), at(k0), at(k0 + 1), at(k0 + 2));
                    x[i] = p1 + 0.5 * u * (p2 - p0 + u * (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3 + u * (3.0 * (p1 - p2) + p3 - p0)));
                }
            }
        }
        if fx.warmth > 0.0 {
            self.warmth(fx.warmth, left, right);
        }
    }

    fn warmth(&mut self, w: f64, left: &mut [f64], right: &mut [f64]) {
        if w <= 0.0 {
            return;
        }
        let rate = self.rate as f64;
        let drive = 1.0 + 3.0 * w;
        let norm = 1.0 / drive.tanh();
        let fc = 20_000.0 - 13_000.0 * w;
        let a = 1.0 - (-2.0 * PI * fc.min(0.49 * rate) / rate).exp();
        let p = WARMTH_REFERENCE;
        for (ch, x) in [left, right].into_iter().enumerate() {
            let mut state = self.warm_lp[ch];
            for v in x.iter_mut() {
                let s = (drive * *v / p).tanh() * norm * p;
                state += a * (s - state);
                *v = state;
            }
            self.warm_lp[ch] = state;
        }
    }
}

/// The gain applied at absolute sample `frame` of stage `cur`, given its gain, the gain of the stage before (if any)
/// and of the stage after (if known): ramps over [`GAIN_RAMP_SECS`] — down at the end of a stage before a quieter
/// one, up at the start of a stage after a quieter one.
pub fn gain_at(frame: u64, cur_start: u64, cur_end: u64, g: f64, before: Option<f64>, after: Option<f64>, rate: u32) -> f64 {
    let ramp = (GAIN_RAMP_SECS * rate as f64).max(1.0);
    let smooth = |u: f64| {
        let u = u.clamp(0.0, 1.0);
        0.5 - 0.5 * (PI * u).cos()
    };
    let mut out = g;
    if let Some(b) = before {
        if b < g {
            let u = (frame - cur_start) as f64 / ramp;
            out = b + (g - b) * smooth(u);
        }
    }
    if let Some(a) = after {
        if a < g {
            let left = (cur_end - frame) as f64 / ramp;
            out = out.min(a + (g - a) * smooth(left));
        }
    }
    out
}

// ------------------------------------------------------------------------------------------------ the stream

/// How many rendered stages to keep behind the playing one (the world draws the stage before the present's).
const KEEP_BEHIND: u64 = 2;

/// A stage as the host reads it.
pub struct StageRecord {
    pub stage: Stage,
    /// Display lines (see [`Stage::display`]).
    pub display: Vec<String>,
    /// The voices present, in roster order.
    pub voices: Vec<String>,
}

struct Shared {
    composed: Vec<Arc<StageRecord>>,
    rendered: VecDeque<Arc<Rendered>>,
    rendered_count: u64,
    read_frame: u64,
    lookahead: u64,
    stop: bool,
    error: Option<String>,
    /// The song has ended (a book stream): every stage is rendered, and the audio stops at this frame.
    end_frame: Option<u64>,
}

/// An endless song: a composer and a renderer on a worker thread that keeps `lookahead` stages rendered ahead of the
/// stage being read.
pub struct Stream {
    shared: Arc<(Mutex<Shared>, Condvar)>,
    worker: Option<JoinHandle<()>>,
    pub bpm: f64,
    pub rate: u32,
    pub step: usize,
    pub reference_hz: f64,
    pub roster: Vec<String>,
    /// A song holding every roster voice (for the voices' kinds and settings).
    pub roster_song: Song,
    pub seed: u64,
    pub origin: String,
    composer_seed_text: String,
    /// The generator version composing this stream (compose::stream::GENERATOR_VERSION unless started at another).
    pub version: u32,
    /// What started it, so a seed file can start it again.
    started_by: Seed,
}

struct Work {
    composer: Composer,
    renderer: Renderer,
}

impl Work {
    fn compose_to(&mut self, shared: &(Mutex<Shared>, Condvar), index: u64) {
        let have = shared.0.lock().expect("stream lock").composed.len() as u64;
        if have > index {
            return;
        }
        self.composer.ensure(index);
        let index = index.min(self.composer.stages().len() as u64 - 1);
        let mut recs = Vec::new();
        for k in have..=index {
            let st = self.composer.stage(k).expect("ensured").clone();
            let before = if k > 0 { self.composer.stage(k - 1).map(|s| s.eq.clone()) } else { None };
            let display = st.display(before.as_ref());
            let voices = st.eq.voices.iter().map(|b| b.name.clone()).collect();
            recs.push(Arc::new(StageRecord { stage: st, display, voices }));
        }
        let mut s = shared.0.lock().expect("stream lock");
        s.composed.extend(recs);
        shared.1.notify_all();
    }

    fn render_next(&mut self, shared: &(Mutex<Shared>, Condvar)) -> Result<(), String> {
        let k = shared.0.lock().expect("stream lock").rendered_count;
        self.compose_to(shared, k + 1);
        let text = shared.0.lock().expect("stream lock").composed[k as usize].stage.text.clone();
        let song = Song::parse(&text).map_err(|e| format!("stage {} does not parse: {e}", k + 1))?;
        let r = self.renderer.render(&song).map_err(|e| format!("stage {}: {e}", k + 1))?;
        // The last stage of a song that has ended: the audio stops once its ring-out has decayed below -80 dBFS.
        let end = if self.composer.ended() && k + 1 == self.composer.stages().len() as u64 {
            let quiet = (1e-4 / r.gain.max(1e-9)) as f32;
            let last = (0..r.frames()).rev().find(|&i| r.audio[2 * i].abs() > quiet || r.audio[2 * i + 1].abs() > quiet).map_or(0, |i| i + 1);
            let tail = (self.renderer.rate as usize / 10).min(r.frames() - last);
            Some(r.start + (last + tail) as u64)
        } else {
            None
        };
        let mut s = shared.0.lock().expect("stream lock");
        if end.is_some() {
            s.end_frame = end;
        }
        s.rendered.push_back(Arc::new(r));
        s.rendered_count += 1;
        let playing = stage_of(s.read_frame, self.renderer.bpm, self.renderer.rate);
        while s.rendered.front().is_some_and(|f| f.index + KEEP_BEHIND < playing) {
            s.rendered.pop_front();
        }
        shared.1.notify_all();
        Ok(())
    }
}

impl Stream {
    /// Start a stream: stage 1 is composed and rendered before this returns, then a worker keeps `lookahead` (at
    /// least 1) stages ahead. `step` is the trace step in samples (0: one frame per millisecond).
    pub fn start(seed: Seed, lookahead: u64, step: usize) -> Result<Stream, String> {
        Stream::start_version(seed, crate::compose::stream::GENERATOR_VERSION, lookahead, step)
    }

    /// As [`Stream::start`], composed by generator `version` (a seed file's): the same seed and version replay the same
    /// song, every stage and every instrument.
    pub fn start_version(seed: Seed, version: u32, lookahead: u64, step: usize) -> Result<Stream, String> {
        let started_by = seed.clone();
        let seed_text = match &seed {
            Seed::Song { text, .. } => text.clone(),
            Seed::Grammar(_) | Seed::Book(_) | Seed::Species(_) | Seed::Named { .. } | Seed::Explore(_) => String::new(),
        };
        let composer = Composer::with_version(seed, version)?;
        let roster = composer.roster().to_vec();
        let roster_song = Song::parse(&composer.roster_song_text()).map_err(|e| format!("the stream's voices: {e}"))?;
        let first = Song::parse(&composer.stage(0).expect("stage 0").text).map_err(|e| e.to_string())?;
        let step = if step == 0 { (first.rate as usize / 1000).max(1) } else { step };
        let roster_drum = roster.iter().map(|n| roster_song.voices.iter().any(|v| &v.name == n && matches!(v.kind, VoiceKind::Drum(_)))).collect();
        let renderer = Renderer::new(&first, roster.clone(), roster_drum, step);
        let shared = Arc::new((
            Mutex::new(Shared {
                composed: Vec::new(),
                rendered: VecDeque::new(),
                rendered_count: 0,
                read_frame: 0,
                lookahead: lookahead.max(1),
                stop: false,
                error: None,
                end_frame: None,
            }),
            Condvar::new(),
        ));
        let (bpm, rate, reference_hz) = (first.bpm, first.rate, first.reference_hz);
        let seed_n = composer.seed();
        let origin = composer.origin.clone();
        let mut work = Work { composer, renderer };
        work.render_next(&shared)?;
        let sh = shared.clone();
        let worker = std::thread::Builder::new()
            .name("luminesce-stream".into())
            .spawn(move || loop {
                {
                    let (m, cv) = &*sh;
                    let mut s = m.lock().expect("stream lock");
                    loop {
                        if s.stop {
                            return;
                        }
                        let playing = stage_of(s.read_frame, work.renderer.bpm, work.renderer.rate);
                        if s.end_frame.is_none() && s.rendered_count < playing + 1 + s.lookahead {
                            break;
                        }
                        s = cv.wait(s).expect("stream lock");
                    }
                }
                if let Err(e) = work.render_next(&sh) {
                    let (m, cv) = &*sh;
                    let mut s = m.lock().expect("stream lock");
                    s.error = Some(e);
                    cv.notify_all();
                    return;
                }
            })
            .map_err(|e| format!("cannot start the stream's worker: {e}"))?;
        Ok(Stream {
            shared,
            worker: Some(worker),
            bpm,
            rate,
            step,
            reference_hz,
            roster,
            roster_song,
            seed: seed_n,
            origin,
            composer_seed_text: seed_text,
            version,
            started_by,
        })
    }

    /// This stream as a seed file ([`seed_file_text`]): opening it replays the whole stream exactly.
    pub fn seed_file(&self) -> Result<String, String> {
        seed_file_text(&self.started_by, self.version, &self.origin)
    }

    /// Write the stream's seed file to `path`.
    pub fn save_seed(&self, path: &std::path::Path) -> Result<String, String> {
        let text = self.seed_file()?;
        std::fs::write(path, &text).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
        Ok(text)
    }

    pub fn stage_start(&self, k: u64) -> u64 {
        stage_start(k, self.bpm, self.rate)
    }

    pub fn stage_of(&self, frame: u64) -> u64 {
        stage_of(frame, self.bpm, self.rate)
    }

    /// The stage record `k`, once composed (the playing stage's next is always composed).
    pub fn record(&self, k: u64) -> Option<Arc<StageRecord>> {
        self.shared.0.lock().ok()?.composed.get(k as usize).cloned()
    }

    pub fn composed(&self) -> u64 {
        self.shared.0.lock().map(|s| s.composed.len() as u64).unwrap_or(0)
    }

    pub fn rendered(&self) -> u64 {
        self.shared.0.lock().map(|s| s.rendered_count).unwrap_or(0)
    }

    pub fn frames_read(&self) -> u64 {
        self.shared.0.lock().map(|s| s.read_frame).unwrap_or(0)
    }

    /// When the song has ended (a book stream composes its own ending): its length in frames, the ring-out included.
    pub fn ended_frames(&self) -> Option<u64> {
        self.shared.0.lock().ok()?.end_frame
    }

    pub fn error(&self) -> Option<String> {
        self.shared.0.lock().ok()?.error.clone()
    }

    /// A rendered stage, if it is still kept. With `wait`, blocks until it is rendered (or the stream fails).
    pub fn rendered_stage(&self, k: u64, wait: bool) -> Result<Option<Arc<Rendered>>, String> {
        let (m, cv) = &*self.shared;
        let mut s = m.lock().map_err(|_| "the stream's lock is poisoned".to_string())?;
        loop {
            if let Some(r) = s.rendered.iter().find(|r| r.index == k) {
                return Ok(Some(r.clone()));
            }
            if let Some(e) = &s.error {
                return Err(e.clone());
            }
            if k < s.rendered_count || !wait || s.stop || s.end_frame.is_some() {
                return Ok(None);
            }
            s = cv.wait(s).map_err(|_| "the stream's lock is poisoned".to_string())?;
        }
    }

    /// Read the next `out.len() / 2` stereo frames (interleaved f32), with each stage's gain. Blocks while the stage
    /// it needs is still rendering. Returns the frames written: fewer than asked only at the end of a song that has
    /// ended ([`Stream::ended_frames`]; then 0 from there on).
    pub fn pull(&self, out: &mut [f32]) -> Result<usize, String> {
        let want = out.len() / 2;
        let mut done = 0;
        while done < want {
            let frame = self.frames_read();
            let k = self.stage_of(frame);
            // Wait for the stage (or the song's end) before deciding.
            self.rendered_stage(k, true)?;
            if let Some(end) = self.ended_frames() {
                if frame >= end {
                    break;
                }
            }
            let cur = self.rendered_stage(k, true)?.ok_or_else(|| format!("stage {} is no longer kept", k + 1))?;
            let before = if k > 0 { self.rendered_stage(k - 1, false)?.map(|r| r.gain) } else { None };
            let end = cur.start + cur.frames() as u64;
            // The next stage's gain matters only in this stage's last half second: wait for it only there.
            let ramp = (GAIN_RAMP_SECS * self.rate as f64).ceil() as u64;
            let tail = end - frame <= ramp;
            let next = self.rendered_stage(k + 1, tail)?;
            let until = if tail || next.is_some() { end } else { end - ramp };
            let until = self.ended_frames().map_or(until, |e| until.min(e));
            let take = ((until - frame) as usize).min(want - done);
            for j in 0..take {
                let f = frame + j as u64;
                let g = gain_at(f, cur.start, end, cur.gain, before, next.as_ref().map(|r| r.gain), self.rate) as f32;
                let at = (f - cur.start) as usize * 2;
                out[2 * (done + j)] = cur.audio[at] * g;
                out[2 * (done + j) + 1] = cur.audio[at + 1] * g;
            }
            done += take;
            let (m, cv) = &*self.shared;
            let mut s = m.lock().map_err(|_| "the stream's lock is poisoned".to_string())?;
            s.read_frame = frame + take as u64;
            cv.notify_all();
        }
        Ok(done)
    }

    /// The stages kept and rendered, from `first` to `last`, as one window: its first absolute sample (a multiple of
    /// the trace step) and, per roster voice, the frames and hits (hit samples relative to that start).
    pub fn window(&self, first: u64, last: u64) -> Result<(u64, Vec<VoiceTrace>, Vec<f32>), String> {
        if last < first {
            return Err("the window's last stage is before its first".into());
        }
        let mut stages = Vec::new();
        for k in first..=last {
            stages.push(self.rendered_stage(k, false)?.ok_or_else(|| format!("stage {} is not rendered or no longer kept", k + 1))?);
        }
        let step = self.step as u64;
        let s0 = stages[0].start;
        let w = s0 / step * step;
        let prev = if w < s0 && first > 0 { self.rendered_stage(first - 1, false)? } else { None };
        let mut traces: Vec<VoiceTrace> = Vec::new();
        for ri in 0..self.roster.len() {
            let mut t = match &stages[0].traces[ri] {
                VoiceTrace::Tonal(_) => {
                    let mut f = Vec::new();
                    if w < s0 {
                        let last = prev.as_ref().and_then(|p| match &p.traces[ri] {
                            VoiceTrace::Tonal(fr) => fr.last().copied(),
                            _ => None,
                        });
                        f.push(last.unwrap_or(TonalFrame { gate: 0.0, pitch: f64::NAN, amp: f64::NAN, level: 0.0 }));
                    }
                    VoiceTrace::Tonal(f)
                }
                VoiceTrace::Drum(_) => VoiceTrace::Drum(Vec::new()),
            };
            for st in &stages {
                match (&mut t, &st.traces[ri]) {
                    (VoiceTrace::Tonal(a), VoiceTrace::Tonal(b)) => a.extend_from_slice(b),
                    (VoiceTrace::Drum(a), VoiceTrace::Drum(b)) => a.extend(b.iter().map(|h| DrumHit { sample: h.sample - w, amp: h.amp })),
                    _ => {}
                }
            }
            traces.push(t);
        }
        let mut audio = vec![0.0f32; 2 * (s0 - w) as usize];
        for st in &stages {
            audio.extend(st.audio.iter().map(|x| x * st.gain as f32));
        }
        Ok((w, traces, audio))
    }

    /// Write stages `last + 1 - count ..= last` as one ordinary song file (see [`save_text`]).
    pub fn save(&self, last: u64, count: u32, path: &std::path::Path) -> Result<String, String> {
        let count = count.max(1) as u64;
        let first = (last + 1).saturating_sub(count);
        let mut stages = Vec::new();
        for k in first..=last {
            stages.push(self.record(k).ok_or_else(|| format!("stage {} has not been composed", k + 1))?.stage.clone());
        }
        let text = save_text(&stages, &self.roster, &self.origin, self.seed)?;
        std::fs::write(path, &text).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
        Ok(text)
    }

    /// The seed song's text (empty for a grammar seed).
    pub fn seed_text(&self) -> &str {
        &self.composer_seed_text
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        {
            let (m, cv) = &*self.shared;
            if let Ok(mut s) = m.lock() {
                s.stop = true;
            }
            cv.notify_all();
        }
        if let Some(w) = self.worker.take() {
            let _ = w.join();
        }
    }
}

// ------------------------------------------------------------------------------------------------ saving

/// Rename names in an expression: `t` becomes `time`, and every name in `map` its new name, except a function's own
/// parameters.
fn rename(expr: &str, map: &HashMap<String, String>, time: &str, params: &[String]) -> String {
    let toks = crate::compose::display::tokens(expr);
    let mut out = String::new();
    let mut at = 0;
    for t in &toks {
        if let crate::compose::display::Tok::Ident(name) = &t.tok {
            let new = if params.contains(name) {
                None
            } else if name == "t" {
                Some(time.to_string())
            } else {
                map.get(name).cloned()
            };
            if let Some(n) = new {
                out += &expr[at..t.start];
                // `4t` must stay a product: `4T_1` lexes as 4 times T_1.
                out += &n;
                at = t.end;
            }
        }
    }
    out += &expr[at..];
    out
}

/// Several stages as one ordinary song: stage i's definitions get the suffix `_i` (those identical in every stage and
/// free of `t` are shared and keep their names), its time is `T_i = t - 32 i`, and every voice's functions choose
/// stage i's by `Q = floor(t / 32)`. Voice settings (gain, envelope, timbre) are the last stage's that has the voice.
pub fn save_text(stages: &[Stage], roster: &[String], origin: &str, seed: u64) -> Result<String, String> {
    // What was heard: each stage's played text, whose voices wear the song's drawn instruments (compose/timbre.rs), not
    // the undressed equation the composer composes from. Saving the undressed one lost the instruments (2026-11-04).
    let played = stages
        .iter()
        .map(|s| {
            let eq = crate::compose::stream::parse_equation(&s.text).map_err(|e| format!("stage {}: {e}", s.index + 1))?;
            Ok(Stage { eq, ..s.clone() })
        })
        .collect::<Result<Vec<Stage>, String>>()?;
    let stages = &played[..];
    let count = stages.len();
    if count == 0 {
        return Err("nothing to save".into());
    }
    let last = &stages[count - 1];
    // Shared definitions: identical line in every stage, and nothing in them reads t or a definition that differs.
    let mut shared: Vec<String> = last
        .eq
        .defs
        .iter()
        .filter(|d| stages.iter().all(|s| s.eq.def(&d.name).map(|x| &x.line) == Some(&d.line)))
        .map(|d| d.name.clone())
        .collect();
    loop {
        let before = shared.len();
        let snapshot = shared.clone();
        shared.retain(|n| {
            let d = last.eq.def(n).expect("shared exists in the last stage");
            let params: Vec<String> = d.line.split_once('=').map(|(l, _)| l).unwrap_or("").split(['(', ')', ',']).skip(1).map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect();
            crate::compose::display::tokens(d.rhs()).iter().all(|t| match &t.tok {
                crate::compose::display::Tok::Ident(x) => {
                    if params.contains(x) {
                        true
                    } else if x == "t" {
                        false
                    } else {
                        last.eq.def(x).is_none() || snapshot.contains(x)
                    }
                }
                _ => true,
            })
        });
        if shared.len() == before {
            break;
        }
    }
    let taken: Vec<String> = stages.iter().flat_map(|s| s.eq.defs.iter().map(|d| d.name.clone())).collect();
    let fresh = |base: &str| -> String {
        if !taken.iter().any(|n| n == base) {
            return base.to_string();
        }
        (2..).map(|k| format!("{base}{k}")).find(|n| !taken.contains(n)).expect("a free name")
    };
    let q = fresh("Q");
    // Each stage's time is T_i, unless a stage-local definition is itself named T: renamed T_i, it would collide
    // (found 2026-11-05 on a lens stretch; tests/stream.rs `every_lens_stretch_saves`).
    let local_t = stages.iter().any(|st| st.eq.defs.iter().any(|d| d.name == "T" && !shared.contains(&d.name)));
    let tb = if local_t { fresh("T") } else { "T".to_string() };
    let mut text = format!(
        "# Saved from a Dreamride stream (from {origin}, seed {seed}): stages {} to {}, {} bars.\n#\n",
        stages[0].index + 1,
        last.index + 1,
        8 * count
    );
    for s in stages {
        text += &format!("# Stage {}: {}.\n", s.index + 1, s.summary);
    }
    text += &format!("#\n# Each stage's definitions carry its number (M_0, M_1, ...) and its own time {tb}_i = t - 32 i; {q} = floor(t / 32) picks the stage.\n\n");
    for st in &last.eq.settings {
        text += st;
        text += "\n";
    }
    text += &format!("beats {}\n\n", STAGE_BEATS as usize * count);
    let mut lines = vec![format!("{q} = floor(t / 32)")];
    for d in &last.eq.defs {
        if shared.contains(&d.name) {
            lines.push(d.line.clone());
        }
    }
    let mut maps = Vec::new();
    for (i, s) in stages.iter().enumerate() {
        let time = format!("{tb}_{i}");
        lines.push(format!("{time} = t - {}", 32 * i));
        let map: HashMap<String, String> =
            s.eq.defs.iter().filter(|d| !shared.contains(&d.name)).map(|d| (d.name.clone(), format!("{}_{i}", d.name))).collect();
        for d in &s.eq.defs {
            if shared.contains(&d.name) {
                continue;
            }
            let (lhs, rhs) = d.line.split_once('=').expect("a definition has '='");
            let params: Vec<String> = lhs.split(['(', ')', ',']).skip(1).map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect();
            let new_lhs = if params.is_empty() { format!("{}_{i}", d.name) } else { format!("{}_{i}({})", d.name, params.join(", ")) };
            lines.push(format!("{new_lhs} = {}", rename(rhs.trim(), &map, &time, &params)));
        }
        maps.push((map, time));
    }
    text += &lines.join("\n");
    text += "\n";
    for name in roster {
        let present: Vec<(usize, &crate::compose::stream::Block)> =
            stages.iter().enumerate().filter_map(|(i, s)| s.eq.voice(name).map(|b| (i, b))).collect();
        let Some(&(_, template)) = present.last() else { continue };
        let mut fields: Vec<(String, String)> = Vec::new();
        for (key, value) in &template.fields {
            let is_expr = matches!(key.as_str(), "pitch" | "gate" | "amp" | "trigger" | "cutoff");
            if !is_expr {
                fields.push((key.clone(), value.clone()));
                continue;
            }
            let mut e = String::new();
            for i in (0..count).rev() {
                let here = stages[i].eq.voice(name).and_then(|b| b.get(key)).map(|v| rename(v, &maps[i].0, &maps[i].1, &[]));
                let here = here.unwrap_or_else(|| if key == "amp" && stages[i].eq.voice(name).is_some() { "1".into() } else { "0".into() });
                e = if e.is_empty() { here } else { format!("if({q} == {i}, {here}, {e})") };
            }
            fields.push((key.clone(), e));
        }
        text += &format!("\n{} {name} {{\n", template.kind);
        for (k, v) in fields {
            text += &format!("  {k} = {v}\n");
        }
        text += "}\n";
    }
    Song::parse(&text).map_err(|e| format!("the saved song does not parse: {e}"))?;
    Ok(text)
}

// ------------------------------------------------------------------------------------------------ seed files

/// A seed file: what starts a stream and the generator version that composes it, so opening it replays the whole song
/// (every stage, every instrument), not one saved stretch. Plain text, one key a line, `#` comments:
///
/// ```text
/// # Dreamride seed: the automaton species, seed 152637
/// seed 152637
/// kind species
/// version 1
/// ```
///
/// `kind` is `species`, `book` (the lens), `explore`, `grammar`, or `named` with a `species NAME` line. A stream started
/// from a song's text has no seed file: its song is saved as a song.
pub fn seed_file_text(seed: &Seed, version: u32, origin: &str) -> Result<String, String> {
    let (kind, n, species) = match seed {
        Seed::Species(n) => ("species", *n, None),
        Seed::Book(n) => ("book", *n, None),
        Seed::Explore(n) => ("explore", *n, None),
        Seed::Grammar(n) => ("grammar", *n, None),
        Seed::Named { species, seed } => ("named", *seed, Some(species.as_str())),
        Seed::Song { .. } => return Err("a stream started from a song has no seed file; save its stages as a song".into()),
    };
    let mut t = format!("# Dreamride seed: {origin}. Opening this file replays the whole stream, at generator version {version}.\nseed {n}\nkind {kind}\n");
    if let Some(sp) = species {
        t += &format!("species {sp}\n");
    }
    t += &format!("version {version}\n");
    Ok(t)
}

/// Read a seed file: the seed that starts the stream and its generator version. Errors name the line.
pub fn read_seed_file(text: &str) -> Result<(Seed, u32), String> {
    let (mut seed, mut kind, mut version, mut species) = (None, None, None, None);
    for (i, raw) in text.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let (k, v) = line.split_once(char::is_whitespace).map(|(k, v)| (k, v.trim())).ok_or_else(|| format!("line {}: '{line}' has no value", i + 1))?;
        let num = |v: &str| v.parse::<u64>().map_err(|_| format!("line {}: '{v}' is not a whole number", i + 1));
        match k {
            "seed" => seed = Some(num(v)?),
            "kind" => kind = Some(v.to_string()),
            "version" => version = Some(num(v)? as u32),
            "species" => species = Some(v.to_string()),
            _ => return Err(format!("line {}: unknown key '{k}' (seed, kind, version, species)", i + 1)),
        }
    }
    let n = seed.ok_or("a seed file needs a 'seed' line")?;
    let version = version.ok_or("a seed file needs a 'version' line")?;
    let seed = match kind.as_deref().ok_or("a seed file needs a 'kind' line")? {
        "species" => Seed::Species(n),
        "book" => Seed::Book(n),
        "explore" => Seed::Explore(n),
        "grammar" => Seed::Grammar(n),
        "named" => Seed::Named { species: species.ok_or("kind named needs a 'species' line")?, seed: n },
        other => return Err(format!("unknown kind '{other}' (species, book, explore, grammar, named)")),
    };
    Ok((seed, version))
}
