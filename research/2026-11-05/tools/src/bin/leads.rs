// How far apart are songs' leads as heard? Brightness (spectral centroid, in harmonics) with and without the song's
// rolloff, at the lead's usual pitch, and the envelope the lead plays with after dressing.
use luminesce::compose::book::{self, Slot};
use luminesce::compose::stream::{Composer, Seed};
fn centroid(partials: &[f64], roll: Option<f64>, hz: f64, cutoff: Option<f64>) -> f64 {
    let mut num = 0.0; let mut den = 0.0;
    for (i, d) in partials.iter().enumerate() {
        let n = (i + 1) as f64; let f = n * hz;
        let a = 10f64.powf(d / 20.0) * roll.map_or(1.0, |r| (r / f).min(1.0)) * cutoff.map_or(1.0, |c| 1.0 / (1.0 + (f / c).powi(4)).sqrt());
        num += n * a * a; den += a * a;
    }
    num / den
}
fn nums(s: Option<&str>) -> Vec<f64> { s.unwrap_or("").split(',').filter_map(|x| x.trim().parse().ok()).collect() }
fn main() {
    for seed in 1..=8u64 {
        let c = Composer::new(Seed::Book(seed)).unwrap();
        let eq = c.roster_eq();
        let d = c.dressed(&eq);
        let (w, v) = (eq.voice("lead").unwrap(), d.voice("lead").unwrap());
        let p = |b: &luminesce::compose::stream::Block, k: &str| b.get(k).and_then(|x| x.trim().parse::<f64>().ok());
        println!("seed {seed}: {:22} written centroid {:.2} (no cap {:.2}) | drawn centroid {:.2} (no cap {:.2})  sustain {:?} decay {:?} cutoff {:?}",
            book::current(&eq, Slot::Lead).unwrap_or("-"),
            centroid(&nums(w.get("partials")), p(w, "rolloff"), 830.0, p(w, "cutoff")), centroid(&nums(w.get("partials")), None, 830.0, p(w, "cutoff")),
            centroid(&nums(v.get("partials")), p(v, "rolloff"), 830.0, p(v, "cutoff")), centroid(&nums(v.get("partials")), None, 830.0, p(v, "cutoff")),
            v.get("sustain"), v.get("decay"), v.get("cutoff"));
    }
}
