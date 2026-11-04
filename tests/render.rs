//! Synthesis and mixing: pitch accuracy, envelopes, drums, headroom, NaN reporting, determinism.

use luminesce::render::{render, Mix, CEILING};
use luminesce::song::Song;

fn song(body: &str) -> Song {
    Song::parse(body).unwrap_or_else(|e| panic!("song failed: {e}"))
}

fn mix(body: &str) -> Mix {
    render(&song(body)).unwrap_or_else(|e| panic!("render failed: {e}"))
}

fn sine_song(pitch: &str, extra: &str) -> String {
    format!("bpm 60\nbeats 1\n{extra}\nvoice s {{\n wave = sine\n pitch = {pitch}\n gate = 1\n gain = 0.5\n}}\n")
}

/// Frequency from rising zero crossings (linearly interpolated), measured from the first to the
/// last crossing, skipping the attack.
fn measure_hz(x: &[f64], rate: f64) -> f64 {
    let mut crossings = Vec::new();
    for i in 1..x.len() {
        if x[i - 1] < 0.0 && x[i] >= 0.0 {
            let frac = -x[i - 1] / (x[i] - x[i - 1]);
            crossings.push((i - 1) as f64 + frac);
        }
    }
    assert!(crossings.len() > 10, "too few crossings: {}", crossings.len());
    let (a, b) = (crossings[2], *crossings.last().unwrap());
    (crossings.len() - 3) as f64 * rate / (b - a)
}

#[test]
fn pitch_zero_is_440_hz() {
    let m = mix(&sine_song("0", ""));
    let hz = measure_hz(&m.left, 48_000.0);
    assert!((hz - 440.0).abs() < 0.5, "measured {hz} Hz");
}

#[test]
fn twelve_semitones_is_an_octave_and_seven_is_a_fifth() {
    let hz = measure_hz(&mix(&sine_song("12", "")).left, 48_000.0);
    assert!((hz - 880.0).abs() < 0.5, "measured {hz} Hz");
    let hz = measure_hz(&mix(&sine_song("-12", "")).left, 48_000.0);
    assert!((hz - 220.0).abs() < 0.5, "measured {hz} Hz");
    let hz = measure_hz(&mix(&sine_song("7", "")).left, 48_000.0);
    let fifth = 440.0 * 2f64.powf(7.0 / 12.0);
    assert!((hz - fifth).abs() < 0.5, "measured {hz} Hz, want {fifth}");
}

#[test]
fn the_reference_setting_moves_pitch_zero() {
    let hz = measure_hz(&mix(&sine_song("0", "reference 432")).left, 48_000.0);
    assert!((hz - 432.0).abs() < 0.5, "measured {hz} Hz");
}

#[test]
fn every_wave_measures_at_its_pitch() {
    for wave in ["triangle", "saw", "square"] {
        let src = sine_song("0", "").replace("wave = sine", &format!("wave = {wave}"));
        let hz = measure_hz(&mix(&src).left, 48_000.0);
        assert!((hz - 440.0).abs() < 0.5, "{wave}: measured {hz} Hz");
    }
}

#[test]
fn the_buffer_has_the_exact_sample_count() {
    let s = song(&sine_song("0", "").replace("bpm 60", "bpm 7"));
    let m = render(&s).unwrap();
    assert_eq!(m.left.len(), s.sample_count());
    assert_eq!(m.right.len(), s.sample_count());
    assert_eq!(m.left.len(), 411_429);
}

#[test]
fn a_centred_voice_is_equal_in_both_channels_and_pan_moves_it() {
    let m = mix(&sine_song("0", ""));
    assert_eq!(m.left, m.right);
    let m = mix(&sine_song("0", "").replace("gain = 0.5", "gain = 0.5\n pan = -1"));
    assert!(m.right.iter().all(|&x| x.abs() < 1e-12), "hard left leaves the right channel silent");
    assert!(m.left.iter().any(|&x| x.abs() > 0.1));
}

#[test]
fn onsets_start_from_silence_and_ramp_over_the_attack() {
    // A saw starts at -1, so without an attack the first sample would jump to full level.
    let src = "bpm 60\nbeats 1\nvoice s {\n wave = saw\n pitch = 0\n gate = t - 0.5\n gain = 1\n attack = 5\n}\n";
    let m = mix(src);
    let onset = 24_000; // gate goes above 0 just after t = 0.5 beat = 0.5 s
    assert!(m.left[..=onset].iter().all(|&x| x == 0.0), "silent while the gate is not above 0");
    let step = 1.0 / (0.005 * 48_000.0);
    for k in 1..240 {
        let x = m.left[onset + k].abs();
        assert!(x <= k as f64 * step * 1.05, "sample {k} after the onset is {x}, above the attack ramp");
    }
    assert!(m.left[onset + 240..onset + 400].iter().any(|&x| x.abs() > 0.5), "full level after the attack");
}

