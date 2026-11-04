//! The critic: scores any parsed song from measurable properties, each with a cited rationale (docs/composer.md has
//! the full table). It reads the song's functions on a grid of 48 points a beat (cheap: no audio); only the sound
//! component needs a render, and the search renders only its finalists.
//!
//! Every component is a measurement turned into a score in [0, 1] by a stated target band. The bands come from the
//! cited literature, from canonical patterns (the syncopation band), from intervals computed by the roughness model
//! itself (the roughness anchors), or from the reference recordings (the sound bands); **none was fitted to the three
//! hand-written graph songs**, since the critic's first test is whether it ranks those above random grammar samples.

use super::sound::SoundStats;
use crate::song::{Song, VoiceKind, Wave};

/// Grid points per beat.
pub const GRID: usize = 48;

// ------------------------------------------------------------------------------------------------ roughness

/// Sethares' parametrisation of the Plomp-Levelt dissonance curve for two partials (Sethares, *Tuning, Timbre,
/// Spectrum, Scale*; the constants as read from his `disscurve2.cpp`, re-implemented here):
/// `s = 0.24 / (0.0207 f_min + 18.96)`, `d = min(a1, a2) (5 e^(-3.51 s df) - 5 e^(-5.75 s df))`.
pub fn pair_roughness(f1: f64, a1: f64, f2: f64, a2: f64) -> f64 {
    let fmin = f1.min(f2);
    let df = (f1 - f2).abs();
    let s = 0.24 / (0.0207 * fmin + 18.96);
    a1.min(a2) * (5.0 * (-3.51 * s * df).exp() - 5.0 * (-5.75 * s * df).exp())
}

/// A tone: its partials as (Hz, amplitude).
pub type Spectrum = Vec<(f64, f64)>;

/// The dissonance of one spectrum: the sum over every pair of its partials (Sethares' `dissmeasure`).
pub fn spectrum_roughness(s: &[(f64, f64)]) -> f64 {
    let mut r = 0.0;
    for i in 0..s.len() {
        for j in i + 1..s.len() {
            r += pair_roughness(s[i].0, s[i].1, s[j].0, s[j].1);
        }
    }
    r
}

/// The roughness two tones add by sounding together: the dissonance of their combined spectrum (partials at the same
/// frequency merged, their amplitudes added) less each tone's own, divided by the two tones' total amplitude so it does
/// not grow with loudness. A unison of identical tones adds nothing. (A first version summed only the pairs across the
/// two tones; that counts each tone's own neighbouring harmonics again, so a unison scored rougher than an octave. The
/// unison test caught it.)
pub fn tone_roughness(a: &[(f64, f64)], b: &[(f64, f64)]) -> f64 {
    let mut merged: Vec<(f64, f64)> = a.to_vec();
    for &(f, amp) in b {
        match merged.iter_mut().find(|p| (p.0 - f).abs() <= 1e-9 * f) {
            Some(p) => p.1 += amp,
            None => merged.push((f, amp)),
        }
    }
    let added = spectrum_roughness(&merged) - spectrum_roughness(a) - spectrum_roughness(b);
    let total: f64 = a.iter().chain(b).map(|p| p.1).sum();
    if total > 0.0 {
        added.max(0.0) / total
    } else {
        0.0
    }
}

/// The roughness of a sonority: the mean over its pairs of tones, weighted by the quieter tone's gain.
pub fn chord_roughness(tones: &[(Spectrum, f64)]) -> Option<f64> {
    let mut num = 0.0;
    let mut den = 0.0;
    for i in 0..tones.len() {
        for j in i + 1..tones.len() {
            let w = tones[i].1.min(tones[j].1);
            num += w * tone_roughness(&tones[i].0, &tones[j].0);
            den += w;
        }
    }
    if den > 0.0 {
        Some(num / den)
    } else {
        None
    }
}

/// A harmonic spectrum at `hz` from partial levels in dB, with the Desmos cap `min(1, 660 / f)` if `rolloff` > 0.
pub fn harmonic(hz: f64, levels_db: &[f64], rolloff: f64, gain: f64) -> Spectrum {
    levels_db
        .iter()
        .enumerate()
        .map(|(k, db)| {
            let f = hz * (k + 1) as f64;
            let cap = if rolloff > 0.0 { (rolloff / f).min(1.0) } else { 1.0 };
            (f, gain * 10f64.powf(db / 20.0) * cap)
        })
        .filter(|(f, a)| *f < 8000.0 && *a > 1e-5)
        .collect()
}

