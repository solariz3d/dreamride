//! Synthesis and mixing.
//!
//! Every voice is rendered sample by sample into a stereo f64 buffer: time in beats is
//! `t = i * bpm / (60 * rate)`. Tonal voices run a phase accumulator (so pitch glides are smooth)
//! through a linear attack/release envelope driven by the gate. Drums start a hit on each rising
//! edge of their trigger; hits are pure functions of their age and a seed, so they overlap freely
//! and render identically every time. Any NaN or infinity from an expression stops the render
//! with an error naming the voice, the quantity and the beat.
//!
//! Headroom: the mix is summed without limiting. If its peak exceeds [`CEILING`] (-1 dBFS), the
//! whole mix is scaled down to exactly the ceiling and the applied gain is reported in [`Mix`].
//! Nothing is ever clipped.

use crate::error::Error;
use crate::expr::At;
use crate::song::{Dash, Drum, DrumKind, Song, Tonal, Voice, VoiceKind, Wave};
use std::f64::consts::{FRAC_PI_4, TAU};

/// The normalisation ceiling: -1 dBFS.
pub const CEILING: f64 = 0.891_250_938_133_745_5; // 10^(-1/20)

/// A rendered stereo mix.
#[derive(Debug, Clone)]
pub struct Mix {
    pub rate: u32,
    pub left: Vec<f64>,
    pub right: Vec<f64>,
    /// Peak absolute sample before normalisation.
    pub raw_peak: f64,
    /// Gain applied by normalisation (1 when none was needed).
    pub gain: f64,
    /// Peak absolute sample of the final mix.
    pub peak: f64,
}

impl Mix {
    pub fn frames(&self) -> usize {
        self.left.len()
    }
}

/// Peak level in dBFS (negative infinity for silence).
pub fn dbfs(x: f64) -> f64 {
    20.0 * x.log10()
}

pub fn render(song: &Song) -> Result<Mix, Error> {
    render_with(song, None).map(|(mix, _)| mix)
}

/// What the renderer used, recorded while it rendered, so the world draws the song from the same
/// numbers it plays: see [`render_traced`].
#[derive(Debug, Clone)]
pub struct Trace {
    /// Samples between recorded frames of a tonal voice (frame `k` is sample `k * step`).
    pub step: usize,
    /// One entry per voice, in the song's order.
    pub voices: Vec<VoiceTrace>,
}

#[derive(Debug, Clone)]
pub enum VoiceTrace {
    /// Frames at samples 0, step, 2 step, ... while samples last.
    Tonal(Vec<TonalFrame>),
    /// Every hit, in order.
    Drum(Vec<DrumHit>),
}

/// A tonal voice's state at one sample, exactly as the renderer used it.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct TonalFrame {
    /// The gate expression's value (the voice sounds while it is > 0).
    pub gate: f64,
    /// Semitones above the reference, as evaluated for the oscillator; NaN where the renderer did
    /// not evaluate it (the gate is closed and the release has finished).
    pub pitch: f64,
    /// The `amp` expression's value (1 without one); NaN where the pitch is NaN.
    pub amp: f64,
    /// What the oscillator was multiplied by: envelope x amp x gain (0 when silent).
    pub level: f64,
}

/// A drum hit, exactly as the renderer started it.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct DrumHit {
    /// The sample of the onset.
    pub sample: u64,
    /// The `amp` expression's value at the onset (1 without one), before the voice's gain.
    pub amp: f64,
}

/// Render and record every voice's state: tonal voices every `step` samples (`step` >= 1), drums
/// at every hit. The mix is identical to [`render`]'s, bit for bit.
pub fn render_traced(song: &Song, step: usize) -> Result<(Mix, Trace), Error> {
    if step == 0 {
        return Err(Error::msg("the trace step must be at least 1 sample"));
    }
    render_with(song, Some(step)).map(|(mix, trace)| (mix, trace.expect("a step was given")))
}

/// One voice's output before the pan and the sends: the value it added at each sample, and
/// whether it added one there (a silent voice adds nothing, as it always has).
struct VoiceBuf {
    x: Vec<f64>,
    written: Vec<bool>,
}

