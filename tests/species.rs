//! Song species (src/compose/book.rs, src/compose/species/*.txt): each species is its own universe of constraints
//! (tuning, meter, generator, form, timbre, tempo) written as its own book. These tests hold every species to the
//! stream's guarantees (it parses, renders without NaN, stays in level, its seams are seamless, a seed always gives
//! the same song, its stages are audible within the budget), check that the scaffolds really differ, and prove the
//! maths each tuning and meter stands on: exact ratios, the 19-tone and Bohlen-Pierce steps, loops that realign only
//! at their least common multiple, the logistic map and the cellular automaton as written.

use luminesce::compose::audible::{change, profile, THRESHOLD};
use luminesce::compose::book::{self, all, explore_for_seed, named, opening_shape, solo_song, species_for_seed, Book, Slot};
use luminesce::compose::harmony;
use luminesce::compose::stream::{def_count, layers, Composer, Seed, MAX_BOOK_DEFS, MAX_LAYERS, TIME_NAMES};
use luminesce::expr::{ca_step, Program};
use luminesce::render::render;
use luminesce::song::Song;
use std::collections::{BTreeMap, BTreeSet};

/// The roster's drum names: a voice keeps its kind in every species (the world draws a roster voice one way).
const DRUMS: [&str; 5] = ["kick", "clap", "hat", "perc", "fill"];

fn species() -> Vec<&'static Book> {
    all().iter().collect()
}

fn named_chain(name: &str, seed: u64, stages: u64) -> Composer {
    let mut c = Composer::new(Seed::Named { species: name.to_string(), seed }).unwrap_or_else(|e| panic!("{name}: {e}"));
    c.ensure(stages - 1);
    c
}

// ------------------------------------------------------------------------------------------------ the books

#[test]
fn there_are_at_least_nine_species_each_a_complete_book() {
    let names: Vec<&str> = species().iter().map(|b| b.species.name.as_str()).collect();
    println!("{names:?}");
    assert!(names.len() >= 9, "{names:?}");
    assert_eq!(names[0], "lens");
    assert_eq!(names.iter().collect::<BTreeSet<_>>().len(), names.len(), "a name twice");
    let mut ids = BTreeSet::new();
    for b in species() {
        let s = &b.species;
        for field in [&s.title, &s.tuning, &s.meter, &s.generator, &s.form, &s.timbre] {
            assert!(field.len() > 3, "{}: an empty header field", s.name);
        }
        assert!(b.slot(Slot::Progression).count() >= 2, "{}: progressions", s.name);
        assert!(b.slot(Slot::Lead).count() >= 3, "{}: leads", s.name);
        assert!(b.slot(Slot::Seam).count() >= 1, "{}: seams", s.name);
        let here: BTreeSet<String> = b
            .modules
            .iter()
            .filter(|m| m.slot != Slot::Seam)
            .flat_map(|m| m.blocks.iter().map(|bl| luminesce::compose::audible::layer_of(&bl.name)))
            .collect();
        for o in &s.open {
            for l in o {
                assert!(here.contains(l), "{}: opens on {l}, which none of its modules plays", s.name);
            }
        }
        for m in &b.modules {
            assert!(ids.insert(m.id.clone()), "{} twice", m.id);
            if s.name != "lens" {
                assert!(m.id.starts_with(&format!("{}/", s.name)), "{}", m.id);
            }
            for bl in &m.blocks {
                assert!(book::ROSTER.contains(&bl.name.as_str()), "{}: voice {}", m.id, bl.name);
                assert_eq!(bl.is_drum(), DRUMS.contains(&bl.name.as_str()), "{}: {} is a {}", m.id, bl.name, bl.kind);
            }
        }
    }
}

/// Every name a species defines, its time names and the stream's own (`roll`, `sweep`, the stage clock `S`, the
/// key `K` a modulation adds) left out, and the written harmony's shared vocabulary too (the frame, the helpers and a
/// progression's tables, the same in every harmony species by design since 2026-11-03: the harmony is written once,
/// and what makes a species its own is everything played on top of it).
fn vocabulary(b: &Book) -> BTreeSet<String> {
    let shared: BTreeSet<String> = format!("{}\n{}", harmony::FRAME, harmony::HELPERS)
        .lines()
        .filter_map(|l| l.split(['(', '=']).next().map(|n| n.trim().to_string()))
        .filter(|n| !n.is_empty())
        .chain(harmony::TABLE_NAMES.iter().map(|n| n.to_string()))
        .chain(["c".to_string()])
        .collect();
    b.helpers
        .iter()
        .chain(&b.frame)
        .chain(b.modules.iter().flat_map(|m| &m.defs))
        .map(|d| d.name.clone())
        .filter(|n| !TIME_NAMES.contains(&n.as_str()) && !book::SEAM_DEFS.contains(&n.as_str()) && n != "S" && n != "K")
        .filter(|n| !(b.species.harmony && shared.contains(n)))
        .collect()
}

