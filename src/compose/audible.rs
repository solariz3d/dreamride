//! How audible a stage change is, read from the functions alone (no render).
//!
//! Listening to stream mode v1 (2026-11-03) found the equation did not change enough every eight bars, and that its
//! subtle changes stopped working as it grew more complex. Most v1 edits were a melody phase moved by a
//! quarter, one chord digit, or one rhythm bit, and they were hard to hear. This module gives that a number, so the
//! composer can refuse a change nobody would notice.
//!
//! A stage is sampled 16 times a beat over its 32 beats. For every voice (the passing `fill` roll and a seam's `rise`
//! are left out) it records the onsets (where the gate or trigger rises above 0) and, for tonal voices, the notes: the
//! pitch at each onset, and (since the move book, 2026-11-03) each new pitch that settles for a sixteenth inside a held
//! note, so a legato line that glides from note to note is heard as a melody and not as one long note. Two
//! stages then differ by six components, each scaled so that one plainly audible event is about 1:
//!
//! | component | measure | weight |
//! |---|---|---|
//! | layers | layers that entered or left (the pads count as one layer, the drone pair as one) | 1.0 each |
//! | pitch | total variation between the stages' pitch-class distributions (each voice weighted equally), 0..1 | 2.0 |
//! | rhythm | the largest Jaccard distance between one voice's onset sixteenths in the two stages, 0..1 | 1.5 |
//! | density | the change in onsets per beat, as \|log2\| of the ratio (a doubling is 1) | 1.0 |
//! | register | the change in the lead's median pitch, in octaves | 1.0 |
//! | melody | the share of sixteenths where the lead's sounding note differs, 0..1 | 1.0 |
//!
//! A change is **audible** when the sum is at least [`THRESHOLD`] (1.0): one layer entering or leaving clears it on
//! its own; a new chord progression, a new groove or a new melody generator clears it with what comes along (the
//! notes and onsets they change).

use crate::song::{Song, VoiceKind};

/// Samples per beat.
const RES: usize = 16;
/// Beats sampled (one stage).
const BEATS: usize = 32;

/// The sum a stage change must reach to count as audible.
pub const THRESHOLD: f64 = 1.0;

pub const W_LAYERS: f64 = 1.0;
pub const W_PITCH: f64 = 2.0;
pub const W_RHYTHM: f64 = 1.5;
pub const W_DENSITY: f64 = 1.0;
pub const W_REGISTER: f64 = 1.0;
pub const W_MELODY: f64 = 1.0;

/// One voice of a stage, sampled.
#[derive(Debug, Clone)]
pub struct VoiceProfile {
    pub name: String,
    pub layer: String,
    pub tonal: bool,
    /// Onsets, one flag a sixteenth (128).
    pub onsets: Vec<bool>,
    /// The pitch at each onset (semitones, rounded), for tonal voices: (sixteenth, pitch).
    pub notes: Vec<(usize, i64)>,
}

/// A stage, sampled.
#[derive(Debug, Clone, Default)]
pub struct Profile {
    pub voices: Vec<VoiceProfile>,
}

/// The layer a voice belongs to: `pad1`..`pad4` are the pads, `drone` and `ratio` the drone.
pub fn layer_of(name: &str) -> String {
    if name.starts_with("pad") && name[3..].chars().all(|c| c.is_ascii_digit()) {
        "pads".into()
    } else if name == "drone" || name == "ratio" {
        "drone".into()
    } else {
        name.to_string()
    }
}

/// Sample a stage's voices (32 beats from its own time 0).
pub fn profile(song: &Song) -> Profile {
    let steps = BEATS * RES;
    let mut voices = Vec::new();
    for v in &song.voices {
        if v.name == "fill" || v.name == "rise" {
            continue;
        }
        let mut onsets = vec![false; BEATS * 4];
        let mut notes = Vec::new();
        let mut prev = 0.0;
        let tonal = matches!(v.kind, VoiceKind::Tonal(_));
        for i in 0..steps {
            let t = i as f64 / RES as f64 + 1e-6;
            let g = match &v.kind {
                VoiceKind::Tonal(tn) => tn.gate.eval(&song.program, t),
                VoiceKind::Drum(d) => d.trigger.eval(&song.program, t),
            };
            let g = if g.is_finite() { g } else { 0.0 };
            if g > 0.0 && prev <= 0.0 {
                let six = i * 4 / RES;
                onsets[six] = true;
                if let VoiceKind::Tonal(tn) = &v.kind {
                    let p = tn.pitch.eval(&song.program, t + 1e-4);
                    if p.is_finite() {
                        notes.push((six, p.round() as i64));
                    }
                }
            } else if g > 0.0 && i % (RES / 4) == 0 {
                // Inside a held note, a new pitch that settles for a sixteenth is a note too (a legato line that
                // glides from note to note): heard as melody and pitch, not as a new onset.
                if let (VoiceKind::Tonal(tn), Some(&(_, last))) = (&v.kind, notes.last()) {
                    let p = tn.pitch.eval(&song.program, t + 1e-4);
                    let q = tn.pitch.eval(&song.program, t + 0.25 - 1e-4);
                    if p.is_finite() && q.is_finite() && p.round() != last as f64 && p.round() == q.round() {
                        notes.push((i * 4 / RES, p.round() as i64));
                    }
                }
            }
            prev = g;
        }
        voices.push(VoiceProfile { name: v.name.clone(), layer: layer_of(&v.name), tonal, onsets, notes });
    }
    Profile { voices }
}

