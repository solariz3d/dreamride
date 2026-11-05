// Hypothesis K (docs/what_makes_music_bad.md, registered before this ran): per judged song, each stage's best
// Krumhansl-Kessler key over the stage alone; key distance between consecutive stages inside a judged stretch on the
// circle of fifths (a minor key takes its relative major's signature); a song fails if any distance >= 2 or any fit < 0.65.
// Args: label seed version first-last[,first-last...]
use luminesce::compose::critic::read_tracks;
use luminesce::compose::measure::{key_correlations, pitch_class_weights};
use luminesce::compose::stream::{Composer, Seed};
use luminesce::song::Song;
const NAMES: [&str; 12] = ["A", "Bb", "B", "C", "C#", "D", "Eb", "E", "F", "F#", "G", "Ab"];
/// Sharps (+) or flats (-) of a key's signature, -5..6. Pitch classes are relative to A = 0, so C is 3.
fn signature(tonic: usize, major: bool) -> i64 {
    let major_tonic = if major { tonic } else { (tonic + 3) % 12 };
    let pos = ((major_tonic as i64 - 3) * 7).rem_euclid(12);
    if pos > 6 { pos - 12 } else { pos }
}
fn dist(a: i64, b: i64) -> i64 {
    let d = (a - b).rem_euclid(12);
    d.min(12 - d)
}
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let (label, seed, version) = (&a[0], a[1].parse::<u64>().unwrap(), a[2].parse::<u32>().unwrap());
    let ranges: Vec<(u64, u64)> = a[3].split(',').map(|r| { let (x, y) = r.split_once('-').unwrap(); (x.parse().unwrap(), y.parse().unwrap()) }).collect();
    let last = ranges.iter().map(|r| r.1).max().unwrap();
    let mut c = Composer::with_version(Seed::Species(seed), version).unwrap();
    c.ensure(last - 1);
    let key = |k: u64| {
        let s = &c.stages()[(k - 1) as usize];
        let song = Song::parse(&s.text).unwrap();
        let tr = read_tracks(&song);
        let w = pitch_class_weights(&tr, 0, tr[0].open.len());
        key_correlations(&w)[0]
    };
    let (mut max_d, mut min_fit, mut path) = (0i64, f64::INFINITY, Vec::new());
    for (x, y) in &ranges {
        let mut prev: Option<i64> = None;
        for k in *x..=*y {
            let (fit, tonic, major) = key(k);
            let sig = signature(tonic, major);
            if let Some(p) = prev {
                max_d = max_d.max(dist(p, sig));
            }
            prev = Some(sig);
            min_fit = min_fit.min(fit);
            path.push(format!("{}{} {:.2}", NAMES[tonic], if major { "" } else { "m" }, fit));
        }
        path.push("|".into());
    }
    let fails = max_d >= 2 || min_fit < 0.65;
    println!("{label:<5} {seed:>7} v{version}  max key distance {max_d}  min fit {min_fit:.2}  {}  [{}]", if fails { "FAILS" } else { "passes" }, path.join(", "));
}
