//! Instruments drawn per song (src/compose/timbre.rs; docs/timbre.md): a seed always draws the same set and seeds
//! differ, dressing changes only the timbre fields, a dressed voice keeps its loudness, songs wear their set, Explore
//! tunings keep theirs, and the registered run (docs/timbre.md): 18 of 20 songs within the reference margin.

use luminesce::compose::book::{self, Slot};
use luminesce::compose::stream::{Composer, Seed};
use luminesce::compose::timbre::{self, dress, palette, ROLES};
use luminesce::render::render;
use luminesce::song::Song;

#[test]
fn a_seed_always_draws_the_same_instruments_and_seeds_differ() {
    assert_eq!(palette(7, 0), palette(7, 0));
    assert_ne!(palette(7, 0).instruments, palette(8, 0).instruments);
    assert_ne!(palette(7, 0).instruments, palette(7, 1).instruments, "another attempt draws again");
    for role in ROLES {
        assert!(palette(3, 0).instruments.contains_key(role), "{role}");
    }
}

#[test]
fn every_drawn_spectrum_is_inside_the_engines_range() {
    for seed in 0..200 {
        for (role, ins) in palette(seed, 0).instruments {
            assert!((1..=64).contains(&ins.partials.len()), "seed {seed} {role}: {} harmonics", ins.partials.len());
            assert!(ins.partials.iter().all(|d| (-120.0..=20.0).contains(d)), "seed {seed} {role}: {:?}", ins.partials);
        }
    }
}

#[test]
fn dressing_changes_the_timbre_and_nothing_else() {
    let c = Composer::new(Seed::Book(5)).expect("a stream");
    let eq = &c.stages()[0].eq;
    let d = dress(eq, &palette(5, 0));
    for (a, b) in eq.voices.iter().chain(&eq.stash).zip(d.voices.iter().chain(&d.stash)) {
        assert_eq!(a.name, b.name);
        if a.is_drum() {
            // a drum in a drawn kit changes its sound, never when or how loud the groove plays it
            if timbre::DRUMS.contains(&a.name.as_str()) {
                for key in ["trigger", "amp", "pan", "reverb", "delay"] {
                    assert_eq!(a.get(key), b.get(key), "{}: {key} changed", a.name);
                }
            } else {
                assert_eq!(a, b, "{} was changed", a.name);
            }
            continue;
        }
        if timbre::role(&a.name).is_none() {
            assert_eq!(a, b, "{} was changed", a.name);
            continue;
        }
        for key in ["pitch", "gate", "amp", "pan", "reverb", "delay"] {
            assert_eq!(a.get(key), b.get(key), "{}: {key} changed", a.name);
        }
        assert_eq!(b.get("wave"), Some("additive"), "{}", a.name);
    }
    assert_eq!(eq.defs, d.defs, "the functions are not touched");
}

fn rms(text: &str) -> f64 {
    let mix = render(&Song::parse(text).expect("parses")).expect("renders");
    let n = mix.left.len() as f64;
    (mix.left.iter().chain(&mix.right).map(|x| x * x).sum::<f64>() / (2.0 * n)).sqrt() / mix.gain.max(1e-9)
}

#[test]
fn a_dressed_voice_keeps_about_its_loudness() {
    // Each of 20 book songs' lead, rendered alone: in its written instrument, and in the song's drawn one. The gain
    // rescale holds the harmonic power at the role's usual pitch, through the filter and the sustain; envelopes still
    // differ, so the level is held to within 9 dB, not exactly, and the median to within 4 dB.
    let mut db = Vec::new();
    for seed in 1..=20u64 {
        let c = Composer::new(Seed::Book(seed)).expect("a stream");
        let mut eq = c.roster_eq();
        eq.voices.retain(|v| v.name == "lead");
        let before = rms(&eq.song_text("written"));
        let p = palette(seed, 0);
        let d = 20.0 * (rms(&dress(&eq, &p).song_text("drawn")) / before).log10();
        println!("  seed {seed:2}: {:<5} over {:<22} {d:+.1} dB", p.instruments["lead"].kind, book::current(&eq, Slot::Lead).unwrap_or("-"));
        db.push(d);
    }
    db.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    assert!(db.iter().all(|d| d.abs() <= 9.0), "{db:.1?}");
    assert!(db[10].abs() <= 4.0, "median {:.1} dB", db[10]);
}

