//! A small deterministic random source for the composer: SplitMix64 (Steele, Lea and Flood, "Fast splittable
//! pseudorandom number generators", OOPSLA 2014), the same generator the renderer seeds its noise with. The same seed
//! always gives the same song.

#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }

    /// A generator for candidate `index` of run `seed`: independent of how many candidates came before, so a run is
    /// the same whether it is scored on one thread or many.
    pub fn for_candidate(seed: u64, index: u64) -> Rng {
        let mut r = Rng(seed ^ 0x9E37_79B9_7F4A_7C15u64.wrapping_mul(index.wrapping_add(1)));
        r.next_u64();
        r
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in [0, 1).
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniform integer in [lo, hi] (inclusive).
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        assert!(hi >= lo, "empty range {lo}..={hi}");
        lo + (self.next_u64() % ((hi - lo + 1) as u64)) as i64
    }

    /// Uniform in [lo, hi), rounded to `step`.
    pub fn real(&mut self, lo: f64, hi: f64, step: f64) -> f64 {
        let v = lo + (hi - lo) * self.unit();
        (v / step).round() * step
    }

    /// One element of a slice, uniformly.
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.range(0, xs.len() as i64 - 1) as usize]
    }

    /// An index chosen with the given relative weights.
    pub fn weighted(&mut self, weights: &[f64]) -> usize {
        let total: f64 = weights.iter().sum();
        let mut x = self.unit() * total;
        for (i, w) in weights.iter().enumerate() {
            if x < *w {
                return i;
            }
            x -= w;
        }
        weights.len() - 1
    }

    pub fn chance(&mut self, p: f64) -> bool {
        self.unit() < p
    }
}
