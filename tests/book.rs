//! The move book (src/compose/book.txt, src/compose/book.rs): every module parses and renders alone and over every
//! progression, the melodic modules sit in the corpus's bands, and the stream that draws from the book keeps v2's
//! guarantees (determinism, audibility, the budget, seamless seams) and chooses modules by the arc's need.

use luminesce::compose::audible::{change, profile, THRESHOLD};
use luminesce::compose::book::{self, book, line_stats, solo_song, Module, Need, Slot};
use luminesce::compose::rng::Rng;
use luminesce::compose::stream::{
    apply_with, def_count, grammar_lead, layers, Composer, Ctx, Move, Role, Seed, MAX_BOOK_DEFS, MAX_LAYERS, STAGE_BEATS,
};
use luminesce::render::render;
use luminesce::song::Song;
use std::collections::{BTreeMap, BTreeSet};

fn progressions() -> Vec<&'static Module> {
    book().slot(Slot::Progression).collect()
}

fn generators() -> Vec<&'static Module> {
    book().modules.iter().filter(|m| !matches!(m.slot, Slot::Progression | Slot::Seam)).collect()
}

// ------------------------------------------------------------------------------------------------ the book itself

#[test]
fn the_book_has_its_modules_with_descriptions_and_tags() {
    let b = book();
    let count = |s: Slot| b.slot(s).count();
    let gens = generators().len();
    println!("generators {gens}, progressions {}, seams {}", count(Slot::Progression), count(Slot::Seam));
    for s in Slot::ALL {
        println!("  {:12} {}", s.name(), count(s));
    }
    assert!(gens >= 30, "{gens} generator modules");
    // The progressions are the written ones of harmony.txt (2026-11-03), 16 to 24 of them.
    assert!((16..=24).contains(&count(Slot::Progression)));
    assert!(count(Slot::Seam) >= 8);
    for s in [Slot::Lead, Slot::Counter, Slot::Arp, Slot::Bass, Slot::Pads, Slot::Kick, Slot::Hat, Slot::Clap, Slot::Perc] {
        assert!(count(s) >= 2, "{} has {}", s.name(), count(s));
    }
    let mut ids = BTreeSet::new();
    for m in &b.modules {
        assert!(ids.insert(m.id.clone()), "{} twice", m.id);
        assert!(m.describe.len() > 20, "{}: {}", m.id, m.describe);
        assert!((1..=3).contains(&m.energy) && (1..=3).contains(&m.density), "{}", m.id);
        assert!(!m.mood.is_empty(), "{} has no mood", m.id);
        assert!(m.id.starts_with(&format!("{}.", m.slot.name())) || m.slot == Slot::Progression && m.id.starts_with("prog."), "{}", m.id);
    }
    // Every slot has a quiet and a busier choice, so the arc can choose by need.
    for s in [Slot::Lead, Slot::Bass, Slot::Kick, Slot::Hat] {
        let e: BTreeSet<u8> = b.slot(s).map(|m| m.energy).collect();
        assert!(e.len() >= 2, "{}: energies {e:?}", s.name());
    }
}

#[test]
fn modules_of_different_slots_never_disagree_about_a_name() {
    let b = book();
    let mut owner: BTreeMap<String, (Slot, String, String)> = BTreeMap::new();
    let helper_names: Vec<&str> = b.helpers.iter().chain(&b.frame).map(|d| d.name.as_str()).collect();
    for m in &b.modules {
        for d in &m.defs {
            assert!(!helper_names.contains(&d.name.as_str()), "{} redefines the shared {}", m.id, d.name);
            if let Some((slot, id, line)) = owner.get(&d.name) {
                assert!(*slot == m.slot || *line == d.line, "{} and {} both write {} differently", id, m.id, d.name);
            } else {
                owner.insert(d.name.clone(), (m.slot, m.id.clone(), d.line.clone()));
            }
        }
        // A module's voices are its slot's.
        for bl in &m.blocks {
            assert!(book::ROSTER.contains(&bl.name.as_str()), "{}: voice {}", m.id, bl.name);
        }
    }
}

