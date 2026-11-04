//! Send effects and master colour, applied to the summed mix: a reverb, a tempo-synced ping-pong
//! delay, tape wow and warmth. Everything is deterministic and linear in its input except warmth.
//!
//! **Reverb: Freeverb** (Jezar at Dreampoint, public domain, 2000), from its description in J. O.
//! Smith, *Physical Audio Signal Processing*, "Freeverb" (CCRMA): per channel, eight parallel
//! lowpass-feedback comb filters into four series Schroeder allpasses. Comb:
//! `y = x[n-N] ... ; store = (1-d) y + d store; buf = x + f store`, with feedback
//! `f = 0.7 + 0.28 size` and damping `d = 0.4 damp`; allpass gain 0.5. The delay lengths are the
//! published tuning (combs 1116 .. 1617, allpasses 556, 441, 341, 225 samples at 44.1 kHz), scaled
//! to the song's rate, with the right channel 23 samples longer. Written here from those equations.
//!
//! **Tails and exact length.** The output is always exactly as long as the song. With `loop 1`
//! every effect runs over the song twice and keeps the second pass, so the reverb and echoes
//! left ringing at the end sound over the start, exactly as they would when the file is looped
//! (the effects are linear, so this is the steady state of the loop, up to tails longer than the
//! song itself). Without it the effects start from silence and anything past the end is cut.

use crate::song::{Effects, Song};
use std::f64::consts::{LN_2, PI, TAU};

const COMBS: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
const ALLPASSES: [usize; 4] = [556, 441, 341, 225];
const STEREO_SPREAD: usize = 23;
const TUNED_RATE: f64 = 44_100.0;
/// Freeverb's fixed input gain (0.015) applied to L + R of a centred mono send: 2 x 0.015.
pub const REVERB_INPUT_GAIN: f64 = 0.03;

fn scaled(len: usize, rate: f64) -> usize {
    ((len as f64 * rate / TUNED_RATE).round() as usize).max(1)
}

struct Comb {
    buf: Vec<f64>,
    i: usize,
    store: f64,
}

impl Comb {
    fn new(len: usize) -> Comb {
        Comb { buf: vec![0.0; len], i: 0, store: 0.0 }
    }

    #[inline]
    fn tick(&mut self, x: f64, feedback: f64, damp: f64) -> f64 {
        let y = self.buf[self.i];
        self.store = y * (1.0 - damp) + self.store * damp;
        self.buf[self.i] = x + self.store * feedback;
        self.i += 1;
        if self.i == self.buf.len() {
            self.i = 0;
        }
        y
    }
}

struct Allpass {
    buf: Vec<f64>,
    i: usize,
}

impl Allpass {
    #[inline]
    fn tick(&mut self, x: f64) -> f64 {
        let b = self.buf[self.i];
        self.buf[self.i] = x + b * 0.5;
        self.i += 1;
        if self.i == self.buf.len() {
            self.i = 0;
        }
        b - x
    }
}

/// One Freeverb, stereo out from a mono in.
pub struct Freeverb {
    combs: [Vec<Comb>; 2],
    allpasses: [Vec<Allpass>; 2],
    feedback: f64,
    damp: f64,
    wet1: f64,
    wet2: f64,
}

impl Freeverb {
    pub fn new(rate: f64, size: f64, damp: f64, width: f64) -> Freeverb {
        let side = |spread: usize| -> (Vec<Comb>, Vec<Allpass>) {
            (
                COMBS.iter().map(|&n| Comb::new(scaled(n + spread, rate))).collect(),
                ALLPASSES.iter().map(|&n| Allpass { buf: vec![0.0; scaled(n + spread, rate)], i: 0 }).collect(),
            )
        };
        let (cl, al) = side(0);
        let (cr, ar) = side(STEREO_SPREAD);
        Freeverb {
            combs: [cl, cr],
            allpasses: [al, ar],
            feedback: 0.7 + 0.28 * size.max(0.0).min(1.0),
            damp: 0.4 * damp.max(0.0).min(1.0),
            wet1: width / 2.0 + 0.5,
            wet2: (1.0 - width) / 2.0,
        }
    }

    #[inline]
    pub fn tick(&mut self, x: f64) -> (f64, f64) {
        let input = x * REVERB_INPUT_GAIN;
        let mut out = [0.0; 2];
        for ch in 0..2 {
            let mut s = 0.0;
            for c in &mut self.combs[ch] {
                s += c.tick(input, self.feedback, self.damp);
            }
            for a in &mut self.allpasses[ch] {
                s = a.tick(s);
            }
            out[ch] = s;
        }
        (out[0] * self.wet1 + out[1] * self.wet2, out[1] * self.wet1 + out[0] * self.wet2)
    }
}

/// The stereo delay: one line per side, each `len` samples. The send enters the left line; each
/// line's output, darkened by a one-pole low-pass, feeds the other line: a ping-pong. The first
/// repeat is the send exactly `len` samples later on the left, the second `2 len` on the right.
pub struct PingPong {
    left: Vec<f64>,
    right: Vec<f64>,
    i: usize,
    feedback: f64,
    a: f64,
    lp: (f64, f64),
}

impl PingPong {
    pub fn new(len: usize, feedback: f64, tone_hz: f64, rate: f64) -> PingPong {
        let len = len.max(1);
        let a = 1.0 - (-TAU * tone_hz.min(0.49 * rate) / rate).exp();
        PingPong { left: vec![0.0; len], right: vec![0.0; len], i: 0, feedback, a, lp: (0.0, 0.0) }
    }