/// The components of a stage change and their weighted sum.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Change {
    pub layers: f64,
    pub pitch: f64,
    pub rhythm: f64,
    pub density: f64,
    pub register: f64,
    pub melody: f64,
}

impl Change {
    pub fn total(&self) -> f64 {
        W_LAYERS * self.layers
            + W_PITCH * self.pitch
            + W_RHYTHM * self.rhythm
            + W_DENSITY * self.density
            + W_REGISTER * self.register
            + W_MELODY * self.melody
    }
}

fn layers(p: &Profile) -> std::collections::BTreeSet<String> {
    p.voices.iter().filter(|v| v.onsets.iter().any(|&o| o)).map(|v| v.layer.clone()).collect()
}

/// Pitch-class distribution: each tonal voice's onsets normalised, then averaged over the voices.
fn pitch_classes(p: &Profile) -> Option<[f64; 12]> {
    let mut h = [0.0; 12];
    let mut n = 0;
    for v in p.voices.iter().filter(|v| v.tonal && !v.notes.is_empty()) {
        let w = 1.0 / v.notes.len() as f64;
        for &(_, pitch) in &v.notes {
            h[pitch.rem_euclid(12) as usize] += w;
        }
        n += 1;
    }
    if n == 0 {
        return None;
    }
    h.iter_mut().for_each(|x| *x /= n as f64);
    Some(h)
}

fn density(p: &Profile) -> f64 {
    p.voices.iter().map(|v| v.onsets.iter().filter(|&&o| o).count()).sum::<usize>() as f64 / BEATS as f64
}

fn lead_line(p: &Profile) -> Option<Vec<Option<i64>>> {
    let v = p.voices.iter().find(|v| v.name == "lead" && !v.notes.is_empty())?;
    let mut line = vec![None; BEATS * 4];
    let mut cur = None;
    let mut it = v.notes.iter().peekable();
    for (s, slot) in line.iter_mut().enumerate() {
        while let Some(&&(at, pitch)) = it.peek() {
            if at <= s {
                cur = Some(pitch);
                it.next();
            } else {
                break;
            }
        }
        *slot = cur;
    }
    Some(line)
}

fn median(mut xs: Vec<i64>) -> Option<f64> {
    if xs.is_empty() {
        return None;
    }
    xs.sort_unstable();
    Some(xs[xs.len() / 2] as f64)
}

/// How much stage `b` differs from stage `a`, by component.
pub fn change(a: &Profile, b: &Profile) -> Change {
    let (la, lb) = (layers(a), layers(b));
    let layer_change = la.symmetric_difference(&lb).count() as f64;
    let pitch = match (pitch_classes(a), pitch_classes(b)) {
        (Some(x), Some(y)) => 0.5 * x.iter().zip(&y).map(|(p, q)| (p - q).abs()).sum::<f64>(),
        _ => 0.0,
    };
    let mut rhythm: f64 = 0.0;
    for va in &a.voices {
        let Some(vb) = b.voices.iter().find(|v| v.name == va.name) else { continue };
        let (mut inter, mut union) = (0, 0);
        for (x, y) in va.onsets.iter().zip(&vb.onsets) {
            inter += (*x && *y) as usize;
            union += (*x || *y) as usize;
        }
        // Only a voice heard in both stages (one that came or went is counted as a layer).
        if va.onsets.iter().any(|&o| o) && vb.onsets.iter().any(|&o| o) {
            rhythm = rhythm.max(1.0 - inter as f64 / union as f64);
        }
    }
    let density = ((density(b) + 1.0) / (density(a) + 1.0)).log2().abs();
    let lead_notes = |p: &Profile| p.voices.iter().find(|v| v.name == "lead").map(|v| v.notes.iter().map(|n| n.1).collect::<Vec<_>>());
    let register = match (lead_notes(a).and_then(median), lead_notes(b).and_then(median)) {
        (Some(x), Some(y)) => (y - x).abs() / 12.0,
        _ => 0.0,
    };
    let melody = match (lead_line(a), lead_line(b)) {
        (Some(x), Some(y)) => {
            let (mut diff, mut n) = (0, 0);
            for (p, q) in x.iter().zip(&y) {
                if p.is_some() || q.is_some() {
                    n += 1;
                    diff += (p != q) as usize;
                }
            }
            if n == 0 {
                0.0
            } else {
                diff as f64 / n as f64
            }
        }
        _ => 0.0,
    };
    Change { layers: layer_change, pitch, rhythm, density, register, melody }
}
