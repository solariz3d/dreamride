//! Expressive performance: the phrase arch, phrase time, the ritard, melodic charge, vibrato, and
//! the note-time variables that let a pitch know where it is inside its note.

use luminesce::expr::Program;
use luminesce::expressive::{arch, charge, phrase, ritard, smooth, vibrato};
use luminesce::render::render;
use luminesce::song::Song;

#[test]
fn the_arch_is_zero_at_the_phrase_ends_and_one_at_its_peak() {
    for p in [0.3, 0.5, 0.62, 0.8] {
        assert_eq!(arch(0.0, p), 0.0);
        assert_eq!(arch(1.0, p), 0.0);
        assert_eq!(arch(p, p), 1.0);
        // It rises all the way to the peak and falls all the way after it.
        let xs: Vec<f64> = (0..=1000).map(|i| i as f64 / 1000.0).collect();
        for w in xs.windows(2) {
            let (a, b) = (arch(w[0], p), arch(w[1], p));
            if w[1] <= p {
                assert!(b > a, "p {p}: not rising at {}", w[0]);
            } else if w[0] >= p {
                assert!(b < a, "p {p}: not falling at {}", w[0]);
            }
        }
        let (argmax, _) = xs.iter().map(|&x| (x, arch(x, p))).fold((0.0, -1.0), |m, v| if v.1 > m.1 { v } else { m });
        assert!((argmax - p).abs() <= 0.001, "p {p}: the maximum is at {argmax}");
    }
    // Outside the phrase it stays at its ends rather than going negative.
    assert_eq!(arch(-0.5, 0.5), 0.0);
    assert_eq!(arch(1.5, 0.5), 0.0);
}

#[test]
fn phrase_time_keeps_every_phrase_its_written_length_and_never_reorders_notes() {
    for depth in [0.0, 0.1, 0.5, 1.0] {
        for n in -2..5 {
            let c = n as f64;
            assert!((phrase(c, depth, 0.6) - c).abs() < 1e-12, "depth {depth}: phrase boundary {c} moved");
        }
        let mut prev = phrase(0.0, depth, 0.6);
        for i in 1..=4000 {
            let v = phrase(i as f64 / 1000.0, depth, 0.6);
            assert!(v > prev, "depth {depth}: not strictly increasing at {}", i as f64 / 1000.0);
            prev = v;
        }
    }
}

#[test]
fn phrase_time_is_slow_at_the_ends_and_fast_at_the_peak_by_the_arch() {
    let (d, p, h) = (0.3, 0.6, 1e-6);
    let tempo = |c: f64| (phrase(c + h, d, p) - phrase(c - h, d, p)) / (2.0 * h);
    // Tempo = 1 + depth (arch - 2/3): 1 - 2d/3 at the ends, 1 + d/3 at the peak.
    assert!((tempo(0.0 + 2.0 * h) - (1.0 - 2.0 * d / 3.0)).abs() < 1e-4, "{}", tempo(2.0 * h));
    assert!((tempo(1.0 - 2.0 * h) - (1.0 - 2.0 * d / 3.0)).abs() < 1e-4, "{}", tempo(1.0 - 2.0 * h));
    assert!((tempo(p) - (1.0 + d / 3.0)).abs() < 1e-4, "{}", tempo(p));
    // Final lengthening: the phrase's last eighth (of written time) takes longer than an eighth of the clock.
    let clock_for = |a: f64, b: f64| {
        // Invert the warp by bisection: the clock time at which written time `w` is reached.
        let inv = |w: f64| {
            let (mut lo, mut hi) = (0.0, 1.0);
            for _ in 0..60 {
                let m = 0.5 * (lo + hi);
                if phrase(m, d, p) < w { lo = m } else { hi = m }
            }
            lo
        };
        inv(b) - inv(a)
    };
    assert!(clock_for(0.875, 1.0) > 0.125 * 1.05, "the last eighth is not lengthened: {}", clock_for(0.875, 1.0));
    assert!(clock_for(0.5, 0.625) < 0.125, "the middle is not pushed: {}", clock_for(0.5, 0.625));
}