#[test]
fn every_module_renders_alone_over_every_progression() {
    let mut n = 0;
    for p in progressions() {
        for m in book().modules.iter().filter(|m| m.slot != Slot::Progression) {
            let text = solo_song(m, p, 24_000);
            let song = Song::parse(&text).unwrap_or_else(|e| panic!("{} over {}: {e}\n{text}", m.id, p.id));
            assert_eq!(song.beats, STAGE_BEATS);
            let mix = render(&song).unwrap_or_else(|e| panic!("{} over {}: {e}", m.id, p.id));
            assert!(mix.left.iter().chain(&mix.right).all(|x| x.is_finite()), "{} over {}: NaN", m.id, p.id);
            assert!(mix.peak > 0.0, "{} over {} is silent", m.id, p.id);
            // Levels in range: one module alone stays well under the whole mix's ceiling before any scaling.
            assert!(mix.raw_peak < 1.0, "{} over {} peaks at {:.2}", m.id, p.id, mix.raw_peak);
            n += 1;
        }
    }
    println!("{n} renders");
}

#[test]
fn all_modules_together_render_over_every_progression() {
    // The fullest combination: one module of every generator slot at once (the first of each), over each progression.
    for p in progressions() {
        let mut eq = book::install(&book::assemble(1), p);
        for b in std::mem::take(&mut eq.stash) {
            eq.insert_voice(b);
        }
        book::full(&mut eq);
        let song = Song::parse(&eq.song_text("all")).unwrap_or_else(|e| panic!("{}: {e}", p.id));
        let mix = render(&song).unwrap();
        assert!(mix.left.iter().all(|x| x.is_finite()) && mix.peak > 0.0, "{}", p.id);
        assert!(mix.raw_peak < 4.0, "{}: raw peak {:.2}", p.id, mix.raw_peak);
    }
}

/// A melodic module's statistics, averaged over every progression.
fn melodic_stats(m: &Module) -> book::LineStats {
    let ps = progressions();
    let mut acc = book::LineStats::default();
    for p in &ps {
        let song = Song::parse(&solo_song(m, p, 12_000)).unwrap();
        let s = line_stats(&song, m.blocks[0].name.as_str()).unwrap();
        acc.chord_all += s.chord_all / ps.len() as f64;
        acc.chord_strong += s.chord_strong / ps.len() as f64;
        acc.steps += s.steps / ps.len() as f64;
        acc.range += s.range / ps.len() as f64;
        acc.repetition += s.repetition / ps.len() as f64;
        acc.notes_per_beat += s.notes_per_beat / ps.len() as f64;
        acc.glide_share += s.glide_share / ps.len() as f64;
        acc.median += s.median / ps.len() as f64;
    }
    acc
}

#[test]
fn melodic_modules_sit_in_the_corpus_bands() {
    // Corpus (docs/research/desmos_corpus.md, section 4): beat notes on triad tones 0.72-0.75 (critic band 0.6-0.9,
    // random 0.43); steps 0.23-0.77 (band 0.2-0.8); lead range 9-12 (band 9-14); bar repetition 0.36-0.58.
    println!("{:28} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6}", "module", "chord", "strong", "steps", "range", "repeat", "notes", "glide", "median");
    for m in book().modules.iter().filter(|m| m.slot.melodic()) {
        let s = melodic_stats(m);
        println!(
            "{:28} {:6.2} {:6.2} {:6.2} {:6.1} {:6.2} {:6.2} {:6.2} {:6.1}",
            m.id, s.chord_all, s.chord_strong, s.steps, s.range, s.repetition, s.notes_per_beat, s.glide_share, s.median
        );
        if std::env::var("BOOK_STATS").is_ok() {
            continue; // print the whole table while tuning
        }
        assert!((0.6..=0.9).contains(&s.chord_all), "{}: beat notes on chord tones {:.2}", m.id, s.chord_all);
        assert!(s.chord_strong >= 0.8, "{}: strong beats on chord tones {:.2}", m.id, s.chord_strong);
        if m.slot == Slot::Lead {
            // The corpus's step and range bands were measured on leads.
            assert!((0.2..=0.8).contains(&s.steps), "{}: steps {:.2}", m.id, s.steps);
            assert!((8.0..=15.0).contains(&s.range), "{}: range {:.1}", m.id, s.range);
            assert!((-3.0..=20.0).contains(&s.median), "{}: median {:.1}", m.id, s.median);
        }
    }
}

#[test]
fn the_glide_tags_are_true_and_a_third_of_the_melodic_modules_glide() {
    let mel: Vec<&Module> = book().modules.iter().filter(|m| matches!(m.slot, Slot::Lead | Slot::Counter | Slot::Arp)).collect();
    let p = progressions()[0];
    let mut gliding = 0;
    for m in &mel {
        let song = Song::parse(&solo_song(m, p, 12_000)).unwrap();
        let s = line_stats(&song, m.blocks[0].name.as_str()).unwrap();
        println!("{:28} glide share {:.3} (tagged {})", m.id, s.glide_share, m.glide);
        if m.glide {
            assert!(s.glide_share >= 0.04, "{} is tagged glide but bends only {:.3} of the time", m.id, s.glide_share);
            gliding += 1;
        } else {
            assert!(s.glide_share < 0.04, "{} bends {:.3} of the time but is not tagged", m.id, s.glide_share);
        }
    }
    println!("{gliding} of {} melodic modules glide", mel.len());
    assert!(3 * gliding >= mel.len());
}

