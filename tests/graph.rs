//! Graph-native instruments: additive voices (sums of sine partials, as a list of Desmos `tone()`
//! sines adds up to), the Desmos gain cap as a roll-off, and the dash (a sine whose frequency falls
//! exponentially: the graph kick). Measured from rendered audio wherever the claim is about sound.

use luminesce::render::{dash_cycles, dash_hz, render, Mix};
use luminesce::song::{Dash, Song};

const RATE: f64 = 48_000.0;

fn song(body: &str) -> Song {
    Song::parse(body).unwrap_or_else(|e| panic!("song failed: {e}"))
}

fn mix(body: &str) -> Mix {
    render(&song(body)).unwrap_or_else(|e| panic!("render failed: {e}"))
}

fn parse_err(body: &str) -> String {
    match Song::parse(body) {
        Ok(_) => panic!("expected an error"),
        Err(e) => e.to_string(),
    }
}

/// A held voice at `pitch` for 2 s with the given extra fields.
fn held(wave: &str, pitch: &str, fields: &str) -> String {
    format!("bpm 60\nbeats 2\nvoice s {{\n wave = {wave}\n pitch = {pitch}\n gate = 1\n attack = 0.1\n gain = 0.2\n{fields}\n}}\n")
}

/// Amplitude of the component at `hz` over samples [0.5 s, 1.5 s) of the left channel: one second,
/// so every whole-Hz frequency fits a whole number of cycles and there is no leakage.
fn amp_at(x: &[f64], hz: f64) -> f64 {
    let (a, n) = ((0.5 * RATE) as usize, RATE as usize);
    let (mut re, mut im) = (0.0, 0.0);
    for i in 0..n {
        let w = std::f64::consts::TAU * hz * i as f64 / RATE;
        re += x[a + i] * w.cos();
        im += x[a + i] * w.sin();
    }
    2.0 * (re * re + im * im).sqrt() / n as f64
}

fn db(x: f64) -> f64 {
    20.0 * x.log10()
}

#[test]
fn additive_partials_sound_at_whole_multiples_of_the_pitch_at_their_levels() {
    // Pitch -12 is A3, 220 Hz.
    let m = mix(&held("additive", "-12", " partials = 0, -6, -12"));
    let h1 = amp_at(&m.left, 220.0);
    let (h2, h3) = (db(amp_at(&m.left, 440.0) / h1), db(amp_at(&m.left, 660.0) / h1));
    assert!((h2 + 6.0).abs() < 0.02, "h2 at {h2} dB");
    assert!((h3 + 12.0).abs() < 0.02, "h3 at {h3} dB");
    for hz in [110.0, 330.0, 550.0, 880.0] {
        let r = db(amp_at(&m.left, hz) / h1);
        assert!(r < -90.0, "{hz} Hz is not a listed partial but sounds at {r} dB");
    }
}

#[test]
fn a_single_0_db_partial_is_the_sine_sample_for_sample() {
    let a = mix(&held("additive", "3.5 + sin(t)", " partials = 0"));
    let b = mix(&held("sine", "3.5 + sin(t)", ""));
    assert_eq!(a.left, b.left);
    assert_eq!(a.right, b.right);
}

#[test]
fn rolloff_scales_each_partial_by_the_corner_over_its_frequency() {
    // Three equal partials of A4 with the Desmos corner, 660 Hz: 440 Hz is below it (x1),
    // 880 Hz is scaled by 660/880 (-2.50 dB) and 1320 Hz by 1/2 (-6.02 dB).
    let m = mix(&held("additive", "0", " partials = 0, 0, 0\n rolloff = 660"));
    let h1 = amp_at(&m.left, 440.0);
    let (h2, h3) = (db(amp_at(&m.left, 880.0) / h1), db(amp_at(&m.left, 1320.0) / h1));
    assert!((h2 - db(660.0 / 880.0)).abs() < 0.02, "h2 at {h2} dB");
    assert!((h3 - db(0.5)).abs() < 0.02, "h3 at {h3} dB");
}