#[test]
fn the_ritard_starts_in_tempo_ends_at_its_final_tempo_and_only_slows() {
    for q in [1.0, 2.0, 3.0] {
        assert!((ritard(0.0, q, 0.5) - 1.0).abs() < 1e-12);
        assert!((ritard(1.0, q, 0.5) - 0.5).abs() < 1e-12, "q {q}");
        let mut prev = 1.0;
        for i in 1..=100 {
            let v = ritard(i as f64 / 100.0, q, 0.5);
            assert!(v < prev, "q {q}: speeds up at {i}");
            prev = v;
        }
    }
    // q = 1 is linear in position; larger q holds the tempo longer and brakes later.
    assert!((ritard(0.5, 1.0, 0.5) - 0.75).abs() < 1e-12);
    assert!(ritard(0.5, 3.0, 0.5) > ritard(0.5, 2.0, 0.5) && ritard(0.5, 2.0, 0.5) > ritard(0.5, 1.0, 0.5));
}

#[test]
fn melodic_charge_follows_the_kth_table_around_the_circle_of_fifths() {
    let want = [(0.0, 0.0), (7.0, 1.0), (2.0, 2.0), (9.0, 3.0), (4.0, 4.0), (11.0, 5.0), (6.0, 6.0), (1.0, 6.5), (8.0, 5.5), (3.0, 4.5), (10.0, 3.5), (5.0, 2.5)];
    for (semis, c) in want {
        assert_eq!(charge(semis), c, "{semis} semitones");
        assert_eq!(charge(semis + 24.0), c, "two octaves up");
        assert_eq!(charge(semis - 12.0), c, "an octave down");
    }
    assert!(charge(f64::NAN).is_nan());
}

#[test]
fn vibrato_waits_for_its_delay_then_grows_to_its_depth() {
    let (delay, ramp, depth, rate) = (0.25, 0.4, 0.3, 5.5);
    for i in 0..=250 {
        assert_eq!(vibrato(i as f64 / 1000.0, delay, ramp, depth, rate), 0.0, "sounds before its delay");
    }
    // It starts from zero (no jump at the delay) and stays inside its envelope while it grows.
    assert!(vibrato(delay + 1e-6, delay, ramp, depth, rate).abs() < 1e-6);
    for i in 0..400 {
        let a = delay + i as f64 / 1000.0;
        assert!(vibrato(a, delay, ramp, depth, rate).abs() <= depth * smooth((a - delay) / ramp) + 1e-12);
    }
    // After the ramp, one cycle swings to the full depth both ways.
    let cycle: Vec<f64> = (0..2000).map(|i| vibrato(delay + ramp + 0.1 + i as f64 / 2000.0 / rate, delay, ramp, depth, rate)).collect();
    let (lo, hi) = cycle.iter().fold((f64::MAX, f64::MIN), |(l, h), &v| (l.min(v), h.max(v)));
    assert!((hi - depth).abs() < 1e-5 && (lo + depth).abs() < 1e-5, "swing {lo} .. {hi}");
}

#[test]
fn the_expressive_functions_are_built_ins_of_the_language() {
    let p = Program::from_source("").unwrap();
    let ev = |s: &str| p.compile_expr(s).unwrap_or_else(|e| panic!("{s}: {e}")).eval(&p, 0.0);
    assert_eq!(ev("arch(0.6, 0.6)"), 1.0);
    assert_eq!(ev("phrase(2, 0.5, 0.6)"), 2.0);
    assert_eq!(ev("ritard(1, 2, 0.4)"), 0.4);
    assert_eq!(ev("charge(16)"), 4.0);
    assert!((ev("db(-6)") - 0.501_187_233_627_272_2).abs() < 1e-15);
    assert_eq!(ev("smooth(2)"), 1.0);
    assert_eq!(ev("vibrato(0.1, 0.2, 0.3, 0.5, 6)"), 0.0);
    assert!(p.compile_expr("vibrato(1, 2)").unwrap_err().message.contains("takes 5 arguments"));
    assert!(Program::from_source("arch = 3").is_err(), "a built-in cannot be redefined");
    assert!(Program::from_source("note_age = 3").is_err(), "nor can a note-time variable");
}

// ---------------------------------------------------------------- note time in a render

fn render_song(src: &str) -> luminesce::render::Mix {
    render(&Song::parse(src).unwrap_or_else(|e| panic!("{e}"))).unwrap_or_else(|e| panic!("{e}"))
}

/// Two half-second notes at 0 s and 1 s, with `amp` given.
fn two_notes(amp: &str) -> String {
    format!("bpm 60\nbeats 2\nvoice v {{\n wave = sine\n pitch = 0\n gate = 0.5 - frac(t)\n amp = {amp}\n gain = 0.5\n attack = 0.1\n release = 0.1\n}}\n")
}

fn rms(x: &[f64]) -> f64 {
    (x.iter().map(|v| v * v).sum::<f64>() / x.len() as f64).sqrt()
}