// ------------------------------------------------------------------------------------------------ the stream

fn chain(seed: u64, stages: u64) -> Composer {
    let mut c = Composer::new(Seed::Book(seed)).expect("the book composes");
    c.ensure(stages - 1);
    c
}

#[test]
fn a_book_stream_is_deterministic_and_every_stage_renders() {
    let a = chain(7, 12);
    let b = chain(7, 12);
    let ta: Vec<&str> = a.stages().iter().map(|s| s.text.as_str()).collect();
    assert_eq!(ta, b.stages().iter().map(|s| s.text.as_str()).collect::<Vec<_>>());
    assert_ne!(ta, chain(8, 12).stages().iter().map(|s| s.text.as_str()).collect::<Vec<_>>());
    for s in a.stages() {
        let song = Song::parse(&s.text).unwrap_or_else(|e| panic!("stage {}: {e}\n{}", s.index + 1, s.text));
        let mix = render(&song).unwrap();
        assert!(mix.peak > 0.0);
        assert!(!s.eq.book.is_empty());
    }
    // It opens on one of the lens species' openings (since 2026-11-03 not always the bass and the hats: every stream
    // was heard to start the same way; tests/species.rs measures the spread over many seeds).
    let opened = layers(&a.stages()[0].eq);
    assert!(book().species.open.iter().any(|o| { let mut x = o.clone(); x.sort(); let mut y = opened.clone(); y.sort(); x == y }), "{opened:?}");
}

#[test]
fn every_book_stage_is_audible_within_the_budget_and_never_repeats() {
    let mut all = Vec::new();
    for seed in [1u64, 2, 3, 4, 5] {
        let c = chain(seed, 41);
        let profiles: Vec<_> = c.stages().iter().map(|s| profile(&Song::parse(&s.text).unwrap())).collect();
        // The recorded audibility is what the composer measured: the played text up to version 2; from version 3 the
        // stage without the voices' listening (Composer::judged_text, docs/dialogue.md), so a seed takes the same moves
        // at both versions. The played change (x) must still be audible.
        let judged: Vec<_> = c.stages().iter().map(|s| profile(&Song::parse(&c.judged_text(s)).unwrap())).collect();
        for (i, w) in profiles.windows(2).enumerate() {
            let s = &c.stages()[i + 1];
            let x = change(&w[0], &w[1]).total();
            let j = change(&judged[i], &judged[i + 1]).total();
            assert!((j - s.audibility).abs() < 1e-9, "seed {seed} stage {}: recorded {} measured {j}", s.index + 1, s.audibility);
            assert!(x >= THRESHOLD, "seed {seed} stage {} changes by {x:.2}: {}", s.index + 1, s.summary);
            all.push(x);
        }
        for (i, s) in c.stages().iter().enumerate() {
            assert!(layers(&s.eq).len() <= MAX_LAYERS, "stage {}", i + 1);
            assert!(def_count(&s.eq) <= MAX_BOOK_DEFS, "seed {seed} stage {} has {} definitions", i + 1, def_count(&s.eq));
            for back in 1..=8.min(i) {
                let o = &c.stages()[i - back];
                assert!(o.eq.defs != s.eq.defs || o.eq.voices != s.eq.voices, "stage {} repeats stage {}", i + 1, i + 1 - back);
            }
        }
    }
    let mean = all.iter().sum::<f64>() / all.len() as f64;
    println!("book: {} changes, mean {mean:.2}, min {:.2}", all.len(), all.iter().cloned().fold(f64::INFINITY, f64::min));
}

#[test]
fn the_stream_draws_its_content_from_the_book() {
    let c = chain(3, 30);
    let mut seen = BTreeSet::new();
    let mut counts = BTreeMap::new();
    for s in c.stages() {
        for (slot, id) in &s.eq.book {
            seen.insert(id.clone());
            // The coda's progression is its home progression's first four bars and a written IV V7 I (harmony::coda).
            let coda = slot == "progression" && id.strip_suffix(".coda").is_some_and(|home| book().get(home).is_some());
            assert!(book().get(id).is_some() || id.starts_with("lead.grammar-") || coda, "{slot}: {id}");
        }
        for word in ["a new melody, lead.", "a new progression, prog.", "takes ", "into the seam, seam."] {
            if s.summary.contains(word) {
                *counts.entry(word).or_insert(0) += 1;
            }
        }
    }
    println!("{} modules heard in 30 stages; {counts:?}", seen.len());
    assert!(seen.len() >= 15);
    assert!(counts.len() == 4, "{counts:?}");
}

