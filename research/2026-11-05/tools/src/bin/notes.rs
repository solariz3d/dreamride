// A voice's notes, bar by bar, in one stage of a named-species stream: pitch names with their onset (in eighths).
use luminesce::compose::critic::read_tracks;
use luminesce::compose::stream::{Composer, Seed};
use luminesce::song::Song;
const N: [&str; 12] = ["A", "Bb", "B", "C", "C#", "D", "Eb", "E", "F", "F#", "G", "Ab"];
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (species, seed, stage, voice) = (a[1].clone(), a[2].parse::<u64>().unwrap(), a[3].parse::<usize>().unwrap(), &a[4]);
    let mut c = Composer::with_version(Seed::Named { species, seed }, 3).unwrap();
    c.ensure(stage as u64);
    let s = &c.stages()[stage - 1];
    let song = Song::parse(&s.text).unwrap();
    println!("stage {stage}: {} | lead module {:?}", s.summary, luminesce::compose::book::current(&s.eq, luminesce::compose::book::Slot::Lead));
    let tr = read_tracks(&song);
    let Some(t) = tr.iter().find(|t| t.name == *voice) else { println!("no {voice}"); return };
    for bar in 0..8 {
        let notes: Vec<String> = t
            .events
            .iter()
            .filter(|e| e.t >= bar as f64 * 4.0 && e.t < bar as f64 * 4.0 + 4.0)
            .map(|e| format!("{}{}@{}", N[(e.pitch.round() as i64).rem_euclid(12) as usize], (e.pitch.round() as i64).div_euclid(12) + 4, ((e.t - bar as f64 * 4.0) * 2.0).round()))
            .collect();
        println!("  bar {}: {}", bar + 1, notes.join(" "));
    }
}
