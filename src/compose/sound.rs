//! Sound measurements on a rendered mix, written in Rust from `tools/measure/analyze.py` so the composer can measure
//! with the engine's own render and no Python: band energy in five bands, and spectral peaks per frame.
//!
//! Differences from the Python, on purpose: band energy is the average of Hann-windowed 8192-sample frames (Welch's
//! method) rather than one FFT of the whole file; both estimate the same power spectrum, and the frame form needs no
//! FFT of a non-power-of-two length. The peak count is the Python's rule exactly: per 8192-sample frame, local maxima
//! of the magnitude spectrum between 80 Hz and 8 kHz within 40 dB of the frame's largest, frames whose largest
//! magnitude is below 1e-3 skipped, and the median over frames.

use crate::render::Mix;
use std::f64::consts::PI;

pub const FRAME: usize = 8192;
/// Band edges in Hz: <150, 150-500, 500-2k, 2k-6k, >6k (as in analyze.py).
pub const BAND_EDGES: [f64; 6] = [0.0, 150.0, 500.0, 2000.0, 6000.0, 24000.0];

/// In-place iterative radix-2 FFT (Cooley-Tukey). `re.len()` must be a power of two.
pub fn fft(re: &mut [f64], im: &mut [f64]) {
    let n = re.len();
    assert!(n.is_power_of_two() && im.len() == n, "fft needs a power-of-two length");
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let ang = -2.0 * PI / len as f64;
        let (wr, wi) = (ang.cos(), ang.sin());
        for start in (0..n).step_by(len) {
            let (mut cr, mut ci) = (1.0, 0.0);
            for k in 0..len / 2 {
                let (a, b) = (start + k, start + k + len / 2);
                let tr = re[b] * cr - im[b] * ci;
                let ti = re[b] * ci + im[b] * cr;
                re[b] = re[a] - tr;
                im[b] = im[a] - ti;
                re[a] += tr;
                im[a] += ti;
                let nr = cr * wr - ci * wi;
                ci = cr * wi + ci * wr;
                cr = nr;
            }
        }
        len <<= 1;
    }
}

/// The mix as mono, normalised to peak 1 (as analyze.py reads a file).
pub fn mono(mix: &Mix) -> Vec<f64> {
    let m: Vec<f64> = mix.left.iter().zip(&mix.right).map(|(l, r)| 0.5 * (l + r)).collect();
    let peak = m.iter().fold(0.0f64, |a, x| a.max(x.abs()));
    m.iter().map(|x| x / (peak + 1e-12)).collect()
}

fn hann(n: usize) -> Vec<f64> {
    // numpy.hanning: symmetric, 0.5 - 0.5 cos(2 pi i / (n - 1)).
    (0..n).map(|i| 0.5 - 0.5 * (2.0 * PI * i as f64 / (n - 1) as f64).cos()).collect()
}

/// Magnitude spectrum of each non-overlapping Hann-windowed frame.
fn frames(x: &[f64]) -> Vec<Vec<f64>> {
    let w = hann(FRAME);
    let mut out = Vec::new();
    let mut i = 0;
    while i + FRAME < x.len() {
        let mut re: Vec<f64> = (0..FRAME).map(|k| x[i + k] * w[k]).collect();
        let mut im = vec![0.0; FRAME];
        fft(&mut re, &mut im);
        out.push((0..=FRAME / 2).map(|k| (re[k] * re[k] + im[k] * im[k]).sqrt()).collect());
        i += FRAME;
    }
    out
}

/// Sound measurements of one mix.
#[derive(Debug, Clone)]
pub struct SoundStats {
    /// Percent of energy in each band of `BAND_EDGES`.
    pub bands: [f64; 5],
    /// Median spectral peaks per frame within 40 dB, 80 Hz - 8 kHz.
    pub peaks: f64,
}

pub fn measure(mix: &Mix) -> SoundStats {
    measure_mono(&mono(mix), mix.rate)
}

pub fn measure_mono(x: &[f64], rate: u32) -> SoundStats {
    let spectra = frames(x);
    let df = rate as f64 / FRAME as f64;
    let mut energy = [0.0f64; 5];
    let mut counts = Vec::new();
    for s in &spectra {
        for (k, m) in s.iter().enumerate() {
            let f = k as f64 * df;
            for b in 0..5 {
                if f >= BAND_EDGES[b] && f < BAND_EDGES[b + 1] {
                    energy[b] += m * m;
                }
            }
        }
        let lo = (80.0 / df).floor() as usize + 1;
        let hi = ((8000.0 / df).ceil() as usize).min(s.len());
        let band = &s[lo..hi];
        let max = band.iter().cloned().fold(0.0f64, f64::max);
        if max < 1e-3 {
            continue;
        }
        let db: Vec<f64> = band.iter().map(|m| 20.0 * (m / max + 1e-12).log10()).collect();
        let n = (1..db.len().saturating_sub(1)).filter(|&i| db[i] > db[i - 1] && db[i] > db[i + 1] && db[i] > -40.0).count();
        counts.push(n as f64);
    }
    let total: f64 = energy.iter().sum::<f64>().max(1e-300);
    let mut bands = [0.0; 5];
    for b in 0..5 {
        bands[b] = 100.0 * energy[b] / total;
    }
    counts.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let peaks = if counts.is_empty() {
        0.0
    } else if counts.len() % 2 == 1 {
        counts[counts.len() / 2]
    } else {
        0.5 * (counts[counts.len() / 2 - 1] + counts[counts.len() / 2])
    };
    SoundStats { bands, peaks }
}
