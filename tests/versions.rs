//! Generator versions (docs/versions.md): a seed plus its version is a song, forever. Version 1 is held, byte for
//! byte, to the stretches of seed 152637 the keeper listened to for five minutes (examples/*_heard.txt). If this test
//! fails, a change altered what version 1 plays: make the change a new version and keep version 1 as it was.

use luminesce::compose::stream::{Composer, Seed, GENERATOR_VERSION, GENERATOR_VERSIONS};
use luminesce::stream::save_text;

fn heard(name: &str) -> String {
    let p = format!("{}/examples/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{p}: {e}")).replace("\r\n", "\n")
}

#[test]
fn version_1_replays_the_keepers_saved_seed_byte_for_byte() {
    let mut c = Composer::with_version(Seed::Species(152637), 1).expect("version 1");
    c.ensure(14);
    for (first, last) in [(1usize, 2usize), (12, 15)] {
        let text = save_text(&c.stages()[first - 1..last], c.roster(), &c.origin, c.seed()).expect("saves");
        let want = heard(&format!("stream_s152637_{first}-{last}_heard.txt"));
        assert!(text.replace("\r\n", "\n") == want, "version 1 no longer plays seed 152637 stages {first}-{last} as heard");
    }
}

#[test]
fn a_new_stream_is_the_current_version_and_versions_differ() {
    let now = Composer::new(Seed::Species(152637)).expect("current");
    assert_eq!(now.version(), GENERATOR_VERSION);
    let old = Composer::with_version(Seed::Species(152637), 1).expect("version 1");
    assert_eq!(old.version(), 1);
    let lead = |c: &Composer| c.roster_song_text().split("voice lead {").nth(1).map(|r| r.split('}').next().unwrap_or("").to_string());
    assert_ne!(lead(&now), lead(&old), "version 2 draws other instruments than version 1");
}

#[test]
fn an_unknown_version_is_refused() {
    let e = Composer::with_version(Seed::Species(1), 999).err().expect("refused");
    assert!(e.contains("999") && e.contains(&format!("{GENERATOR_VERSIONS:?}")), "{e}");
    assert!(Composer::with_version(Seed::Species(1), 0).is_err());
}

#[test]
fn a_seed_file_round_trips_every_kind() {
    use luminesce::stream::{read_seed_file, seed_file_text};
    for (seed, version) in [
        (Seed::Species(152637), 1u32),
        (Seed::Book(7), 2),
        (Seed::Explore(9), 2),
        (Seed::Grammar(3), 1),
        (Seed::Named { species: "automaton".into(), seed: 11 }, 2),
    ] {
        let text = seed_file_text(&seed, version, "a test").expect("writes");
        let (back, v) = read_seed_file(&text).expect("reads");
        assert_eq!(format!("{back:?}"), format!("{seed:?}"));
        assert_eq!(v, version);
    }
}

#[test]
fn a_bad_seed_file_names_what_is_wrong() {
    use luminesce::stream::{read_seed_file, seed_file_text};
    let e = read_seed_file("seed 1\nkind species\ncolour blue\nversion 1\n").unwrap_err();
    assert!(e.contains("line 3") && e.contains("colour"), "{e}");
    assert!(read_seed_file("seed x\nkind species\nversion 1\n").unwrap_err().contains("line 1"));
    assert!(read_seed_file("seed 1\nkind species\n").unwrap_err().contains("version"));
    assert!(read_seed_file("seed 1\nkind sideways\nversion 1\n").unwrap_err().contains("sideways"));
    assert!(read_seed_file("seed 1\nkind named\nversion 1\n").unwrap_err().contains("species"));
    assert!(seed_file_text(&Seed::default_song(), 2, "G").is_err(), "a song-started stream has no seed file");
}

#[test]
fn the_keepers_seed_file_replays_what_he_heard() {
    use luminesce::stream::{read_seed_file, seed_file_text};
    let text = seed_file_text(&Seed::Species(152637), 1, "the automaton species, seed 152637").expect("writes");
    let (seed, version) = read_seed_file(&text).expect("reads");
    let mut c = Composer::with_version(seed, version).expect("starts");
    c.ensure(14);
    let saved = save_text(&c.stages()[11..15], c.roster(), &c.origin, c.seed()).expect("saves");
    assert!(saved.replace("\r\n", "\n") == heard("stream_s152637_12-15_heard.txt"));
}

#[test]
fn version_2_plays_the_first_reference_song_whole() {
    // examples/reference_s885243_v2_whole_song.txt: the first song the keeper heard through to its own ending and kept
    // as the reference (2026-11-04, "the one i saved right now is pretty gas", "first fully completed song"). It must
    // stay what version 2 plays; and the stretch the app saved while he listened must be part of it, byte for byte.
    let mut c = Composer::with_version(Seed::Species(885243), 2).expect("version 2");
    c.ensure(1000);
    assert!(c.ended(), "the reference song ends");
    let whole = save_text(c.stages(), c.roster(), &c.origin, c.seed()).expect("saves");
    assert!(whole.replace("\r\n", "\n") == heard("reference_s885243_v2_whole_song.txt"), "version 2 no longer plays the reference song");
    let stretch = save_text(&c.stages()[10..14], c.roster(), &c.origin, c.seed()).expect("saves");
    assert!(stretch.replace("\r\n", "\n") == heard("stream_s885243_11-14.txt"), "the reference is not what was heard (stages 11-14 as the app saved them)");
}
