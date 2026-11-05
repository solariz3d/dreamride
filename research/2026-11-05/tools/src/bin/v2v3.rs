// For a species seed: does version 3 play any stage differently from version 2? And which species does each stage
// belong to? Also: does the saved stretch equal the regenerated one at v3 (the app's save is what we regenerate)?
use luminesce::compose::stream::{Composer, Seed};
use luminesce::stream::save_text;
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let (seed, first, last, saved) = (a[0].parse::<u64>().unwrap(), a[1].parse::<usize>().unwrap(), a[2].parse::<usize>().unwrap(), &a[3]);
    let mut v2 = Composer::with_version(Seed::Species(seed), 2).unwrap();
    let mut v3 = Composer::with_version(Seed::Species(seed), 3).unwrap();
    v2.ensure(last as u64 + 8);
    v3.ensure(last as u64 + 8);
    let n = v2.stages().len().min(v3.stages().len());
    let mut differ = Vec::new();
    for k in 0..n {
        if v2.stages()[k].text != v3.stages()[k].text {
            differ.push(k + 1);
        }
    }
    let species: Vec<String> = v3.stages()[..n].iter().map(|s| s.eq.species.clone()).collect();
    let mine = save_text(&v3.stages()[first - 1..last], v3.roster(), &v3.origin, v3.seed()).unwrap();
    let theirs = std::fs::read_to_string(saved).unwrap().replace("\r\n", "\n");
    println!(
        "seed {seed}: {n} stages compared; v3 differs from v2 at stages {differ:?}; species by stage {species:?}; saved {first}-{last} equals regenerated v3: {}",
        mine.replace("\r\n", "\n") == theirs
    );
}