#[test]
fn offsets_release_to_exact_silence() {
    let src = "bpm 60\nbeats 1\nvoice s {\n wave = square\n pitch = 0\n gate = 0.5 - t\n gain = 1\n release = 10\n}\n";
    let m = mix(src);
    let off = 24_000;
    let release = (0.010 * 48_000.0) as usize;
    let jumps = m.left[off - 5..off + release].windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0, f64::max);
    assert!(jumps < 2.05, "a square wave's own edges are 2; the release must not add a cliff ({jumps})");
    assert!(m.left[off + release + 2..].iter().all(|&x| x == 0.0), "silent after the release");
    assert!(m.left[off - 100..off].iter().any(|&x| x.abs() > 0.5), "still sounding before the gate closes");
}

#[test]
fn a_drum_sounds_only_after_its_trigger_rises() {
    for kind in ["kick", "snare", "hat"] {
        let src = format!("bpm 60\nbeats 2\ndrum d {{\n kind = {kind}\n trigger = t - 1\n}}\n");
        let m = mix(&src);
        assert!(m.left[..48_000].iter().all(|&x| x == 0.0), "{kind}: silent before the trigger");
        assert!(m.left[48_000..49_000].iter().any(|&x| x.abs() > 0.01), "{kind}: sounds after the trigger");
        assert!(m.left[48_000 + 40_000..].iter().all(|&x| x == 0.0), "{kind}: the hit has ended within 0.8 s");
    }
}

#[test]
fn a_drum_that_never_rises_is_silent() {
    let m = mix("bpm 60\nbeats 1\ndrum d {\n kind = kick\n trigger = -1\n}\n");
    assert!(m.left.iter().all(|&x| x == 0.0));
    assert_eq!(m.peak, 0.0);
}

#[test]
fn one_hit_per_rising_edge() {
    // Four rising edges in four beats: the start of each beat.
    let m = mix("bpm 60\nbeats 4\ndrum d {\n kind = snare\n trigger = 0.25 - frac(t)\n}\n");
    let starts: Vec<usize> = (1..m.left.len())
        .filter(|&i| m.left[i - 1] == 0.0 && m.left[i] != 0.0 && (i < 10 || m.left[i - 10..i].iter().all(|&x| x == 0.0)))
        .collect();
    assert_eq!(starts.len(), 4, "hit starts: {starts:?}");
}

#[test]
fn a_loud_mix_is_normalised_to_the_ceiling_and_reported() {
    let mut src = String::from("bpm 60\nbeats 1\n");
    for k in 0..6 {
        src += &format!("voice v{k} {{\n wave = square\n pitch = 0\n gate = 1\n gain = 5\n}}\n");
    }
    let m = mix(&src);
    assert!(m.raw_peak > 1.0, "the test needs a mix that would clip ({})", m.raw_peak);
    assert!(m.gain < 1.0);
    assert!((m.peak - CEILING).abs() < 1e-12, "peak {} vs ceiling {CEILING}", m.peak);
    let measured = m.left.iter().chain(&m.right).fold(0.0f64, |a, &x| a.max(x.abs()));
    assert!((measured - m.peak).abs() < 1e-12);
}

#[test]
fn a_quiet_mix_is_left_alone() {
    let m = mix(&sine_song("0", ""));
    assert_eq!(m.gain, 1.0);
    assert!(m.peak < CEILING);
    assert_eq!(m.peak, m.raw_peak);
}

#[test]
fn a_nan_pitch_fails_loudly_with_the_voice_and_beat() {
    let e = render(&song(&sine_song("sqrt(t - 0.5)", ""))).unwrap_err();
    assert!(e.message.contains("voice 's'") && e.message.contains("pitch is NaN"), "{e}");
    assert_eq!(e.line, Some(4));
}

#[test]
fn an_infinite_gate_or_trigger_fails_loudly() {
    let src = sine_song("0", "").replace("gate = 1", "gate = 1 / (t - t)");
    assert!(render(&song(&src)).unwrap_err().message.contains("gate is"));
    let src = "bpm 60\nbeats 1\ndrum d {\n kind = hat\n trigger = 1 / (t - t)\n}\n";
    assert!(render(&song(src)).unwrap_err().message.contains("trigger is"));
}

#[test]
fn a_pitch_above_nyquist_fails_loudly() {
    let e = render(&song(&sine_song("100", ""))).unwrap_err();
    assert!(e.message.contains("Nyquist"), "{e}");
}

// ---------------------------------------------------------------- filter, envelope, drive

