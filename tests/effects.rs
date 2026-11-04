//! Send effects: the reverb decays and stays finite, the delay lands on the beat to the sample,
//! looped tails wrap, wow bends pitch by what it says, and lengths stay exact.

use luminesce::effects::{Freeverb, PingPong};
use luminesce::render::{render, Mix};
use luminesce::song::Song;

fn mix(src: &str) -> Mix {
    render(&Song::parse(src).unwrap_or_else(|e| panic!("{e}"))).unwrap_or_else(|e| panic!("{e}"))
}

fn energy(x: &[f64]) -> f64 {
    x.iter().map(|v| v * v).sum()
}

#[test]
fn the_reverb_decays_and_stays_finite_even_at_full_size() {
    for size in [0.0, 0.5, 1.0] {
        let mut verb = Freeverb::new(48_000.0, size, 0.5, 1.0);
        let n = 48_000 * 20;
        let mut l = Vec::with_capacity(n);
        for i in 0..n {
            let (a, b) = verb.tick(if i == 0 { 1.0 } else { 0.0 });
            assert!(a.is_finite() && b.is_finite(), "size {size}: not finite at {i}");
            l.push(a);
        }
        // Energy per half second, after the first 0.2 s of early reflections, only falls.
        let windows: Vec<f64> = l[9600..].chunks(24_000).map(energy).collect();
        for (k, w) in windows.windows(2).enumerate() {
            assert!(w[1] < w[0], "size {size}: window {} has more energy than {k}", k + 1);
        }
        // And it dies away: the last window is more than 60 dB below the loudest.
        let loudest = windows.iter().cloned().fold(0.0, f64::max);
        let last = *windows.last().unwrap();
        assert!(10.0 * (last / loudest).log10() < -60.0, "size {size}: tail at {:.1} dB", 10.0 * (last / loudest).log10());
    }
}

#[test]
fn a_bigger_room_rings_longer() {
    let tail = |size: f64| {
        let mut v = Freeverb::new(48_000.0, size, 0.5, 1.0);
        let x: Vec<f64> = (0..96_000).map(|i| v.tick(if i == 0 { 1.0 } else { 0.0 }).0).collect();
        energy(&x[48_000..]) / energy(&x)
    };
    assert!(tail(0.9) > 2.0 * tail(0.3), "{} vs {}", tail(0.9), tail(0.3));
}

#[test]
fn the_ping_pong_repeats_left_then_right_at_exact_multiples_of_its_length() {
    let mut d = PingPong::new(1000, 0.5, 20_000.0, 48_000.0);
    let out: Vec<(f64, f64)> = (0..3500).map(|i| d.tick(if i == 7 { 1.0 } else { 0.0 })).collect();
    let first = |ch: usize| out.iter().position(|o| (if ch == 0 { o.0 } else { o.1 }).abs() > 0.0).unwrap();
    assert_eq!(first(0), 1007, "the first repeat, on the left");
    assert_eq!(out[1007].0, 1.0, "the first repeat is the send itself");
    assert_eq!(first(1), 2007, "the second, on the right");
    // Each later repeat is quieter.
    let peak = |a: usize, b: usize, ch: usize| out[a..b].iter().map(|o| if ch == 0 { o.0 } else { o.1 }.abs()).fold(0.0, f64::max);
    assert!(peak(3000, 3500, 0) < peak(1000, 1500, 0));
}

/// A short sine burst on a beat, sent to the delay.
fn burst(on_beat: f64, send: f64, extra: &str) -> String {
    format!(
        "bpm 100\nbeats 8\n{extra}\nvoice v {{\n wave = sine\n pitch = 0\n gate = if(min(t >= {on_beat}, t < {on_beat} + 0.1), 1, -1)\n gain = 0.2\n delay = {send}\n}}\n"
    )
}

#[test]
fn the_delay_lands_on_the_beat_to_the_sample() {
    // At 100 bpm a dotted eighth (0.75 beat) is 0.45 s = 21600 samples exactly.
    let dry = mix(&burst(1.0, 0.0, ""));
    let wet = mix(&burst(1.0, 0.5, "delay_beats 0.75\ndelay_feedback 0.4"));
    let start = dry.left.iter().position(|x| x.abs() > 0.0).unwrap();
    // The gate opens on beat 1 (sample 28800); a sine starts at sin(0) = 0, so it is heard from the next.
    assert_eq!(start, 28_801, "the burst starts on beat 1");
    let diff = |ch: &[f64], d: &[f64]| ch.iter().zip(d).position(|(a, b)| (a - b).abs() > 0.0).unwrap();
    assert_eq!(diff(&wet.left, &dry.left), start + 21_600, "the first repeat is at beat 1.75");
    assert_eq!(diff(&wet.right, &dry.right), start + 43_200, "the second is at beat 2.5, on the right");
    // A half-beat delay at 100 bpm: 14400 samples.
    let wet = mix(&burst(1.0, 0.5, "delay_beats 0.5"));
    assert_eq!(diff(&wet.left, &dry.left), start + 14_400);
}

#[test]
fn a_looped_song_hears_its_own_tail_at_the_start_and_keeps_its_length() {
    // A note in the last beat only, with reverb: looped, the room is already ringing at sample 0.
    let src = |looped: u8| {
        format!(
            "bpm 120\nbeats 8\nloop {looped}\nreverb_size 0.8\nvoice v {{\n wave = saw\n pitch = 0\n gate = if(t >= 7, 0.5 - frac(t), -1)\n gain = 0.2\n reverb = 0.8\n}}\n"
        )
    };
    let cut = mix(&src(0));
    let looped = mix(&src(1));
    assert_eq!(cut.frames(), 192_000);
    assert_eq!(looped.frames(), 192_000);
    assert!(cut.left[..1000].iter().all(|&x| x == 0.0), "unlooped, the start is silent");
    assert!(energy(&looped.left[..24_000]) > 0.0, "looped, the tail rings over the start");
    assert!(looped.left.iter().chain(&looped.right).all(|x| x.is_finite()));
}

