// The dialogue A/B stretch: the FIRST lens seed (Seed::Book, from 1 up) with four consecutive stages of which at least
// three play both lead and counter, saved at version 2 and version 3. Chosen by that rule, not by listening.
use luminesce::compose::dialogue::draw;
use luminesce::compose::stream::{Composer, Seed};
use luminesce::stream::save_text;
fn duet(eq: &luminesce::compose::stream::Equation) -> bool {
    eq.voice("lead").is_some() && eq.voice("counter").is_some()
}
fn main() {
    let out = std::env::args().nth(1).expect("out dir");
    for seed in 1..=200u64 {
        let mut v2 = Composer::with_version(Seed::Book(seed), 2).unwrap();
        v2.ensure(40);
        let st = v2.stages();
        let Some(w) = (0..st.len().saturating_sub(3)).find(|&i| st[i..i + 4].iter().filter(|s| duet(&s.eq)).count() >= 3) else { continue };
        let mut v3 = Composer::with_version(Seed::Book(seed), 3).unwrap();
        v3.ensure(w as u64 + 3);
        for k in w..w + 4 {
            assert!(v2.stages()[k].eq == v3.stages()[k].eq, "v3 composed stage {} differently", k + 1);
        }
        for (v, c) in [(2, &v2), (3, &v3)] {
            let text = save_text(&c.stages()[w..w + 4], c.roster(), &c.origin, c.seed()).unwrap();
            std::fs::write(format!("{out}/v{v}.txt"), &text).unwrap();
        }
        let duets: Vec<bool> = st[w..w + 4].iter().map(|s| duet(&s.eq)).collect();
        println!("seed {seed}, stages {}-{}, duets {duets:?}, kind {:?}", w + 1, w + 4, draw(seed));
        return;
    }
    panic!("no seed");
}