/// Steady-state amplitude of a sine-like signal: the peak over the last half of the buffer.
fn steady_peak(x: &[f64]) -> f64 {
    x[x.len() / 2..].iter().fold(0.0f64, |a, &v| a.max(v.abs()))
}

/// A one-second sine at `hz` (pitch 0 with `reference hz`), with `fields` added to the voice.
fn sine_at(hz: f64, fields: &str) -> Mix {
    mix(&format!(
        "bpm 60\nbeats 1\nreference {hz}\nvoice s {{\n wave = sine\n pitch = 0\n gate = 1\n gain = 0.1\n{fields}\n}}\n"
    ))
}

fn db(x: f64) -> f64 {
    20.0 * x.log10()
}

#[test]
fn the_low_pass_is_three_db_down_at_its_cutoff() {
    for hz in [100.0, 1000.0, 5000.0, 15000.0] {
        let dry = steady_peak(&sine_at(hz, "").left);
        let wet = steady_peak(&sine_at(hz, &format!(" cutoff = {hz}")).left);
        let loss = db(wet / dry);
        assert!((loss + 3.0103).abs() < 0.05, "at {hz} Hz the loss is {loss:.4} dB, want -3.01");
    }
}

#[test]
fn the_low_pass_passes_lows_and_falls_twelve_db_per_octave() {
    let dry = steady_peak(&sine_at(100.0, "").left);
    let wet = steady_peak(&sine_at(100.0, " cutoff = 2000").left);
    assert!(db(wet / dry).abs() < 0.05, "a tone 4 octaves below the cutoff passes ({:.3} dB)", db(wet / dry));
    let dry = steady_peak(&sine_at(4000.0, "").left);
    let wet = steady_peak(&sine_at(4000.0, " cutoff = 1000").left);
    let loss = db(wet / dry);
    assert!((-26.0..-22.0).contains(&loss), "two octaves above the cutoff is about -24 dB, got {loss:.2}");
}

#[test]
fn resonance_lifts_the_cutoff_frequency() {
    let dry = steady_peak(&sine_at(1000.0, "").left);
    let wet = steady_peak(&sine_at(1000.0, " cutoff = 1000\n resonance = 0.8").left);
    // Gain at the cutoff is Q = 1 / (sqrt 2 (1 - r)) = 3.54, +11 dB.
    let lift = db(wet / dry);
    assert!((lift - 10.97).abs() < 0.1, "lift at the cutoff is {lift:.3} dB");
}

#[test]
fn a_fast_swept_resonant_filter_stays_finite_and_bounded() {
    let src = "bpm 60\nbeats 2\nvoice s {\n wave = saw\n pitch = -24 + 30 frac(3t)\n gate = 0.9 - frac(4t)\n \
               cutoff = 30 + 23000 frac(37t)^2\n resonance = 0.95\n filter_env = 4\n filter_decay = 20\n}\n";
    let m = mix(src);
    assert!(m.left.iter().all(|x| x.is_finite()));
    assert!(m.raw_peak < 40.0, "a resonance of 0.95 (Q 14) cannot ring above ~15x the input: {}", m.raw_peak);
}

#[test]
fn the_filter_envelope_opens_the_filter_at_each_note_and_closes_it() {
    // A saw through a low cutoff; the envelope adds 4 octaves at the onset, decaying with 50 ms.
    let src = "bpm 60\nbeats 1\nvoice s {\n wave = saw\n pitch = -21\n gate = 1\n gain = 0.2\n cutoff = 300\n filter_env = 4\n filter_decay = 50\n}\n";
    let with_env = mix(src);
    let without = mix(&src.replace(" filter_env = 4\n", ""));
    // Brightness: the RMS of the first difference (a crude high-pass), over a 10 ms window.
    let bright = |m: &Mix, a: usize| {
        let w = &m.left[a..a + 480];
        (w.windows(2).map(|p| (p[1] - p[0]).powi(2)).sum::<f64>() / 480.0).sqrt()
    };
    let (early_env, early_plain) = (bright(&with_env, 240), bright(&without, 240)); // 5..15 ms
    assert!(early_env > 3.0 * early_plain, "the onset is brighter with the envelope: {early_env} vs {early_plain}");
    let (late_env, late_plain) = (bright(&with_env, 24_000), bright(&without, 24_000)); // ten time constants on
    assert!((late_env / late_plain - 1.0).abs() < 0.01, "and has closed again: {late_env} vs {late_plain}");
}

