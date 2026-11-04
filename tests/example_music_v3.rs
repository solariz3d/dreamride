//! The v3 examples' expressive claims, checked from their own functions: where the appoggiaturas
//! fall, that the climax note is saved, that the borrowed iv and the tonic come where the files
//! say, that lengthened notes never run into the next, and that the held breath is silent.

use luminesce::song::Song;

const V3: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/citypop_v3.txt");
const DARK3: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/dark_v3.txt");

fn load(path: &str) -> Song {
    Song::parse(&std::fs::read_to_string(path).unwrap()).unwrap_or_else(|e| panic!("{e}"))
}

fn f(s: &Song, name: &str, args: &[f64]) -> f64 {
    s.program.call(name, args).unwrap_or_else(|e| panic!("{name}{args:?}: {e}"))
}

const NAMES: [&str; 12] = ["C", "C#", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"];

// ---------------------------------------------------------------- city pop v3

/// Pitch class (C = 0) of a pitch given in semitones above F4.
fn pc_f(p: f64) -> i64 {
    (p.round() as i64 + 5).rem_euclid(12)
}

/// The lead, step by step: (step, onset step, pitch above F4) for every step a note sounds.
fn lead(s: &Song) -> Vec<(usize, usize, f64)> {
    (0..256)
        .filter(|&st| f(s, "lr", &[st as f64]) != 0.0)
        .map(|st| {
            let on = f(s, "lon", &[st as f64]);
            (st, on as usize, f(s, "lnote", &[on]))
        })
        .collect()
}

/// Pitch classes of the chord at step `st`: the root and its four upper tones.
fn chord(s: &Song, st: f64) -> Vec<i64> {
    let mut v: Vec<i64> = (0..4).map(|j| pc_f(f(s, "tone", &[j as f64, st]))).collect();
    v.push(pc_f(f(s, "h", &[f(s, "deg", &[st])])));
    v
}

/// `cargo test --release --test example_music_v3 v3_dump -- --nocapture --ignored`
#[test]
#[ignore]
fn v3_dump() {
    let s = load(V3);
    for (st, on, p) in lead(&s) {
        let c: Vec<&str> = chord(&s, st as f64).iter().map(|&c| NAMES[c as usize]).collect();
        println!(
            "bar {:2} st {:2} {} {}{} dB {:+.2} chord {:?}",
            st / 16 + 1,
            st % 16,
            if st == on { "on  " } else { "hold" },
            NAMES[pc_f(p) as usize],
            (p.round() as i64 + 5 + 48) / 12,
            f(&s, "ldb", &[on as f64]),
            c
        );
    }
}

#[test]
fn v3_every_lead_note_on_a_beat_is_a_chord_tone_or_an_appoggiatura_that_resolves_down_by_step() {
    let s = load(V3);
    let line = lead(&s);
    let mut appoggiaturas = Vec::new();
    for &(st, on, p) in &line {
        if st % 4 != 0 || chord(&s, st as f64).contains(&pc_f(p)) {
            continue;
        }
        // Not a chord tone: it must start on this beat, and the next note must be a step (one or
        // two semitones) lower and a chord tone.
        assert_eq!(st, on, "bar {} step {}: a held non-chord tone {}", st / 16 + 1, st % 16, NAMES[pc_f(p) as usize]);
        let &(nst, _, np) = line.iter().find(|&&(x, o, _)| x > st && x == o).expect("no next note");
        assert!((1.0..=2.0).contains(&(p - np)), "bar {}: {} does not step down ({p} -> {np})", st / 16 + 1, NAMES[pc_f(p) as usize]);
        assert!(chord(&s, nst as f64).contains(&pc_f(np)), "bar {}: resolves to a non-chord tone", st / 16 + 1);
        appoggiaturas.push(st / 16 + 1);
    }
    println!("appoggiaturas in bars {appoggiaturas:?}");
    assert!(appoggiaturas.contains(&6), "the hook's peak (bar 6) is an appoggiatura");
    assert!((2..=4).contains(&appoggiaturas.len()), "appoggiaturas are used sparingly: {appoggiaturas:?}");
}

#[test]
fn v3_the_climax_note_is_saved_for_the_last_section() {
    let s = load(V3);
    let line = lead(&s);
    let top = line.iter().map(|l| l.2).fold(f64::MIN, f64::max);
    let bars: Vec<usize> = line.iter().filter(|l| l.2 == top).map(|l| l.0 / 16 + 1).collect();
    assert!(bars.iter().all(|&b| b == 13 || b == 14), "the highest note sounds in bars {bars:?}");
    let hook_top = line.iter().filter(|l| l.0 < 128).map(|l| l.2).fold(f64::MIN, f64::max);
    assert!(top - hook_top >= 3.0, "the climax ({top}) is not clearly above the hook's peak ({hook_top})");
}

#[test]
fn v3_the_borrowed_minor_iv_comes_once_and_the_melody_sings_its_borrowed_note() {
    let s = load(V3);
    let is_iv = |st: usize| f(&s, "deg", &[st as f64]) == 3.0 && f(&s, "qual", &[st as f64]) == 2.0;
    let bars: Vec<usize> = (0..256).filter(|&st| is_iv(st)).map(|st| st / 16 + 1).collect();
    assert!(!bars.is_empty() && bars.iter().all(|&b| b == 14), "Bbm9 sounds in bars {bars:?}");
    // The borrowed notes (written flat: Db and Ab, outside F major) sound over the iv and only
    // there. (C#, the same pitch class as Db, is A7's third in bar 6: a sharp, not a borrowing.)
    let flats: Vec<(usize, i64)> =
        lead(&s).iter().filter(|l| f(&s, "la", &[l.1 as f64]) == 1.0).map(|l| (l.0 / 16 + 1, pc_f(l.2))).collect();
    assert!(flats.iter().all(|&(b, _)| b == 14), "flats in bars {flats:?}");
    assert!(flats.iter().any(|&(_, pc)| pc == 1), "the lead never sings Db");
}

#[test]
fn v3_the_tonic_is_withheld_until_bar_15() {
    let s = load(V3);
    let imaj = |st: usize| f(&s, "deg", &[st as f64]) == 0.0 && f(&s, "qual", &[st as f64]) == 1.0;
    let bars: Vec<usize> = (0..256).filter(|&st| imaj(st)).map(|st| st / 16 + 1).collect();
    assert!(!bars.is_empty() && bars.iter().all(|&b| b == 15), "Fmaj9 sounds in bars {bars:?}");
    let mut landed = false;
    for (st, _, p) in lead(&s) {
        if st % 4 == 0 && f(&s, "deg", &[st as f64]) == 0.0 && pc_f(p) == 5 {
            assert_eq!(st / 16 + 1, 15, "the melody rests on the tonic in bar {}", st / 16 + 1);
            landed = true;
        }
    }
    assert!(landed, "the melody never lands on the tonic at all");
}

#[test]
fn v3_final_lengthening_never_runs_a_note_into_the_next_one() {
    // Walk the lead's gate in fine steps of the band's time. Whenever a note starts straight after
    // another (no rest between), the gate must have closed inside the step before, or the two
    // would merge into one note. And the lead's own clock never runs backwards.
    let s = load(V3);
    let (mut prev_step, mut closed, mut prev_lx, mut checked) = (-1.0, true, f64::MIN, 0);
    for i in 0..64 * 400 {
        let x = i as f64 / 400.0;
        let lx = f(&s, "lx", &[x]);
        assert!(lx > prev_lx, "the lead's clock ran backwards at beat {x}");
        prev_lx = lx;
        let st = f(&s, "lst", &[x]);
        if st != prev_step {
            if f(&s, "lr", &[st]) == 1.0 && prev_step >= 0.0 && f(&s, "lr", &[prev_step]) != 0.0 {
                assert!(closed, "beat {x}: the note before step {st} never released");
                checked += 1;
            }
            prev_step = st;
            closed = false;
        }
        if f(&s, "lgate", &[x]) <= 0.0 {
            closed = true;
        }
    }
    assert!(checked >= 20, "only {checked} note-to-note joins checked");
}

#[test]
fn v3_the_lead_is_shaped_by_its_phrase_louder_in_the_middle_than_at_the_ends() {
    // Same pitch, same chord context aside, a note at a phrase's peak is louder than one at its
    // start: compare the arch term alone across every onset.
    let s = load(V3);
    let arch_db = |on: f64| 4.0 * (f(&s, "lphrase", &[on]) - 1.0);
    let onsets: Vec<f64> = lead(&s).iter().filter(|l| l.0 == l.1).map(|l| l.1 as f64).collect();
    let edge = onsets.iter().filter(|&&o| (o % 32.0) < 4.0).map(|&o| arch_db(o)).fold(f64::MIN, f64::max);
    let middle = onsets.iter().filter(|&&o| (16.0..24.0).contains(&(o % 32.0))).map(|&o| arch_db(o)).fold(f64::MAX, f64::min);
    assert!(middle > edge + 2.0, "middle {middle:.2} dB, phrase start {edge:.2} dB");
}

#[test]
fn v3_the_breakdown_ends_in_a_held_breath() {
    let s = load(V3);
    for st in 188..192 {
        let sf = st as f64;
        for name in ["kickd", "snared", "hatd", "bassd", "lr", "keyd"] {
            assert_eq!(f(&s, name, &[sf]), 0.0, "{name} sounds at step {st}");
        }
        assert!(f(&s, "padgate", &[sf / 4.0 + 0.1]) < 0.0, "the pads sound at step {st}");
    }
    // In the rendered loop that beat is only the room ringing: far below the bar after it.
    let m = luminesce::render::render(&s).unwrap();
    let beat = m.frames() as f64 / 64.0;
    let rms = |a: f64, b: f64| {
        let w = &m.left[(a * beat) as usize..(b * beat) as usize];
        (w.iter().map(|x| x * x).sum::<f64>() / w.len() as f64).sqrt()
    };
    let (breath, after) = (rms(47.4, 48.0), rms(48.0, 52.0));
    println!("the held breath: {:.1} dB below the return", 20.0 * (after / breath).log10());
    assert!(breath > 0.0 && breath < after / 10.0, "breath {breath}, return {after}");
}

/// Per-bar levels of both v3 renders, and how much of each is the effects' return:
/// `cargo test --release --test example_music_v3 levels -- --nocapture --ignored`.
#[test]
#[ignore]
fn levels() {
    for path in [V3, DARK3] {
        let src = std::fs::read_to_string(path).unwrap();
        let dry_src: String = src
            .lines()
            .map(|l| if l.trim_start().starts_with("reverb =") || l.trim_start().starts_with("delay =") { "" } else { l })
            .collect::<Vec<_>>()
            .join("\n");
        let full = luminesce::render::render(&Song::parse(&src).unwrap()).unwrap();
        let dry = luminesce::render::render(&Song::parse(&dry_src).unwrap()).unwrap();
        let bar = full.frames() / 16;
        let e = |x: &[f64]| x.iter().map(|v| v * v).sum::<f64>();
        println!("{path}");
        for b in 0..16 {
            let r = b * bar..(b + 1) * bar;
            let (f_, d) = (e(&full.left[r.clone()]) / full.gain.powi(2), e(&dry.left[r.clone()]) / dry.gain.powi(2));
            let rms = (e(&full.left[r]) / bar as f64).sqrt();
            println!("  bar {:2}: rms {:6.1} dBFS, effects add {:+.1} dB", b + 1, 20.0 * rms.log10(), 10.0 * (f_ / d).log10());
        }
    }
}

// ---------------------------------------------------------------- dark v3

/// The bell's notes as (onset step, semitones above E5).
fn bell_notes(s: &Song) -> Vec<(usize, f64)> {
    (0..256).filter(|&st| f(s, "bellr", &[st as f64]) == 1.0).map(|st| (st, f(s, "bnote", &[st as f64]))).collect()
}

#[test]
fn dark3_every_motif_a_ends_by_pushing_up_from_d_sharp_into_e() {
    let s = load(DARK3);
    let notes = bell_notes(&s);
    let mut endings = 0;
    for bar in (0..16).filter(|b| b % 2 == 0 && !(12..14).contains(b)) {
        let in_bar: Vec<f64> = notes.iter().filter(|n| n.0 / 16 == bar).map(|n| n.1).collect();
        let k = in_bar.len();
        assert!(k >= 2, "bar {}: {in_bar:?}", bar + 1);
        assert_eq!((in_bar[k - 2], in_bar[k - 1]), (-1.0, 0.0), "bar {}: does not end D#5 -> E5", bar + 1);
        endings += 1;
    }
    assert_eq!(endings, 7);
}

#[test]
fn dark3_the_crack_is_the_one_major_key_moment_and_the_drums_stop_for_it() {
    let s = load(DARK3);
    for st in 0..256 {
        let sf = st as f64;
        let in_crack = (192..224).contains(&st);
        let g_major = f(&s, "croot", &[sf]) == 3.0;
        assert_eq!(g_major, in_crack, "step {st}: G major outside the crack, or not in it");
        assert_eq!(f(&s, "cryr", &[sf]) != 0.0, in_crack && f(&s, "cryr", &[sf]) != 0.0, "the crying voice outside the crack");
        if in_crack {
            assert_eq!(f(&s, "kickd", &[sf]) + f(&s, "snared", &[sf]), 0.0, "drums in the crack at step {st}");
            assert_eq!(f(&s, "bellr", &[sf]), 0.0, "the bell plays in the crack at step {st}");
        }
    }
    // The crying voice opens on an appoggiatura: C over G (not in the triad) on the downbeat,
    // leaning down a semitone onto B, the chord's third.
    let first = f(&s, "cnote", &[192.0]);
    let second = (193..224).find(|&st| f(&s, "cryr", &[st as f64]) == 1.0).map(|st| f(&s, "cnote", &[st as f64])).unwrap();
    assert_eq!((first, second), (8.0, 7.0), "C6 -> B5");
    // Its line only falls: each note is lower than the one before.
    let line: Vec<f64> = (192..224).filter(|&st| f(&s, "cryr", &[st as f64]) == 1.0).map(|st| f(&s, "cnote", &[st as f64])).collect();
    assert!(line.windows(2).all(|w| w[1] < w[0]), "{line:?}");
}

#[test]
fn dark3_the_808_sags_only_through_the_crack() {
    // `sag` at the moment its note ends (note_left = 0, as when called outside a render).
    let s = load(DARK3);
    for st in 0..256 {
        let want = if (192..224).contains(&st) { -3.0 } else { 0.0 };
        assert_eq!(f(&s, "sag", &[st as f64 / 4.0 + 0.1]), want, "step {st}");
    }
}

#[test]
fn dark3_the_final_lift_to_e6_is_saved_for_the_last_bar() {
    let s = load(DARK3);
    let notes = bell_notes(&s);
    let top = notes.iter().map(|n| n.1).fold(f64::MIN, f64::max);
    assert_eq!(top, 12.0, "E6");
    let bars: Vec<usize> = notes.iter().filter(|n| n.1 == top).map(|n| n.0 / 16 + 1).collect();
    assert_eq!(bars, vec![16], "E6 sounds in bars {bars:?}");
    for st in 252..256 {
        assert_eq!(f(&s, "b808d", &[st as f64]), 0.0, "the 808 drops out for the last beat (step {st})");
    }
}

#[test]
fn the_v3_examples_render_finite_loop_exact_and_under_the_ceiling() {
    for (path, frames) in [(V3, 1_960_851), (DARK3, 1_280_000)] {
        let s = load(path);
        let m = luminesce::render::render(&s).unwrap();
        assert_eq!(m.frames(), frames, "{path}");
        assert!(m.left.iter().chain(&m.right).all(|x| x.is_finite()), "{path}");
        assert!(m.peak <= luminesce::render::CEILING + 1e-12 && m.peak > 0.5, "{path}: peak {}", m.peak);
    }
}
