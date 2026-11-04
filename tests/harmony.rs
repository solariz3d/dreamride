//! The progression layer (src/compose/harmony.txt, src/compose/harmony.rs): every progression parses into eight bars of
//! written chords, starts home and cadences at every phrase end (a dominant into the tonic, or a marked deceptive turn),
//! changes chord only on bar lines and half bars, voices every chord with the least movement its registers allow and
//! no crossing, and is stored in digit tables the song language reads back exactly, in any key.

use luminesce::compose::harmony::{
    self, all, band, bass_digits, candidates, coda, movement, off_centre, options, pad_digits, parse, parse_chord, table, Mode,
    Progression, BAND_SPAN, FRAME, HELPERS, MIN_GAP, SLOTS,
};
use luminesce::expr::Program;
use std::collections::BTreeSet;

fn program(p: &Progression, key: i32) -> Program {
    let defs: Vec<String> = harmony::defs(p).unwrap().into_iter().map(|d| d.line).collect();
    let src = format!("{}\n{HELPERS}\n{}\n", FRAME.replace("K = 0", &format!("K = {key}")), defs.join("\n"));
    Program::from_source(&src).unwrap_or_else(|e| panic!("{}: {e}\n{src}", p.name))
}

#[test]
fn every_progression_parses_into_eight_bars_of_written_chords() {
    let ps = all();
    println!("{} progressions", ps.len());
    assert!((16..=24).contains(&ps.len()), "{} progressions", ps.len());
    let mut names = BTreeSet::new();
    for p in ps {
        println!("{:20} {:10} phrase {} e{} d{}  {}", p.name, p.mode.name(), p.phrase, p.energy, p.density, harmony::chart(p));
        assert!(names.insert(p.name.clone()), "{} twice", p.name);
        assert_eq!(p.bars.len(), 8, "{}", p.name);
        assert_eq!(p.slots.len(), SLOTS, "{}", p.name);
        assert_eq!(p.voicing.len(), SLOTS, "{}", p.name);
        assert!(p.describe.len() > 30, "{}", p.name);
    }
    // Both kinds of home, and both kinds of harmonic rhythm.
    assert!(ps.iter().any(|p| p.mode.minor()) && ps.iter().any(|p| !p.mode.minor()));
    assert!(ps.iter().any(|p| p.bars.iter().any(|b| b.chords.len() == 2)) && ps.iter().any(|p| p.bars.iter().any(|b| b.held)));
    // Exactly one progression carries the deceptive turn the stream uses once a song.
    assert_eq!(ps.iter().filter(|p| p.deceptive()).count(), 1);
}

#[test]
fn chord_symbols_read_as_written() {
    let c = parse_chord("V7", Mode::Major).unwrap();
    assert_eq!((c.root, c.bass, c.tones.clone()), (7, 7, vec![0, 4, 7, 10]));
    // Minor counts the natural minor: VI is the flat sixth, VII the flat seventh; lower case is minor.
    assert_eq!(parse_chord("VImaj7", Mode::Minor).unwrap().root, 8);
    assert_eq!(parse_chord("vii7", Mode::Minor).unwrap().root, 10);
    assert_eq!(parse_chord("iv9", Mode::Minor).unwrap().tones, vec![0, 3, 7, 10, 14]);
    // Accidentals, the tritone substitute and the backdoor.
    assert_eq!(parse_chord("bII7", Mode::Major).unwrap().root, 1);
    assert_eq!(parse_chord("bVII9", Mode::Major).unwrap().root, 10);
    // Inversions name the chord's own third, fifth, seventh.
    assert_eq!(parse_chord("Imaj7/7", Mode::Major).unwrap().bass, 11);
    assert_eq!(parse_chord("I7/7", Mode::Major).unwrap().bass, 10);
    assert_eq!(parse_chord("ivm6/3", Mode::Major).unwrap().bass, 8);
    assert_eq!(parse_chord("V/3", Mode::Major).unwrap().bass, 11);
    assert!(parse_chord("vi9!", Mode::Major).unwrap().deceptive);
    for bad in ["X7", "Vx", "IIII", "V/9", "iV7", "I/7", ""] {
        assert!(parse_chord(bad, Mode::Major).is_err(), "'{bad}' should not parse");
    }
}

#[test]
fn the_dominant_test_knows_its_chords() {
    let d = |s: &str, m: Mode| parse_chord(s, m).unwrap().is_dominant();
    for s in ["V", "V7", "V13", "V7b9", "V7#9", "V7sus", "V9sus", "viiø7", "vii°7", "bII7", "bII9", "bVII9", "bVII13", "V/3"] {
        assert!(d(s, Mode::Major), "{s} is a dominant");
    }
    assert!(d("VII7", Mode::Minor) && d("V7b9", Mode::Minor));
    for s in ["I", "IVmaj7", "ii7", "iii7", "vi9", "bVII", "bIImaj7", "ivm6", "VI7", "III7"] {
        assert!(!d(s, Mode::Major), "{s} is not a dominant");
    }
    assert!(!d("v7", Mode::Minor), "the minor v is not a dominant");
}