#[test]
fn modules_are_chosen_by_the_arcs_need() {
    // Over many picks, a peak's choices carry more energy than a breakdown's.
    let mut r = Rng::new(5);
    for slot in [Slot::Lead, Slot::Bass, Slot::Hat, Slot::Progression] {
        let mean = |need: Need, r: &mut Rng| -> f64 {
            (0..400).map(|_| book::pick(book(), slot, need, None, &[], r).unwrap().energy as f64).sum::<f64>() / 400.0
        };
        let (lo, hi) = (mean(Need { energy: 1, density: 1 }, &mut r), mean(Need { energy: 3, density: 3 }, &mut r));
        println!("{}: breakdown {lo:.2}, peak {hi:.2}", slot.name());
        assert!(hi > lo + 0.5, "{}: {lo} vs {hi}", slot.name());
    }
    // In a stream: the modules that come in at peaks are busier than those at intros and breakdowns.
    let (mut quiet, mut loud) = (Vec::new(), Vec::new());
    for seed in 1..=4 {
        let c = chain(seed, 30);
        for w in c.stages().windows(2) {
            for (slot, id) in &w[1].eq.book {
                if slot == "seam" || w[0].eq.book.iter().any(|(_, x)| x == id) {
                    continue;
                }
                let e = book().get(id).map_or(2, |m| m.energy) as f64;
                match w[1].role {
                    Role::Intro | Role::Strip => quiet.push(e),
                    Role::Peak | Role::Return => loud.push(e),
                    _ => {}
                }
            }
        }
    }
    let m = |v: &[f64]| v.iter().sum::<f64>() / v.len().max(1) as f64;
    println!("new modules: intro/breakdown energy {:.2} ({}), peak/return {:.2} ({})", m(&quiet), quiet.len(), m(&loud), loud.len());
    assert!(m(&loud) > m(&quiet));
}

#[test]
fn a_seam_module_marks_the_seams_into_a_return_and_a_breakdown_and_leaves_after() {
    let c = chain(4, 24);
    let st = c.stages();
    let mut marked = 0;
    for w in st.windows(2) {
        if matches!(w[1].role, Role::Return | Role::Strip) {
            assert!(w[0].eq.book.iter().any(|(s, _)| s == "seam"), "stage {} leads into a {} without a seam", w[0].index + 1, w[1].role.word());
            marked += 1;
        }
        // A seam lasts one stage: a stage holds one only when it placed one into its own next seam.
        if w[1].eq.book.iter().any(|(s, _)| s == "seam") {
            assert!(w[1].summary.contains("into the seam"), "stage {} keeps a seam: {}", w[1].index + 1, w[1].summary);
        }
    }
    assert!(marked >= 4);
}

#[test]
fn the_grammar_remains_the_fallback_when_the_book_has_no_lead_left() {
    let c = chain(2, 6);
    let mut eq = c.stages().last().unwrap().eq.clone();
    if eq.voice("lead").is_none() {
        let b = eq.stash.iter().find(|b| b.name == "lead").unwrap().clone();
        eq.insert_voice(b);
    }
    let every_lead: Vec<String> = book().slot(Slot::Lead).map(|m| m.id.clone()).collect();
    let ctx = Ctx { need: Need::default(), avoid: every_lead, stale: Vec::new() };
    let e = apply_with(Move::Melody, &eq, &mut Rng::new(3), &ctx).expect("the fallback applies");
    assert!(e.words.contains("from the grammar"), "{}", e.words);
    let id = &e.eq.book.iter().find(|(s, _)| s == "lead").unwrap().1;
    assert!(id.starts_with("lead.grammar-"), "{id}");
    let song = Song::parse(&e.eq.song_text("fallback")).unwrap();
    render(&song).unwrap();
    // And straight: every family writes a parsing lead.
    for k in 0..20 {
        let (e, _) = grammar_lead(&eq, &mut Rng::new(k)).unwrap();
        Song::parse(&e.song_text("x")).unwrap_or_else(|err| panic!("{err}\n{}", e.song_text("x")));
    }
}

