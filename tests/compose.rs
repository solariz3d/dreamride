//! The composer (src/compose): the critic's maths on known answers, the sound analyser against the measurements in
//! docs, the grammar's files (they parse, render finite, and read their tables back), determinism per seed, the ratings
//! file, and the critic's first claim: the hand-written graph songs score above the median random sample.

use luminesce::compose::critic::{
    self, chord_roughness, harmonic, lhl_per_bar, log_band, pair_roughness, roughness_anchors, tone_roughness, trapezoid,
};
use luminesce::compose::grammar::{self, collatz, num, pisano, primes, rule30, Family};
use luminesce::compose::search::{self, parse_ratings, percentile, Rating, Verdict};
use luminesce::compose::sound::{self, fft};
use luminesce::render::render;
use luminesce::song::Song;

const GRAPH_SONGS: [&str; 3] = ["graph_takagi.txt", "graph_lissajous.txt", "graph_pisano.txt"];

fn example(name: &str) -> String {
    std::fs::read_to_string(format!("{}/examples/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

fn hz(p: f64) -> f64 {
    440.0 * 2f64.powf(p / 12.0)
}

/// Roughness of two pure tones `semis` apart above A4.
fn pure(semis: f64) -> f64 {
    pair_roughness(440.0, 1.0, hz(semis), 1.0)
}

/// Roughness of two harmonic tones (the graph pad timbre) `semis` apart above C4.
fn rich(semis: f64) -> f64 {
    let pad = [0.0, -12.0, -5.0, -11.0, -9.0, -12.0, -11.0, -13.0];
    tone_roughness(&harmonic(hz(-9.0), &pad, 660.0, 1.0), &harmonic(hz(-9.0 + semis), &pad, 660.0, 1.0))
}

// ---------------------------------------------------------------------------------------------------- roughness

#[test]
fn roughness_of_a_unison_is_zero_and_lowest() {
    assert_eq!(pure(0.0), 0.0);
    for s in 1..=12 {
        assert!(pure(s as f64) > 0.0, "{s} semitones");
        assert!(rich(s as f64) > rich(0.0), "{s} semitones against the unison, harmonic tones");
    }
}

#[test]
fn roughness_orders_the_fifth_below_the_tritone_and_the_minor_second_highest() {
    for f in [pure, rich] {
        assert!(f(7.0) < f(6.0), "fifth {} < tritone {}", f(7.0), f(6.0));
        assert!(f(12.0) < f(6.0), "octave below tritone");
        let m2 = f(1.0);
        for s in 2..=12 {
            assert!(f(s as f64) < m2, "{s} semitones below the minor second");
        }
    }
    // With harmonic partials the consonances are the ones whose partials coincide: octave and fifth beat the third.
    assert!(rich(12.0) < rich(4.0) && rich(7.0) < rich(4.0));
}

#[test]
fn pure_tone_roughness_peaks_near_a_quarter_of_a_critical_band() {
    // Plomp and Levelt: maximal at about 25% of the critical bandwidth; at 440 Hz (CB about 90-100 Hz, Zwicker) the
    // peak should sit near 20-30 Hz above.
    let (mut best, mut best_df) = (0.0, 0.0);
    for df in 1..200 {
        let r = pair_roughness(440.0, 1.0, 440.0 + df as f64, 1.0);
        if r > best {
            best = r;
            best_df = df as f64;
        }
    }
    assert!((15.0..=35.0).contains(&best_df), "peak at {best_df} Hz");
}

#[test]
fn roughness_does_not_grow_with_loudness_and_anchors_order_triad_below_cluster() {
    let a = harmonic(220.0, &[0.0, -6.0], 0.0, 1.0);
    let b = harmonic(233.08, &[0.0, -6.0], 0.0, 1.0);
    let a3 = harmonic(220.0, &[0.0, -6.0], 0.0, 3.0);
    let b3 = harmonic(233.08, &[0.0, -6.0], 0.0, 3.0);
    assert!((tone_roughness(&a, &b) - tone_roughness(&a3, &b3)).abs() < 1e-12);
    let (lo, hi) = roughness_anchors();
    assert!(lo > 0.0 && lo < hi, "triad {lo}, cluster {hi}");
    assert_eq!(chord_roughness(&[(a, 1.0)]), None, "one tone has no roughness");
}

// ---------------------------------------------------------------------------------------------------- syncopation

fn bar(onsets: &[usize]) -> Vec<bool> {
    (0..16).map(|i| onsets.contains(&i)).collect()
}

#[test]
fn syncopation_of_on_beat_patterns_is_zero() {
    assert_eq!(lhl_per_bar(&bar(&[0, 4, 8, 12])), vec![0.0], "four on the floor");
    assert_eq!(lhl_per_bar(&bar(&(0..16).collect::<Vec<_>>())), vec![0.0], "every sixteenth");
    assert_eq!(lhl_per_bar(&bar(&[0])), vec![0.0], "the downbeat alone");
    assert_eq!(lhl_per_bar(&bar(&[])), vec![0.0], "silence");
}

#[test]
fn syncopation_of_known_patterns() {
    // Son clave 3-2 as sixteenths 0, 3, 6, 10, 12: the onsets on 3 and 6 precede silent beats 2 (weight -2) and 3 (-1).
    assert_eq!(lhl_per_bar(&bar(&[0, 3, 6, 10, 12])), vec![4.0]);
    // Tresillo 3-3-2: only the onset on 6 syncopates, against beat 3.
    assert_eq!(lhl_per_bar(&bar(&[0, 6, 12])), vec![2.0]);
    // Every offbeat sixteenth, nothing on the grid: each onset syncopates against the next position.
    assert_eq!(lhl_per_bar(&bar(&[1, 3, 5, 7, 9, 11, 13, 15])), vec![15.0]);
    // An anticipation: the last sixteenth of bar 1 tied over an empty downbeat of bar 2 scores 4, in bar 1.
    let mut two = bar(&[0, 4, 8, 15]);
    two.extend(bar(&[4, 8, 12]));
    assert_eq!(lhl_per_bar(&two), vec![4.0, 0.0]);
}

#[test]
fn syncopation_counts_the_wrap_round_of_a_loop() {
    // One bar looped: an onset on the last sixteenth with the downbeat silent syncopates against the next downbeat.
    // Beat 2 to the last sixteenth leaves beat 3 silent (1), and the last sixteenth leaves the next downbeat silent (4).
    assert_eq!(lhl_per_bar(&bar(&[4, 15])), vec![5.0]);
}

// ---------------------------------------------------------------------------------------------------- score shapes

#[test]
fn trapezoid_and_log_band_have_their_stated_shape() {
    assert_eq!(trapezoid(5.0, 0.0, 2.0, 8.0, 10.0), 1.0);
    assert_eq!(trapezoid(1.0, 0.0, 2.0, 8.0, 10.0), 0.5);
    assert_eq!(trapezoid(9.0, 0.0, 2.0, 8.0, 10.0), 0.5);
    assert_eq!(trapezoid(-1.0, 0.0, 2.0, 8.0, 10.0), 0.0);
    assert_eq!(trapezoid(f64::NAN, 0.0, 2.0, 8.0, 10.0), 0.0);
    assert_eq!(log_band(50.0, 41.0, 60.0, 0.5), 1.0);
    // Gaussian in the log distance: at twice the width outside the band, e^-4 (an exponential would give e^-2).
    assert!((log_band(60.0 * 1f64.exp(), 41.0, 60.0, 0.5) - (-4f64).exp()).abs() < 1e-12);
    assert!((log_band(41.0 / 1f64.exp(), 41.0, 60.0, 0.5) - (-4f64).exp()).abs() < 1e-12);
    assert_eq!(log_band(0.0, 41.0, 60.0, 0.5), 0.0);
}

// ---------------------------------------------------------------------------------------------------- sound

#[test]
fn fft_finds_a_bin_sine_and_keeps_energy() {
    let n = 1024;
    let mut re: Vec<f64> = (0..n).map(|i| (2.0 * std::f64::consts::PI * 37.0 * i as f64 / n as f64).sin()).collect();
    let mut im = vec![0.0; n];
    let e_time: f64 = re.iter().map(|x| x * x).sum();
    fft(&mut re, &mut im);
    let mags: Vec<f64> = (0..n).map(|k| (re[k] * re[k] + im[k] * im[k]).sqrt()).collect();
    let peak = (0..n / 2).max_by(|&a, &b| mags[a].partial_cmp(&mags[b]).unwrap()).unwrap();
    assert_eq!(peak, 37);
    let e_freq: f64 = mags.iter().map(|m| m * m).sum::<f64>() / n as f64;
    assert!((e_time - e_freq).abs() < 1e-9 * e_time, "Parseval");
}

#[test]
fn band_energy_puts_a_low_sine_in_the_lowest_band() {
    let rate = 48_000;
    let x: Vec<f64> = (0..rate).map(|i| (2.0 * std::f64::consts::PI * 100.0 * i as f64 / rate as f64).sin()).collect();
    let s = sound::measure_mono(&x, rate as u32);
    assert!(s.bands[0] > 99.9, "{:?}", s.bands);
    assert!(s.peaks >= 1.0 && s.peaks <= 2.0, "one tone, at most a sidelobe: {}", s.peaks);
}

#[test]
fn the_sound_analyser_reproduces_the_documented_takagi_measurement() {
    // README / graph_takagi.txt: tools/measure/analyze.py gave 43/29/26/1.7/0.3 % and 110 peaks per frame.
    let song = Song::parse(&example("graph_takagi.txt")).unwrap();
    let s = sound::measure(&render(&song).unwrap());
    let doc = [43.0, 29.0, 26.0, 1.7, 0.3];
    for b in 0..3 {
        assert!((s.bands[b] - doc[b]).abs() < 3.0, "band {b}: {:.1} against {}", s.bands[b], doc[b]);
    }
    for b in 3..5 {
        assert!((s.bands[b] / doc[b]).ln().abs() < 0.4, "band {b}: {:.2} against {}", s.bands[b], doc[b]);
    }
    assert!((s.peaks - 110.0).abs() <= 10.0, "peaks {} against 110", s.peaks);
}

// ---------------------------------------------------------------------------------------------------- the grammar

#[test]
fn number_formatting_never_writes_an_exponent() {
    for x in [1e-7, -1e-7, 123456789.0, 0.1 + 0.2, -0.00005, 2.5, 34.6241] {
        let s = num(x);
        assert!(!s.contains('e') && !s.contains('E'), "{x} -> {s}");
        assert!(s.parse::<f64>().is_ok(), "{s}");
    }
    assert_eq!(num(0.1 + 0.2), "0.3");
    assert_eq!(num(-0.00004), "0");
    assert_eq!(num(3.0), "3");
}

#[test]
fn the_tables_are_the_sequences_they_name() {
    assert_eq!(pisano(2), 3);
    assert_eq!(pisano(5), 20);
    assert_eq!(pisano(7), 16);
    assert_eq!(pisano(8), 12);
    assert_eq!(primes(6), vec![2, 3, 5, 7, 11, 13]);
    assert_eq!(collatz(6, 10), vec![6, 3, 10, 5, 16, 8, 4, 2, 1, 4]);
    // OEIS A051023, the centre column of rule 30.
    assert_eq!(rule30(20), vec![1, 1, 0, 1, 1, 1, 0, 0, 1, 1, 0, 0, 0, 1, 0, 1, 1, 0, 0, 1]);
}

#[test]
fn every_generated_file_parses_and_every_family_appears() {
    let mut seen = std::collections::HashSet::new();
    for seed in [1u64, 7, 12345] {
        for i in 0..200 {
            let d = grammar::compose(seed, i).unwrap_or_else(|e| panic!("{e}"));
            let song = Song::parse(&d.text).unwrap();
            assert_eq!(song.beats, 64.0);
            assert!(song.voices.iter().any(|v| v.name == "lead"));
            let defs = d.text.lines().filter(|l| !l.starts_with('#') && l.contains(" = ") && !l.starts_with(' ')).count();
            assert!((8..=24).contains(&defs), "seed {seed} candidate {i}: {defs} definitions\n{}", d.text);
            seen.insert(d.choices.family);
        }
    }
    for f in Family::ALL {
        assert!(seen.contains(&f), "{f:?} never sampled");
    }
}

#[test]
fn generated_tables_read_back_their_values_through_the_song_language() {
    let mut checked = [false; 2];
    for i in 0..400 {
        let d = grammar::compose(3, i).unwrap();
        let song = Song::parse(&d.text).unwrap();
        match d.choices.family {
            Family::Primes | Family::Collatz if !checked[0] => {
                let n: u64 = d.params().iter().find(|p| p.0 == "digits").unwrap().1.parse().unwrap();
                let want: Vec<u64> = format!("{n:015}").bytes().map(|b| (b - b'0') as u64).collect();
                for k in 0..30 {
                    let got = song.program.call("D", &[k as f64]).unwrap();
                    assert_eq!(got, want[k % 15] as f64, "D({k}) in candidate {i}");
                }
                checked[0] = true;
            }
            Family::Rule30 if !checked[1] => {
                let bits = rule30(48);
                for k in 0..96 {
                    assert_eq!(song.program.call("B", &[k as f64]).unwrap(), bits[k % 48] as f64, "B({k})");
                }
                checked[1] = true;
            }
            _ => {}
        }
    }
    assert_eq!(checked, [true, true], "both kinds of table were found and checked");
}

/// Render at a lower rate to keep the test quick; every generated pitch and dash is far below its Nyquist.
fn render_quick(text: &str) -> luminesce::render::Mix {
    let t = text.replacen("beats 64", "beats 64\nrate 22050", 1);
    render(&Song::parse(&t).unwrap()).unwrap_or_else(|e| panic!("{e}\n{text}"))
}

#[test]
fn one_song_of_every_family_renders_finite_and_not_silent() {
    let mut done = std::collections::HashSet::new();
    for i in 0..200 {
        let d = grammar::compose(11, i).unwrap();
        if !done.insert(d.choices.family) {
            continue;
        }
        let mix = render_quick(&d.text);
        assert!(mix.left.iter().chain(&mix.right).all(|x| x.is_finite()), "{:?} candidate {i}", d.choices.family);
        assert!(mix.peak > 0.01, "{:?} candidate {i} is silent", d.choices.family);
        let s = sound::measure(&mix);
        assert!(s.bands.iter().all(|b| b.is_finite()) && s.peaks.is_finite());
    }
    assert_eq!(done.len(), Family::ALL.len());
}

#[test]
fn the_same_seed_writes_the_same_songs_and_scores() {
    for i in 0..20 {
        let a = search::candidate(42, i).unwrap();
        let b = search::candidate(42, i).unwrap();
        assert_eq!(a.text, b.text);
        assert_eq!(a.report.function_score().to_bits(), b.report.function_score().to_bits());
    }
    let differ = (0..20).filter(|&i| grammar::compose(42, i).unwrap().text != grammar::compose(43, i).unwrap().text).count();
    assert!(differ >= 19, "another seed writes other songs ({differ} of 20 differ)");
}

#[test]
fn candidates_do_not_depend_on_the_thread_count() {
    let one = search::parallel(12, 1, |i| search::candidate(5, i as u64).unwrap().text);
    let many = search::parallel(12, 6, |i| search::candidate(5, i as u64).unwrap().text);
    assert_eq!(one, many);
}

// ---------------------------------------------------------------------------------------------------- the critic

#[test]
fn every_score_is_finite_and_in_range() {
    for i in 0..60 {
        let c = search::candidate(9, i).unwrap();
        for comp in &c.report.components {
            assert!((0.0..=1.0).contains(&comp.score), "{} = {} in candidate {i}", comp.name, comp.score);
        }
        let f = c.report.function_score();
        assert!(f.is_finite() && (0.0..=1.0).contains(&f));
    }
    let w: f64 = critic::WEIGHTS.iter().map(|w| w.1).sum();
    assert!((w - 1.0).abs() < 1e-12, "weights sum to {w}");
}

#[test]
fn a_song_without_a_melody_scores_low_on_melody_and_does_not_fail() {
    let src = "bpm 100\nbeats 16\nvoice pad {\n  wave = sine\n  pitch = 0\n  gate = 1\n}\n";
    let r = critic::score(&Song::parse(src).unwrap());
    for n in ["melody steps", "melody contour", "melody repetition", "harmony roughness"] {
        assert_eq!(r.get(n).unwrap().score, 0.0, "{n}");
    }
    assert!(r.function_score().is_finite());
}

#[test]
fn the_critic_hears_a_cluster_as_rougher_than_a_triad() {
    let song = |p: [i32; 3]| {
        let mut s = String::from("bpm 100\nbeats 16\n");
        for (k, x) in p.iter().enumerate() {
            s += &format!("voice v{k} {{\n  wave = additive\n  partials = 0, -12, -5, -11\n  pitch = {x}\n  gate = 1\n}}\n");
        }
        critic::score(&Song::parse(&s).unwrap()).get("harmony roughness").unwrap().score
    };
    let triad = song([-9, -5, -2]);
    let cluster = song([-9, -8, -7]);
    assert!(triad > 0.9 && cluster < 0.1, "triad {triad}, cluster {cluster}");
}

/// Criterion 1 of docs/composer.md in its weaker form. It FAILED on 2026-11-02 (the hand songs scored above 10-46% of
/// random samples by function score): the grammar and the critic share their priors on form. Kept, ignored, so the
/// claim can be re-run with `cargo test --test compose -- --ignored`; to be un-ignored only when it passes.
#[test]
#[ignore = "registered falsifier, failed 2026-11-02: see docs/composer.md, 'The registered run'"]
fn the_hand_written_graph_songs_score_above_the_median_random_sample() {
    let pool: Vec<f64> = search::parallel(100, 8, |i| search::candidate(2026, i as u64).unwrap().report.function_score());
    for name in GRAPH_SONGS {
        let s = critic::score(&Song::parse(&example(name)).unwrap()).function_score();
        let p = percentile(s, &pool);
        assert!(p > 0.5, "{name}: function score {s:.3} is above only {:.0}% of 100 random samples", 100.0 * p);
    }
}

#[test]
fn pair_roughness_scales_with_the_quieter_partial() {
    // Sethares: d = min(a1, a2) x the curve, so the louder partial does not matter.
    let full = pair_roughness(440.0, 1.0, 466.0, 1.0);
    assert!((pair_roughness(440.0, 1.0, 466.0, 0.1) - 0.1 * full).abs() < 1e-12);
    assert!((pair_roughness(440.0, 0.1, 466.0, 1.0) - 0.1 * full).abs() < 1e-12);
}

#[test]
fn the_peak_count_ignores_everything_below_80_hz() {
    // Bin-centred sines, so a Hann window leaves no sidelobe peaks: one at bin 8 (46.9 Hz, louder) and one at bin 200
    // (1171.9 Hz). Only the second is in the 80 Hz - 8 kHz range the count reads.
    let rate = 48_000.0;
    let df = rate / sound::FRAME as f64;
    let x: Vec<f64> = (0..4 * sound::FRAME)
        .map(|i| {
            let t = i as f64 / rate;
            (2.0 * std::f64::consts::PI * 8.0 * df * t).sin() + 0.5 * (2.0 * std::f64::consts::PI * 200.0 * df * t).sin()
        })
        .collect();
    assert_eq!(sound::measure_mono(&x, 48_000).peaks, 1.0);
}

// ---------------------------------------------------------------------------------------------------- ratings

#[test]
fn ratings_parse_keep_skip_and_stars_and_reject_the_rest() {
    let src = format!("{}s7-c12 keep  the glide in bar 9\ns7-c3 skip\n\ns7-c40 4\n", search::RATINGS_HEADER);
    let r = parse_ratings(&src).unwrap();
    assert_eq!(
        r,
        vec![
            Rating { id: "s7-c12".into(), verdict: Verdict::Keep, note: "the glide in bar 9".into() },
            Rating { id: "s7-c3".into(), verdict: Verdict::Skip, note: String::new() },
            Rating { id: "s7-c40".into(), verdict: Verdict::Stars(4), note: String::new() },
        ]
    );
    assert!(parse_ratings("s1-c1 6\n").unwrap_err().contains("line 1"));
    assert!(parse_ratings("s1-c1\n").unwrap_err().contains("line 1"));
    assert!(parse_ratings("s1-c1 maybe\n").is_err());
    assert_eq!(parse_ratings("").unwrap(), vec![]);
}