/// The last chord of each phrase, and the chord that follows it (the first of the next stage after the last phrase).
fn phrase_ends(p: &Progression) -> Vec<(usize, &harmony::Chord, &harmony::Chord)> {
    (1..=8 / p.phrase)
        .map(|k| {
            let end = k * p.phrase * 2 - 1;
            (end, &p.slots[end], &p.slots[(end + 1) % SLOTS])
        })
        .collect()
}

#[test]
fn every_progression_starts_home_and_every_phrase_ends_on_a_dominant_into_the_tonic() {
    for p in all() {
        assert!(p.slots[0].is_tonic() && !p.slots[0].deceptive, "{} starts on {}", p.name, p.slots[0].symbol);
        for (end, last, next) in phrase_ends(p) {
            println!("{:20} half bar {end:2}: {} -> {}", p.name, last.symbol, next.symbol);
            assert!(last.is_dominant(), "{}: the phrase ending at half bar {end} ends on {}, not a dominant", p.name, last.symbol);
            assert!(next.is_tonic() || next.deceptive, "{}: {} resolves to {}, neither the tonic nor a marked turn", p.name, last.symbol, next.symbol);
        }
        // A marked turn stands only where a phrase was due home, after a dominant.
        for (s, c) in p.events().into_iter().filter(|(_, c)| c.deceptive) {
            assert!(s % (2 * p.phrase) == 0 && p.slots[s - 1].is_dominant(), "{}: {} marked deceptive at half bar {s}", p.name, c.symbol);
        }
    }
}

#[test]
fn every_coda_lands_on_the_tonic_after_a_dominant_where_the_ritard_ends() {
    for p in all() {
        let c = coda(p);
        assert_eq!(c.slots.len(), SLOTS);
        // Bars 0-3 are the progression's own; the last two bars are home, after a dominant.
        assert_eq!(&c.slots[..8], &p.slots[..8], "{}", p.name);
        assert!(c.slots[11].is_dominant(), "{}: the coda's bar 6 ends on {}", p.name, c.slots[11].symbol);
        assert!(c.slots[12..].iter().all(|x| x.is_tonic()), "{}: the coda's last bars are not home", p.name);
        // The fifth bar never takes a dominant back to IV.
        assert!(!(c.slots[7].is_dominant() && c.slots[8].root == 5), "{}: V falls back to IV", p.name);
    }
}

#[test]
fn chords_change_only_on_bar_lines_and_half_bars_where_written() {
    for p in all() {
        // The data: a new chord starts on a bar line, or at the half bar only in a bar written with two chords.
        for (s, c) in p.events() {
            assert!(s % 2 == 0 || p.bars[s / 2].chords.len() == 2, "{}: {} starts mid-bar at half bar {s}", p.name, c.symbol);
        }
        // What a song reads: the chord's pitches as functions of time, sampled every sixteenth.
        let prog = program(p, -3);
        let at = |t: f64| -> Vec<i64> {
            let i = (t / 2.0).floor();
            let mut v = vec![prog.call("Hr", &[i]).unwrap() as i64];
            v.extend((0..3).map(|m| prog.call("Hv", &[m as f64, i]).unwrap() as i64));
            v
        };
        let mut changes = Vec::new();
        for n in 1..(32 * 4) {
            let t = n as f64 / 4.0;
            if at(t) != at(t - 0.25) {
                assert!(t.fract() == 0.0 && t as usize % 2 == 0, "{}: the chord changes at beat {t}", p.name);
                let bar = t as usize / 4;
                assert!(t as usize % 4 == 0 || p.bars[bar].chords.len() == 2, "{}: a change at the half bar of bar {bar}, written with one chord", p.name);
                changes.push(t);
            }
        }
        assert!(!changes.is_empty(), "{}: never changes", p.name);
    }
}

