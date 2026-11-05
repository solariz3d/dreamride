// Over many species seeds (what the app plays): the share of stages in the lens, and the share of all stages that
// version 3 plays differently from version 2.
use luminesce::compose::book::species_for_seed;
use luminesce::compose::stream::{Composer, Seed};
fn main() {
    let n: u64 = std::env::args().nth(1).unwrap().parse().unwrap();
    let (mut stages, mut lens, mut differ, mut lens_start) = (0, 0, 0, 0);
    for seed in 1..=n {
        let s = seed * 7919 + 13;
        if species_for_seed(s).species.name == "lens" {
            lens_start += 1;
        }
        let mut v2 = Composer::with_version(Seed::Species(s), 2).unwrap();
        let mut v3 = Composer::with_version(Seed::Species(s), 3).unwrap();
        v2.ensure(23);
        v3.ensure(23);
        for (a, b) in v2.stages().iter().zip(v3.stages()) {
            stages += 1;
            if b.eq.species.is_empty() || b.eq.species == "lens" {
                lens += 1;
            }
            if a.text != b.text {
                differ += 1;
            }
        }
    }
    println!("{n} seeds: {lens_start} start in the lens; {stages} stages, {lens} in the lens, {differ} played differently at v3");
}
