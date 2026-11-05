//! The song measured against itself (docs/what_makes_music_bad.md, registered 2026-11-05 before any bad mark). The
//! critic judges a stage on its own; these measure a stretch of a song against the song so far: how surprising its lead
//! is given what came before, how much it repeats the song's own bars, how many pitch classes it uses and how clearly
//! it has a centre, how stepwise its lead moves, and how rough it is (the critic's own roughness).
//!
//! Every measure reads the song through the critic's grid (`critic::read_tracks`), so it sees exactly what the critic
//! sees: the functions evaluated, not audio.

use super::critic::{chord_roughness, harmonic, read_tracks, Track, GRID};
use crate::song::Song;

/// What a stretch measures (NaN where a measure has nothing to read, e.g. no lead).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Measures {
    /// Mean information content of the stretch's lead notes (bits), given every lead note before it in the song.
    pub self_surprise: f64,
    /// Median roughness every 16th (the critic's `chord_roughness`).
    pub roughness: f64,
    /// Share of the stretch's bars whose lead and bass equal a bar heard earlier in the song.
    pub repetition: f64,
    /// Pitch classes holding at least 5% of the sounding weight, median over two-stage windows.
    pub macroharmony: f64,
    /// Krumhansl-Schmuckler clarity, best over second-best key correlation, median over two-stage windows.
    pub centricity: f64,
    /// Share of lead moves of at most 2 semitones.
    pub conjunct: f64,
}

/// The measures' names, in [`Measures::values`] order, with the direction the registration predicts for a bad stretch
/// (+1: bad is higher; -1: bad is lower).
pub const NAMES: [(&str, f64); 6] = [
    ("self_surprise", 1.0),
    ("roughness", 1.0),
    ("repetition", -1.0),
    ("macroharmony", 1.0),
    ("centricity", -1.0),
    ("conjunct", -1.0),
];

impl Measures {
    pub fn values(&self) -> [f64; 6] {
        [self.self_surprise, self.roughness, self.repetition, self.macroharmony, self.centricity, self.conjunct]
    }
}

/// Beats in a bar (4/4) and in a stage (eight bars).
const BAR: f64 = 4.0;
const STAGE: f64 = 32.0;
/// Grid points in a 16th.
const SIXTEENTH: usize = GRID / 4;

fn median(mut xs: Vec<f64>) -> f64 {
    xs.retain(|x| x.is_finite());
    if xs.is_empty() {
        return f64::NAN;
    }
    xs.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    xs[xs.len() / 2]
}

/// Measure the part of `song` from beat `from` to its end, with everything before `from` as its history.
pub fn measure(song: &Song, from: f64) -> Measures {
    let tracks = read_tracks(song);
    let lead = tracks.iter().find(|t| t.name == "lead" && t.tonal);
    let bass = tracks.iter().find(|t| t.name == "bass" && t.tonal);
    let notes: Vec<(f64, f64)> = lead.map(|t| t.events.iter().filter(|e| e.pitch.is_finite()).map(|e| (e.t, e.pitch)).collect()).unwrap_or_default();
    let start = (from * GRID as f64).round() as usize;
    let n = tracks.first().map(|t| t.open.len()).unwrap_or(0);
    let ws = windows(&tracks, from, song.beats);
    Measures {
        self_surprise: self_surprise(&notes, from),
        roughness: roughness(song, &tracks, start, n),
        repetition: repetition(lead, bass, from, song.beats),
        macroharmony: median(ws.iter().map(macroharmony).collect()),
        centricity: median(ws.iter().map(centricity).collect()),
        conjunct: conjunct(&notes, from),
    }
}

// ------------------------------------------------------------------------------------------------ self-surprise

/// Intervals are clamped to this many semitones each way: 49 symbols.
const SPAN: i64 = 24;
const ALPHABET: f64 = (2 * SPAN + 1) as f64;
const ORDERS: usize = 3;

/// A minimal variable-order model of intervals (a small IDyOM): orders 0 to 3, each estimate (c + 1/49) / (C + 1),
/// blended with weights equal to each order's context count (order 0 weighted 1), updated after every note.
#[derive(Default)]
pub struct IntervalModel {
    counts: std::collections::HashMap<Vec<i64>, std::collections::HashMap<i64, f64>>,
    totals: std::collections::HashMap<Vec<i64>, f64>,
    history: Vec<i64>,
}