#[test]
fn note_age_restarts_at_every_onset() {
    // Silent for the first 0.1 s of each note, then sounding.
    let m = render_song(&two_notes("if(note_age < 0.1, 0, 1)"));
    let s = |a: f64, b: f64| rms(&m.left[(a * 48_000.0) as usize..(b * 48_000.0) as usize]);
    assert_eq!(s(0.0, 0.099), 0.0, "first note, before 0.1 s");
    assert!(s(0.15, 0.35) > 0.1, "first note, after 0.1 s");
    assert_eq!(s(1.0, 1.099), 0.0, "second note, before 0.1 s of ITS age");
    assert!(s(1.15, 1.35) > 0.1);
}

#[test]
fn note_left_counts_down_to_the_gate_closing() {
    // Silent for the last 0.1 s before each gate closes (0.4 .. 0.5 s), sounding before that.
    let m = render_song(&two_notes("if(note_left < 0.1, 0, 1)"));
    let s = |a: f64, b: f64| rms(&m.left[(a * 48_000.0) as usize..(b * 48_000.0) as usize]);
    assert!(s(0.1, 0.39) > 0.1);
    assert_eq!(s(0.401, 0.5), 0.0, "the last 0.1 s of the first note");
    assert!(s(1.1, 1.39) > 0.1);
    assert_eq!(s(1.401, 1.5), 0.0, "the last 0.1 s of the second note");
}

#[test]
fn a_gate_or_trigger_that_reads_the_note_is_an_error() {
    let e = Song::parse("bpm 60\nbeats 1\nvoice v {\n wave = sine\n pitch = 0\n gate = 1 - note_age\n}\n").unwrap_err();
    assert!(e.message.contains("only pitch, amp and cutoff"), "{e}");
    // Also through a definition.
    let e = Song::parse("bpm 60\nbeats 1\nlate(x) = x + note_left\ndrum d {\n kind = hat\n trigger = late(t)\n}\n").unwrap_err();
    assert!(e.message.contains("only pitch, amp and cutoff"), "{e}");
}

#[test]
fn a_definition_that_reads_the_note_is_not_cached_across_notes() {
    // `lift(k)` takes an argument and does not read t, but it reads note_age, so it must not be
    // treated as a pure function of k: the same k gives different values in different notes.
    let m = render_song(&two_notes("lift(1)").replace("bpm 60", "bpm 60\nlift(k) = k * if(note_age < 0.1, 0, 1)"));
    let s = |a: f64, b: f64| rms(&m.left[(a * 48_000.0) as usize..(b * 48_000.0) as usize]);
    assert_eq!(s(1.0, 1.099), 0.0, "the second note's start reused a cached value from the first");
    assert!(s(1.15, 1.35) > 0.1);
}

#[test]
fn detune_beats_at_the_difference_of_its_two_oscillators() {
    // 20 cents apart at 440 Hz: the two sines beat at 440 (2^(10/1200) - 2^(-10/1200)) = 5.08 Hz.
    let src = "bpm 60\nbeats 2\nvoice v {\n wave = sine\n pitch = 0\n gate = 1\n gain = 0.5\n detune = 20\n}\n";
    let m = render_song(src);
    let win = 480; // 10 ms
    let env: Vec<f64> = m.left[4800..96_000 - 4800].chunks(win).map(rms).collect();
    let (lo, hi) = env.iter().fold((f64::MAX, 0.0f64), |(l, h), &v| (l.min(v), h.max(v)));
    assert!(lo < 0.1 * hi, "no beating: {lo} .. {hi}");
    // Count the envelope's minima over 1.8 s: about 5.08 beats per second.
    let minima = (1..env.len() - 1).filter(|&i| env[i] < env[i - 1] && env[i] <= env[i + 1] && env[i] < 0.3 * hi).count();
    let want = 440.0 * (2f64.powf(10.0 / 1200.0) - 2f64.powf(-10.0 / 1200.0)) * env.len() as f64 * 0.01;
    assert!((minima as f64 - want).abs() <= 1.0, "{minima} beats, want about {want:.1}");
}

#[test]
fn vibrato_with_no_ramp_starts_at_full_depth_but_still_waits_for_its_delay() {
    for i in 0..=200 {
        assert_eq!(vibrato(i as f64 / 1000.0, 0.2, 0.0, 0.3, 5.0), 0.0, "sounds before its delay at {i} ms");
    }
    let quarter = 0.2 + 0.25 / 5.0; // a quarter cycle after the delay: the first crest
    assert!((vibrato(quarter, 0.2, 0.0, 0.3, 5.0) - 0.3).abs() < 1e-12);
}
