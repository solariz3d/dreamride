//! The three graph pieces (examples/graph_*.txt) claim that their music comes out of one generator each, through a
//! scale lens. Checked here, from the files' own functions: the Takagi piece's form is the curve and its kick lands
//! only on beats although nothing puts it there; the Lissajous piece's drone interval, hats and claps are its ratio p:q;
//! the Pisano piece answers itself with its inversion and its kick is a 3-3-2; every tonal voice that the files say is
//! on the scale is on it; and each file renders finite and exactly as long as asked.

use luminesce::render::render;
use luminesce::song::{Drum, Song, Tonal, VoiceKind};

const TAKAGI: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/graph_takagi.txt");
const LISSAJOUS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/graph_lissajous.txt");
const PISANO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/graph_pisano.txt");

fn load(path: &str) -> Song {
    Song::parse(&std::fs::read_to_string(path).unwrap()).unwrap_or_else(|e| panic!("{e}"))
}

fn f(s: &Song, name: &str, args: &[f64]) -> f64 {
    s.program.call(name, args).unwrap_or_else(|e| panic!("{name}{args:?}: {e}"))
}

fn tonal<'a>(s: &'a Song, name: &str) -> &'a Tonal {
    match &s.voices.iter().find(|v| v.name == name).unwrap_or_else(|| panic!("no voice {name}")).kind {
        VoiceKind::Tonal(t) => t,
        VoiceKind::Drum(_) => panic!("{name} is a drum"),
    }
}

fn drum<'a>(s: &'a Song, name: &str) -> &'a Drum {
    match &s.voices.iter().find(|v| v.name == name).unwrap_or_else(|| panic!("no drum {name}")).kind {
        VoiceKind::Drum(d) => d,
        VoiceKind::Tonal(_) => panic!("{name} is a tonal voice"),
    }
}

/// The beats at which a drum's trigger rises from <= 0 to > 0, sampled 96 times a beat, as the renderer reads it.
fn onsets(s: &Song, name: &str, from: f64, to: f64) -> Vec<f64> {
    let d = drum(s, name);
    let mut out = Vec::new();
    let mut prev = d.trigger.eval(&s.program, from - 1.0 / 96.0);
    let mut i = (from * 96.0).round() as i64;
    while (i as f64) < to * 96.0 {
        let t = i as f64 / 96.0;
        let v = d.trigger.eval(&s.program, t);
        if prev <= 0.0 && v > 0.0 {
            out.push(t);
        }
        prev = v;
        i += 1;
    }
    out
}

/// Is `pitch` (semitones above A4) on the mode whose step pattern from `tonic` is `steps`?
fn on_mode(pitch: f64, tonic: f64, steps: &[i64]) -> bool {
    let k = pitch - tonic;
    k == k.round() && steps.contains(&(k.round() as i64).rem_euclid(12))
}

const MAJOR: [i64; 7] = [0, 2, 4, 5, 7, 9, 11];

// ---------------------------------------------------------------------------------------------------- the lens

#[test]
fn the_lens_constant_walks_the_seven_modes_one_sharp_at_a_time() {
    // h(d) = floor((12d + c) / 7): c = 0 is Locrian, ... 5 Ionian, 6 Lydian. Each step of c raises exactly one degree.
    let modes: Vec<Vec<i64>> = (0..7i64).map(|c| (0..7i64).map(|d| (12 * d + c).div_euclid(7)).collect()).collect();
    assert_eq!(modes[0], [0, 1, 3, 5, 6, 8, 10], "Locrian");
    assert_eq!(modes[2], [0, 2, 3, 5, 7, 8, 10], "Aeolian");
    assert_eq!(modes[3], [0, 2, 3, 5, 7, 9, 10], "Dorian");
    assert_eq!(modes[4], [0, 2, 4, 5, 7, 9, 10], "Mixolydian");
    assert_eq!(modes[5], MAJOR, "Ionian");
    assert_eq!(modes[6], [0, 2, 4, 6, 7, 9, 11], "Lydian");
    for c in 0..6 {
        let raised: i64 = (0..7).map(|d| modes[c + 1][d] - modes[c][d]).sum();
        assert_eq!(raised, 1, "c = {c} to {} raises one degree by one semitone", c + 1);
    }
    // And the files' h is that formula: the Lissajous piece's lens at its first section (c = 3) is Dorian.
    let s = load(LISSAJOUS);
    let lens: Vec<f64> = (0..7).map(|d| s.program.call_at("h", &[d as f64], 0.5).unwrap()).collect();
    assert_eq!(lens, [0.0, 2.0, 3.0, 5.0, 7.0, 9.0, 10.0]);
}

