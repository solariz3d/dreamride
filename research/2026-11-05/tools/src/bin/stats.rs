// The engine's own line measurement (book::line_stats) on the `lead` voice of each song file given.
use luminesce::{compose::book, song::Song};
fn main() {
    for f in std::env::args().skip(1) {
        let song = Song::parse(&std::fs::read_to_string(&f).unwrap()).unwrap_or_else(|e| panic!("{f}: {e:?}"));
        let s = book::line_stats(&song, "lead").expect("no lead stats");
        println!("{f}: notes/beat {:.2}  repetition {:.2}  chord tones strong {:.2} all {:.2}  steps {:.2}  range {:.1}  glide {:.2}  median {:.1}",
            s.notes_per_beat, s.repetition, s.chord_strong, s.chord_all, s.steps, s.range, s.glide_share, s.median);
    }
}
