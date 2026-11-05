// Per stage of the A/B stretch: the lead's open share, the counter's open share, and how often both sound, at v2 and v3.
use luminesce::compose::stream::{Composer, Seed};
use luminesce::song::{Song, VoiceKind};
fn open(song: &Song, name: &str) -> Vec<bool> {
    let n = (song.beats * 4.0) as usize;
    match song.voices.iter().find(|v| v.name == name).map(|v| &v.kind) {
        Some(VoiceKind::Tonal(tn)) => (0..n).map(|k| tn.gate.eval(&song.program, k as f64 / 4.0 + 0.01) > 0.0).collect(),
        _ => vec![false; n],
    }
}
fn main() {
    let a: Vec<u64> = std::env::args().skip(1).map(|x| x.parse().unwrap()).collect();
    let (seed, first, last) = (a[0], a[1] as usize, a[2] as usize);
    for v in [2u32, 3] {
        let mut c = Composer::with_version(Seed::Book(seed), v).unwrap();
        c.ensure(last as u64);
        for k in first - 1..last {
            let song = Song::parse(&c.stages()[k].text).unwrap();
            let (l, u) = (open(&song, "lead"), open(&song, "counter"));
            let n = l.len() as f64;
            let share = |x: &Vec<bool>| x.iter().filter(|b| **b).count() as f64 / n;
            let both = l.iter().zip(&u).filter(|(a, b)| **a && **b).count() as f64 / n;
            println!("v{v} stage {}: lead open {:.2}, counter open {:.2}, both {:.2}", k + 1, share(&l), share(&u), both);
        }
    }
}
