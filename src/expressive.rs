//! Expressive performance: pure functions that shape a note by where it sits in its phrase.
//!
//! These are the language's expression built-ins (`arch`, `phrase`, `ritard`, `charge`,
//! `vibrato`, `db`, `smooth`), kept here as plain functions so their maths can be pinned by tests.
//! They encode published findings, not taste:
//!
//! - **Phrase arch** (Todd 1992, "The dynamics of dynamics", JASA 91:3540; the KTH Phrase rule,
//!   Friberg 1991, CMJ 15(2)): players rise in tempo and loudness into the middle of a phrase and
//!   fall away at its ends, along a roughly parabolic curve. [`arch`] is that curve with a movable
//!   peak; [`phrase`] is a time warp whose tempo follows it and which keeps every phrase exactly
//!   its written length, so a loop stays loop-exact.
//! - **Final ritard** (Friberg & Sundberg 1999, "Does music performance allude to locomotion?",
//!   JASA 105:1469): `v(x) = [1 + (w^q - 1) x]^(1/q)`, tempo against score position, from the
//!   constant braking power of stopping runners; listeners preferred q = 2 and q = 3.
//! - **Melodic charge** (Sundberg; KTH rule DPC 2A, Friberg 1991): a note's "remarkableness" by
//!   its distance from the chord root around the circle of fifths. The rule adds 0.2 dB per unit.
//! - **Vibrato** that waits and then grows, as singers and string players do (rate 5-7 Hz, extent
//!   up to about a semitone in operatic singing; far less on a synth lead).

use std::f64::consts::TAU;

/// Smoothstep on `u` clamped to [0, 1]: 0, then an S-curve, then 1.
#[inline]
pub fn smooth(u: f64) -> f64 {
    let u = u.max(0.0).min(1.0);
    u * u * (3.0 - 2.0 * u)
}

/// Decibels to a gain: `10^(x/20)`.
#[inline]
pub fn db(x: f64) -> f64 {
    10f64.powf(x / 20.0)
}

#[inline]
fn peak_of(peak: f64) -> f64 {
    peak.max(0.05).min(0.95)
}

/// The phrase arch: 0 at the start and end of a phrase (`x` = 0 and 1), 1 at `peak`, two
/// parabolas meeting with zero slope at the peak. `x` is clamped to [0, 1] and `peak` to
/// [0.05, 0.95].
pub fn arch(x: f64, peak: f64) -> f64 {
    let p = peak_of(peak);
    let x = x.max(0.0).min(1.0);
    let u = if x <= p { (p - x) / p } else { (x - p) / (1.0 - p) };
    1.0 - u * u
}

/// The integral of [`arch`] from 0 to `x` (for `x` in [0, 1]). Over the whole phrase it is 2/3.
fn arch_area(x: f64, p: f64) -> f64 {
    if x <= p {
        x - (p * p * p - (p - x).powi(3)) / (3.0 * p * p)
    } else {
        let d = x - p;
        2.0 * p / 3.0 + d - d * d * d / (3.0 * (1.0 - p) * (1.0 - p))
    }
}

/// Phrase time: a warp of `c` (counted in phrases) whose tempo is `1 + depth (arch - 2/3)`, slow
/// at each phrase's ends and fast at its peak. Every whole number maps to itself, so each phrase
/// keeps its exact length and notes at phrase boundaries stay on the grid. `depth` is clamped to
/// [0, 1], which keeps the tempo above 1/3 and the warp strictly increasing: no note can pass
/// another.
pub fn phrase(c: f64, depth: f64, peak: f64) -> f64 {
    let d = depth.max(0.0).min(1.0);
    let p = peak_of(peak);
    let n = c.floor();
    let f = c - n;
    n + f + d * (arch_area(f, p) - 2.0 * f / 3.0)
}

/// The Friberg-Sundberg final ritard: tempo (1 at the start) at position `x` in [0, 1] of the
/// ritard, falling to `w` at the end with curvature `q` (q = 1 is linear in position, q = 3 is the
/// runners' constant braking power). `w` is clamped to [0.05, 1], `q` to [0.5, 8].
pub fn ritard(x: f64, q: f64, w: f64) -> f64 {
    let x = x.max(0.0).min(1.0);
    let q = q.max(0.5).min(8.0);
    let w = w.max(0.05).min(1.0);
    (1.0 + (w.powf(q) - 1.0) * x).powf(1.0 / q)
}

/// Melodic charge of a note `semis` semitones above the chord root (any octave), from the KTH
/// table: the root 0, the fifth 1, the 9th 2, the 6th 3, the 3rd 4, the major 7th 5, the
/// tritone 6, the b9 6.5, the b6 5.5, the minor 3rd 4.5, the b7 3.5, the 4th 2.5. A fractional
/// interval is rounded to the nearest semitone.
pub fn charge(semis: f64) -> f64 {
    const TABLE: [f64; 12] = [0.0, 6.5, 2.0, 4.5, 4.0, 2.5, 6.0, 1.0, 5.5, 3.0, 3.5, 5.0];
    if !semis.is_finite() {
        return f64::NAN;
    }
    TABLE[(semis.round() as i64).rem_euclid(12) as usize]
}

/// Delayed vibrato, in semitones: silent until `delay` seconds into the note, then a sine at
/// `rate` Hz whose depth grows smoothly to `depth` semitones (peak) over `ramp` seconds. `age` is
/// the time since the note's onset in seconds (`note_age` in the language). It starts at a zero
/// crossing, so it never jumps.
pub fn vibrato(age: f64, delay: f64, ramp: f64, depth: f64, rate: f64) -> f64 {
    let a = age - delay;
    if !(a > 0.0) {
        return if a.is_nan() { f64::NAN } else { 0.0 };
    }
    let grow = if ramp > 0.0 { smooth(a / ramp) } else { 1.0 };
    depth * grow * (TAU * rate * a).sin()
}