#[test]
fn the_envelope_decays_to_its_sustain_and_releases_from_there() {
    // attack 5 ms, decay 50 ms to 0.4, gate closes at 0.5 s, release 100 ms (full scale).
    let src = "bpm 60\nbeats 1\nvoice s {\n wave = sine\n pitch = 0\n gate = 0.5 - t\n gain = 1\n \
               attack = 5\n decay = 50\n sustain = 0.4\n release = 100\n}\n";
    let m = mix(src);
    let peak = |a: usize, b: usize| m.left[a..b].iter().fold(0.0f64, |x, &v| x.max(v.abs()));
    let pan = std::f64::consts::FRAC_1_SQRT_2; // centre pan
    assert!(peak(240, 480) > 0.85 * pan, "near full level just after the attack");
    assert!((peak(4_000, 24_000) / pan - 0.4).abs() < 0.01, "holds the sustain level");
    // Release slope is full scale per 100 ms, so from 0.4 it takes 40 ms: silent after 41 ms.
    assert!(peak(24_000 + 1_968, 48_000) == 0.0, "silent once released");
    assert!(peak(24_000, 24_000 + 900) > 0.1 * pan, "still sounding early in the release");
}

#[test]
fn defaults_leave_the_old_envelope_and_no_filter() {
    let plain = mix(&sine_song("0", ""));
    let explicit = mix(&sine_song("0", "").replace(
        "gain = 0.5",
        "gain = 0.5\n attack = 5\n decay = 100\n sustain = 1\n release = 30\n resonance = 0\n filter_env = 0\n drive = 0",
    ));
    assert_eq!(plain.left, explicit.left, "the defaults are the step-1 behaviour exactly");
}

#[test]
fn drive_saturates_softly_and_never_exceeds_the_gain() {
    let clean = sine_at(220.0, "");
    let driven = sine_at(220.0, " drive = 4");
    let (c, d) = (steady_peak(&clean.left), steady_peak(&driven.left));
    assert!((d - c).abs() < 1e-9, "drive is normalised so a full-scale peak stays full scale ({c} vs {d})");
    // Saturation fattens the wave.
    let rms = |x: &[f64]| (x.iter().map(|v| v * v).sum::<f64>() / x.len() as f64).sqrt();
    assert!(rms(&driven.left[24_000..]) > 1.15 * rms(&clean.left[24_000..]), "driven RMS is higher at the same peak");
}

#[test]
fn polyblep_cuts_alias_energy_against_a_naive_saw() {
    // A high saw: 3520 Hz (pitch 36) at 48 kHz. One second holds exactly 3520 cycles, so the DFT
    // has no leakage and every component sits on a 1 Hz bin. Energy outside the harmonic bins
    // (multiples of 3520 Hz) is aliasing.
    let src = "bpm 60\nbeats 2\nvoice s {\n wave = saw\n pitch = 36\n gate = 1\n gain = 1\n}\n";
    let m = mix(src);
    let pan = std::f64::consts::FRAC_1_SQRT_2;
    let engine: Vec<f64> = m.left[24_000..72_000].iter().map(|v| v / pan).collect();
    // The naive saw over the same window, phase-aligned by the engine's own phase accumulator.
    let dt = 3520.0 / 48_000.0;
    let mut phase = 0.0f64;
    let mut naive = Vec::with_capacity(96_000);
    for _ in 0..72_000 {
        naive.push(2.0 * phase - 1.0);
        phase += dt;
        phase -= phase.floor();
    }
    let naive = &naive[24_000..];
    let alias = |x: &[f64]| {
        let total: f64 = x.iter().map(|v| v * v).sum::<f64>();
        let mut harm = 0.0;
        let n = x.len() as f64;
        for k in 1..=6 {
            let w = std::f64::consts::TAU * (3520.0 * k as f64) / 48_000.0;
            let (mut re, mut im) = (0.0, 0.0);
            for (i, v) in x.iter().enumerate() {
                re += v * (w * i as f64).cos();
                im -= v * (w * i as f64).sin();
            }
            harm += 2.0 * (re * re + im * im) / n; // both +f and -f bins
        }
        let mean = x.iter().sum::<f64>() / n;
        (total - harm - mean * mean * n) / total
    };
    let (a_naive, a_blep) = (alias(naive), alias(&engine));
    let drop = 10.0 * (a_naive / a_blep).log10();
    println!("alias energy: naive {:.2} dB, polyblep {:.2} dB, drop {drop:.1} dB", 10.0 * a_naive.log10(), 10.0 * a_blep.log10());
    assert!(drop > 6.0, "PolyBLEP should cut alias energy clearly (drop {drop:.1} dB)");
}

#[test]
fn a_nan_cutoff_fails_loudly_with_the_voice_and_beat() {
    let e = render(&song(&sine_song("0", "").replace("gain = 0.5", "gain = 0.5\n cutoff = sqrt(0.5 - t)"))).unwrap_err();
    assert!(e.message.contains("voice 's'") && e.message.contains("cutoff is NaN"), "{e}");
}