impl IntervalModel {
    /// The probability of `x` after what the model has seen.
    pub fn p(&self, x: i64) -> f64 {
        let (mut num, mut den) = (0.0, 0.0);
        for k in 0..=ORDERS.min(self.history.len()) {
            let ctx = self.history[self.history.len() - k..].to_vec();
            let total = self.totals.get(&ctx).copied().unwrap_or(0.0);
            let c = self.counts.get(&ctx).and_then(|m| m.get(&x)).copied().unwrap_or(0.0);
            let est = (c + 1.0 / ALPHABET) / (total + 1.0);
            let w = if k == 0 { 1.0 } else { total };
            num += w * est;
            den += w;
        }
        num / den
    }

    /// Learn `x` in every context the history gives.
    pub fn learn(&mut self, x: i64) {
        for k in 0..=ORDERS.min(self.history.len()) {
            let ctx = self.history[self.history.len() - k..].to_vec();
            *self.counts.entry(ctx.clone()).or_default().entry(x).or_insert(0.0) += 1.0;
            *self.totals.entry(ctx).or_insert(0.0) += 1.0;
        }
        self.history.push(x);
    }
}

fn interval(a: f64, b: f64) -> i64 {
    ((b - a).round() as i64).clamp(-SPAN, SPAN)
}

fn self_surprise(notes: &[(f64, f64)], from: f64) -> f64 {
    let mut m = IntervalModel::default();
    let mut ics = Vec::new();
    for w in notes.windows(2) {
        let x = interval(w[0].1, w[1].1);
        if w[1].0 >= from {
            ics.push(-m.p(x).log2());
        }
        m.learn(x);
    }
    if ics.is_empty() {
        f64::NAN
    } else {
        ics.iter().sum::<f64>() / ics.len() as f64
    }
}

// ------------------------------------------------------------------------------------------------ roughness

fn roughness(song: &Song, tracks: &[Track], start: usize, n: usize) -> f64 {
    let mut rs = Vec::new();
    let mut i = start + SIXTEENTH / 2;
    while i < n {
        let mut tones = Vec::new();
        for t in tracks.iter().filter(|t| t.tonal && !t.perc) {
            if t.open[i] && t.pitch[i].is_finite() && t.amp[i] > 0.0 {
                let hz = song.reference_hz * 2f64.powf(t.pitch[i] / 12.0);
                tones.push((harmonic(hz, &t.partials_db, t.rolloff, 1.0), t.gain * t.amp[i]));
            }
        }
        if let Some(r) = chord_roughness(&tones) {
            rs.push(r);
        }
        i += SIXTEENTH;
    }
    median(rs)
}

// ------------------------------------------------------------------------------------------------ repetition

/// A bar's signature: one voice's rounded pitch at each 16th, i64::MIN for a rest.
fn bar_line(t: Option<&Track>, bar: usize) -> Vec<i64> {
    let per = (BAR as usize) * 4;
    (0..per)
        .map(|s| {
            let i = (bar * per + s) * SIXTEENTH + SIXTEENTH / 2;
            match t {
                Some(t) if i < t.open.len() && t.open[i] && t.pitch[i].is_finite() => t.pitch[i].round() as i64,
                _ => i64::MIN,
            }
        })
        .collect()
}

fn repetition(lead: Option<&Track>, bass: Option<&Track>, from: f64, beats: f64) -> f64 {
    let bars = (beats / BAR).floor() as usize;
    let first = (from / BAR).round() as usize;
    if first >= bars {
        return f64::NAN;
    }
    let sig: Vec<(Vec<i64>, Vec<i64>)> = (0..bars).map(|b| (bar_line(lead, b), bar_line(bass, b))).collect();
    // A bar where neither the lead nor the bass sounds has nothing to repeat: it is left out, not counted as a repeat of
    // the silence before it (found on the first bad mark, 2026-11-05: an opening of pads and hat scored 0.94).
    let silent = |s: &(Vec<i64>, Vec<i64>)| s.0.iter().chain(&s.1).all(|p| *p == i64::MIN);
    let sounding: Vec<usize> = (first..bars).filter(|&b| !silent(&sig[b])).collect();
    if sounding.is_empty() {
        return f64::NAN;
    }
    let repeated = sounding.iter().filter(|&&b| sig[..b].contains(&sig[b])).count();
    repeated as f64 / sounding.len() as f64
}

// ------------------------------------------------------------------------------------------------ pitch classes

/// Pitch-class weights (relative to the reference pitch, A = 0) of the tonal, non-percussive voices over grid points
/// [a, b), each open 16th weighted by gain times amp.
pub fn pitch_class_weights(tracks: &[Track], a: usize, b: usize) -> [f64; 12] {
    let mut w = [0.0; 12];
    for t in tracks.iter().filter(|t| t.tonal && !t.perc) {
        let mut i = a + SIXTEENTH / 2;
        while i < b.min(t.open.len()) {
            if t.open[i] && t.pitch[i].is_finite() {
                w[(t.pitch[i].round() as i64).rem_euclid(12) as usize] += t.gain * t.amp[i];
            }
            i += SIXTEENTH;
        }
    }
    w
}