/// The pad timbre of the graph songs, used for the roughness anchors.
const ANCHOR_PARTIALS: [f64; 8] = [0.0, -12.0, -5.0, -11.0, -9.0, -12.0, -11.0, -13.0];

fn anchor(pitches: &[f64]) -> f64 {
    let tones: Vec<(Spectrum, f64)> =
        pitches.iter().map(|p| (harmonic(440.0 * 2f64.powf(p / 12.0), &ANCHOR_PARTIALS, 660.0, 1.0), 1.0)).collect();
    chord_roughness(&tones).unwrap()
}

/// The roughness anchors: a major triad at C4 (score 1) and a chromatic cluster C4 C#4 D4 (score 0), both in the
/// graph pad timbre, computed by the same model.
pub fn roughness_anchors() -> (f64, f64) {
    (anchor(&[-9.0, -5.0, -2.0]), anchor(&[-9.0, -8.0, -7.0]))
}

// ------------------------------------------------------------------------------------------------ syncopation

/// Metrical weights of the sixteen sixteenths of a 4/4 bar, Longuet-Higgins and Lee (1984) as tabulated by Fitch and
/// Rosenfeld (2007): 0 on the downbeat, -1 on beat 3, -2 on beats 2 and 4, -3 on the eighths, -4 on the sixteenths.
pub const LHL_WEIGHTS: [i32; 16] = [0, -4, -3, -4, -2, -4, -3, -4, -1, -4, -3, -4, -2, -4, -3, -4];

/// The Longuet-Higgins and Lee syncopation of one onset stream, `slots` sixteenths long and read as a loop: for each
/// onset, if a silent position before the next onset is metrically stronger, the syncopation is the difference of
/// their weights. Returns the total for onsets in each bar.
pub fn lhl_per_bar(slots: &[bool]) -> Vec<f64> {
    let n = slots.len();
    let bars = n / 16;
    let mut out = vec![0.0; bars];
    let onsets: Vec<usize> = (0..n).filter(|&i| slots[i]).collect();
    if onsets.is_empty() {
        return out;
    }
    for (oi, &i) in onsets.iter().enumerate() {
        let next = if oi + 1 < onsets.len() { onsets[oi + 1] } else { onsets[0] + n };
        let wn = LHL_WEIGHTS[i % 16];
        let rest_max = (i + 1..next).map(|k| LHL_WEIGHTS[k % n % 16]).max();
        if let Some(wr) = rest_max {
            if wr > wn {
                out[i / 16] += (wr - wn) as f64;
            }
        }
    }
    out
}

// ------------------------------------------------------------------------------------------------ scoring helpers

/// 0 below `a`, rising to 1 at `b`, 1 to `c`, falling to 0 at `d`.
pub fn trapezoid(x: f64, a: f64, b: f64, c: f64, d: f64) -> f64 {
    if !x.is_finite() {
        return 0.0;
    }
    if x < b {
        if b <= a {
            return if x >= a { 1.0 } else { 0.0 };
        }
        ((x - a) / (b - a)).clamp(0.0, 1.0)
    } else if x <= c {
        1.0
    } else {
        if d <= c {
            return 0.0;
        }
        ((d - x) / (d - c)).clamp(0.0, 1.0)
    }
}

/// 1 inside [lo, hi], falling as a Gaussian of the log-ratio distance outside it (width `w` in natural-log units).
pub fn log_band(x: f64, lo: f64, hi: f64, w: f64) -> f64 {
    if !(x > 0.0) {
        return 0.0;
    }
    let d = if x < lo { (lo / x).ln() } else if x > hi { (x / hi).ln() } else { 0.0 };
    (-(d / w).powi(2)).exp()
}

// ------------------------------------------------------------------------------------------------ reading the song

/// One melodic event: an onset, or a new settled pitch inside a held note (a glide that lands).
#[derive(Debug, Clone, Copy)]
pub struct Event {
    pub t: f64,
    pub pitch: f64,
}