#[test]
fn no_bass_module_is_the_bare_root_on_every_beat() {
    // Basslines should be fluid and more complex, never the root on every beat.
    // In every bar of every bass module over every progression: an onset off the beat (syncopation, a push, a ghost),
    // or a sounding pitch other than the bar's bass note (a fifth, an octave, an approach, a slide).
    use luminesce::compose::critic::{read_tracks, GRID};
    for m in book().slot(Slot::Bass) {
        for p in progressions() {
            let song = Song::parse(&solo_song(m, p, 24_000)).unwrap();
            let tracks = read_tracks(&song);
            let tr = tracks.iter().find(|t| t.name == "bass").unwrap();
            for bar in 0..8 {
                let (a, b) = (bar * 4 * GRID, (bar + 1) * 4 * GRID);
                let off_beat = (a..b).any(|i| i > 0 && tr.open[i] && !tr.open[i - 1] && i % GRID != 0);
                let t = bar as f64 * 4.0 + 1e-6;
                let rp = song.program.call_at("Hr", &[2.0 * bar as f64], t).unwrap().round();
                let moves = (a..b).any(|i| tr.open[i] && tr.pitch[i].is_finite() && (tr.pitch[i].round() - rp).rem_euclid(12.0) != 0.0);
                assert!(off_beat || moves, "{} over {}: bar {} is the root on the beat and nothing else", m.id, p.id, bar + 1);
            }
        }
    }
}

#[test]
fn bass_lines_rise_in_activity_with_energy() {
    // Staged by energy: the intro basses play fewer notes than the peak basses.
    let p = progressions()[0];
    let mut by: BTreeMap<u8, Vec<f64>> = BTreeMap::new();
    for m in book().slot(Slot::Bass) {
        let song = Song::parse(&solo_song(m, p, 24_000)).unwrap();
        let s = line_stats(&song, "bass").unwrap();
        println!("{:18} energy {} notes a beat {:.2} glide {:.2}", m.id, m.energy, s.notes_per_beat, s.glide_share);
        by.entry(m.energy).or_default().push(s.notes_per_beat);
    }
    let mean = |v: &Vec<f64>| v.iter().sum::<f64>() / v.len() as f64;
    assert!(mean(&by[&1]) < mean(&by[&2]) && mean(&by[&2]) < mean(&by[&3]), "{by:?}");
}

/// The roughest moment at a seam (within two samples of it) against the music's own roughest in the quarter second
/// before it and in the quarter second after it. A click is a spike above both; a new line that starts higher is as
/// rough after the seam as at it (the second difference grows with the pitch), so it is not counted as one.
fn seam_ratio(audio: &[f32], seam: usize, rate: usize) -> (f64, f64) {
    let d2 = |i: usize, ch: usize| (audio[2 * (i + 1) + ch] - 2.0 * audio[2 * i + ch] + audio[2 * (i - 1) + ch]).abs() as f64;
    let at = (seam - 2..=seam + 2).flat_map(|i| [d2(i, 0), d2(i, 1)]).fold(0.0, f64::max);
    let before = (seam - rate / 4..seam - 3).flat_map(|i| [d2(i, 0), d2(i, 1)]).fold(0.0, f64::max);
    let after = (seam + 3..seam + rate / 4).flat_map(|i| [d2(i, 0), d2(i, 1)]).fold(0.0, f64::max);
    (at, before.max(after))
}

#[test]
fn book_seams_are_no_rougher_than_the_music_and_would_be_without_the_carried_state() {
    use luminesce::song::VoiceKind;
    use luminesce::stream::{stage_start, Renderer};
    for seed in [11u64, 4] {
        let c = chain(seed, 12);
        let first = Song::parse(&c.stages()[0].text).unwrap();
        let roster = c.roster().to_vec();
        let rs = Song::parse(&c.roster_song_text()).unwrap();
        let drums: Vec<bool> = roster.iter().map(|n| rs.voices.iter().any(|v| &v.name == n && matches!(v.kind, VoiceKind::Drum(_)))).collect();
        let run = |reset: bool| -> Vec<f32> {
            let mut r = Renderer::new(&first, roster.clone(), drums.clone(), 48);
            r.reset_at_seams = reset;
            c.stages().iter().flat_map(|s| r.render(&Song::parse(&s.text).unwrap()).unwrap().audio).collect()
        };
        let (carried, reset) = (run(false), run(true));
        let mut caught = 0;
        for k in 1..12u64 {
            let seam = stage_start(k, first.bpm, first.rate) as usize;
            let (at, music) = seam_ratio(&carried, seam, 48_000);
            println!("seed {seed} seam {k}: {at:.5} against {music:.5} ({:.2}x)", at / music);
            assert!(at <= music, "seed {seed} seam {k}: {at} is rougher than the music around it ({music})");
            let (at_r, music_r) = seam_ratio(&reset, seam, 48_000);
            if at_r > music_r {
                caught += 1;
            }
        }
        println!("seed {seed}: the measure catches {caught} of 11 seams of a renderer that drops its state");
        assert!(caught >= 3);
    }
}

