// Per stage of a species seed (version 3): the best two keys (Krumhansl-Kessler), centricity, macroharmony and the
// stage's summary. Pitch classes are relative to A = 0.
use luminesce::compose::critic::read_tracks;
use luminesce::compose::measure::{centricity, key_correlations, macroharmony, pitch_class_weights};
use luminesce::compose::stream::{Composer, Seed};
use luminesce::song::Song;
const NAMES: [&str; 12] = ["A", "Bb", "B", "C", "C#", "D", "Eb", "E", "F", "F#", "G", "Ab"];
fn main() {
    let a: Vec<u64> = std::env::args().skip(1).map(|x| x.parse().unwrap()).collect();
    let (seed, last) = (a[0], a[1]);
    let mut c = Composer::with_version(Seed::Species(seed), 3).unwrap();
    c.ensure(last - 1);
    for s in &c.stages()[..last as usize] {
        let song = Song::parse(&s.text).unwrap();
        let tr = read_tracks(&song);
        let n = tr[0].open.len();
        let w = pitch_class_weights(&tr, 0, n);
        let k = key_correlations(&w);
        let name = |x: &(f64, usize, bool)| format!("{} {} ({:.2})", NAMES[x.1], if x.2 { "major" } else { "minor" }, x.0);
        println!("stage {}: {} | then {} | centricity {:.3} | pitch classes {} | {}", s.index + 1, name(&k[0]), name(&k[1]), centricity(&w), macroharmony(&w), s.summary);
    }
}
