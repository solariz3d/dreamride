//! Known values of the @measure_plan-style formulas, evaluated through the example song itself,
//! so a change to a formula in `examples/citypop_sketch.txt` turns these red.

use luminesce::expr::Program;
use luminesce::song::Song;

fn example() -> Song {
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/citypop_sketch.txt")).unwrap();
    Song::parse(&src).unwrap_or_else(|e| panic!("example failed to parse: {e}"))
}

fn call(s: &Song, name: &str, x: f64) -> f64 {
    s.program.call(name, &[x]).unwrap()
}

#[test]
fn h_maps_degrees_0_to_7_onto_the_major_scale() {
    let s = example();
    let got: Vec<f64> = (0..8).map(|d| call(&s, "h", d as f64)).collect();
    assert_eq!(got, vec![0.0, 2.0, 4.0, 5.0, 7.0, 9.0, 11.0, 12.0]);
}

#[test]
fn h_continues_by_octaves_below_zero() {
    let s = example();
    let got: Vec<f64> = (-7..0).map(|d| call(&s, "h", d as f64)).collect();
    assert_eq!(got, vec![-12.0, -10.0, -8.0, -7.0, -5.0, -3.0, -1.0]);
}

/// DESIGN.md writes h with +2. Pinned here so the difference stays visible: that form is the
/// natural minor scale, not major.
#[test]
fn h_as_written_in_design_md_gives_natural_minor() {
    let p = Program::from_source("h(d) = floor((12d + 2) / 7)").unwrap();
    let got: Vec<f64> = (0..8).map(|d| p.call("h", &[d as f64]).unwrap()).collect();
    assert_eq!(got, vec![0.0, 2.0, 3.0, 5.0, 7.0, 8.0, 10.0, 12.0]);
}

#[test]
fn root_reads_the_digits_of_342258473_minus_5() {
    let s = example();
    let got: Vec<f64> = (0..9).map(|i| call(&s, "root", i as f64)).collect();
    let want: Vec<f64> = "342258473".chars().map(|c| c.to_digit(10).unwrap() as f64 - 5.0).collect();
    assert_eq!(got, want);
}

#[test]
fn fold_maps_integers_into_minus_9_to_2_preserving_pitch_class() {
    let s = example();
    for z in -40..=40 {
        let f = call(&s, "fold", z as f64);
        assert!((-9.0..=2.0).contains(&f), "fold({z}) = {f}");
        assert_eq!((f - z as f64).rem_euclid(12.0), 0.0, "fold({z}) = {f} changed the pitch class");
    }
    assert_eq!(call(&s, "fold", 2.0), 2.0);
    assert_eq!(call(&s, "fold", 3.0), -9.0);
    assert_eq!(call(&s, "fold", -10.0), 2.0);
}

#[test]
fn fold_keeps_real_values_in_minus_9_to_3() {
    let s = example();
    for k in 0..1000 {
        let z = -30.0 + k as f64 * 0.0613;
        let f = call(&s, "fold", z);
        assert!((-9.0..3.0).contains(&f), "fold({z}) = {f}");
    }
}

#[test]
fn smoothstep_known_values() {
    let s = example();
    assert_eq!(call(&s, "smoothstep", 0.0), 0.0);
    assert_eq!(call(&s, "smoothstep", 0.5), 0.5);
    assert_eq!(call(&s, "smoothstep", 1.0), 1.0);
    assert!((call(&s, "smoothstep", 0.25) - 0.15625).abs() < 1e-15);
}

#[test]
fn swing_is_monotonic_on_0_to_1() {
    let s = example();
    let mut prev = call(&s, "sw", 0.0);
    for k in 1..=10_000 {
        let x = k as f64 / 10_000.0;
        let y = call(&s, "sw", x);
        assert!(y > prev, "sw is not increasing at x = {x}: {prev} then {y}");
        prev = y;
    }
}

#[test]
fn swing_fixes_the_beat_and_half_beat_boundaries() {
    let s = example();
    for k in 0..=8 {
        let x = k as f64 / 2.0;
        assert_eq!(call(&s, "sw", x), x, "sw({x})");
    }
}

#[test]
fn swing_is_continuous_across_the_half_beat_boundaries() {
    // Approaching each boundary from below must arrive at it: no jump in warped time.
    let s = example();
    for k in 1..=4 {
        let x = k as f64 / 2.0;
        let below = call(&s, "sw", x - 1e-9);
        assert!((below - x).abs() < 1e-6, "sw({x} - 1e-9) = {below}, a jump at the boundary");
    }
}

#[test]
fn swing_delays_the_second_sixteenth() {
    // frac(2x) = 0.58 is where the two branches meet: warped time reaches the halfway point (the
    // second sixteenth of the half beat) at 58% instead of 50%.
    let s = example();
    assert!((call(&s, "sw", 0.29) - 0.25).abs() < 1e-12);
    assert!(call(&s, "sw", 0.25) < 0.25, "on the straight sixteenth, swung time is still behind");
}

#[test]
fn the_lead_digits_read_out_in_order() {
    let s = example();
    let n = 4024642099402460.0;
    let got: String = (0..16).map(|i| s.program.call("digit16", &[n, i as f64]).unwrap().to_string()).collect();
    assert_eq!(got, "4024642099402460");
}
