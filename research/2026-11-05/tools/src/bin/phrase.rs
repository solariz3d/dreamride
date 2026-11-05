// A named phrase-species stream: its stages, motif number and definitions, and its first stages saved as one song.
use luminesce::compose::stream::{def_count, Composer, Seed};
use luminesce::stream::save_text;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (seed, stages, out) = (a[1].parse::<u64>().unwrap(), a[2].parse::<u64>().unwrap(), &a[3]);
    let mut c = Composer::with_version(Seed::Named { species: "phrase".into(), seed }, 3).unwrap();
    c.ensure(stages - 1);
    let mn = c.stages()[0].eq.def("Mn").map(|d| d.line.clone());
    let k = c.stages()[0].eq.def("K").map(|d| d.line.clone());
    println!("seed {seed}: {mn:?} {k:?} bpm line {:?}", c.stages()[0].eq.settings.iter().find(|s| s.starts_with("bpm")));
    for s in c.stages() {
        println!("  stage {:2} defs {:2} | {}", s.index + 1, def_count(&s.eq), s.summary);
    }
    let n = c.stages().len().min(stages as usize);
    let text = save_text(&c.stages()[..n], c.roster(), &c.origin, c.seed()).unwrap();
    std::fs::write(out, text).unwrap();
}