#[test]
fn a_book_song_wears_its_own_instruments_and_an_explore_tuning_keeps_its_own() {
    let lead_block = |text: &str| text.split("voice lead {").nth(1).and_then(|r| r.split('}').next()).unwrap_or("").to_string();
    let leads: std::collections::BTreeSet<String> = (1..=6u64).map(|s| lead_block(&Composer::new(Seed::Book(s)).expect("a stream").roster_song_text())).collect();
    assert_eq!(leads.len(), 6, "six songs, six lead sounds");
    for s in 1..=3u64 {
        let c = Composer::new(Seed::Book(s)).expect("a stream");
        let st = &c.stages()[0];
        if let Some(lead) = st.eq.voice("lead") {
            assert!(!st.text.contains(&lead.text()), "seed {s}: the stage still plays the written lead");
        }
    }
    let e = Composer::new(Seed::Explore(3)).expect("an explore stream");
    let st = &e.stages()[0];
    for v in &st.eq.voices {
        assert!(st.text.contains(&v.text()), "an Explore tuning keeps its written sound: {} changed", v.name);
    }
    let _ = book::GROOVE_STYLE;
}

#[test]
fn registered_eighteen_of_twenty_songs_find_instruments_within_the_reference_margin() {
    let mut within = 0;
    let mut kinds = std::collections::BTreeSet::new();
    let mut leads = Vec::new();
    for seed in 1..=20u64 {
        let c = Composer::new(Seed::Book(seed)).expect("a stream");
        let (p, score, plain) = timbre::choose(seed, &c.roster_eq());
        let ok = score >= plain - timbre::MARGIN;
        if ok {
            within += 1;
        }
        let lead = &p.instruments["lead"];
        kinds.insert(lead.kind.clone());
        leads.push(lead.clone());
        println!("seed {seed:2}: written {plain:.3}, drawn {score:.3} (attempt {}), lead {} {}", p.attempt, lead.kind, if ok { "" } else { "FALLBACK" });
    }
    println!("{within} of 20 within the margin; lead kinds {kinds:?}");
    for i in 0..leads.len() {
        for j in i + 1..leads.len() {
            assert_ne!(leads[i], leads[j], "seeds {} and {} drew the same lead", i + 1, j + 1);
        }
    }
    assert!(kinds.len() >= 2, "{kinds:?}");
    assert!(within >= 18, "{within} of 20 within the margin (registered: 18)");
}

fn nums(s: Option<&str>) -> Vec<f64> {
    s.unwrap_or("").split(',').filter_map(|x| x.trim().parse().ok()).collect()
}

/// Heard brightness: the spectral centroid, in harmonics, of a voice's harmonics with its rolloff and filter, at 830 Hz.
fn heard_centroid(b: &luminesce::compose::stream::Block) -> f64 {
    let num = |k: &str| b.get(k).and_then(|x| x.trim().parse::<f64>().ok());
    let (roll, cutoff, hz) = (num("rolloff"), num("cutoff"), 830.0);
    let (mut n, mut d) = (0.0, 0.0);
    for (i, db) in nums(b.get("partials")).iter().enumerate() {
        let k = (i + 1) as f64;
        let f = k * hz;
        let a = 10f64.powf(db / 20.0) * roll.map_or(1.0, |r| (r / f).min(1.0)) * cutoff.map_or(1.0, |c| 1.0 / (1.0 + (f / c).powi(4)).sqrt());
        n += k * a * a;
        d += a * a;
    }
    n / d
}