/// Two-stage windows inside the stretch, stepping one stage (one window if the stretch is shorter).
fn windows(tracks: &[Track], from: f64, beats: f64) -> Vec<[f64; 12]> {
    let g = GRID as f64;
    let mut out = Vec::new();
    let mut a = from;
    while a + 2.0 * STAGE <= beats + 1e-9 {
        out.push(pitch_class_weights(tracks, (a * g) as usize, ((a + 2.0 * STAGE) * g) as usize));
        a += STAGE;
    }
    if out.is_empty() {
        out.push(pitch_class_weights(tracks, (from * g) as usize, (beats * g) as usize));
    }
    out
}

/// Pitch classes holding at least 5% of the weight.
pub fn macroharmony(w: &[f64; 12]) -> f64 {
    let total: f64 = w.iter().sum();
    if total <= 0.0 {
        return f64::NAN;
    }
    w.iter().filter(|x| **x / total >= 0.05).count() as f64
}

/// Krumhansl & Kessler's major and minor key profiles, tonic first.
const KK_MAJOR: [f64; 12] = [6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88];
const KK_MINOR: [f64; 12] = [6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17];

fn pearson(a: &[f64; 12], b: &[f64; 12]) -> f64 {
    let (ma, mb) = (a.iter().sum::<f64>() / 12.0, b.iter().sum::<f64>() / 12.0);
    let (mut sab, mut saa, mut sbb) = (0.0, 0.0, 0.0);
    for i in 0..12 {
        sab += (a[i] - ma) * (b[i] - mb);
        saa += (a[i] - ma).powi(2);
        sbb += (b[i] - mb).powi(2);
    }
    if saa <= 0.0 || sbb <= 0.0 {
        0.0
    } else {
        sab / (saa * sbb).sqrt()
    }
}

/// The 24 key correlations, best first: (correlation, tonic pitch class, major).
pub fn key_correlations(w: &[f64; 12]) -> Vec<(f64, usize, bool)> {
    let mut out = Vec::new();
    for tonic in 0..12 {
        for (major, prof) in [(true, &KK_MAJOR), (false, &KK_MINOR)] {
            let mut rot = [0.0; 12];
            for i in 0..12 {
                rot[(tonic + i) % 12] = prof[i];
            }
            out.push((pearson(w, &rot), tonic, major));
        }
    }
    out.sort_by(|a, b| b.0.partial_cmp(&a.0).expect("finite"));
    out
}

/// Best over second-best key correlation (NaN when the best is not positive: no key to be clear about).
pub fn centricity(w: &[f64; 12]) -> f64 {
    if w.iter().sum::<f64>() <= 0.0 {
        return f64::NAN;
    }
    let k = key_correlations(w);
    if k[0].0 <= 0.0 || k[1].0 <= 0.0 {
        return f64::NAN;
    }
    k[0].0 / k[1].0
}

// ------------------------------------------------------------------------------------------------ conjunct motion

fn conjunct(notes: &[(f64, f64)], from: f64) -> f64 {
    let moves: Vec<f64> = notes.windows(2).filter(|w| w[1].0 >= from).map(|w| (w[1].1 - w[0].1).abs()).collect();
    if moves.is_empty() {
        f64::NAN
    } else {
        moves.iter().filter(|d| d.round() <= 2.0).count() as f64 / moves.len() as f64
    }
}

// ------------------------------------------------------------------------------------------------ separation

/// The probability that a random `bad` value is further in `direction` (+1 higher, -1 lower) than a random `good` one,
/// ties counting half; NaN values are left out, and NaN is returned when either side is empty.
pub fn auc(bad: &[f64], good: &[f64], direction: f64) -> f64 {
    let b: Vec<f64> = bad.iter().copied().filter(|x| x.is_finite()).collect();
    let g: Vec<f64> = good.iter().copied().filter(|x| x.is_finite()).collect();
    if b.is_empty() || g.is_empty() {
        return f64::NAN;
    }
    let mut s = 0.0;
    for x in &b {
        for y in &g {
            let d = direction * (x - y);
            s += if d > 0.0 { 1.0 } else if d == 0.0 { 0.5 } else { 0.0 };
        }
    }
    s / (b.len() * g.len()) as f64
}