#[test]
fn songs_without_effects_build_no_buses_and_render_unchanged() {
    let src = "bpm 60\nbeats 1\nvoice v {\n wave = saw\n pitch = 0\n gate = 1\n gain = 0.3\n}\n";
    let a = mix(src);
    let b = mix(&src.replace("bpm 60", "bpm 60\nreverb_size 0.9\ndelay_beats 1"));
    assert_eq!(a.left, b.left, "effect settings without sends change nothing");
}

/// Instantaneous frequency from successive rising zero crossings.
fn local_hz(x: &[f64], rate: f64) -> Vec<f64> {
    let mut c = Vec::new();
    for i in 1..x.len() {
        if x[i - 1] < 0.0 && x[i] >= 0.0 {
            c.push((i - 1) as f64 + -x[i - 1] / (x[i] - x[i - 1]));
        }
    }
    c.windows(2).map(|w| rate / (w[1] - w[0])).collect()
}

#[test]
fn wow_bends_the_pitch_by_its_depth_in_cents() {
    let src = "bpm 60\nbeats 4\nwow 20\nwow_rate 1\nvoice v {\n wave = sine\n pitch = 0\n gate = 1\n gain = 0.3\n attack = 1\n}\n";
    let m = mix(src);
    let hz = local_hz(&m.left[4800..192_000 - 4800], 48_000.0);
    let cents: Vec<f64> = hz.iter().map(|f| 1200.0 * (f / 440.0).log2()).collect();
    let (lo, hi) = cents.iter().fold((f64::MAX, f64::MIN), |(l, h), &c| (l.min(c), h.max(c)));
    assert!((hi - 20.0).abs() < 1.5 && (lo + 20.0).abs() < 1.5, "pitch swings {lo:.2} .. {hi:.2} cents");
}

#[test]
fn looped_wow_completes_whole_cycles_over_the_song() {
    let fx = Song::parse("bpm 60\nbeats 10\nloop 1\nwow 5\nwow_rate 0.44\nvoice v {\n wave = sine\n pitch = 0\n gate = 1\n}\n").unwrap().effects;
    let f = luminesce::effects::wow_rate(&fx, 10.0);
    assert!((f * 10.0 - (f * 10.0).round()).abs() < 1e-12 && (f - 0.4).abs() < 1e-12, "{f}");
}

#[test]
fn warmth_saturates_softly_and_stays_finite_and_under_the_ceiling() {
    let src = "bpm 60\nbeats 1\nwarmth 1\nvoice v {\n wave = saw\n pitch = 0\n gate = 1\n gain = 2\n}\n";
    let m = mix(src);
    assert!(m.left.iter().all(|x| x.is_finite()));
    assert!(m.peak <= luminesce::render::CEILING + 1e-12);
}

#[test]
fn effect_settings_are_range_checked() {
    let base = "bpm 60\nbeats 1\nvoice v {\n wave = sine\n pitch = 0\n gate = 1\n}\n";
    for (bad, msg) in [
        ("reverb_size 2", "`reverb_size` must be between 0 and 1"),
        ("delay_feedback 1", "`delay_feedback` must be between 0 and 0.95"),
        ("loop 2", "`loop` must be 0 or 1"),
        ("wow -1", "`wow` must be between 0 and 100"),
    ] {
        let e = Song::parse(&format!("{bad}\n{base}")).unwrap_err();
        assert!(e.message.contains(msg), "{bad}: {e}");
    }
    let e = Song::parse(&base.replace("gate = 1", "gate = 1\n reverb = 1.5")).unwrap_err();
    assert!(e.message.contains("`reverb` must be between 0 and 1"), "{e}");
}

#[test]
fn renders_with_every_effect_are_byte_identical() {
    let src = "bpm 100\nbeats 8\nloop 1\nwow 6\nwarmth 0.4\nreverb_size 0.7\nreverb_predelay 20\ndrum s {\n kind = snare\n trigger = 0.5 - frac(t)\n reverb = 0.5\n delay = 0.3\n}\nvoice v {\n wave = saw\n pitch = 3 + vibrato(note_age, 0.1, 0.2, 0.3, 5)\n gate = 0.5 - frac(2t)\n detune = 8\n reverb = 0.4\n delay = 0.3\n}\n";
    let a = mix(src);
    let b = mix(src);
    assert!(a.left == b.left && a.right == b.right);
}

#[test]
fn the_reverb_predelay_shifts_the_room_by_exactly_its_length() {
    // The room's first sound comes `reverb_predelay` ms later: 10 ms is 480 samples at 48 kHz.
    let song = |pre: &str| format!("bpm 60\nbeats 2\n{pre}\nvoice v {{\n wave = saw\n pitch = 0\n gate = 0.2 - t\n gain = 0.2\n reverb = 0.5\n}}\n");
    let dry = mix(&song("").replace(" reverb = 0.5\n", ""));
    let first_wet = |m: &Mix| m.left.iter().zip(&dry.left).position(|(a, b)| (a - b).abs() > 0.0).unwrap();
    let a = first_wet(&mix(&song("")));
    let b = first_wet(&mix(&song("reverb_predelay 10")));
    assert_eq!(b - a, 480, "the room starts at {a} without predelay and {b} with 10 ms");
}