impl VoiceBuf {
    #[inline]
    fn add(&mut self, i: usize, x: f64) {
        self.x[i] = x;
        self.written[i] = true;
    }
}

fn render_voice(song: &Song, index: usize, step: Option<usize>) -> Result<(VoiceBuf, Option<VoiceTrace>), Error> {
    let n = song.sample_count();
    let mut out = VoiceBuf { x: vec![0.0; n], written: vec![false; n] };
    let voice = &song.voices[index];
    let trace = match &voice.kind {
        VoiceKind::Tonal(tonal) => {
            let mut frames = step.map(|s| Vec::with_capacity(n / s + 1));
            render_tonal(song, voice, tonal, &mut out, step.zip(frames.as_mut()))?;
            frames.map(VoiceTrace::Tonal)
        }
        VoiceKind::Drum(drum) => {
            let mut hits = Vec::new();
            render_drum(song, voice, drum, voice_seed(index), &mut out, &mut hits)?;
            step.map(|_| VoiceTrace::Drum(hits))
        }
    };
    Ok((out, trace))
}

/// Voices are independent until they are summed, so they render on several threads. Each writes
/// its own buffer, and the buffers are summed in the song's voice order, sample by sample, with the
/// same operations a voice used to do writing straight into the mix. So the mix is bit-identical
/// to rendering the voices one after another, and the error reported is the first voice's.
fn render_with(song: &Song, step: Option<usize>) -> Result<(Mix, Option<Trace>), Error> {
    let n = song.sample_count();
    let mut left = vec![0.0; n];
    let mut right = vec![0.0; n];
    // The send buses exist only when some voice uses them, so songs without effects render as before.
    let mut reverb_bus = if song.voices.iter().any(|v| v.reverb > 0.0) { vec![0.0; n] } else { Vec::new() };
    let mut delay_bus = if song.voices.iter().any(|v| v.delay > 0.0) { vec![0.0; n] } else { Vec::new() };
    let mut traces = Vec::with_capacity(song.voices.len());
    // At most this many voice buffers are alive at once (9 bytes per sample each).
    let batch = std::thread::available_parallelism().map_or(1, |p| p.get()).clamp(1, 8);
    let indices: Vec<usize> = (0..song.voices.len()).collect();
    for group in indices.chunks(batch) {
        let results: Vec<Result<(VoiceBuf, Option<VoiceTrace>), Error>> = if group.len() == 1 {
            vec![render_voice(song, group[0], step)]
        } else {
            std::thread::scope(|scope| {
                let handles: Vec<_> = group
                    .iter()
                    .map(|&index| {
                        // Each thread has its own copy: definitions keep a result cache, and their
                        // values are identical with or without it.
                        let own = song.clone();
                        scope.spawn(move || render_voice(&own, index, step))
                    })
                    .collect();
                handles.into_iter().map(|h| h.join().expect("a voice thread panicked")).collect()
            })
        };
        for (&index, result) in group.iter().zip(results) {
            let (buf, trace) = result?;
            let voice = &song.voices[index];
            let (gl, gr) = pan_gains(voice.pan);
            for i in 0..n {
                if !buf.written[i] {
                    continue;
                }
                let x = buf.x[i];
                left[i] += x * gl;
                right[i] += x * gr;
                // Sends are taken before the pan, so a voice's echo and room do not follow it to one side.
                if voice.reverb > 0.0 {
                    reverb_bus[i] += x * voice.reverb;
                }
                if voice.delay > 0.0 {
                    delay_bus[i] += x * voice.delay;
                }
            }
            if let Some(t) = trace {
                traces.push(t);
            }
        }
    }
    crate::effects::apply(song, &mut left, &mut right, &reverb_bus, &delay_bus);

    let mut raw_peak = 0.0f64;
    for (i, &x) in left.iter().chain(&right).enumerate() {
        if !x.is_finite() {
            return Err(Error::msg(format!("the mix is not finite at sample {} ({x})", i % n.max(1))));
        }
        raw_peak = raw_peak.max(x.abs());
    }
    let gain = if raw_peak > CEILING { CEILING / raw_peak } else { 1.0 };
    if gain != 1.0 {
        for x in left.iter_mut().chain(right.iter_mut()) {
            *x *= gain;
        }
    }
    let peak = left.iter().chain(&right).fold(0.0f64, |a, &x| a.max(x.abs()));
    let trace = step.map(|step| Trace { step, voices: traces });
    Ok((Mix { rate: song.rate, left, right, raw_peak, gain, peak }, trace))
}