/// What the critic reads from one voice on the grid.
#[derive(Debug, Clone)]
pub struct Track {
    pub name: String,
    pub tonal: bool,
    /// Percussive: a drum, or a tonal voice named like one (the hand songs' clap is a falling tonal line).
    pub perc: bool,
    pub gain: f64,
    pub partials_db: Vec<f64>,
    pub rolloff: f64,
    /// Gate open (tonal) at each grid point.
    pub open: Vec<bool>,
    /// Pitch at each grid point (tonal), NaN when not finite.
    pub pitch: Vec<f64>,
    /// Amplitude multiplier at each grid point (tonal `amp`, 1 if none).
    pub amp: Vec<f64>,
    pub events: Vec<Event>,
}

fn wave_partials(w: Wave, partials: &[f64]) -> Vec<f64> {
    let db = |a: f64| 20.0 * a.log10();
    match w {
        Wave::Additive => partials.to_vec(),
        Wave::Sine => vec![0.0],
        Wave::Triangle => (1..=8).map(|k| if k % 2 == 1 { db(1.0 / (k * k) as f64) } else { -120.0 }).collect(),
        Wave::Saw => (1..=8).map(|k| db(1.0 / k as f64)).collect(),
        Wave::Square => (1..=8).map(|k| if k % 2 == 1 { db(1.0 / k as f64) } else { -120.0 }).collect(),
    }
}

const PERC_NAMES: [&str; 6] = ["kick", "snare", "clap", "hat", "perc", "rim"];

/// Read every voice of a song on the grid.
pub fn read_tracks(song: &Song) -> Vec<Track> {
    let n = (song.beats * GRID as f64).round() as usize;
    let at = |i: usize| i as f64 / GRID as f64 + 1e-6;
    let mut out = Vec::new();
    for v in &song.voices {
        let named_perc = PERC_NAMES.iter().any(|p| v.name.to_lowercase().contains(p));
        match &v.kind {
            VoiceKind::Tonal(tn) => {
                let mut open = Vec::with_capacity(n);
                let mut pitch = Vec::with_capacity(n);
                let mut amp = Vec::with_capacity(n);
                for i in 0..n {
                    let g = tn.gate.eval(&song.program, at(i));
                    let o = g > 0.0;
                    open.push(o);
                    let p = if o { tn.pitch.eval(&song.program, at(i)) } else { f64::NAN };
                    pitch.push(if p.is_finite() { p } else { f64::NAN });
                    let a = match (&tn.amp, o) {
                        (Some(e), true) => e.eval(&song.program, at(i)),
                        _ => 1.0,
                    };
                    amp.push(if a.is_finite() { a.max(0.0) } else { 0.0 });
                }
                // Events: a gate rise, or a new pitch that holds within half a semitone for a sixteenth.
                let mut events = Vec::new();
                let hold = GRID / 16;
                let mut cur = f64::NAN;
                for i in 0..n {
                    if !open[i] || pitch[i].is_nan() {
                        cur = f64::NAN;
                        continue;
                    }
                    let rise = i == 0 || !open[i - 1];
                    let r = pitch[i].round();
                    if rise {
                        cur = r;
                        events.push(Event { t: i as f64 / GRID as f64, pitch: pitch[i] });
                    } else if r != cur {
                        let stable = (i..(i + hold).min(n)).all(|k| open[k] && (pitch[k] - r).abs() <= 0.5);
                        if stable && i + hold <= n {
                            cur = r;
                            events.push(Event { t: i as f64 / GRID as f64, pitch: pitch[i] });
                        }
                    }
                }
                out.push(Track {
                    name: v.name.clone(),
                    tonal: true,
                    perc: named_perc,
                    gain: v.gain,
                    partials_db: wave_partials(tn.wave, &tn.partials),
                    rolloff: tn.rolloff_hz,
                    open,
                    pitch,
                    amp,
                    events,
                });
            }
            VoiceKind::Drum(d) => {
                let mut events = Vec::new();
                let mut prev = d.trigger.eval(&song.program, at(0) - 1.0 / GRID as f64);
                for i in 0..n {
                    let v = d.trigger.eval(&song.program, at(i));
                    if prev <= 0.0 && v > 0.0 {
                        let a = d.amp.as_ref().map(|e| e.eval(&song.program, at(i))).unwrap_or(1.0);
                        if a > 0.0 {
                            events.push(Event { t: i as f64 / GRID as f64, pitch: f64::NAN });
                        }
                    }
                    prev = v;
                }
                out.push(Track {
                    name: v.name.clone(),
                    tonal: false,
                    perc: true,
                    gain: v.gain,
                    partials_db: Vec::new(),
                    rolloff: 0.0,
                    open: vec![false; n],
                    pitch: vec![f64::NAN; n],
                    amp: vec![1.0; n],
                    events,
                });
            }
        }
    }
    out
}