#[test]
fn the_glide_lens_holds_each_scale_tone_then_slides_to_the_next() {
    let s = load(TAKAGI);
    for d in -8..16 {
        let d = d as f64;
        let (here, next) = (f(&s, "h", &[d]), f(&s, "h", &[d + 1.0]));
        for i in 0..=75 {
            let u = d + i as f64 / 100.0;
            assert_eq!(f(&s, "g", &[u]), here, "g holds h({d}) at u = {u}");
        }
        let mut last = here;
        for i in 76..100 {
            let v = f(&s, "g", &[d + i as f64 / 100.0]);
            assert!(v >= last && v <= next, "g slides monotonically from h({d}) to h({d} + 1)");
            last = v;
        }
    }
}

// ---------------------------------------------------------------------------------------------------- Takagi

#[test]
fn takagi_chord_roots_are_the_curve_across_the_song_and_mirror_round_its_middle() {
    let s = load(TAKAGI);
    let roots: Vec<f64> = (0..16).map(|bar| f(&s, "R", &[16.0 * bar as f64])).collect();
    assert_eq!(roots, [0., 1., 2., 3., 3., 4., 4., 4., 3., 4., 4., 4., 3., 3., 2., 1.]);
    for bar in 1..16 {
        assert_eq!(roots[bar], roots[16 - bar], "bar {bar} mirrors bar {}", 16 - bar);
    }
}

#[test]
fn takagi_the_kick_follows_the_falling_lead_and_so_lands_only_on_beats_but_not_on_all_of_them() {
    // Nothing in the file puts the kick on a beat: it hits wherever the lead's degree falls by 3 or more. The curve's
    // beat-scale tent restarts at every beat, so that is where the lead falls; and the larger scales decide which.
    let s = load(TAKAGI);
    let kicks = onsets(&s, "kick", 0.0, 64.0);
    assert!(!kicks.is_empty());
    for &t in &kicks {
        assert_eq!(t, t.round(), "a kick off the beat, at beat {t}");
    }
    let per_bar: Vec<usize> = (0..16).map(|b| kicks.iter().filter(|&&t| (t / 4.0).floor() as usize == b).count()).collect();
    assert!(per_bar.iter().all(|&n| n >= 2), "every bar has a kick on at least two beats: {per_bar:?}");
    assert!(per_bar.iter().any(|&n| n < 4), "and it is not four on the floor everywhere: {per_bar:?}");
    assert!(kicks.contains(&0.0), "the song's downbeat");
}

#[test]
fn takagi_every_voice_but_the_sweeps_sounds_g_major_at_each_step() {
    let s = load(TAKAGI);
    for name in ["lead", "pad1", "pad2", "pad3", "pad4", "padup1", "padup2", "arp", "bass"] {
        let v = tonal(&s, name);
        for st in 0..256 {
            let t = st as f64 / 4.0 + 0.01; // inside the step, before any glide
            let p = v.pitch.eval(&s.program, t);
            assert!(on_mode(p, -2.0, &MAJOR), "{name} at beat {t} sounds {p}, off G major");
        }
    }
}

// ---------------------------------------------------------------------------------------------------- Lissajous

#[test]
fn lissajous_the_drone_interval_is_the_sections_ratio_in_just_intonation() {
    let s = load(LISSAJOUS);
    let (drone, ratio) = (tonal(&s, "drone"), tonal(&s, "ratio"));
    for sec in 0..4 {
        let t = 16.0 * sec as f64 + 3.0;
        let (p, q) = ((sec + 2) as f64, (sec + 1) as f64);
        let interval = ratio.pitch.eval(&s.program, t) - drone.pitch.eval(&s.program, t);
        let just = 12.0 * (p / q).log2();
        assert!((interval - just).abs() < 0.001, "section {sec}: {interval} semitones, the ratio {p}:{q} is {just}");
    }
}

#[test]
fn lissajous_the_rhythm_is_the_ratio_p_hats_a_beat_and_q_claps_a_bar() {
    let s = load(LISSAJOUS);
    for sec in 0..4 {
        let (p, q) = (sec + 2, sec + 1);
        // Away from the section's edges, where the rate changes.
        let (a, b) = (16.0 * sec as f64 + 4.0, 16.0 * sec as f64 + 12.0);
        assert_eq!(onsets(&s, "hat", a, b).len(), 8 * p, "section {sec}: {p} hats a beat");
        assert_eq!(onsets(&s, "clap", a, b).len(), 2 * q, "section {sec}: {q} claps a bar");
    }
}

