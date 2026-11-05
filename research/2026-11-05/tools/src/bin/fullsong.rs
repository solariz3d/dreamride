// A seed's whole song (every stage, start to ring-out) as one song file, at a generator version.
use luminesce::compose::stream::{Composer, Seed};
use luminesce::stream::save_text;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (seed, version, out) = (a[1].parse::<u64>().unwrap(), a[2].parse::<u32>().unwrap(), &a[3]);
    let mut c = Composer::with_version(Seed::Species(seed), version).unwrap();
    c.ensure(1000);
    assert!(c.ended(), "the song has not ended");
    let text = save_text(c.stages(), c.roster(), &c.origin, c.seed()).unwrap();
    std::fs::write(out, &text).unwrap();
    println!("{} stages, {} bytes", c.stages().len(), text.len());
}