#[test]
fn every_species_writes_its_own_equation() {
    // Every stream's function used to look nearly the same. No two species may share more than a small
    // fraction of their definition names (Jaccard index, shared over all).
    let sp = species();
    let mut worst = 0.0f64;
    for (i, a) in sp.iter().enumerate() {
        let va = vocabulary(a);
        // A harmony species gave its own chord arithmetic to the shared layer (2026-11-03): ten names of its own at least.
        let least = if a.species.harmony { 10 } else { 12 };
        assert!(va.len() >= least, "{}: only {} names", a.species.name, va.len());
        for b in &sp[i + 1..] {
            let vb = vocabulary(b);
            if [a.species.name.as_str(), b.species.name.as_str()] == ["bounce", "maqam"] {
                continue; // one species in two tunings, by design (see species_differ_on_several_axes_at_once)
            }
            let shared: Vec<&String> = va.intersection(&vb).collect();
            let j = shared.len() as f64 / va.union(&vb).count() as f64;
            worst = worst.max(j);
            println!("{:9} {:9} share {:2} names ({:.2}) {shared:?}", a.species.name, b.species.name, shared.len(), j);
            assert!(j <= 0.1, "{} and {} share {:.2} of their names: {shared:?}", a.species.name, b.species.name, j);
        }
    }
    println!("the largest overlap between two species: {worst:.2}");
}

#[test]
fn species_differ_on_several_axes_at_once() {
    let sp = species();
    for (i, a) in sp.iter().enumerate() {
        for b in &sp[i + 1..] {
            let (x, y) = (&a.species, &b.species);
            // Two harmony species share their tuning by design (12-tone, a written progression in the song's key):
            // they must then differ on all four other axes.
            let tuning = !(x.harmony && y.harmony) && x.tuning != y.tuning;
            let axes = [tuning, x.meter != y.meter, x.generator != y.generator, x.timbre != y.timbre, x.bpm != y.bpm];
            let n = axes.iter().filter(|d| **d).count();
            // `maqam` is the bounce as it was, in quarter tones, kept whole behind Explore when the bounce took written
            // harmony (2026-11-03): one species in two tunings, by design.
            if [x.name.as_str(), y.name.as_str()] == ["bounce", "maqam"] {
                assert!(x.tuning != y.tuning && x.harmony && y.explore);
                continue;
            }
            assert!(n >= 4, "{} and {} differ on only {n} axes", x.name, y.name);
        }
    }
}

#[test]
fn every_module_of_every_species_renders_alone_over_each_of_its_progressions() {
    let mut n = 0;
    let mut silent = Vec::new();
    for b in species().into_iter().skip(1) {
        for p in b.slot(Slot::Progression) {
            for m in b.modules.iter().filter(|m| m.slot != Slot::Progression) {
                let text = solo_song(m, p, 24_000);
                let song = Song::parse(&text).unwrap_or_else(|e| panic!("{} over {}: {e}\n{text}", m.id, p.id));
                let mix = render(&song).unwrap_or_else(|e| panic!("{} over {}: {e}", m.id, p.id));
                if !(mix.raw_peak.is_finite() && mix.raw_peak > 1e-4) {
                    silent.push(format!("{} over {}", m.id, p.id));
                }
                n += 1;
            }
        }
    }
    println!("{n} renders");
    assert!(silent.is_empty(), "silent: {silent:?}");
}

// ------------------------------------------------------------------------------------------------ the streams

