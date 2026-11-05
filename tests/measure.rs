//! The song measured against itself (src/compose/measure.rs; docs/what_makes_music_bad.md). Each measure is checked on
//! a case whose answer is known before it is run, including the one property they all exist for: a stretch is measured
//! against the song's own history, so the same stretch measures differently after a different past.

use luminesce::compose::measure::{auc, centricity, key_correlations, macroharmony, measure, IntervalModel};
use luminesce::song::Song;

/// A song of `beats` beats with one sine lead on sixteenths at `pitch`, and an optional second voice.
fn song(beats: u32, pitch: &str, extra: &str) -> Song {
    let text = format!("bpm 120\nbeats {beats}\nvoice lead {{\n  wave = sine\n  pitch = {pitch}\n  gate = frac(4t) < 0.8\n}}\n{extra}");
    Song::parse(&text).unwrap_or_else(|e| panic!("{e}\n{text}"))
}

/// A pseudo-random line: no bar repeats inside 64 beats, no interval pattern a short model can learn.
const RANDOM: &str = "mod(floor(4t)^3 + 7 floor(4t), 211) / 211 * 24 - 12";
/// One bar's figure, the same every bar.
const LOOP: &str = "2 mod(floor(4t), 4) + 3 (mod(floor(t), 4) == 2)";

#[test]
fn a_looped_bar_repeats_and_is_unsurprising_a_random_line_is_neither() {
    let looped = measure(&song(64, LOOP, ""), 32.0);
    let random = measure(&song(64, RANDOM, ""), 32.0);
    assert_eq!(looped.repetition, 1.0, "every bar of the stretch was heard before");
    assert_eq!(random.repetition, 0.0, "no bar of the random line recurs");
    assert!(looped.self_surprise < 1.0, "a learned loop is nearly certain: {}", looped.self_surprise);
    assert!(random.self_surprise > looped.self_surprise + 2.0, "random {} against looped {}", random.self_surprise, looped.self_surprise);
}

#[test]
fn the_same_stretch_is_less_surprising_after_a_past_that_matches_it() {
    // Stage 2 is the same random line in both songs. In one, stage 1 was that line already; in the other, the loop.
    let other_past = measure(&song(64, &format!("if(t < 32, {LOOP}, {})", RANDOM.replace("4t", "4(t - 32)")), ""), 32.0);
    let repeat = measure(&song(64, &format!("if(t < 32, {}, {})", RANDOM, RANDOM.replace("4t", "4(t - 32)")), ""), 32.0);
    assert!(repeat.self_surprise < other_past.self_surprise - 1.0, "heard before {} against new {}", repeat.self_surprise, other_past.self_surprise);
    assert_eq!(repeat.repetition, 1.0, "every bar of stage 2 is a bar of stage 1");
}

#[test]
fn silence_is_not_repetition() {
    // Found on the first bad mark: bars with no lead and no bass all looked alike, so they all "repeated".
    let quiet = gated(64, "0", "0");
    assert!(measure(&quiet, 32.0).repetition.is_nan(), "nothing sounding has nothing to repeat");
    let half = measure(&gated(64, LOOP, "(t < 48) (frac(4t) < 0.8)"), 32.0);
    assert_eq!(half.repetition, 1.0, "the four sounding bars repeat; the four silent ones are left out, not counted");
}

/// Like `song`, with the lead's gate given.
fn gated(beats: u32, pitch: &str, gate: &str) -> Song {
    let text = format!("bpm 120\nbeats {beats}\nvoice lead {{\n  wave = sine\n  pitch = {pitch}\n  gate = {gate}\n}}\n");
    Song::parse(&text).unwrap_or_else(|e| panic!("{e}\n{text}"))
}

#[test]
fn macroharmony_counts_the_pitch_classes_that_matter() {
    let mut major = [0.0; 12];
    for pc in [0, 2, 4, 5, 7, 9, 11] {
        major[pc] = 1.0;
    }
    assert_eq!(macroharmony(&major), 7.0);
    assert_eq!(macroharmony(&[1.0; 12]), 12.0);
    let mut faint = major;
    faint[1] = 0.1; // 0.1 of 7.1 is under 5%
    assert_eq!(macroharmony(&faint), 7.0);
    assert!(macroharmony(&[0.0; 12]).is_nan(), "nothing sounding has no pitch classes to count");
}

#[test]
fn a_tonal_profile_has_a_clear_centre_and_the_chromatic_total_has_none() {
    // C major relative to A = 0: C is pitch class 3. Weighted like a melody that sits on the triad.
    let mut c = [0.0; 12];
    for (pc, w) in [(3, 5.0), (5, 2.0), (7, 4.0), (8, 2.0), (10, 4.0), (0, 2.0), (2, 1.0)] {
        c[pc] = w;
    }
    let best = key_correlations(&c)[0];
    assert_eq!((best.1, best.2), (3, true), "the key found is C major: {best:?}");
    assert!(centricity(&c) > 1.1, "{}", centricity(&c));
    assert!(centricity(&[1.0; 12]).is_nan(), "every key correlates 0 with the chromatic total");
}

#[test]
fn scales_move_by_step_and_arpeggios_of_fourths_do_not() {
    let scale = measure(&song(32, "floor(4t) - 2 floor(2t)", ""), 0.0); // up and down by one semitone and back
    let fourths = measure(&song(32, "5 mod(floor(4t), 3)", ""), 0.0);
    assert_eq!(scale.conjunct, 1.0);
    assert_eq!(fourths.conjunct, 0.0);
}

#[test]
fn a_semitone_is_rougher_than_a_fifth() {
    let other = |iv: i32| format!("voice pad {{\n  wave = sine\n  pitch = {iv}\n  gate = 1\n}}\n");
    let semitone = measure(&song(8, "0", &other(1)), 0.0);
    let fifth = measure(&song(8, "0", &other(7)), 0.0);
    assert!(semitone.roughness > fifth.roughness, "semitone {} fifth {}", semitone.roughness, fifth.roughness);
}

#[test]
fn the_interval_model_learns_what_it_hears() {
    let mut m = IntervalModel::default();
    let first = m.p(2);
    for _ in 0..20 {
        m.learn(2);
    }
    assert!((first - 1.0 / 49.0).abs() < 1e-12, "an empty model is uniform over 49 intervals: {first}");
    assert!(m.p(2) > 0.9 && m.p(-5) < 0.01, "after twenty 2s: p(2) {} p(-5) {}", m.p(2), m.p(-5));
}

#[test]
fn auc_reads_separation_in_the_predicted_direction() {
    assert_eq!(auc(&[3.0, 4.0], &[1.0, 2.0], 1.0), 1.0);
    assert_eq!(auc(&[3.0, 4.0], &[1.0, 2.0], -1.0), 0.0);
    assert_eq!(auc(&[1.0, 1.0], &[1.0], 1.0), 0.5);
    assert!(auc(&[], &[1.0], 1.0).is_nan());
    assert_eq!(auc(&[f64::NAN, 3.0], &[1.0], 1.0), 1.0, "a stretch with nothing to measure is left out, not counted");
}