/// Equal-power pan: -1 hard left, 0 centre (both at 1/sqrt 2), 1 hard right.
pub(crate) fn pan_gains(pan: f64) -> (f64, f64) {
    let angle = (pan + 1.0) * FRAC_PI_4;
    let (l, r) = (angle.cos(), angle.sin());
    // cos(pi/2) is 6e-17, not 0: make hard pans exact.
    (if pan >= 1.0 { 0.0 } else { l }, if pan <= -1.0 { 0.0 } else { r })
}

#[inline]
fn beat_at(song: &Song, i: usize) -> f64 {
    i as f64 * song.bpm / (60.0 * song.rate as f64)
}

pub(crate) fn not_finite(voice: &Voice, what: &str, value: f64, t: f64) -> Error {
    Error::line(voice.line, format!("{}: {what} is {value} at beat {t:.6}", voice.label()))
}

fn render_tonal(
    song: &Song,
    voice: &Voice,
    tv: &Tonal,
    out: &mut VoiceBuf,
    mut trace: Option<(usize, &mut Vec<TonalFrame>)>,
) -> Result<(), Error> {
    let p = &song.program;
    let rate = song.rate as f64;
    let nyquist = rate / 2.0;
    let attack_step = 1.0 / (tv.attack_ms * 0.001 * rate);
    let decay_step = (1.0 - tv.sustain) / (tv.decay_ms * 0.001 * rate);
    let release_step = 1.0 / (tv.release_ms * 0.001 * rate);
    // The filter envelope jumps to 1 at each onset and falls by this factor per sample.
    let fenv_fall = (-1.0 / (tv.filter_decay_ms * 0.001 * rate)).exp();
    let k = std::f64::consts::SQRT_2 * (1.0 - tv.resonance);
    let drive_gain = 1.0 + tv.drive;
    let drive_norm = 1.0 / drive_gain.tanh();
    let mut env = 0.0f64;
    let mut attacking = false;
    let mut prev_gate = 0.0f64;
    let mut fenv = 0.0f64;
    let mut filter = Svf::default();
    let mut phase = 0.0f64;
    let mut phase2 = 0.0f64;
    // Detune: two oscillators, half the interval either side of the pitch, averaged.
    let detune = if tv.detune_cents > 0.0 { Some((tv.detune_cents / 2400.0).exp2()) } else { None };
    // The roll-off corner in cycles per sample, the unit of `dt`.
    let roll = tv.rolloff_hz / rate;
    let n = out.x.len();
    // The gate is evaluated once for the whole voice first, so each note knows where it ends:
    // `note_left` needs the time until the gate closes, which lies ahead of the sample.
    let mut gates = Vec::with_capacity(n);
    for i in 0..n {
        let t = beat_at(song, i);
        let g = tv.gate.eval(p, t);
        if !g.is_finite() {
            return Err(not_finite(voice, "gate", g, t));
        }
        gates.push(g);
    }
    // Before the first note, `note_age` counts from the song's start and `note_left` is negative.
    let (mut note_start, mut note_end) = (0usize, 0usize);
    for i in 0..n {
        let t = beat_at(song, i);
        let g = gates[i];
        let onset = g > 0.0 && prev_gate <= 0.0;
        if onset {
            note_start = i;
            note_end = (i..n).find(|&j| gates[j] <= 0.0).unwrap_or(n);
        }
        let at = At { t, age: (i - note_start) as f64 / rate, left: (note_end as f64 - i as f64) / rate };
        prev_gate = g;
        let record = matches!(&trace, Some((step, _)) if i % *step == 0);
        if g > 0.0 {
            if onset {
                attacking = true;
                fenv = 1.0;
            }
            // Attack (linear, full scale over `attack`), then decay (linear, to the sustain level
            // over `decay`), then hold. With sustain 1 this is the step-1 envelope exactly.
            if attacking {
                env = (env + attack_step).min(1.0);
                if env >= 1.0 {
                    attacking = false;
                }
            } else {
                env = (env - decay_step).max(tv.sustain);
            }
        } else {
            attacking = false;
            env = (env - release_step).max(0.0);
            if env == 0.0 {
                if let (true, Some((_, frames))) = (record, trace.as_mut()) {
                    frames.push(TonalFrame { gate: g, pitch: f64::NAN, amp: f64::NAN, level: 0.0 });
                }
                phase = 0.0; // the next note starts from phase 0
                phase2 = 0.0;
                filter = Svf::default(); // and from a rested filter
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
                format!(
                    "{}: pitch {semis} semitones ({freq:.1} Hz) at beat {t:.6} is above the Nyquist limit ({nyquist} Hz)",
                    voice.label()
                ),
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
                let s = tone(tv, roll, phase, dt);
                phase += dt;
                s
            }
            Some(r) => {
                let (d1, d2) = (dt / r, dt * r);
                let s = 0.5 * (tone(tv, roll, phase, d1) + tone(tv, roll, phase2, d2));
                phase += d1;
                phase2 += d2;
                phase2 -= phase2.floor();
                s
            }
        };
        phase -= phase.floor();
        if let Some(c) = &tv.cutoff {
            let hz = c.eval_at(p, &at);
            if !hz.is_finite() {
                return Err(not_finite(voice, "cutoff", hz, t));
            }
            let hz = hz * (tv.filter_env * fenv).exp2();
            // Limited to [10 Hz, 0.45 rate]: below is silence, above is an open filter, and tan()
            // must stay away from pi/2.
            s = filter.lowpass(s, (hz.max(10.0).min(0.45 * rate) / rate * std::f64::consts::PI).tan(), k);
        }
        fenv *= fenv_fall;
        if tv.drive > 0.0 {
            s = (drive_gain * s).tanh() * drive_norm;
        }
        if let (true, Some((_, frames))) = (record, trace.as_mut()) {
            frames.push(TonalFrame { gate: g, pitch: semis, amp, level: env * amp * voice.gain });
        }
        out.add(i, s * env * amp * voice.gain);
    }
    Ok(())
}

