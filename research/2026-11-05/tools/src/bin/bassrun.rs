// Seed 2 of the book stream, stage by stage: is the bass playing, which bass module, what changed.
use luminesce::compose::book::{self, Slot};
use luminesce::compose::stream::{layers, Composer, Seed};
fn main() {
    let seed: u64 = std::env::args().nth(1).map(|s| s.parse().unwrap()).unwrap_or(2);
    let mut c = Composer::new(Seed::Book(seed)).unwrap();
    c.ensure(21);
    for s in c.stages() {
        let bass = layers(&s.eq).iter().any(|l| l == "bass");
        println!("{:2} {:?} bass {:5} {:16} | {}", s.index, s.role, bass, book::current(&s.eq, Slot::Bass).unwrap_or("-"), s.summary.chars().take(110).collect::<String>());
    }
}