fn median(xs: &mut [f64]) -> f64 {
    if xs.is_empty() {
        return f64::NAN;
    }
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    xs[xs.len() / 2]
}

fn track_median_pitch(t: &Track) -> f64 {
    let mut ps: Vec<f64> = t.events.iter().map(|e| e.pitch).filter(|p| p.is_finite()).collect();
    median(&mut ps)
}

/// The lead: the voice named "lead", or else the melodic voice with the most events above A3.
fn find_lead(tracks: &[Track]) -> Option<usize> {
    if let Some(i) = tracks.iter().position(|t| t.name == "lead" && t.tonal) {
        return Some(i);
    }
    tracks
        .iter()
        .enumerate()
        .filter(|(_, t)| t.tonal && !t.perc && track_median_pitch(t) >= -12.0)
        .max_by_key(|(_, t)| t.events.len())
        .map(|(i, _)| i)
}

/// The bass: the voice named "bass", or else the lowest melodic voice.
fn find_bass(tracks: &[Track]) -> Option<usize> {
    if let Some(i) = tracks.iter().position(|t| t.name == "bass" && t.tonal) {
        return Some(i);
    }
    tracks
        .iter()
        .enumerate()
        .filter(|(_, t)| t.tonal && !t.perc && !t.events.is_empty())
        .min_by(|a, b| track_median_pitch(a.1).partial_cmp(&track_median_pitch(b.1)).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
}

// ------------------------------------------------------------------------------------------------ the components

/// One scored component.
#[derive(Debug, Clone)]
pub struct Component {
    pub name: &'static str,
    /// The measurement, in its own unit.
    pub value: f64,
    pub score: f64,
    pub weight: f64,
}

/// The weights, in the order the components are reported. They sum to 1. Sound is 0.15 and needs a render.
pub const WEIGHTS: [(&str, f64); 13] = [
    ("melody range", 0.05),
    ("melody steps", 0.06),
    ("melody contour", 0.06),
    ("melody repetition", 0.08),
    ("rhythm syncopation", 0.12),
    ("rhythm density", 0.08),
    ("harmony roughness", 0.12),
    ("harmony roots", 0.08),
    ("structure contrast", 0.07),
    ("structure return", 0.06),
    ("structure energy", 0.07),
    ("sound bands", 0.09),
    ("sound peaks", 0.06),
];

fn weight(name: &str) -> f64 {
    WEIGHTS.iter().find(|w| w.0 == name).map(|w| w.1).unwrap_or(0.0)
}

/// A song's scores.
#[derive(Debug, Clone)]
pub struct Report {
    pub components: Vec<Component>,
}

impl Report {
    /// The score from the functions alone (no render), renormalised over the weights it has: 0..1.
    pub fn function_score(&self) -> f64 {
        let comps: Vec<&Component> = self.components.iter().filter(|c| !c.name.starts_with("sound")).collect();
        let w: f64 = comps.iter().map(|c| c.weight).sum();
        comps.iter().map(|c| c.weight * c.score).sum::<f64>() / w
    }

    /// The full score, once the sound components are in: 0..1. Without them it equals `function_score`.
    pub fn total(&self) -> f64 {
        let w: f64 = self.components.iter().map(|c| c.weight).sum();
        self.components.iter().map(|c| c.weight * c.score).sum::<f64>() / w
    }

    pub fn has_sound(&self) -> bool {
        self.components.iter().any(|c| c.name.starts_with("sound"))
    }

    pub fn get(&self, name: &str) -> Option<&Component> {
        self.components.iter().find(|c| c.name == name)
    }

    /// Add the sound components from a render's measurements.
    pub fn add_sound(&mut self, s: &SoundStats) {
        self.components.retain(|c| !c.name.starts_with("sound"));
        // The two reference recordings (docs/research/graph_sound.md): 41/37/20/1.6/0.2 % and 60/26/13/0.9/0.3 %.
        let lo = [41.0, 26.0, 13.0, 0.9, 0.2];
        let hi = [60.0, 37.0, 20.0, 1.6, 0.3];
        let band_score = (0..5).map(|b| log_band(s.bands[b], lo[b], hi[b], 0.5)).sum::<f64>() / 5.0;
        self.components.push(Component {
            name: "sound bands",
            value: s.bands[0],
            score: band_score,
            weight: weight("sound bands"),
        });
        // The references count 108 and 143 peaks per frame: that interval, with a log-distance falloff outside it.
        self.components.push(Component {
            name: "sound peaks",
            value: s.peaks,
            score: log_band(s.peaks, 108.0, 143.0, 0.35),
            weight: weight("sound peaks"),
        });
    }
}

fn comp(name: &'static str, value: f64, score: f64) -> Component {
    Component { name, value, score: if score.is_finite() { score.clamp(0.0, 1.0) } else { 0.0 }, weight: weight(name) }
}

/// The lead's pitch at each sixteenth of each bar (rounded), or None for a rest, with whether a note starts there.
fn bar_slots(t: &Track, bars: usize) -> Vec<Vec<(Option<i64>, bool)>> {
    let step = GRID / 4;
    (0..bars)
        .map(|b| {
            (0..16)
                .map(|s| {
                    let i = (b * 16 + s) * step;
                    let p = if i < t.open.len() && t.open[i] && t.pitch[i].is_finite() { Some(t.pitch[i].round() as i64) } else { None };
                    let onset = t.events.iter().any(|e| ((e.t * 4.0).round() as usize) == b * 16 + s);
                    (p, onset)
                })
                .collect()
        })
        .collect()
}

fn slot_similarity(a: &[(Option<i64>, bool)], b: &[(Option<i64>, bool)]) -> f64 {
    a.iter().zip(b).filter(|(x, y)| x == y).count() as f64 / a.len().max(1) as f64
}

/// Score a parsed song from its functions (no render). Add the sound with `Report::add_sound`.
pub fn score(song: &Song) -> Report {
    let tracks = read_tracks(song);
    score_tracks(song, &tracks)
}

pub fn score_tracks(song: &Song, tracks: &[Track]) -> Report {
    let bars = (song.beats / 4.0).floor().max(1.0) as usize;
    let mut comps = Vec::new();

    // ---- melody (the lead's events)
    let lead = find_lead(tracks);
    let lead_ev: Vec<Event> = lead.map(|i| tracks[i].events.clone()).unwrap_or_default();
    let ps: Vec<f64> = lead_ev.iter().map(|e| e.pitch.round()).collect();
    // Range: 5th to 95th percentile, semitones. Popular melodies span about an octave to a 12th; a hook about a 10th
    // (docs/research/genres.md, melody); below a fifth is a drone, beyond two octaves is no longer one voice.
    let range = if ps.len() >= 2 {
        let mut s = ps.clone();
        s.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let last = (s.len() - 1) as f64;
        s[(last * 0.95).round() as usize] - s[(last * 0.05).round() as usize]
    } else {
        0.0
    };
    comps.push(comp("melody range", range, trapezoid(range, 4.0, 9.0, 17.0, 26.0)));
    // Steps and leaps: intervals between successive events less than a bar apart. Small intervals dominate melodies
    // across cultures (pitch proximity: Vos and Troost 1989; Huron 2006 ch. 5).
    let mut ints = Vec::new();
    for w in lead_ev.windows(2) {
        if w[1].t - w[0].t <= 4.0 {
            ints.push(w[1].pitch.round() - w[0].pitch.round());
        }
    }
    let moves: Vec<f64> = ints.iter().cloned().filter(|d| *d != 0.0).collect();
    let steps = moves.iter().filter(|d| d.abs() <= 2.0).count() as f64;
    let step_ratio = if moves.is_empty() { 0.0 } else { steps / moves.len() as f64 };
    let unisons = if ints.is_empty() { 1.0 } else { (ints.len() - moves.len()) as f64 / ints.len() as f64 };
    // Too many repeated notes is a monotone: the step score is scaled down when over half the intervals are unisons.
    let step_score = trapezoid(step_ratio, 0.25, 0.5, 0.9, 1.01) * trapezoid(unisons, -1.0, 0.0, 0.5, 0.85);
    comps.push(comp("melody steps", step_ratio, step_score));
    // Contour: a large leap (5+ semitones) is usually followed by a change of direction (von Hippel and Huron 2000,
    // "Why do skips precede reversals?"), and a melody's rises and falls roughly balance (Huron 1996, the melodic arch).
    let mut leaps = 0.0;
    let mut reversals = 0.0;
    for w in moves.windows(2) {
        if w[0].abs() >= 5.0 {
            leaps += 1.0;
            if w[1].signum() != w[0].signum() {
                reversals += 1.0;
            }
        }
    }
    let rev = if leaps > 0.0 { reversals / leaps } else { 1.0 };
    let up = moves.iter().filter(|d| **d > 0.0).count() as f64 / moves.len().max(1) as f64;
    let contour = 0.5 * trapezoid(rev, 0.2, 0.55, 1.0, 1.01) + 0.5 * trapezoid(up, 0.25, 0.4, 0.6, 0.75);
    comps.push(comp("melody contour", rev, if moves.is_empty() { 0.0 } else { contour }));
    // Repetition against novelty: for each bar of the lead after its first, its best match with any earlier bar
    // (sixteenth slots, pitch and onset). Repeat more than feels safe, vary less than feels necessary (Margulis 2014,
    // *On Repeat*; genres.md 3.4): a middle band, neither a one-bar loop nor no return at all.
    let rep = match lead {
        Some(i) => {
            let slots = bar_slots(&tracks[i], bars);
            let active: Vec<usize> = (0..bars).filter(|b| slots[*b].iter().any(|s| s.1)).collect();
            if active.len() < 2 {
                f64::NAN
            } else {
                let mut sum = 0.0;
                for (k, &b) in active.iter().enumerate().skip(1) {
                    let best = active[..k].iter().map(|&a| slot_similarity(&slots[a], &slots[b])).fold(0.0, f64::max);
                    sum += best;
                }
                sum / (active.len() - 1) as f64
            }
        }
        None => f64::NAN,
    };
    comps.push(comp("melody repetition", rep, trapezoid(rep, 0.2, 0.45, 0.8, 0.98)));

    // ---- rhythm
    // Syncopation: Longuet-Higgins and Lee's index per bar on the drum and bass onset streams, summed over streams
    // (Witek et al. 2014 sum their instruments too). Medium syncopation is where groove and pleasure peak (Witek et al.
    // 2014, the inverted U). Their index has instrument weights we do not have, so their 0-81 scale does not transfer;
    // the band here is set by canonical patterns (docs/composer.md): a straight four-on-the-floor scores 0, a son clave
    // 4, every offbeat sixteenth 15.
    let bass = find_bass(tracks);
    let n16 = bars * 16;
    let mut sync = vec![0.0; bars];
    let mut any = vec![false; bars];
    for (i, t) in tracks.iter().enumerate() {
        if !(t.perc || Some(i) == bass) {
            continue;
        }
        let mut slots = vec![false; n16];
        for e in &t.events {
            let k = (e.t * 4.0).round() as usize;
            if k < n16 {
                slots[k] = true;
            }
        }
        for (b, v) in lhl_per_bar(&slots).iter().enumerate() {
            sync[b] += v;
        }
        for k in 0..n16 {
            if slots[k] {
                any[k / 16] = true;
            }
        }
    }
    let active: Vec<f64> = (0..bars).filter(|b| any[*b]).map(|b| sync[b]).collect();
    let sync_mean = if active.is_empty() { 0.0 } else { active.iter().sum::<f64>() / active.len() as f64 };
    comps.push(comp("rhythm syncopation", sync_mean, trapezoid(sync_mean, 0.5, 2.5, 9.0, 18.0)));
    // Density: events a beat over all voices. Fewer than about 2 is a sketch; beyond about 14 the parts blur
    // (convention: a sixteenth hat alone is 4).
    let total_events: usize = tracks.iter().map(|t| t.events.len()).sum();
    let density = total_events as f64 / song.beats;
    comps.push(comp("rhythm density", density, trapezoid(density, 1.0, 3.0, 11.0, 18.0)));

    // ---- harmony
    // Roughness twice a beat over the sounding melodic voices, scored between a major triad (1) and a cluster (0).
    let (lo, hi) = roughness_anchors();
    let mut rs = Vec::new();
    let n = tracks.first().map(|t| t.open.len()).unwrap_or(0);
    let mut i = GRID / 4;
    while i < n {
        let mut tones = Vec::new();
        for t in tracks.iter().filter(|t| t.tonal && !t.perc) {
            if t.open[i] && t.pitch[i].is_finite() && t.amp[i] > 0.0 {
                let hz = song.reference_hz * 2f64.powf(t.pitch[i] / 12.0);
                let g = t.gain * t.amp[i];
                tones.push((harmonic(hz, &t.partials_db, t.rolloff, 1.0), g));
            }
        }
        if let Some(r) = chord_roughness(&tones) {
            rs.push(r);
        }
        i += GRID / 2;
    }
    let rough = if rs.is_empty() { f64::NAN } else { rs.iter().sum::<f64>() / rs.len() as f64 };
    let rough_score = if rs.is_empty() {
        0.0
    } else {
        rs.iter().map(|r| 1.0 - ((r - lo) / (hi - lo)).clamp(0.0, 1.0)).sum::<f64>() / rs.len() as f64
    };
    comps.push(comp("harmony roughness", rough, rough_score));
    // Root motion: the bass's pitch class at each bar's start. Root motion by fourth or fifth is the most common in
    // tonal and popular harmony, then by step, then by third; the tritone the least (de Clercq and Temperley 2011, rock;
    // Rohrmeier and Cross 2008, Bach chorales; both from memory, not re-read for this). No change in 16 bars is a drone.
    let roots_score = match bass {
        Some(bi) => {
            let t = &tracks[bi];
            let pcs: Vec<Option<i64>> = (0..bars)
                .map(|b| {
                    t.events
                        .iter()
                        .find(|e| e.t >= b as f64 * 4.0 && e.t < b as f64 * 4.0 + 4.0)
                        .map(|e| (e.pitch.round() as i64).rem_euclid(12))
                })
                .collect();
            let mut ws = Vec::new();
            for w in pcs.windows(2) {
                if let (Some(a), Some(b)) = (w[0], w[1]) {
                    if a != b {
                        ws.push(match (b - a).rem_euclid(12) {
                            5 => 1.0,
                            7 => 0.85,
                            2 | 10 => 0.75,
                            3 | 4 | 8 | 9 => 0.65,
                            1 | 11 => 0.5,
                            _ => 0.25,
                        });
                    }
                }
            }
            let mean = if ws.is_empty() { 0.4 } else { ws.iter().sum::<f64>() / ws.len() as f64 };
            // Fewer than two changes in the song: a drone, worth at most 0.4.
            if ws.len() < 2 {
                mean.min(0.4)
            } else {
                mean
            }
        }
        None => 0.0,
    };
    comps.push(comp("harmony roots", roots_score, roots_score));

    // ---- structure (sections of 16 beats)
    let secs = ((song.beats / 16.0).floor() as usize).max(1);
    let sec_len = n / secs;
    let mut act: Vec<Vec<f64>> = vec![Vec::new(); secs];
    let mut dens = vec![0.0; secs];
    let mut energy = vec![0.0; secs];
    let mut lead_mean = vec![f64::NAN; secs];
    let mut root_hist = vec![[0.0f64; 12]; secs];
    for s in 0..secs {
        let (a, b) = (s * sec_len, (s + 1) * sec_len);
        let (ta, tb) = (a as f64 / GRID as f64, b as f64 / GRID as f64);
        for (k, t) in tracks.iter().enumerate() {
            let ev = t.events.iter().filter(|e| e.t >= ta && e.t < tb).count() as f64;
            dens[s] += ev / (tb - ta);
            let a_v = if t.tonal && !t.perc {
                t.open[a..b].iter().filter(|o| **o).count() as f64 / (b - a) as f64
            } else {
                (ev / (tb - ta) / 4.0).min(1.0)
            };
            act[s].push(a_v);
            energy[s] += t.gain * a_v;
            if Some(k) == lead {
                let p: Vec<f64> = t.events.iter().filter(|e| e.t >= ta && e.t < tb).map(|e| e.pitch).collect();
                if !p.is_empty() {
                    lead_mean[s] = p.iter().sum::<f64>() / p.len() as f64;
                }
            }
            if Some(k) == bass {
                for e in t.events.iter().filter(|e| e.t >= ta && e.t < tb) {
                    root_hist[s][(e.pitch.round() as i64).rem_euclid(12) as usize] += 1.0;
                }
            }
        }
    }
    // Contrast between neighbouring sections in more than one dimension: which layers play, the bass's pitch content,
    // the lead's register and the density (genres.md 3.3, rule 10, convention).
    let mut contrast = Vec::new();
    for s in 1..secs {
        let nv = act[s].len().max(1) as f64;
        let d_act = act[s].iter().zip(&act[s - 1]).map(|(x, y)| (x - y).abs()).sum::<f64>() / nv;
        let hist = |h: &[f64; 12]| {
            let t: f64 = h.iter().sum::<f64>().max(1e-9);
            h.map(|x| x / t)
        };
        let (h1, h0) = (hist(&root_hist[s]), hist(&root_hist[s - 1]));
        let d_root = h1.iter().zip(&h0).map(|(x, y)| (x - y).abs()).sum::<f64>() / 2.0;
        let d_lead = if lead_mean[s].is_finite() && lead_mean[s - 1].is_finite() {
            ((lead_mean[s] - lead_mean[s - 1]).abs() / 4.0).min(1.0)
        } else if lead_mean[s].is_finite() != lead_mean[s - 1].is_finite() {
            1.0
        } else {
            0.0
        };
        let d_den = (dens[s] - dens[s - 1]).abs() / dens[s].max(dens[s - 1]).max(1e-9);
        let d = 0.4 * d_act.min(1.0) / 0.25 + 0.2 * d_root + 0.2 * d_lead + 0.2 * d_den;
        contrast.push(d.min(1.0));
    }
    let contrast_v = if contrast.is_empty() { 0.0 } else { contrast.iter().sum::<f64>() / contrast.len() as f64 };
    comps.push(comp("structure contrast", contrast_v, trapezoid(contrast_v, 0.05, 0.3, 1.0, 1.01)));
    // Return: something comes back. The best bar-for-bar match (lead and bass slots) between two sections at least two
    // apart (the order lists' return to the first cycle, prior_art.md 5; the A of AABA).
    let ret = {
        let mut best: f64 = 0.0;
        let slot_tracks: Vec<Vec<Vec<(Option<i64>, bool)>>> =
            [lead, bass].iter().flatten().map(|&i| bar_slots(&tracks[i], bars)).collect();
        let bps = 4;
        for a in 0..secs {
            for b in a + 2..secs {
                let mut sim = 0.0;
                let mut cnt = 0.0;
                for st in &slot_tracks {
                    for k in 0..bps {
                        let (x, y) = (a * bps + k, b * bps + k);
                        if y < st.len() {
                            sim += slot_similarity(&st[x], &st[y]);
                            cnt += 1.0;
                        }
                    }
                }
                if cnt > 0.0 {
                    best = best.max(sim / cnt);
                }
            }
        }
        best
    };
    comps.push(comp("structure return", ret, trapezoid(ret, 0.15, 0.55, 1.0, 1.01)));
    // Energy: a build (the first section below the peak) and a real swing between the quietest and loudest sections.
    // Build-up and drop were the most pleasurable parts for dancers (Solberg and Jensenius 2017); contrast needs a
    // difference (genres.md 3.3).
    let emax = energy.iter().cloned().fold(0.0, f64::max);
    let emin = energy.iter().cloned().fold(f64::INFINITY, f64::min);
    let swing = if emax > 0.0 { (emax - emin) / emax } else { 0.0 };
    let build = if emax > 0.0 && energy[0] <= 0.9 * emax { 1.0 } else { 0.0 };
    let e_score = 0.5 * build + 0.5 * trapezoid(swing, 0.05, 0.15, 0.6, 0.9);
    comps.push(comp("structure energy", swing, e_score));

    Report { components: comps }
}