/// A topology-preserving-transform state-variable filter, low-pass output (V. Zavalishin, "The Art
/// of VA Filter Design", ch. 4; the trapezoidal form of A. Simper, Cytomic, "Linear Trap Integrated
/// SVF", 2013). `g = tan(pi fc / fs)` is prewarped, so the response at the cutoff is exact, and the
/// structure stays stable when `g` changes every sample. `k = 1/Q`; `k = sqrt 2` is Butterworth.
#[derive(Default, Clone)]
pub(crate) struct Svf {
    ic1: f64,
    ic2: f64,
}

impl Svf {
    #[inline]
    pub(crate) fn lowpass(&mut self, x: f64, g: f64, k: f64) -> f64 {
        let a1 = 1.0 / (1.0 + g * (g + k));
        let a2 = g * a1;
        let a3 = g * a2;
        let v3 = x - self.ic2;
        let v1 = a1 * self.ic1 + a2 * v3;
        let v2 = self.ic2 + a2 * self.ic1 + a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        v2
    }
}

/// PolyBLEP residual: smooths the step discontinuity of a saw or square at phase 0.
#[inline]
fn poly_blep(t: f64, dt: f64) -> f64 {
    if t < dt {
        let x = t / dt;
        x + x - x * x - 1.0
    } else if t > 1.0 - dt {
        let x = (t - 1.0) / dt;
        x * x + x + x + 1.0
    } else {
        0.0
    }
}

/// One sample of a tonal voice's oscillator. Additive voices, and sines with a roll-off, are sums
/// of sine partials; the other waves are the step-1 oscillators, unchanged.
#[inline]
pub(crate) fn tone(tv: &Tonal, roll: f64, phase: f64, dt: f64) -> f64 {
    match tv.wave {
        Wave::Additive => additive(&tv.partials, roll, phase, dt),
        Wave::Sine if roll > 0.0 => additive(&[1.0], roll, phase, dt),
        wave => oscillator(wave, phase, dt),
    }
}