#[test]
fn a_book_stream_pulls_the_same_samples_twice_and_never_clips() {
    use luminesce::stream::Stream;
    let s = Stream::start(Seed::Book(11), 1, 0).unwrap();
    let frames = s.stage_start(5) as usize;
    let mut audio = vec![0.0f32; 2 * frames];
    assert_eq!(s.pull(&mut audio).unwrap(), frames);
    let peak = audio.iter().fold(0.0f32, |a, x| a.max(x.abs()));
    assert!(peak as f64 <= luminesce::render::CEILING + 1e-6 && peak > 0.2, "peak {peak}");
    let again = Stream::start(Seed::Book(11), 1, 0).unwrap();
    let mut b = vec![0.0f32; 2 * frames];
    assert_eq!(again.pull(&mut b).unwrap(), frames);
    assert!(audio == b);
}

/// A pad held across a seam whose instrument changes there (another module's partials, gain and amp).
fn swapped_pad(partials: &str, gain: f64, amp: &str) -> String {
    format!(
        "bpm 120\nbeats 32\nvoice pad {{\n  wave = additive\n  partials = {partials}\n  rolloff = 660\n  pitch = -14 + 3 (t > 16)\n  gate = 1\n  amp = {amp}\n  gain = {gain}\n  attack = 50\n  release = 80\n}}\n"
    )
}

#[test]
fn a_voice_whose_instrument_changes_at_a_seam_is_released_and_struck_again_without_a_click() {
    use luminesce::stream::{stage_start, Renderer};
    let a = Song::parse(&swapped_pad("0, -12, -5, -11, -9", 0.2, "1")).unwrap();
    let b = Song::parse(&swapped_pad("0, -9, -14.3, -18.1, -21", 0.25, "0.35 + 0.1 sin(t)")).unwrap();
    let mut r = Renderer::new(&a, vec!["pad".into()], vec![false], 48);
    let mut audio = r.render(&a).unwrap().audio;
    let second = r.render(&b).unwrap();
    audio.extend(&second.audio);
    let seam = stage_start(1, 120.0, a.rate) as usize;
    let (at, music) = seam_ratio(&audio, seam, a.rate as usize);
    println!("instrument swap at a seam: {at:.5} against {music:.5}");
    assert!(at <= music, "a click: {at} against {music}");
    // The old pad rings out under the new one: the trace is the playing one's (a fresh note at the seam).
    let luminesce::render::VoiceTrace::Tonal(f) = &second.traces[0] else { panic!("tonal") };
    assert!(f[0].level < 0.2, "the new pad starts from rest, not at the old one's level ({})", f[0].level);
}

// ------------------------------------------------------------------------------------------------ the ending

use luminesce::compose::stream::{CLOSURE_THRESHOLD, MAX_SONG_SECS, MIN_SONG_SECS, RING_RESERVE_SECS};

fn whole_song(seed: u64) -> Composer {
    let mut c = Composer::new(Seed::Book(seed)).unwrap();
    c.ensure(1000);
    assert!(c.ended(), "seed {seed} never ended");
    c
}

#[test]
fn every_song_ends_within_its_bounds_by_its_own_closure() {
    let mut by_closure = 0;
    for seed in 1..=10u64 {
        let c = whole_song(seed);
        let st = c.stages();
        let n = st.len();
        let (coda, ring) = (&st[n - 2], &st[n - 1]);
        assert_eq!((coda.role, ring.role), (Role::Coda, Role::Coda));
        assert!(st[..n - 2].iter().all(|s| s.role != Role::Coda), "seed {seed}: a coda before the end");
        let t = c.stage_secs();
        let coda_end = (n - 1) as f64 * t;
        println!(
            "seed {seed}: {n} stages, coda ends at {coda_end:.0} s, closure before it {:.2}, {}",
            st[n - 3].closure,
            coda.summary
        );
        assert!(coda_end >= MIN_SONG_SECS, "seed {seed} ends at {coda_end:.0} s");
        assert!(coda_end + RING_RESERVE_SECS <= MAX_SONG_SECS, "seed {seed} ends at {coda_end:.0} s");
        if st[n - 3].closure >= CLOSURE_THRESHOLD {
            by_closure += 1;
        }
        // Every stage of it still parses and is audible.
        for w in st.windows(2) {
            let (a, b) = (profile(&Song::parse(&w[0].text).unwrap()), profile(&Song::parse(&w[1].text).unwrap()));
            assert!(change(&a, &b).total() >= THRESHOLD, "seed {seed} stage {}: {}", w[1].index + 1, w[1].summary);
        }
    }
    println!("{by_closure} of 10 ended because they had said what they had to say");
    assert!(by_closure >= 5, "most songs should end by closure, not by the clock");
}