#[test]
fn lissajous_the_lead_follows_the_mode_as_it_brightens_and_meets_the_just_third_with_a_major_one() {
    let s = load(LISSAJOUS);
    let modes: [[i64; 7]; 4] = [[0, 2, 3, 5, 7, 9, 10], [0, 2, 4, 5, 7, 9, 10], MAJOR, [0, 2, 4, 6, 7, 9, 11]];
    let lead = tonal(&s, "lead");
    for e in 0..128 {
        let t = e as f64 / 2.0 + 0.01;
        let p = lead.pitch.eval(&s.program, t);
        assert!(on_mode(p, 0.0, &modes[e / 32]), "lead at beat {t}: {p} is off the section's mode");
    }
    // In the last section the ratio is 5:4, a just major third (3.86 semitones); the lens's third is major there.
    assert_eq!(s.program.call_at("h", &[2.0], 56.0).unwrap(), 4.0);
}

// ---------------------------------------------------------------------------------------------------- Pisano

#[test]
fn pisano_binets_formula_gives_the_fibonacci_numbers_exactly_where_the_piece_reads_them() {
    let s = load(PISANO);
    let (mut a, mut b) = (0u64, 1u64);
    for k in 0..=40 {
        assert_eq!(f(&s, "F", &[k as f64]), a as f64, "F({k})");
        (a, b) = (b, a + b);
    }
}

#[test]
fn pisano_every_two_bars_the_melody_is_answered_by_its_inversion_and_so_is_the_form() {
    let s = load(PISANO);
    let signed = |k: f64| f(&s, "sg", &[f(&s, "fm", &[k, 7.0, 16.0])]);
    let first: Vec<f64> = (0..8).map(|k| signed(k as f64)).collect();
    assert_eq!(first, [0., 1., 1., 2., 3., -2., 1., -1.]);
    for k in 0..8 {
        assert_eq!(signed(k as f64 + 8.0), -first[k], "eighth {} inverts eighth {k}", k + 8);
    }
    for bar in 0..8 {
        let (a, b) = (f(&s, "R", &[8.0 * bar as f64]), f(&s, "R", &[8.0 * (bar + 8) as f64]));
        assert_eq!(b, -a, "bar {} inverts bar {bar}'s root", bar + 8);
    }
}

#[test]
fn pisano_the_kick_is_a_3_3_2_in_every_bar() {
    let s = load(PISANO);
    let kicks = onsets(&s, "kick", 0.0, 64.0);
    for bar in 0..16 {
        let at: Vec<f64> = kicks.iter().filter(|&&t| (t / 4.0).floor() as usize == bar).map(|&t| 4.0 * (t - 4.0 * bar as f64)).collect();
        assert_eq!(at, [0.0, 6.0, 12.0], "bar {bar}: kicks on sixteenths {at:?}");
    }
}

#[test]
fn pisano_every_voice_but_the_clap_sounds_a_mixolydian_at_each_step() {
    let s = load(PISANO);
    let mixo = [0, 2, 4, 5, 7, 9, 10];
    for name in ["lead", "pad1", "pad2", "pad3", "pad4", "padup", "bass"] {
        let v = tonal(&s, name);
        for st in 0..256 {
            let t = st as f64 / 4.0 + 0.01;
            let p = v.pitch.eval(&s.program, t);
            assert!(on_mode(p, 0.0, &mixo), "{name} at beat {t} sounds {p}, off A Mixolydian");
        }
    }
}

// ---------------------------------------------------------------------------------------------------- all three

#[test]
fn the_graph_pieces_render_finite_and_exactly_as_long_as_asked() {
    for path in [TAKAGI, LISSAJOUS, PISANO] {
        let s = load(path);
        let m = render(&s).unwrap_or_else(|e| panic!("{path}: {e}"));
        assert_eq!(m.frames(), s.sample_count(), "{path}");
        assert!(m.left.iter().chain(&m.right).all(|x| x.is_finite()), "{path}");
        assert!(m.peak <= luminesce::render::CEILING + 1e-12, "{path}");
    }
}

#[test]
fn the_graph_pieces_fit_on_one_screen() {
    // The rule they were written to: about 10-16 definitions, no hand-placed note lists.
    for path in [TAKAGI, LISSAJOUS, PISANO] {
        let src = std::fs::read_to_string(path).unwrap();
        let defs = src.lines().filter(|l| !l.starts_with(' ') && !l.starts_with('#') && l.contains(" = ")).count();
        assert!((10..=18).contains(&defs), "{path}: {defs} definitions");
    }
}