/// A sum of sine partials: harmonic `k` (from 1) at `k dt` cycles per sample, with amplitude
/// `levels[k - 1]`, scaled by `min(1, roll / (k dt))` when `roll` > 0 (the Desmos gain cap
/// `min(10, 660 / f)` as a roll-off of -6 dB per octave above the corner). A partial at or above
/// half the sample rate (`k dt >= 0.5`) is left out, so nothing aliases. Each partial's phase is
/// `k phase` reduced to [0, 1) before the sine, so high partials keep full precision.
#[inline]
pub fn additive(levels: &[f64], roll: f64, phase: f64, dt: f64) -> f64 {
    let mut s = 0.0;
    for (j, &a) in levels.iter().enumerate() {
        let k = (j + 1) as f64;
        let kd = k * dt;
        if kd >= 0.5 {
            break;
        }
        let g = if roll > 0.0 { (roll / kd).min(1.0) } else { 1.0 };
        s += a * g * (TAU * (k * phase).fract()).sin();
    }
    s
}

/// One sample of a waveform at `phase` in [0, 1), with phase increment `dt` per sample.
#[inline]
fn oscillator(wave: Wave, phase: f64, dt: f64) -> f64 {
    match wave {
        Wave::Additive => unreachable!("additive voices are rendered by tone()"),
        Wave::Sine => (TAU * phase).sin(),
        // Starts at 0 and rises, like the sine.
        Wave::Triangle => 1.0 - 4.0 * ((phase + 0.25).fract() - 0.5).abs(),
        Wave::Saw => 2.0 * phase - 1.0 - poly_blep(phase, dt),
        Wave::Square => {
            let naive = if phase < 0.5 { 1.0 } else { -1.0 };
            naive + poly_blep(phase, dt) - poly_blep((phase + 0.5).fract(), dt)
        }
    }
}

// ---------------------------------------------------------------- drums

impl DrumKind {
    /// Length of one hit in seconds.
    pub(crate) fn length(self) -> f64 {
        match self {
            DrumKind::Kick => 0.6,
            DrumKind::Snare => 0.3,
            DrumKind::Hat => 0.12,
            DrumKind::Dash => 0.0, // a dash carries its own length
        }
    }
}