#[test]
fn an_ending_is_deterministic_per_seed() {
    let (a, b) = (whole_song(3), whole_song(3));
    assert_eq!(a.stages().len(), b.stages().len());
    assert!(a.stages().iter().zip(b.stages()).all(|(x, y)| x.text == y.text));
    assert_ne!(a.stages().last().unwrap().text, whole_song(4).stages().last().unwrap().text);
}

#[test]
fn the_closure_rises_over_a_song() {
    for seed in 1..=6u64 {
        let c = whole_song(seed);
        let cl: Vec<f64> = c.stages().iter().map(|s| s.closure).collect();
        let n = cl.len() - 2; // before the coda
        let half = n / 2;
        let first = cl[..half].iter().sum::<f64>() / half as f64;
        let second = cl[half..n].iter().sum::<f64>() / (n - half) as f64;
        println!("seed {seed}: closure {:?}", cl.iter().map(|x| (x * 100.0).round() / 100.0).collect::<Vec<_>>());
        assert!(second > first + 0.15, "seed {seed}: {first:.2} then {second:.2}");
        let opening = cl[..3].iter().cloned().fold(f64::INFINITY, f64::min);
        assert!(cl[n - 1] > opening + 0.3, "seed {seed}: from {opening:.2} to {:.2}", cl[n - 1]);
    }
}

#[test]
fn the_final_stage_resolves_to_the_tonic_with_a_last_kick_and_rings_out() {
    use luminesce::song::VoiceKind;
    for seed in [1u64, 2, 5] {
        let c = whole_song(seed);
        let st = c.stages();
        let coda = Song::parse(&st[st.len() - 2].text).unwrap();
        let p = &coda.program;
        // The progression lands on the tonic for the last two bars, after IV (or iv) and V: its bass notes, half bar by
        // half bar, a fourth, then a fifth, then home (since the progressions are written, harmony::coda).
        let tonic = p.call("h", &[0.0]).unwrap().round();
        let bass: Vec<f64> = (8..16).map(|i| (p.call("Hr", &[i as f64]).unwrap() - tonic).rem_euclid(12.0)).collect();
        assert_eq!(&bass[1..], &[5.0, 7.0, 7.0, 0.0, 0.0, 0.0, 0.0], "seed {seed}: {bass:?}");
        // At score beat 24.5 (the ritard over), the bass and the lead sound the tonic.
        let t = 26.2857 + 1.0;
        for name in ["bass", "lead"] {
            let v = coda.voices.iter().find(|v| v.name == name).unwrap_or_else(|| panic!("seed {seed}: no {name} in the coda"));
            let VoiceKind::Tonal(tn) = &v.kind else { panic!("tonal") };
            assert!(tn.gate.eval(p, t) > 0.0, "seed {seed}: the {name} is not sounding at the end");
            let pc = (tn.pitch.eval(p, t).round() - tonic).rem_euclid(12.0);
            assert_eq!(pc, 0.0, "seed {seed}: the {name} ends off the tonic");
            assert!(tn.gate.eval(p, 30.0) <= 0.0, "seed {seed}: the {name} still plays at the end");
        }
        // The last kick dash, once, on the tonic's downbeat.
        let k = coda.voices.iter().find(|v| v.name == "kick").unwrap();
        let VoiceKind::Drum(d) = &k.kind else { panic!("drum") };
        let hits: Vec<f64> = (1..32 * 96)
            .map(|i| i as f64 / 96.0)
            .filter(|&t| d.trigger.eval(p, t) > 0.0 && d.trigger.eval(p, t - 1.0 / 96.0) <= 0.0)
            .collect();
        let last = *hits.last().unwrap();
        assert!((last - 26.2857).abs() < 0.05, "seed {seed}: the last kick at {last}");
        // The ring-out plays nothing new.
        let ring = Song::parse(&st[st.len() - 1].text).unwrap();
        assert!(profile(&ring).voices.iter().all(|v| v.onsets.iter().all(|o| !o)), "seed {seed}");
    }
}