#[test]
fn the_tables_hold_the_voicing_exactly_in_any_key() {
    for p in all() {
        let (step, o, bass) = bass_digits(p).unwrap();
        assert!([1, 7].contains(&step), "{}", p.name);
        for d in [table(&bass), table(&pad_digits(p, 0)), table(&pad_digits(p, 1)), table(&pad_digits(p, 2))] {
            assert!(d < 1 << 53, "{}: a table of {d} is past exact integers", p.name);
        }
        assert!(bass.iter().all(|d| (0..=9).contains(d)) && (-6..6).contains(&o), "{}", p.name);
        for key in [-5, 0, 2] {
            let prog = program(p, key);
            for i in 0..=SLOTS {
                let s = i % SLOTS; // the slot after the last is the first again (a glide into the next stage reads it)
                let r = prog.call("Hr", &[i as f64]).unwrap();
                assert!((key as f64..key as f64 + 12.0).contains(&r) && (r as i32 - key).rem_euclid(12) == p.slots[s].bass, "{} Hr({i}) = {r}", p.name);
                for m in 0..3 {
                    let v = prog.call("Hv", &[m as f64, i as f64]).unwrap();
                    assert_eq!(v, (key + p.voicing[s][m]) as f64, "{} key {key}: Hv({m}, {i})", p.name);
                }
                // The measurement's reading of the same tables (line_stats), in exact integers.
                let pcs = harmony::chord_tones(&prog, i).unwrap();
                let want: Vec<i64> = std::iter::once(key + p.slots[s].bass).chain((0..3).map(|m| key + p.voicing[s][m])).map(|x| (x as i64).rem_euclid(12)).collect();
                assert_eq!(pcs, want, "{} key {key}: chord_tones({i})", p.name);
                // The arpeggio's ladder: the bass's pitch class just under the pads, then the pads as voiced, then the
                // same an octave up.
                let ladder: Vec<f64> = (0..8).map(|m| prog.call("Hc", &[m as f64, i as f64]).unwrap()).collect();
                assert!(ladder[..4].windows(2).all(|w| w[1] > w[0]) && ladder[0] >= ladder[1] - 12.0, "{} Hc at {i}: {ladder:?}", p.name);
                assert!((0..4).all(|m| ladder[m + 4] == ladder[m] + 12.0), "{} Hc at {i}: {ladder:?}", p.name);
                assert_eq!((ladder[0] as i32 - key).rem_euclid(12), p.slots[s].bass, "{} Hc(0, {i})", p.name);
            }
        }
    }
}

#[test]
fn every_chord_moves_the_least_it_can_and_the_voices_never_cross() {
    let mut total = 0;
    let mut moves = 0;
    for p in all() {
        for (s, v) in p.voicing.iter().enumerate() {
            let c = &p.slots[s];
            assert!(v[0] < v[1] && v[1] < v[2], "{} half bar {s}: crossing {v:?}", p.name);
            for m in 0..3 {
                let (lo, hi) = band(m);
                assert!((lo..=hi).contains(&v[m]) && hi - lo == BAND_SPAN, "{} half bar {s}: voice {m} at {} outside {lo}..{hi}", p.name, v[m]);
            }
            let mut got: Vec<i32> = v.iter().map(|x| x.rem_euclid(12)).collect();
            let mut want: Vec<i32> = c.pads.iter().map(|x| (c.root + x).rem_euclid(12)).collect();
            got.sort();
            want.sort();
            assert_eq!(got, want, "{} half bar {s}: {} voiced as {v:?}", p.name, c.symbol);
            let opts = options(c).unwrap();
            assert!(opts.contains(v));
            if s == 0 {
                assert_eq!(off_centre(v), opts.iter().map(off_centre).min().unwrap(), "{}: the first chord is not the most central", p.name);
            } else {
                let prev = &p.voicing[s - 1];
                let best = opts.iter().map(|o| movement(prev, o)).min().unwrap();
                assert_eq!(movement(prev, v), best, "{} half bar {s}: {} moves {} where {best} was possible", p.name, c.symbol, movement(prev, v));
                total += movement(prev, v);
                moves += (movement(prev, v) > 0) as i32;
            }
        }
    }
    println!("mean movement per chord change: {:.2} semitones over all three voices", total as f64 / moves as f64);
    assert!((total as f64 / moves as f64) <= 6.0, "the pads leap: {total} over {moves} changes");
}

#[test]
fn every_chord_has_a_voicing_at_the_usual_spacing() {
    for p in all() {
        for c in &p.slots {
            assert!(!candidates(c, MIN_GAP).is_empty(), "{}: {} needs the gap relaxed", p.name, c.symbol);
        }
    }
}

#[test]
fn the_pads_keep_the_guide_tones() {
    for p in all() {
        for c in &p.slots {
            let pcs: Vec<i32> = c.pads.iter().map(|x| x.rem_euclid(12)).collect();
            let third = c.tones[1];
            assert!(pcs.contains(&third), "{}: {} without its third (or suspension)", p.name, c.symbol);
            if c.tones.len() >= 4 {
                assert!(pcs.contains(&c.tones[3].rem_euclid(12)), "{}: {} without its seventh (or sixth)", p.name, c.symbol);
            }
        }
    }
}

#[test]
fn a_progression_that_breaks_the_rules_does_not_parse_or_is_caught() {
    // Seven bars, a bar of three chords, and an unknown mode are refused outright.
    let head = "== x | major | phrase 8 | energy 1 | density 1 | mood calm\n# a progression written only to be refused by the parser\n";
    assert!(parse(&format!("{head}| I | I | I | I | I | I | V7 |\n")).is_err());
    assert!(parse(&format!("{head}| I ii V7 | I | I | I | I | I | I | V7 |\n")).is_err());
    assert!(parse("== x | aeolian | phrase 8 | energy 1 | density 1 | mood calm\n# a progression in a mode the layer does not know\n| I | I | I | I | I | I | I | V7 |\n").is_err());
    // One ending on IV parses, and the cadence test catches it.
    let p = &parse(&format!("{head}| Imaj7 | ii7 | iii7 | IVmaj7 | Imaj7 | ii7 | iii7 | IVmaj7 |\n")).unwrap()[0];
    assert!(!p.slots[15].is_dominant());
}