    #[inline]
    pub fn tick(&mut self, x: f64) -> (f64, f64) {
        let (yl, yr) = (self.left[self.i], self.right[self.i]);
        self.lp.0 += self.a * (yl - self.lp.0);
        self.lp.1 += self.a * (yr - self.lp.1);
        self.left[self.i] = x + self.feedback * self.lp.1;
        self.right[self.i] = self.feedback * self.lp.0;
        self.i += 1;
        if self.i == self.left.len() {
            self.i = 0;
        }
        (yl, yr)
    }
}

/// The delay line length for a song: `delay_beats` at its tempo, rounded to the nearest sample.
pub fn delay_samples(song: &Song) -> usize {
    (song.effects.delay_beats * 60.0 / song.bpm * song.rate as f64).round() as usize
}

/// Run a stereo-out effect over a mono bus and add its output into the mix. With `looped`, a
/// first pass primes the effect with the song's own end.
fn run(bus: &[f64], looped: bool, left: &mut [f64], right: &mut [f64], mut tick: impl FnMut(f64) -> (f64, f64)) {
    if looped {
        for &x in bus {
            tick(x);
        }
    }
    for (i, &x) in bus.iter().enumerate() {
        let (l, r) = tick(x);
        left[i] += l;
        right[i] += r;
    }
}

pub fn apply(song: &Song, left: &mut [f64], right: &mut [f64], reverb_bus: &[f64], delay_bus: &[f64]) {
    let fx = &song.effects;
    let rate = song.rate as f64;
    let n = left.len();
    if !reverb_bus.is_empty() {
        let mut verb = Freeverb::new(rate, fx.reverb_size, fx.reverb_damp, fx.reverb_width);
        let pre = (fx.reverb_predelay_ms * 0.001 * rate).round() as usize;
        // The predelay shifts the send; in a loop the shifted-out end comes round to the start.
        let fed: Vec<f64> = (0..n)
            .map(|i| match (i >= pre, fx.looped) {
                (true, _) => reverb_bus[i - pre],
                (false, true) => reverb_bus[(i + n - pre % n.max(1)) % n],
                (false, false) => 0.0,
            })
            .collect();
        run(&fed, fx.looped, left, right, |x| verb.tick(x));
    }
    if !delay_bus.is_empty() {
        let mut delay = PingPong::new(delay_samples(song), fx.delay_feedback, fx.delay_tone_hz, rate);
        run(delay_bus, fx.looped, left, right, |x| delay.tick(x));
    }
    if fx.wow_cents > 0.0 {
        wow(fx, rate, left);
        wow(fx, rate, right);
    }
    if fx.warmth > 0.0 {
        warmth(fx, rate, left, right);
    }
}

/// The wow rate actually used: in a loop, the nearest rate that completes a whole number of cycles
/// (at least one) over the song, so the drift is seamless at the seam.
pub fn wow_rate(fx: &Effects, seconds: f64) -> f64 {
    if fx.looped {
        (fx.wow_rate_hz * seconds).round().max(1.0) / seconds
    } else {
        fx.wow_rate_hz
    }
}

/// Tape wow: the mix is read through a slowly moving playhead, `i + A sin(2 pi f i / rate)`, with
/// `A` set so the peak pitch deviation is `wow` cents (the playhead's speed peaks at
/// `1 + A 2 pi f / rate = 2^(cents/1200)` to first order). Offline, the playhead may read ahead as
/// well as behind, so the mix is not delayed. Cubic (Catmull-Rom) interpolation; outside the song
/// it reads the other end in a loop and silence otherwise.
fn wow(fx: &Effects, rate: f64, x: &mut [f64]) {
    let n = x.len();
    if n < 4 {
        return;
    }
    let f = wow_rate(fx, n as f64 / rate);
    let depth = fx.wow_cents / 1200.0 * LN_2 * rate / (TAU * f);
    let src = x.to_vec();
    let at = |k: i64| -> f64 {
        if fx.looped {
            src[k.rem_euclid(n as i64) as usize]
        } else if k < 0 || k >= n as i64 {
            0.0
        } else {
            src[k as usize]
        }
    };
    for (i, y) in x.iter_mut().enumerate() {
        let pos = i as f64 + depth * (TAU * f * i as f64 / rate).sin();
        let k = pos.floor();
        let u = pos - k;
        let k = k as i64;
        let (p0, p1, p2, p3) = (at(k - 1), at(k), at(k + 1), at(k + 2));
        *y = p1 + 0.5 * u * (p2 - p0 + u * (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3 + u * (3.0 * (p1 - p2) + p3 - p0)));
    }
}

/// Warmth: a soft tanh saturation measured against the mix's own peak (so it does not depend on
/// how loud the voices were written), then a one-pole low-pass from 20 kHz (warmth 0) down to
/// 7 kHz (warmth 1). The peak is restored afterwards; normalisation follows in the renderer.
fn warmth(fx: &Effects, rate: f64, left: &mut [f64], right: &mut [f64]) {
    let peak = left.iter().chain(right.iter()).fold(0.0f64, |a, &x| a.max(x.abs()));
    if !(peak > 0.0) || !peak.is_finite() {
        return;
    }
    let drive = 1.0 + 3.0 * fx.warmth;
    let norm = 1.0 / drive.tanh();
    let fc = 20_000.0 - 13_000.0 * fx.warmth;
    let a = 1.0 - (-2.0 * PI * fc.min(0.49 * rate) / rate).exp();
    for ch in [left, right] {
        for x in ch.iter_mut() {
            *x = (drive * *x / peak).tanh() * norm * peak;
        }
        // The low-pass starts from the song's last sample in a loop, so the seam is continuous.
        let mut state = if fx.looped { ch[ch.len() - 1] } else { 0.0 };
        for x in ch.iter_mut() {
            state += a * (*x - state);
            *x = state;
        }
    }
}