#[test]
fn a_streamed_song_stops_at_its_end_and_says_so_through_the_c_abi() {
    use luminesce::stream::Stream;
    let s = Stream::start(Seed::Book(2), 1, 0).unwrap();
    let mut buf = vec![0f32; 2 * 48_000];
    let mut total = 0usize;
    let mut last_loud = 0usize;
    loop {
        let got = s.pull(&mut buf).unwrap();
        for j in 0..got {
            if buf[2 * j].abs() > 1e-3 {
                last_loud = total + j;
            }
        }
        total += got;
        if got < 48_000 {
            break;
        }
        assert!(total < 48_000 * 600, "the stream never ended");
    }
    let end = s.ended_frames().expect("ended") as usize;
    assert_eq!(total, end);
    assert_eq!(s.pull(&mut buf).unwrap(), 0, "nothing after the end");
    let secs = end as f64 / 48_000.0;
    println!("seed 2 ends after {secs:.1} s (last sound above -60 dBFS at {:.1} s)", last_loud as f64 / 48_000.0);
    assert!((MIN_SONG_SECS..=MAX_SONG_SECS).contains(&secs));
    // It rings out: the last loud sample is in the final seconds, not long before.
    assert!(end - last_loud < 48_000 * 12);
    // The C ABI says the same.
    let mut err = [0 as std::ffi::c_char; 256];
    // The lens stream through the C ABI (since 2026-11-03 a seeded stream without LM_STREAM_LENS picks a species).
    let flags = luminesce::ffi::LM_STREAM_GRAMMAR | luminesce::ffi::LM_STREAM_LENS;
    let h = unsafe { luminesce::ffi::lm_stream_start(std::ptr::null(), 2, flags, 1, 0, err.as_mut_ptr(), err.len()) };
    assert!(!h.is_null());
    let mut f = 0u64;
    assert_eq!(unsafe { luminesce::ffi::lm_stream_ended(h, &mut f) }, 0, "not ended while it plays");
    let mut big = vec![0f32; 2 * 48_000 * 30];
    let mut pulled = 0u64;
    loop {
        let got = unsafe { luminesce::ffi::lm_stream_pull(h, big.as_mut_ptr(), 48_000 * 30) };
        pulled += got;
        if got < 48_000 * 30 {
            break;
        }
    }
    assert_eq!(unsafe { luminesce::ffi::lm_stream_ended(h, &mut f) }, 1);
    assert_eq!((f, pulled), (end as u64, end as u64));
    assert_eq!(unsafe { luminesce::ffi::lm_stream_ended(std::ptr::null(), &mut f) }, 0);
    unsafe { luminesce::ffi::lm_stream_free(h) };
}

#[test]
fn the_deceptive_turn_plays_at_most_once_a_song_and_the_song_keeps_its_home_key() {
    use luminesce::compose::stream::key_of;
    let mut keys = BTreeSet::new();
    let mut deceived = 0;
    for seed in 1..=12u64 {
        let c = whole_song(seed);
        let st = c.stages();
        let dec = |s: &luminesce::compose::stream::Stage| {
            book::current(&s.eq, Slot::Progression).and_then(|id| book().get(id)).is_some_and(|m| m.mood.iter().any(|w| w == "deceptive"))
        };
        // One run of stages at most on the deceptive progression (it may hold for several stages, never come back).
        let runs = st.windows(2).filter(|w| !dec(&w[0]) && dec(&w[1])).count() + dec(&st[0]) as usize;
        assert!(runs <= 1, "seed {seed}: the deceptive turn comes {runs} times");
        deceived += runs;
        // A key from the seed, the same at the coda as at the start, whatever a turn did in between.
        let home = key_of(&st[0].eq).unwrap();
        assert!((-5..=2).contains(&home), "seed {seed}: key {home}");
        assert_eq!(key_of(&st[st.len() - 2].eq), Some(home), "seed {seed}: the coda is not in the home key");
        keys.insert(home);
    }
    println!("{deceived} deceptive turns in 12 songs; keys {keys:?}");
    // The guard itself: once a song has played the deceptive progression, it is spent.
    let c = (1..=12u64).map(whole_song).find(|c| c.stages().iter().any(|s| book::current(&s.eq, Slot::Progression) == Some("prog.deceptive"))).expect("a song with the turn");
    let k = c.stages().iter().position(|s| book::current(&s.eq, Slot::Progression) == Some("prog.deceptive")).unwrap();
    assert!(luminesce::compose::stream::spent_progressions(&c.stages()[..k]).is_empty());
    assert_eq!(luminesce::compose::stream::spent_progressions(&c.stages()[..=k]), ["prog.deceptive"]);
    assert!(keys.len() >= 4, "keys {keys:?}");
}
