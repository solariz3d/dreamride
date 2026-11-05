// How a seed's song ended: its stages, the closure before the coda, the parts of that closure, and its length.
use luminesce::compose::stream::{Composer, Seed, Role};
fn main() {
    let seed: u64 = std::env::args().nth(1).unwrap().parse().unwrap();
    let version: u32 = std::env::args().nth(2).unwrap().parse().unwrap();
    let mut c = Composer::with_version(Seed::Species(seed), version).unwrap();
    c.ensure(60);
    let st = c.stages();
    let n = st.len();
    let coda = st.iter().position(|s| s.role == Role::Coda).unwrap_or(n);
    println!("{n} stages, coda at stage {}, about {:.0} s; closure before the coda {:.2} (ends by closure at 0.70)",
        coda + 1, (n as f64 - 1.0) * c.stage_secs(), st[coda.saturating_sub(1)].closure);
    println!("{}", st[coda].summary);
}