/// SplitMix64: a small, well-mixed, counter-friendly PRNG step.
#[inline]
pub(crate) fn splitmix64(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Deterministic white noise in [-1, 1): sample `n` of the stream `seed`.
#[inline]
fn noise(seed: u64, n: u64) -> f64 {
    let bits = splitmix64(seed ^ n.wrapping_mul(0xD1B5_4A32_D192_ED03)) >> 11;
    bits as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
}

fn voice_seed(index: usize) -> u64 {
    splitmix64(0x4C55_4D49_4E45_5343 ^ index as u64) // "LUMINESC"
}

/// The frequency of a dash `tau` seconds after its onset: `to + (from - to) e^(-tau / dash)`.
pub fn dash_hz(d: &Dash, tau: f64) -> f64 {
    d.to_hz + (d.from_hz - d.to_hz) * (-tau / (d.dash_ms * 0.001)).exp()
}

/// Cycles a dash has turned `tau` seconds after its onset: the exact integral of [`dash_hz`],
/// `to tau + (from - to) k (1 - e^(-tau / k))` with `k` the dash time.
pub fn dash_cycles(d: &Dash, tau: f64) -> f64 {
    let k = d.dash_ms * 0.001;
    d.to_hz * tau + (d.from_hz - d.to_hz) * k * (1.0 - (-tau / k).exp())
}

/// One sample of a dash, before the end fade: its partials at whole multiples of the swept phase,
/// each scaled by the roll-off at its present frequency. A partial that would reach half the sample
/// rate anywhere in the sweep is left out entirely (so it never switches on mid-hit). The level rises
/// over 1 ms, holds for `hold`, then falls by 1/e per `decay`.
fn dash_sample(d: &Dash, tau: f64, rate: f64) -> f64 {
    let f = dash_hz(d, tau);
    let c = dash_cycles(d, tau);
    let top = d.from_hz.max(d.to_hz);
    let mut s = 0.0;
    for (j, &a) in d.partials.iter().enumerate() {
        let k = (j + 1) as f64;
        if k * top >= rate / 2.0 {
            break;
        }
        let g = if d.rolloff_hz > 0.0 { (d.rolloff_hz / (k * f)).min(1.0) } else { 1.0 };
        s += a * g * (TAU * (k * c).fract()).sin();
    }
    let attack = (tau / 0.001).min(1.0);
    let hold = d.hold_ms * 0.001;
    let level = if tau < hold { 1.0 } else { (-(tau - hold) / (d.decay_ms * 0.001)).exp() };
    s * attack * level
}

/// One sample of a drum hit, `age` samples after its onset.
pub(crate) fn drum_sample(kind: DrumKind, dash: Option<&Dash>, age: usize, rate: f64, seed: u64) -> f64 {
    let tau = age as f64 / rate;
    let len = match dash {
        Some(d) => d.length_ms * 0.001,
        None => kind.length(),
    };
    // A short linear fade over the last 10% so a hit ends at exactly zero, without a click.
    let fade = ((len - tau) / (0.1 * len)).clamp(0.0, 1.0);
    let n = age as u64;
    let s = match kind {
        DrumKind::Kick => {
            // A sine swept from 160 Hz down to 48 Hz; the phase is the closed-form integral of
            // f(tau) = f1 + (f0 - f1) e^(-tau/k).
            let (f0, f1, k) = (160.0, 48.0, 0.035);
            let phase = TAU * (f1 * tau + (f0 - f1) * k * (1.0 - (-tau / k).exp()));
            phase.sin() * (-tau / 0.2).exp()
        }
        DrumKind::Snare => {
            let attack = (tau / 0.001).min(1.0);
            let rattle = noise(seed, n) * 0.6 * (-tau / 0.07).exp() * attack;
            let body = (TAU * 185.0 * tau).sin() * 0.5 * (-tau / 0.05).exp();
            rattle + body
        }
        DrumKind::Dash => dash_sample(dash.expect("a dash drum carries its Dash"), tau, rate),
        DrumKind::Hat => {
            // First difference of white noise: a simple high-pass.
            let attack = (tau / 0.0005).min(1.0);
            let hp = (noise(seed, n) - noise(seed, n.wrapping_sub(1))) * 0.5;
            hp * 0.8 * (-tau / 0.025).exp() * attack
        }
    };
    s * fade
}

struct Hit {
    start: usize,
    amp: f64,
    seed: u64,
}

fn render_drum(
    song: &Song,
    voice: &Voice,
    drum: &Drum,
    seed: u64,
    out: &mut VoiceBuf,
    onsets: &mut Vec<DrumHit>,
) -> Result<(), Error> {
    let p = &song.program;
    let rate = song.rate as f64;
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
        None => drum.kind.length(),
    };
    let len = (secs * rate).ceil() as usize;
    let mut hits: Vec<Hit> = Vec::new();
    let mut count: u64 = 0;
    let mut prev = 0.0f64; // before the song starts the trigger counts as 0, so t = 0 can start a hit
    for i in 0..out.x.len() {
        let t = beat_at(song, i);
        let g = drum.trigger.eval(p, t);
        if !g.is_finite() {
            return Err(not_finite(voice, "trigger", g, t));
        }
        if g > 0.0 && prev <= 0.0 {
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
            hits.push(Hit { start: i, amp: amp * voice.gain, seed: splitmix64(seed ^ count) });
            onsets.push(DrumHit { sample: i as u64, amp });
            count += 1;
        }
        prev = g;
        hits.retain(|h| i - h.start < len);
        if hits.is_empty() {
            continue;
        }
        let s: f64 = hits.iter().map(|h| h.amp * drum_sample(drum.kind, drum.dash.as_ref(), i - h.start, rate, h.seed)).sum();
        out.add(i, s);
    }
    Ok(())
}