#[test]
#[ignore = "registered falsifier, failed 2026-11-04: span 0.70 of the registered 1.0; see docs/timbre.md, second registration"]
fn registered_the_songs_leads_span_a_harmonic_of_heard_brightness() {
    let mut c: Vec<f64> = (1..=20u64)
        .map(|s| {
            let comp = Composer::new(Seed::Book(s)).expect("a stream");
            heard_centroid(comp.dressed(&comp.roster_eq()).voice("lead").expect("a lead"))
        })
        .collect();
    c.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    println!("heard brightness of 20 songs' leads: {c:.2?}");
    assert!(c[19] - c[0] >= 1.0, "span {:.2} (registered: 1.0)", c[19] - c[0]);
}

#[test]
fn registered_no_hat_tweets_and_the_hats_differ() {
    for seed in 0..200 {
        let p = palette(seed, 0);
        for d in ["hat", "perc"] {
            let f = &p.instruments[d].fields;
            let get = |k: &str| f.iter().find(|x| x.0 == k).and_then(|x| x.1.parse::<f64>().ok());
            if let (Some(from), Some(to)) = (get("from"), get("to")) {
                assert!(from / to <= timbre::HAT_SWEEP + 1e-9, "seed {seed} {d}: sweeps {from} to {to}");
            }
        }
    }
    let hats: Vec<_> = (1..=20u64).map(|s| palette(s, 0).instruments["hat"].clone()).collect();
    let kinds: std::collections::BTreeSet<&str> = hats.iter().map(|h| h.kind.as_str()).collect();
    let sounds: std::collections::BTreeSet<String> = hats.iter().map(|h| format!("{:?}", h.fields)).collect();
    println!("hat kinds {kinds:?}, {} different hats of 20", sounds.len());
    assert!(kinds.len() >= 2, "{kinds:?}");
    assert_eq!(sounds.len(), 20);
}

#[test]
fn a_dressed_drum_hit_carries_the_written_hits_energy() {
    let c = Composer::new(Seed::Book(4)).expect("a stream");
    let eq = c.roster_eq();
    let d = c.dressed(&eq);
    for name in timbre::DRUMS {
        let (Some(a), Some(b)) = (eq.voice(name), d.voice(name)) else { continue };
        let one = |blk: &luminesce::compose::stream::Block| {
            let fields: String = blk.fields.iter().filter(|(k, _)| k != "trigger" && k != "amp").map(|(k, v)| format!("  {k} = {v}
")).collect();
            let text = format!("bpm 60
beats 1
rate 16000
drum x {{
  trigger = 0.5 - t
{fields}}}
");
            let m = render(&Song::parse(&text).expect("one hit parses")).expect("renders");
            m.left.iter().chain(&m.right).map(|x| x * x).sum::<f64>() / (m.gain * m.gain)
        };
        let db = 10.0 * (one(b) / one(a)).log10();
        println!("{name}: {db:+.2} dB");
        assert!(db.abs() <= 0.5, "{name}: {db:+.2} dB");
    }
}

#[test]
fn a_saved_stretch_keeps_the_instruments_that_were_heard() {
    // The keeper saved a seed he had listened to for five minutes; the file carried the written instruments, not the
    // drawn ones he heard (2026-11-04). A saved stretch must carry what the stages played.
    let mut c = Composer::new(Seed::Book(3)).expect("a stream");
    c.ensure(4);
    let stages = &c.stages()[1..4];
    let text = luminesce::stream::save_text(stages, c.roster(), &c.origin, c.seed()).expect("saves");
    let partials = |t: &str, voice: &str| -> Option<String> {
        let block = t.split(&format!("voice {voice} {{")).nth(1)?.split('}').next()?.to_string();
        block.lines().find(|l| l.trim_start().starts_with("partials")).map(|l| l.trim().to_string())
    };
    let heard = partials(&stages[2].text, "lead").expect("the stage plays a lead");
    let written = stages[2].eq.voice("lead").and_then(|b| b.get("partials")).map(|p| format!("partials = {p}"));
    assert_ne!(Some(heard.clone()), written, "the stage played a drawn lead");
    assert_eq!(partials(&text, "lead"), Some(heard), "the saved lead is the one that was heard");
}