#[test]
fn every_species_streams_deterministically_audibly_and_within_its_budget() {
    for b in species() {
        let name = b.species.name.as_str();
        let a = named_chain(name, 3, 14);
        let again = named_chain(name, 3, 14);
        let ta: Vec<&str> = a.stages().iter().map(|s| s.text.as_str()).collect();
        assert_eq!(ta, again.stages().iter().map(|s| s.text.as_str()).collect::<Vec<_>>(), "{name}: not deterministic");
        let bpm = a.stages()[0].eq.settings.iter().find_map(|l| l.strip_prefix("bpm ").and_then(|v| v.parse::<i64>().ok())).unwrap();
        assert!((b.species.bpm.0..=b.species.bpm.1).contains(&bpm), "{name}: bpm {bpm}");
        let mut below = 0;
        let mut sum = 0.0;
        let profiles: Vec<_> = a.stages().iter().map(|s| profile(&Song::parse(&s.text).unwrap())).collect();
        for (i, s) in a.stages().iter().enumerate() {
            let song = Song::parse(&s.text).unwrap_or_else(|e| panic!("{name} stage {}: {e}\n{}", i + 1, s.text));
            let mix = render(&song).unwrap_or_else(|e| panic!("{name} stage {}: {e}", i + 1));
            // The ring-out (a song's last stage) closes every gate: what sounds there is the coda's tail, carried.
            let ring = s.summary.ends_with("it rings out");
            assert!(mix.raw_peak.is_finite() && (ring || mix.raw_peak > 1e-3), "{name} stage {}: peak {}
{}", i + 1, mix.raw_peak, s.text);
            assert!(layers(&s.eq).len() <= MAX_LAYERS, "{name} stage {}", i + 1);
            assert!(def_count(&s.eq) <= MAX_BOOK_DEFS, "{name} stage {} has {} definitions", i + 1, def_count(&s.eq));
            assert_eq!(s.eq.species, if name == "lens" { "" } else { name }, "{name}: a named stream never changes species");
            if i > 0 {
                let x = change(&profiles[i - 1], &profiles[i]).total();
                sum += x;
                if x < THRESHOLD {
                    below += 1;
                }
            }
        }
        let mean = sum / (a.stages().len() - 1) as f64;
        println!("{name:9} bpm {bpm}: mean audibility {mean:.2}, {below} of {} changes under {THRESHOLD}; {}", a.stages().len() - 1, a.stages()[0].summary);
        assert!(below <= 2, "{name}: {below} stage changes nobody would hear");
        assert!(mean >= 1.5, "{name}: mean change {mean:.2}");
    }
}

#[test]
fn a_seed_lands_on_every_species_and_the_kept_one_most() {
    let mut count: BTreeMap<String, usize> = BTreeMap::new();
    for seed in 1..=600u64 {
        *count.entry(species_for_seed(seed).species.name.clone()).or_default() += 1;
    }
    println!("{count:?}");
    // The default stream reaches every default species and never one behind the Explore option (since 2026-11-03).
    for b in species() {
        let n = count.get(&b.species.name).copied().unwrap_or(0);
        if b.species.explore {
            assert_eq!(n, 0, "{} is behind Explore but the default stream chose it", b.species.name);
        } else if !b.species.drawn {
            // `draw no` (the phrase species, 2026-11-05): reached only by name until a generator version takes it in, so
            // that adding it moves no existing seed to another species.
            assert_eq!(n, 0, "{} is draw no but a species seed chose it", b.species.name);
        } else {
            assert!(b.species.harmony && n >= 20, "{} is reached {count:?}", b.species.name);
        }
    }
    let mut strange: BTreeMap<String, usize> = BTreeMap::new();
    for seed in 1..=300u64 {
        *strange.entry(explore_for_seed(seed).species.name.clone()).or_default() += 1;
    }
    println!("explore: {strange:?}");
    let explore: Vec<&str> = species().iter().filter(|b| b.species.explore).map(|b| b.species.name.as_str()).collect();
    assert_eq!(explore, ["ratios", "loops", "tritave", "maqam"]);
    assert!(explore.iter().all(|n| strange.get(*n).copied().unwrap_or(0) >= 30) && strange.len() == explore.len(), "{strange:?}");
    // Weighted toward what was kept (the lens: the pluck, the wandering blancmange; the dialogue: the voice over
    // stillness), never only those.
    let plain: Vec<usize> = species().iter().filter(|b| b.species.weight == 1.0 && !b.species.explore && b.species.drawn).map(|b| count[&b.species.name]).collect();
    let mean = plain.iter().sum::<usize>() as f64 / plain.len() as f64;
    for kept in ["lens", "dialogue"] {
        assert!(count[kept] as f64 > 1.3 * mean, "{kept}: {} against {mean:.0}", count[kept]);
    }
    assert!(count["lens"] + count["dialogue"] < 600 / 2, "the kept species take {:?}", (count["lens"], count["dialogue"]));
}

#[test]
fn streams_open_many_ways_and_on_many_basses() {
    // 2026-11-03: repeated testing kept hearing the same bassline. Two of the
    // last three streams opened on the same bass module, and every stream opened on the bass and the hats.
    let mut bass: BTreeMap<String, usize> = BTreeMap::new();
    let mut opens: BTreeMap<String, usize> = BTreeMap::new();
    let n = 160;
    for seed in 1..=n as u64 {
        let c = Composer::new(Seed::Species(seed)).unwrap();
        let eq = &c.stages()[0].eq;
        let b = eq.book.iter().find(|(s, _)| s == "bass").map_or("none".to_string(), |x| x.1.clone());
        *bass.entry(b).or_default() += 1;
        let mut l = layers(eq);
        l.sort();
        *opens.entry(l.join("+")).or_default() += 1;
    }
    println!("{} basses: {bass:?}\n{} openings: {opens:?}", bass.len(), opens.len());
    let top_bass = bass.iter().filter(|(k, _)| *k != "none").map(|(_, v)| *v).max().unwrap();
    // Since 2026-11-03 the default stream draws from the six species with written harmony (19 basses, was 27 in nine):
    // a dialogue bass (weight 1.5 of 7.7, one of three) is expected in 6.5% of streams, so the bound is twice the
    // likeliest bass's share rather than the 8% the larger pool allowed.
    assert!(top_bass * 100 <= 13 * n, "one bass opens {top_bass} of {n} streams");
    assert!(bass.len() >= 20, "only {} basses", bass.len());
    let top_open = opens.values().copied().max().unwrap();
    assert!(top_open * 100 <= 20 * n, "one opening takes {top_open} of {n}");
    assert!(opens.get("bass+hat").copied().unwrap_or(0) * 100 <= 10 * n, "bass and hats open {:?} of {n}", opens.get("bass+hat"));
    // And within the lens species alone, which used to open on the bass and the hats every time.
    let mut lens_bass: BTreeMap<String, usize> = BTreeMap::new();
    for seed in 1..=80u64 {
        let c = Composer::new(Seed::Book(seed)).unwrap();
        let b = book::current(&c.stages()[0].eq, Slot::Bass).unwrap().to_string();
        *lens_bass.entry(b).or_default() += 1;
    }
    println!("lens basses: {lens_bass:?}");
    assert!(lens_bass.values().all(|&v| v * 100 <= 25 * 80), "{lens_bass:?}");
    assert_eq!(lens_bass.len(), book::book().slot(Slot::Bass).count());
}

#[test]
fn the_bass_changes_as_a_song_goes_on() {
    // A playing bass whose module has sat for three stages is the first a swap takes: over a song, the longest run of
    // stages on one bass module stays short.
    use luminesce::compose::stream::BASS_STAYS;
    let mut runs = Vec::new();
    let mut total = 0;
    for seed in 1..=6u64 {
        let mut c = Composer::new(Seed::Book(seed)).unwrap();
        c.ensure(21);
        let mut run = 0usize;
        let mut longest = 0usize;
        let mut last: Option<String> = None;
        let mut changes = 0;
        for s in c.stages() {
            if !layers(&s.eq).iter().any(|l| l == "bass") {
                continue;
            }
            let b = book::current(&s.eq, Slot::Bass).map(str::to_string);
            if b == last {
                run += 1;
            } else {
                changes += 1;
                run = 1;
            }
            longest = longest.max(run);
            last = b;
        }
        let playing = c.stages().iter().filter(|s| layers(&s.eq).iter().any(|l| l == "bass")).count();
        println!("seed {seed}: {changes} basses heard in {playing} stages with the bass, the longest run {longest} stages");
        runs.push(longest);
        total += changes;
        // Three basses wherever the bass plays long enough for the swap rule to take it twice (a bass that sits out most
        // of a song, as seed 1's does in 15 of 22 stages, is swapped as often as the rule allows, not three times).
        assert!(changes >= 3 || playing < 3 * BASS_STAYS, "seed {seed}: {changes} basses in {playing} stages with the bass");
    }
    assert!(runs.iter().all(|&r| r <= 6), "{runs:?}");
    assert!(total >= 3 * 6, "{total} basses over six songs");
}

#[test]
fn a_species_stream_sometimes_changes_species_at_a_new_arc_and_a_named_one_never() {
    let mut changed = 0;
    for seed in 1..=30u64 {
        let mut c = Composer::new(Seed::Species(seed)).unwrap();
        c.ensure(17);
        let st = c.stages();
        for w in st.windows(2) {
            if w[0].eq.species != w[1].eq.species {
                changed += 1;
                // A default stream changes only to another default species (written harmony, never a strange tuning).
                let to = if w[1].eq.species.is_empty() { "lens" } else { w[1].eq.species.as_str() };
                assert!(!named(to).unwrap().species.explore, "seed {seed}: turned to {to}, behind Explore");
                assert!(w[1].summary.contains("a new species"), "{}", w[1].summary);
                assert_ne!(w[0].arc, w[1].arc, "seed {seed}: a species changes only at an arc's start");
                let bpm = |s: &luminesce::compose::stream::Stage| s.eq.settings.iter().find(|l| l.starts_with("bpm ")).cloned();
                assert_eq!(bpm(&w[0]), bpm(&w[1]), "the tempo stays");
                Song::parse(&w[1].text).unwrap();
            }
        }
    }
    println!("{changed} species changes in 30 streams of 18 stages");
    assert!((2..=15).contains(&changed), "{changed}");
}

#[test]
fn every_species_song_ends_by_itself_within_its_bounds() {
    use luminesce::compose::stream::{Role, MAX_SONG_SECS, MIN_SONG_SECS};
    for b in species() {
        let name = b.species.name.as_str();
        let mut c = Composer::new(Seed::Named { species: name.to_string(), seed: 5 }).unwrap();
        c.ensure(60);
        assert!(c.ended(), "{name} never ends");
        let st = c.stages();
        let secs = st.len() as f64 * c.stage_secs();
        assert!(st.iter().rev().take(2).all(|s| s.role == Role::Coda), "{name}: ends with a coda and a ring-out");
        assert!(secs >= MIN_SONG_SECS - c.stage_secs() && secs <= MAX_SONG_SECS + c.stage_secs(), "{name}: {secs:.0} s");
        let coda = Song::parse(&st[st.len() - 2].text).unwrap_or_else(|e| panic!("{name} coda: {e}\n{}", st[st.len() - 2].text));
        render(&coda).unwrap();
        println!("{name:9} ends after {} stages, {secs:.0} s", st.len());
    }
}

/// The roughest moment at a seam (within two samples of it) against the music's own roughest in `span` samples either
/// side of it (as in tests/book.rs, where the span is a quarter second).
fn seam_ratio(audio: &[f32], seam: usize, span: usize) -> (f64, f64) {
    let d2 = |i: usize, ch: usize| (audio[2 * (i + 1) + ch] - 2.0 * audio[2 * i + ch] + audio[2 * (i - 1) + ch]).abs() as f64;
    let at = (seam - 2..=seam + 2).flat_map(|i| [d2(i, 0), d2(i, 1)]).fold(0.0, f64::max);
    let before = (seam - span..seam - 3).flat_map(|i| [d2(i, 0), d2(i, 1)]).fold(0.0, f64::max);
    let after = (seam + 3..seam + span).flat_map(|i| [d2(i, 0), d2(i, 1)]).fold(0.0, f64::max);
    (at, before.max(after))
}

#[test]
fn every_species_pulls_the_same_samples_twice_and_never_clips() {
    use luminesce::stream::Stream;
    for b in species() {
        let name = b.species.name.clone();
        let start = || Stream::start(Seed::Named { species: name.clone(), seed: 2 }, 1, 0).unwrap();
        let s = start();
        let frames = s.stage_start(4) as usize;
        let mut audio = vec![0.0f32; 2 * frames];
        assert_eq!(s.pull(&mut audio).unwrap(), frames);
        assert!(audio.iter().all(|x| x.is_finite()), "{name}: not finite");
        let peak = audio.iter().fold(0.0f32, |a, x| a.max(x.abs()));
        assert!(peak as f64 <= luminesce::render::CEILING + 1e-6 && peak > 0.1, "{name}: peak {peak}");
        let mut again = vec![0.0f32; 2 * frames];
        start().pull(&mut again).unwrap();
        assert!(audio == again, "{name}: not the same samples");
        println!("{name:9}: peak {peak:.3}");
    }
}

#[test]
fn every_species_seam_is_no_rougher_than_its_music_and_would_be_without_the_carried_state() {
    // The music around a seam is measured over the two stages that meet there, not a quarter second: the physical
    // species' notes come a ball's bounce apart (two beats and more), so a seam can hold a note's onset with no other
    // onset within a quarter second, and that onset is the music, not a click (measured: bounce, seed 2, seam 4, the
    // lead alone, 0.0287 at the seam against 0.0296 at its own onsets in the next stage). The control is the same
    // stages rendered with every voice's state dropped at each seam: the quarter-second measure must catch it.
    use luminesce::stream::{stage_start, Renderer};
    let mut caught_all = 0;
    for b in species() {
        let name = b.species.name.clone();
        let c = named_chain(&name, 2, 7);
        let first = Song::parse(&c.stages()[0].text).unwrap();
        let roster = c.roster().to_vec();
        let rs = Song::parse(&c.roster_song_text()).unwrap();
        let drums: Vec<bool> =
            roster.iter().map(|n| rs.voices.iter().any(|v| &v.name == n && matches!(v.kind, luminesce::song::VoiceKind::Drum(_)))).collect();
        let run = |reset: bool| -> Vec<f32> {
            let mut r = Renderer::new(&first, roster.clone(), drums.clone(), 48);
            r.reset_at_seams = reset;
            c.stages().iter().flat_map(|s| r.render(&Song::parse(&s.text).unwrap()).unwrap().audio).collect()
        };
        let (carried, reset) = (run(false), run(true));
        let stage = stage_start(1, first.bpm, first.rate) as usize;
        let mut caught = 0;
        for k in 1..c.stages().len() as u64 {
            let seam = stage_start(k, first.bpm, first.rate) as usize;
            let (at, music) = seam_ratio(&carried, seam, stage - 3);
            // A tenth of headroom: where a voice changes instrument at a seam it strikes a new note there, and two onsets
            // that meet at the seam add (bounce, seed 2, seam 5: a kick entering under a new lead, 0.0351 against the
            // stages' 0.0346).
            assert!(at <= 1.1 * music, "{name} seam {k}: {at:.5} is rougher than the music of the stages around it ({music:.5})");
            let (at_r, music_r) = seam_ratio(&reset, seam, first.rate as usize / 4);
            if at_r > music_r {
                caught += 1;
            }
        }
        println!("{name:9}: {} seams no rougher than their stages; the control caught at {caught}", c.stages().len() - 1);
        caught_all += caught;
    }
    assert!(caught_all >= 9, "the measure caught {caught_all} dropped-state seams");
}

// ------------------------------------------------------------------------------------------------ the maths

/// A species' helpers and frame (and, for a harmony species, its first progression's tables), with `extra` defining
/// or overriding names.
fn program(b: &Book, extra: &str) -> Program {
    let given: Vec<&str> = extra.lines().filter_map(|l| l.split('=').next()).map(str::trim).collect();
    let tables = b.slot(Slot::Progression).next().filter(|_| b.species.harmony).map(|m| m.defs.as_slice()).unwrap_or(&[]);
    let mut text = String::new();
    for d in b.helpers.iter().chain(&b.frame).chain(tables).filter(|d| !given.contains(&d.name.as_str())) {
        text += &d.line;
        text += "\n";
    }
    text += extra;
    Program::from_source(&text).unwrap_or_else(|e| panic!("{e}\n{text}"))
}

fn hz(semitones: f64) -> f64 {
    440.0 * 2f64.powf(semitones / 12.0)
}

#[test]
fn ratio_tuning_sounds_exact_whole_number_ratios() {
    let b = named("ratios").unwrap();
    let p = program(b, "Jz = -21\nQn = 17471731\nQd = 14361421\n");
    let root = hz(p.call("J", &[1.0]).unwrap());
    for (n, d) in [(3.0, 2.0), (5.0, 4.0), (7.0, 4.0), (7.0, 6.0), (9.0, 8.0), (6.0, 5.0)] {
        let f = hz(p.call("J", &[n / d]).unwrap());
        assert!((f / root - n / d).abs() < 1e-12, "{n}/{d}: {}", f / root);
    }
    // The progression's fundamentals are the ratios written in its digits: 1 7/4 4/3 7/6 1 7/4 3/2 1.
    let want = [1.0, 7.0 / 4.0, 4.0 / 3.0, 7.0 / 6.0, 1.0, 7.0 / 4.0, 3.0 / 2.0, 1.0];
    for (i, w) in want.iter().enumerate() {
        assert!((p.call("Q", &[i as f64]).unwrap() - w).abs() < 1e-12, "bar {i}");
    }
    // The otonal pads sound 5:6:7 exactly.
    let pads = b.get("ratios/pads.otonal").unwrap();
    let mut q = String::new();
    for d in &pads.defs {
        q += &d.line;
        q += "\n";
    }
    let p = program(b, &format!("Jz = -21\nQn = 17471731\nQd = 14361421\n{q}"));
    let f: Vec<f64> = (0..3).map(|m| hz(p.call("Jp", &[m as f64, 3.0]).unwrap())).collect();
    assert!((f[1] / f[0] - 6.0 / 5.0).abs() < 1e-12 && (f[2] / f[0] - 7.0 / 5.0).abs() < 1e-12, "{f:?}");
}

#[test]
fn nineteen_tone_and_bohlen_pierce_and_quarter_tone_steps_are_exact() {
    let loops = program(named("loops").unwrap(), "Ek = 0\nGn = 17141108060300\n");
    for k in 0..40 {
        let s = loops.call("e19", &[k as f64]).unwrap();
        assert!((s - 12.0 * k as f64 / 19.0).abs() < 1e-12);
    }
    // 19 steps are exactly an octave; the 19-tone major mode is 0 3 6 8 11 14 17, then 19.
    assert!((hz(loops.call("e19", &[19.0]).unwrap()) / hz(0.0) - 2.0).abs() < 1e-12);
    let mode: Vec<f64> = (0..8).map(|d| loops.call("Gm", &[d as f64]).unwrap()).collect();
    assert_eq!(mode, [0.0, 3.0, 6.0, 8.0, 11.0, 14.0, 17.0, 19.0]);

    let bp = program(named("tritave").unwrap(), "Tk = 0\nTn = 442265\n");
    let step = 12.0 * 3f64.log2() / 13.0;
    for k in 0..30 {
        assert!((bp.call("bp", &[k as f64]).unwrap() - step * k as f64).abs() < 1e-12);
    }
    // 13 steps are exactly a tritave, 3:1; there is no octave in it (no step within 25 cents of 1200).
    assert!((hz(bp.call("bp", &[13.0]).unwrap()) / hz(0.0) - 3.0).abs() < 1e-12);
    assert!((0..27).all(|k| ((step * k as f64) - 12.0).abs() > 0.25));
    // The Lambda mode, floor(3d/2): 0 1 3 4 6 7 9 10 12, then 13.
    let lam: Vec<f64> = (0..10).map(|d| bp.call("lam", &[d as f64]).unwrap()).collect();
    assert_eq!(lam, [0.0, 1.0, 3.0, 4.0, 6.0, 7.0, 9.0, 10.0, 12.0, 13.0]);
    // The BP major triad 3:5:7 is steps 0, 6 and 10, each within 0.2 semitone of the just ratio.
    for (k, r) in [(6.0, 5.0 / 3.0), (10.0, 7.0 / 3.0)] {
        assert!((bp.call("bp", &[k]).unwrap() - 12.0 * f64::log2(r)).abs() < 0.2);
    }

    // The quarter-tone bounce, kept whole behind the Explore option as `maqam` (2026-11-03).
    let q = program(named("maqam").unwrap(), "Mk = 0\nMr = 21181410070400\nMn = 330044\n");
    assert!((q.call("qt", &[1.0]).unwrap() - 0.5).abs() < 1e-12);
    let rast: Vec<f64> = (0..8).map(|d| q.call("qt", &[q.call("rq", &[d as f64]).unwrap()]).unwrap()).collect();
    assert_eq!(rast, [0.0, 2.0, 3.5, 5.0, 7.0, 9.0, 10.5, 12.0]);
}

#[test]
fn incommensurable_loops_realign_only_at_their_least_common_multiple() {
    // The loops species' gates lp(L, a, b) on loops of 5, 7, 11 and 13 beats, read on its stream clock w = t + 32 S.
    let b = named("loops").unwrap();
    let mut p = program(b, "Ek = 0\nGn = 17141108060300\n");
    let lens = [5.0, 7.0, 11.0, 13.0];
    let lcm = 5005usize;
    let state = |p: &Program, beat: usize| -> Vec<bool> {
        // Stage and position for the stream clock: w = t + 32 S.
        let t = (beat % 32) as f64 + 0.25;
        lens.iter().map(|&l| p.call_at("lp", &[l, 0.0, 1.0], t).unwrap() > 0.0).collect()
    };
    // Within one stage the clock is t (S = 0); over a whole stream S counts the stages, so build the program per
    // stage the way set_clock writes it.
    let mut pattern = Vec::with_capacity(2 * lcm + 1);
    let mut cur_stage = usize::MAX;
    for beat in 0..=2 * lcm {
        let stage = beat / 32;
        if stage != cur_stage {
            let mut text = String::new();
            for d in b.helpers.iter().chain(&b.frame) {
                text += &if d.name == "S" { format!("S = {stage}") } else { d.line.clone() };
                text += "\n";
            }
            text += "Ek = 0\nGn = 17141108060300\n";
            p = Program::from_source(&text).unwrap();
            cur_stage = stage;
        }
        pattern.push(state(&p, beat));
    }
    let period = (1..=lcm).find(|&shift| (0..=lcm).all(|i| pattern[i] == pattern[i + shift])).unwrap();
    assert_eq!(period, lcm, "the four loops realign every {period} beats");
    // Each loop on its own repeats at its own length.
    for (k, &l) in lens.iter().enumerate() {
        let l = l as usize;
        assert!((0..lcm).all(|i| pattern[i][k] == pattern[i + l][k]));
        assert!((1..l).all(|s| (0..lcm).any(|i| pattern[i][k] != pattern[i + s][k])));
    }
}

#[test]
fn the_stage_clock_carries_the_loops_across_seams() {
    let c = named_chain("loops", 4, 5);
    for (k, s) in c.stages().iter().enumerate() {
        let d = s.eq.def("S").expect("the clock");
        assert_eq!(d.line, format!("S = {k}"));
        let song = Song::parse(&s.text).unwrap();
        let w = song.program.compile_expr("w").unwrap();
        assert!((w.eval(&song.program, 0.0) - 32.0 * k as f64).abs() < 1e-12);
    }
}

#[test]
fn the_logistic_map_is_iterated_as_written() {
    let b = named("chaos").unwrap();
    for r in [3.5, 3.5699, 3.7, 3.8282] {
        let p = program(b, &format!("r = {r}\nY = 0\n"));
        let mut x = 0.3f64;
        for n in 0..300 {
            let got = p.call("X", &[n as f64]).unwrap();
            assert_eq!(got.to_bits(), x.to_bits(), "r {r} n {n}");
            x = r * x * (1.0 - x);
        }
    }
    // r = 3.5 settles into a cycle of four values; r = 3.7 does not repeat within 300 steps.
    let p = program(b, "r = 3.5\nY = 0\n");
    let x = |n: usize| p.call("X", &[n as f64]).unwrap();
    assert!((200..240).all(|n| (x(n) - x(n + 4)).abs() < 1e-9) && (x(200) - x(202)).abs() > 0.1);
    let p = program(b, "r = 3.7\nY = 0\n");
    let x = |n: usize| p.call("X", &[n as f64]).unwrap();
    assert!((1..=64).all(|per| (x(200) - x(200 + per)).abs() > 1e-6 || (x(201) - x(201 + per)).abs() > 1e-6));
}

#[test]
fn iter_rejects_what_is_not_a_function_of_one_value_and_guards_its_count() {
    for bad in ["F = 2\nX(n) = iter(F, 0.3, n)", "F(a, b) = a\nX(n) = iter(F, 0.3, n)", "X(n) = iter(2, 0.3, n)", "X(n) = iter(sin, 0.3, n)"] {
        assert!(Program::from_source(bad).is_err(), "{bad}");
    }
    let p = Program::from_source("F(x) = x + 1\nX(n) = iter(F, 0, n)").unwrap();
    assert_eq!(p.call("X", &[0.0]).unwrap(), 0.0);
    assert_eq!(p.call("X", &[-3.0]).unwrap(), 0.0);
    assert_eq!(p.call("X", &[7.9]).unwrap(), 7.0);
    assert_eq!(p.call("X", &[4096.0]).unwrap(), 4096.0);
    assert!(p.call("X", &[4097.0]).unwrap().is_nan());
    assert!(p.call("X", &[f64::NAN]).unwrap().is_nan());
    // A function that is not a function of one value through recursion: iter may not call what calls it.
    assert!(Program::from_source("F(x) = X(x)\nX(n) = iter(F, 0, n)").is_err());
}

#[test]
fn ca_steps_wolframs_rules_on_a_ring() {
    // Rule 30 from one live cell on ten cells (cell 0 the lowest bit; a cell's left is the next higher bit), checked
    // against a direct reading of the rule table.
    fn step(rule: u64, x: u64, w: u32) -> u64 {
        (0..w).fold(0, |o, i| {
            let l = (x >> ((i + 1) % w)) & 1;
            let c = (x >> i) & 1;
            let r = (x >> ((i + w - 1) % w)) & 1;
            o | (((rule >> (4 * l + 2 * c + r)) & 1) << i)
        })
    }
    for rule in [30u64, 90, 110, 45, 0, 255] {
        let mut x = 1u64;
        for _ in 0..40 {
            let got = ca_step(rule as f64, x as f64, 10.0);
            assert_eq!(got, step(rule, x, 10) as f64, "rule {rule} from {x}");
            x = got as u64;
        }
    }
    // Rule 90 from one cell on a wide ring is the Sierpinski row: 1, then 101, then 10001.
    assert_eq!(ca_step(90.0, 8.0, 16.0), 20.0);
    assert_eq!(ca_step(90.0, 20.0, 16.0), 34.0);
    // Invalid input is NaN: a rule past 255, a width past 52, a row too wide for its width.
    for (r, x, w) in [(256.0, 1.0, 10.0), (30.0, 1.0, 53.0), (30.0, 1024.0, 10.0), (30.0, -1.0, 10.0), (f64::NAN, 1.0, 10.0)] {
        assert!(ca_step(r, x, w).is_nan(), "{r} {x} {w}");
    }
    // In the automaton species the row integer is the CA's: rule 30 from cell 0 on ten cells.
    let b = named("automaton").unwrap();
    let p = program(b, "Wr = 30\nWz = 1\nWk = 0\n");
    let mut x = 1u64;
    for n in 0..30 {
        assert_eq!(p.call("Rw", &[n as f64]).unwrap(), x as f64, "row {n}");
        x = step(30, x, 10);
    }
}

#[test]
fn log2_is_the_octave_count_of_a_ratio() {
    let p = Program::from_source("f(x) = 12 log2(x)").unwrap();
    assert!((p.call("f", &[1.5]).unwrap() - 7.019550008653875).abs() < 1e-12);
    assert_eq!(p.call("f", &[8.0]).unwrap(), 36.0);
    assert!(p.call("f", &[-1.0]).unwrap().is_nan());
    // A constant that is not finite is refused, as any other.
    assert!(Program::from_source("c = log2(0)").is_err());
}

#[test]
fn opening_shapes_follow_the_seed() {
    for b in species() {
        let shapes: BTreeSet<Vec<String>> = (1..=60u64).map(|s| opening_shape(b, s)).collect();
        assert!(shapes.len() >= b.species.open.len().min(4), "{}: {} shapes", b.species.name, shapes.len());
    }
}

#[test]
fn a_name_after_a_number_is_read() {
    // `2v` is 2 times v: the definitions a voice reads that way are kept and their helpers added (the dialogue species
    // reads its breathing clock as floor(2v)).
    use luminesce::compose::grammar::uses;
    assert!(uses("floor(2v)", "v"));
    assert!(uses("0.5v + 1", "v"));
    assert!(!uses("x2v", "v"));
    assert!(!uses("floor(2vv)", "v"));
    assert!(uses("v", "v") && !uses("vx", "v"));
}

#[test]
fn every_species_window_holds_every_roster_voice_with_its_kind() {
    // The world draws every roster voice from the window; a species without an arpeggio, a bass or pads must still
    // hold one of each, silent, of the roster's kind (found by the C ABI window test when a seed landed on `oneline`).
    use luminesce::song::VoiceKind;
    use luminesce::stream::Stream;
    for b in species() {
        let s = Stream::start(Seed::Named { species: b.species.name.clone(), seed: 6 }, 1, 48).unwrap();
        assert_eq!(s.roster, book::ROSTER.iter().map(|n| n.to_string()).collect::<Vec<_>>(), "{}", b.species.name);
        for name in book::ROSTER {
            let v = s.roster_song.voices.iter().find(|v| v.name == name).unwrap_or_else(|| panic!("{}: no {name}", b.species.name));
            assert_eq!(matches!(v.kind, VoiceKind::Drum(_)), book::ROSTER_DRUMS.contains(&name), "{}: {name}", b.species.name);
        }
        let (_, traces, _) = s.window(0, 0).unwrap();
        assert_eq!(traces.len(), book::ROSTER.len());
    }
}

#[test]
fn the_panel_writes_log2_with_its_base_lowered() {
    use luminesce::compose::display::def_markup;
    let m = def_markup("J(r) = Jz + 12 log2(r)", None);
    assert!(m.contains(r"\rm{log}\sub{2}\paren{"), "{m}");
    let m = def_markup("X(n) = iter(F, 0.3, n)", None);
    assert!(m.contains(r"\rm{iter}\paren{"), "{m}");
}

// ------------------------------------------------------------------------------------------------ written harmony

#[test]
fn every_harmony_species_melody_lands_on_the_chord_and_wanders_between() {
    // Harmony is written, never generated (2026-11-03): every lead and counter of a species with written harmony takes
    // chord tones of the progression sounding (its bass note or a pad's pitch class) on most beats, but not only those.
    // The corpus measured 0.72-0.75 of beat notes on chord tones (docs/research/desmos_corpus.md, section 4); random
    // lines land 0.43. Measured on the 4/4 grid every species shares (the lens holds the corpus's own band in
    // tests/book.rs); a 7/8 or 5/4 line's accents fall off it, so the floor here is the corpus's lower edge.
    use luminesce::compose::book::line_stats;
    for b in species().into_iter().filter(|b| b.species.harmony) {
        let ps: Vec<_> = b.slot(Slot::Progression).step_by(3).collect();
        for m in b.modules.iter().filter(|m| m.slot.melodic()) {
            let (mut all_, mut strong) = (0.0, 0.0);
            for p in &ps {
                let song = Song::parse(&solo_song(m, p, 12_000)).unwrap();
                let s = line_stats(&song, &m.blocks[0].name).unwrap();
                all_ += s.chord_all / ps.len() as f64;
                strong += s.chord_strong / ps.len() as f64;
            }
            println!("{:10} {:32} beats on chord tones {all_:.2}, strong beats {strong:.2}", b.species.name, m.id);
            assert!(all_ >= 0.6 && strong >= 0.6, "{}: {all_:.2} of beats, {strong:.2} of strong beats on chord tones", m.id);
        }
    }
}
