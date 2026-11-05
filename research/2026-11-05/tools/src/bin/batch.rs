// The rating batch: for each species seed given, stages 1 to 3 at version 3, as the app would play and save them:
// the stretch file (`stream_s<seed>_1-3.txt`, exactly what S or B would write) and its seed file, into <out>.
use luminesce::compose::stream::{Composer, Seed};
use luminesce::stream::{save_text, seed_file_text};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let out = &a[1];
    for s in &a[2..] {
        let seed: u64 = s.parse().unwrap();
        let mut c = Composer::with_version(Seed::Species(seed), 3).unwrap();
        c.ensure(2);
        let text = save_text(&c.stages()[0..3], c.roster(), &c.origin, c.seed()).unwrap();
        std::fs::write(format!("{out}/stream_s{seed}_1-3.txt"), &text).unwrap();
        let st = seed_file_text(&Seed::Species(seed), 3, &c.origin).unwrap();
        std::fs::write(format!("{out}/stream_s{seed}_v3.seed"), st).unwrap();
        println!("{seed} {}", c.origin);
    }
}