#[test]
fn rolloff_on_a_sine_makes_a_note_above_the_corner_quieter_by_corner_over_frequency() {
    let plain = amp_at(&mix(&held("sine", "0", "")).left, 440.0);
    let capped = amp_at(&mix(&held("sine", "0", " rolloff = 220")).left, 440.0);
    assert!((capped / plain - 0.5).abs() < 1e-6, "ratio {}", capped / plain);
    // Below the corner nothing changes.
    let low = mix(&held("sine", "-24", " rolloff = 660"));
    assert_eq!(low.left, mix(&held("sine", "-24", "")).left);
}

#[test]
fn partials_at_or_above_nyquist_are_left_out_so_nothing_aliases() {
    // 3100 Hz with ten equal partials: the 7th (21.7 kHz) sounds, the 8th to 10th (24.8, 27.9,
    // 31 kHz) are above 24 kHz and would fold to 23.2, 20.1 and 17.0 kHz. Nothing may sound there.
    let pitch = format!("{}", 12.0 * (3100.0f64 / 440.0).log2());
    let m = mix(&held("additive", &pitch, " partials = 0, 0, 0, 0, 0, 0, 0, 0, 0, 0"));
    let h1 = amp_at(&m.left, 3100.0);
    let h7 = db(amp_at(&m.left, 21_700.0) / h1);
    assert!(h7.abs() < 0.05, "h7 at {h7} dB");
    for hz in [23_200.0, 20_100.0, 17_000.0] {
        let r = db(amp_at(&m.left, hz) / h1);
        assert!(r < -90.0, "alias at {hz} Hz: {r} dB");
    }
}

#[test]
fn an_additive_fundamental_above_nyquist_is_still_an_error() {
    let src = held("additive", "60", " partials = 0, -6").replace("bpm 60", "bpm 60\nrate 8000");
    let e = render(&song(&src)).unwrap_err().to_string();
    assert!(e.contains("Nyquist"), "{e}");
}

fn kick() -> Dash {
    Dash {
        from_hz: 450.0,
        to_hz: 60.0,
        dash_ms: 23.0,
        hold_ms: 0.0,
        decay_ms: 150.0,
        length_ms: 400.0,
        partials: vec![1.0],
        rolloff_hz: 0.0,
    }
}

#[test]
fn a_dash_starts_at_from_falls_by_1_over_e_of_the_gap_per_dash_time_and_settles_at_to() {
    let k = kick();
    assert_eq!(dash_hz(&k, 0.0), 450.0);
    let at_k = 60.0 + 390.0 / std::f64::consts::E;
    assert!((dash_hz(&k, 0.023) - at_k).abs() < 1e-9, "{}", dash_hz(&k, 0.023));
    assert!((dash_hz(&k, 0.046) - (60.0 + 390.0 * (-2.0f64).exp())).abs() < 1e-9);
    assert!((dash_hz(&k, 1.0) - 60.0).abs() < 1e-9);
}

#[test]
fn a_dash_phase_is_the_integral_of_its_frequency() {
    let k = kick();
    assert_eq!(dash_cycles(&k, 0.0), 0.0);
    let h = 1e-6;
    for tau in [0.001, 0.01, 0.023, 0.05, 0.2] {
        let slope = (dash_cycles(&k, tau + h) - dash_cycles(&k, tau - h)) / (2.0 * h);
        assert!((slope - dash_hz(&k, tau)).abs() < 1e-4, "at {tau}: {slope} vs {}", dash_hz(&k, tau));
    }
}

/// A one-hit dash song: one hit at t = 0, 3 s long.
fn dash_song(fields: &str) -> String {
    format!("bpm 60\nbeats 3\ndrum d {{\n kind = dash\n trigger = 0.5 - t\n gain = 0.5\n{fields}\n}}\n")
}

/// Local frequency from rising zero crossings within +-`half` s of `at` (s).
fn local_hz(x: &[f64], at: f64, half: f64) -> f64 {
    let (a, b) = (((at - half) * RATE) as usize, ((at + half) * RATE) as usize);
    let mut c = Vec::new();
    for i in a.max(1)..b {
        if x[i - 1] < 0.0 && x[i] >= 0.0 {
            c.push((i - 1) as f64 + -x[i - 1] / (x[i] - x[i - 1]));
        }
    }
    (c.len() - 1) as f64 * RATE / (c[c.len() - 1] - c[0])
}

