// A mixed rating batch: each arg is `phrase:<seed>` (the phrase species, by name) or `now:<seed>` (a species seed, as the
// app draws one). Stages 1 to 3 at version 3, saved exactly as S or B would save them, with their seed files.
use luminesce::compose::stream::{Composer, Seed};
use luminesce::stream::{save_text, seed_file_text};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let out = &a[1];
    for spec in &a[2..] {
        let (kind, n) = spec.split_once(':').unwrap();
        let n: u64 = n.parse().unwrap();
        let seed = if kind == "phrase" { Seed::Named { species: "phrase".into(), seed: n } } else { Seed::Species(n) };
        let mut c = Composer::with_version(seed.clone(), 3).unwrap();
        c.ensure(2);
        let text = save_text(&c.stages()[0..3], c.roster(), &c.origin, c.seed()).unwrap();
        std::fs::write(format!("{out}/{kind}_s{n}_1-3.txt"), &text).unwrap();
        std::fs::write(format!("{out}/{kind}_s{n}_v3.seed"), seed_file_text(&seed, 3, &c.origin).unwrap()).unwrap();
        println!("{kind} {n} {}", c.origin);
    }
}
