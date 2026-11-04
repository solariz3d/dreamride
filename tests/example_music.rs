//! The v2 example's musical claims, checked by evaluating its own functions: voicings never cross
//! and move least, the lead is on chord tones on strong beats, approach notes approach, the loop closes.

use luminesce::song::Song;

const V2: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/citypop_v2.txt");

fn v2() -> Song {
    Song::parse(&std::fs::read_to_string(V2).unwrap()).unwrap_or_else(|e| panic!("{e}"))
}

fn f(s: &Song, name: &str, args: &[f64]) -> f64 {
    s.program.call(name, args).unwrap_or_else(|e| panic!("{name}{args:?}: {e}"))
}

/// Prints the arrangement step by step: `cargo test --test example_music dump -- --nocapture --ignored`.
#[test]
#[ignore]
fn dump() {
    let s = v2();
    let names = ["C", "C#", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"];
    let nm = |p: f64| names[((p as i64 + 9 + 1200) % 12) as usize];
    for st in 0..256 {
        let sf = st as f64;
        let chord: Vec<String> = (0..4).map(|k| nm(f(&s, "voicing", &[k as f64, sf, -9.0]) - 4.0).to_string()).collect();
        let ld = f(&s, "leadd", &[sf]);
        let lead = if ld == 0.0 { "-".into() } else { format!("{}{}", nm(-4.0 + f(&s, "leadnote", &[sf - f(&s, "lheld", &[sf]), 0.0])), ld) };
        let bd = f(&s, "bassd", &[sf]);
        let bass = if bd == 0.0 { "-".into() } else { format!("{}{}", nm(-28.0 + f(&s, "bassat", &[sf])), bd) };
        println!(
            "bar {:2} st {:2} chord {:?} K{} S{} H{} bass {:5} lead {}",
            st / 16 + 1, st % 16, chord, f(&s, "kickd", &[sf]), f(&s, "snared", &[sf]), f(&s, "hatv", &[sf]), bass, lead
        );
    }
}

/// Total voice movement around the loop (32 half-bar slots, wrapping) with the pads' window at `lo`.
fn window_motion(s: &Song, lo: f64) -> f64 {
    let v = |slot: usize| -> Vec<f64> { (0..4).map(|k| f(s, "voicing", &[k as f64, (slot * 8) as f64, lo])).collect() };
    (0..32).map(|i| v(i).iter().zip(v((i + 1) % 32)).map(|(a, b)| (a - b).abs()).sum::<f64>()).sum()
}

#[test]
fn pad_voices_never_cross() {
    let s = v2();
    let lo = f(&s, "padlo", &[]);
    for slot in 0..32 {
        let v: Vec<f64> = (0..4).map(|k| f(&s, "voicing", &[k as f64, (slot * 8) as f64, lo])).collect();
        assert!(v.windows(2).all(|w| w[0] < w[1]), "slot {slot}: {v:?}");
        assert!(v[3] - v[0] < 12.0, "slot {slot} spans an octave or more: {v:?}");
    }
}

#[test]
fn the_pad_window_moves_least_of_any_voicing_rule_that_closes_the_loop() {
    let s = v2();
    let lo = f(&s, "padlo", &[]);
    let chosen = window_motion(&s, lo);
    // Every window position in a sensible register (C3 .. C5).
    let best = (-21..=3).map(|l| window_motion(&s, l as f64)).fold(f64::INFINITY, f64::min);
    println!("pad window at {lo}: {chosen} semitones of movement per loop; best window {best}");
    assert_eq!(chosen, best, "padlo is not the least-moving window");

    // Greedy: from the same first chord, each next chord takes whichever close voicing (any window)
    // moves least from the one before. It cannot look ahead, so it drifts in register.
    let cands = |slot: usize| -> Vec<Vec<f64>> {
        (-30..12).map(|l| (0..4).map(|k| f(&s, "voicing", &[k as f64, (slot * 8) as f64, l as f64])).collect()).collect()
    };
    let start: Vec<f64> = (0..4).map(|k| f(&s, "voicing", &[k as f64, 0.0, lo])).collect();
    let (mut prev, mut greedy) = (start.clone(), 0.0);
    for i in 1..=32 {
        let next = cands(i % 32)
            .into_iter()
            .min_by(|a, b| {
                let d = |c: &Vec<f64>| c.iter().zip(&prev).map(|(x, y)| (x - y).abs()).sum::<f64>();
                d(a).partial_cmp(&d(b)).unwrap()
            })
            .unwrap();
        greedy += next.iter().zip(&prev).map(|(x, y)| (x - y).abs()).sum::<f64>();
        prev = next;
    }
    // Greedy moves less chord to chord, but it drifts: a loop must then jump back at the seam.
    let drift = prev.iter().zip(&start).map(|(x, y)| x - y).sum::<f64>() / 4.0;
    let back: f64 = prev.iter().zip(&start).map(|(x, y)| (x - y).abs()).sum();
    println!("greedy nearest voicing: {greedy} semitones per loop, ending {drift} semitones from where it began ({back} to get back)");
    assert!(drift != 0.0, "this progression makes greedy drift; if it no longer does, re-check the claim in the example");
    assert!(chosen < greedy + back, "the fixed window ({chosen}) should beat greedy plus its return ({})", greedy + back);
}

/// Pitch classes of the chord at step `st`: the root and its four upper tones.
fn chord_pcs(s: &Song, st: f64) -> Vec<i64> {
    let root = f(&s, "h", &[f(s, "deg", &[st])]);
    let mut pcs: Vec<i64> = (0..4).map(|j| (f(s, "tone", &[j as f64, st]) as i64).rem_euclid(12)).collect();
    pcs.push((root as i64).rem_euclid(12));
    pcs
}

#[test]
fn the_lead_is_on_a_chord_tone_on_every_beat_it_sounds() {
    let s = v2();
    let mut checked = 0;
    for st in (0..256).step_by(4) {
        let sf = st as f64;
        if f(&s, "leadd", &[sf]) == 0.0 {
            continue;
        }
        let onset = sf - f(&s, "lheld", &[sf]);
        for shift in [0.0, -1.0] {
            let p = f(&s, "leadnote", &[onset, shift]) as i64;
            assert!(chord_pcs(&s, sf).contains(&p.rem_euclid(12)), "step {st} (shift {shift}): pitch {p} is not in the chord");
        }
        checked += 1;
    }
    assert!(checked >= 20, "only {checked} beats checked");
}

#[test]
fn the_lead_rests_on_most_downbeats_and_its_hook_keeps_to_a_tenth() {
    let s = v2();
    let (mut sounding, mut hook, mut downbeats_rest, mut bars_with_lead) = (Vec::new(), Vec::new(), 0, 0);
    for st in 0..256 {
        let sf = st as f64;
        if f(&s, "leadd", &[sf]) != 0.0 {
            let p = f(&s, "leadnote", &[sf - f(&s, "lheld", &[sf]), 0.0]);
            sounding.push(p);
            if (64..128).contains(&st) {
                hook.push(p);
            }
        }
        if st % 16 == 0 && f(&s, "on", &[f(&s, "leads", &[]), sf]) != 0.0 {
            bars_with_lead += 1;
            if f(&s, "leadd", &[sf]) == 0.0 {
                downbeats_rest += 1;
            }
        }
    }
    let span = |v: &[f64]| v.iter().cloned().fold(f64::MIN, f64::max) - v.iter().cloned().fold(f64::MAX, f64::min);
    let (range, hook_range) = (span(&sounding), span(&hook));
    println!("lead range {range} semitones (hook, bars 5-8: {hook_range}); rests on {downbeats_rest} of {bars_with_lead} downbeats it could play");
    assert!(hook_range <= 16.0, "the hook spans {hook_range} semitones, more than a tenth");
    assert!(range <= 19.0, "the whole line spans {range}, more than a twelfth");
    assert!(downbeats_rest * 2 > bars_with_lead, "space: rest on most downbeats");
    // And it is not a stream: fewer than half the sixteenths start a note.
    let onsets = (0..256).filter(|&st| { let d = f(&s, "leadd", &[st as f64]); d != 0.0 && d != 9.0 }).count();
    assert!(onsets < 64, "{onsets} lead onsets in 256 sixteenths");
}

#[test]
fn the_bass_plays_the_root_on_each_downbeat_and_locks_with_the_kick() {
    let s = v2();
    for st in (0..256).step_by(16) {
        let sf = st as f64;
        if f(&s, "bassd", &[sf]) == 1.0 {
            let root = f(&s, "broot", &[sf]);
            assert_eq!(f(&s, "bassat", &[sf]), root, "bar {} downbeat", st / 16 + 1);
        }
    }
    for st in 0..256 {
        let sf = st as f64;
        if f(&s, "kickd", &[sf]) > 0.0 && st % 8 == 0 {
            let b = f(&s, "bassd", &[sf]);
            assert!(b != 0.0 && b != 9.0, "step {st}: a kick on beat 1 or 3 with no bass note starting under it");
        }
    }
}

#[test]
fn approach_notes_sit_a_semitone_below_the_next_chord_root() {
    let s = v2();
    let mut n = 0;
    for st in 0..256 {
        let sf = st as f64;
        if f(&s, "bassd", &[sf]) == 5.0 {
            assert_eq!(f(&s, "bassat", &[sf]), f(&s, "broot", &[sf + 1.0]) - 1.0, "step {st}");
            n += 1;
        }
    }
    assert!(n >= 16, "only {n} approach notes");
}


/// Prints RMS per bar of the rendered v2, to see the arrangement in the signal:
/// `cargo test --release --test example_music levels -- --nocapture --ignored`.
#[test]
#[ignore]
fn levels() {
    let m = luminesce::render::render(&v2()).unwrap();
    let bar = m.frames() / 16;
    for b in 0..16 {
        let w = &m.left[b * bar..(b + 1) * bar];
        let rms = (w.iter().map(|x| x * x).sum::<f64>() / w.len() as f64).sqrt();
        let pk = w.iter().fold(0.0f64, |a, x| a.max(x.abs()));
        println!("bar {:2}: rms {:6.1} dBFS  peak {:5.1} dBFS", b + 1, 20.0 * rms.log10(), 20.0 * pk.log10());
    }
}

// ---------------------------------------------------------------- the dark half-time example

const DARK: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/dark_halftime.txt");

fn dark() -> Song {
    Song::parse(&std::fs::read_to_string(DARK).unwrap()).unwrap_or_else(|e| panic!("{e}"))
}

/// `cargo test --release --test example_music dark_dump -- --nocapture --ignored`
#[test]
#[ignore]
fn dark_dump() {
    let s = dark();
    let names = ["C", "C#", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"];
    let nm = |p: f64| names[((p as i64 + 9 + 1200) % 12) as usize];
    for st in 0..256 {
        let sf = st as f64;
        let x = sf / 4.0 + 0.01;
        let c = f(&s, "b808d", &[sf]);
        let b = f(&s, "belld", &[sf]);
        println!(
            "bar {:2} st {:2} chord {:2} 808 {} {:3} K{} S{} hatrate {} bell {} {}",
            st / 16 + 1, st % 16, nm(f(&s, "croot", &[sf]) - 5.0), c,
            if c == 0.0 { "-" } else { nm(f(&s, "note8", &[f(&s, "b808d", &[f(&s, "onset8", &[sf])]), f(&s, "onset8", &[sf])]) - 5.0) },
            f(&s, "kickd", &[sf]), f(&s, "snared", &[sf]), f(&s, "hatrate", &[x]), b,
            if b == 0.0 || b == 9.0 { "" } else { nm(f(&s, "bellnote", &[x]) - 5.0) }
        );
    }
    let m = luminesce::render::render(&s).unwrap();
    let bar = m.frames() / 16;
    for b in 0..16 {
        let w = &m.left[b * bar..(b + 1) * bar];
        let rms = (w.iter().map(|x| x * x).sum::<f64>() / w.len() as f64).sqrt();
        println!("level bar {:2}: rms {:6.1} dBFS", b + 1, 20.0 * rms.log10());
    }
}

#[test]
fn the_808_slides_without_re_striking_and_drops_out_before_the_loop() {
    let s = dark();
    let mut slides = 0;
    for st in 1..256 {
        let sf = st as f64;
        if f(&s, "b808d", &[sf]) == 6.0 {
            slides += 1;
            // The gate is open through the end of the step before, so the slide is legato.
            let x = (sf - 1.0) / 4.0 + 0.24;
            assert!(f(&s, "gate808", &[x]) > 0.0, "step {st}: the gate closes before a slide");
            // And the pitch starts an octave below where the slide lands, then arrives.
            let start = f(&s, "p808", &[sf / 4.0]);
            let end = f(&s, "p808", &[sf / 4.0 + 0.2499]);
            assert!((end - start - 12.0).abs() < 0.01, "step {st}: slides {start} -> {end}");
        }
    }
    assert_eq!(slides, 5, "one slide in each main-pattern bar (5, 7, 9, 11, 15)");
    for st in 252..256 {
        assert_eq!(f(&s, "b808d", &[st as f64]), 0.0, "the 808 is silent on the last beat (step {st})");
    }
}

#[test]
fn every_struck_808_in_a_drum_bar_has_a_kick_under_it() {
    let s = dark();
    for st in 0..256 {
        let sf = st as f64;
        let drums = f(&s, "on", &[f(&s, "drums_on", &[]), sf]) > 0.0;
        let struck = f(&s, "struck", &[sf]) > 0.0;
        assert_eq!(f(&s, "kickd", &[sf]) > 0.0, drums && struck, "step {st}");
    }
}

#[test]
fn hat_rolls_come_only_at_the_ends_of_phrases() {
    let s = dark();
    for beat in 0..64 {
        let rolling = f(&s, "hatrate", &[beat as f64 + 0.5]) > 4.0;
        let last_bar_of_four = (beat / 4) % 4 == 3;
        assert!(!rolling || last_bar_of_four, "a roll on beat {beat}, not in a phrase's last bar");
    }
}

#[test]
fn both_new_examples_render_finite_loop_exact_and_under_the_ceiling() {
    for (path, frames) in [(V2, 1_843_200), (DARK, 1_280_000)] {
        let s = Song::parse(&std::fs::read_to_string(path).unwrap()).unwrap();
        let m = luminesce::render::render(&s).unwrap();
        assert_eq!(m.frames(), frames, "{path}");
        assert!(m.left.iter().chain(&m.right).all(|x| x.is_finite()), "{path}");
        assert!(m.peak <= luminesce::render::CEILING + 1e-12 && m.peak > 0.5, "{path}: peak {}", m.peak);
    }
}