#[test]
fn a_rendered_dash_sounds_at_its_formula_frequency_at_set_times() {
    // Slowed down so a zero-crossing count can resolve it: 2000 Hz to 200 Hz, 200 ms dash time.
    let m = mix(&dash_song(" from = 2000\n to = 200\n dash = 200\n hold = 2000\n length = 2500"));
    let d = Dash { from_hz: 2000.0, to_hz: 200.0, dash_ms: 200.0, ..kick() };
    for tau in [0.05, 0.2, 0.4, 0.8] {
        let (got, want) = (local_hz(&m.left, tau, 0.005), dash_hz(&d, tau));
        assert!((got / want - 1.0).abs() < 0.003, "at {tau} s: {got} Hz, want {want}");
    }
}

#[test]
fn the_graph_kick_turns_its_formula_number_of_cycles_in_its_first_dash_time() {
    // The kick shape itself (450 -> 60 Hz, 23 ms): count rising zero crossings over the first 46 ms.
    let m = mix(&dash_song(""));
    let n = (0.046 * RATE) as usize;
    let rising = (1..n).filter(|&i| m.left[i - 1] < 0.0 && m.left[i] >= 0.0).count() as f64;
    let want = dash_cycles(&kick(), 0.046);
    assert!((rising - want).abs() <= 1.0, "{rising} rising crossings, {want} cycles");
}

#[test]
fn a_dash_holds_its_level_then_falls_by_1_over_e_per_decay() {
    // A flat dash (1000 Hz throughout) to read the envelope alone.
    let m = mix(&dash_song(" from = 1000\n to = 1000\n hold = 50\n decay = 100\n length = 1000"));
    let peak = |at: f64| {
        let a = (at * RATE) as usize;
        m.left[a..a + 48].iter().fold(0.0f64, |p, x| p.max(x.abs()))
    };
    let held = peak(0.01);
    assert!((peak(0.045) / held - 1.0).abs() < 0.001, "not held: {}", peak(0.045) / held);
    let r = peak(0.15) / held;
    assert!((r - (-1.0f64).exp()).abs() < 0.01, "after hold + decay: {r}");
}

#[test]
fn dash_harmonics_that_would_reach_nyquist_are_left_out() {
    // From 5 kHz the 5th and 6th harmonics would start at 25 and 30 kHz: listing them changes nothing.
    let four = mix(&dash_song(" from = 5000\n to = 100\n partials = 0, -6, -9, -12"));
    let six = mix(&dash_song(" from = 5000\n to = 100\n partials = 0, -6, -9, -12, 0, 0"));
    assert_eq!(four.left, six.left);
    assert!(four.left.iter().all(|x| x.is_finite()));
}

#[test]
fn a_dash_reaching_nyquist_is_an_error() {
    let e = render(&song(&dash_song(" from = 5000").replace("bpm 60", "bpm 60\nrate 8000"))).unwrap_err().to_string();
    assert!(e.contains("Nyquist"), "{e}");
}

#[test]
fn graph_fields_are_checked_where_they_are_written() {
    let cases = [
        (held("saw", "0", " partials = 0, -6"), "needs `wave = additive`"),
        (held("additive", "0", ""), "needs `partials`"),
        (held("additive", "0", " partials = 0, loud"), "not a number of dB"),
        (held("additive", "0", " partials = 0, -200"), "between -120 and 20 dB"),
        (held("additive", "0", " partials = 0,, -6"), "not a number of dB"),
        (held("square", "0", " rolloff = 660"), "needs `wave = sine` or `wave = additive`"),
        (held("sine", "0", " rolloff = 0.5"), "0 (off) or between 1 and 20000 Hz"),
        (dash_song(" from = 0"), "between 1 and 20000 Hz"),
        (dash_song(" dash = 0"), "between 0.1 and 10000 ms"),
        ("bpm 60\nbeats 1\ndrum k {\n kind = kick\n trigger = 1\n from = 400\n}\n".to_string(), "needs `kind = dash`"),
    ];
    for (src, want) in cases {
        let e = parse_err(&src);
        assert!(e.contains(want), "expected '{want}', got: {e}");
    }
    let many = vec!["-6"; 65].join(", ");
    assert!(parse_err(&held("additive", "0", &format!(" partials = {many}"))).contains("the limit is 64"));
}
