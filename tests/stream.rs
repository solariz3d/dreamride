//! Stream mode: the chain of eight-bar equations (src/compose/stream.rs), the stage-by-stage renderer and its seams
//! (src/stream.rs), the typeset markup (src/compose/display.rs), and the C ABI (src/ffi.rs).

use luminesce::compose::display;
use luminesce::compose::rng::Rng;
use luminesce::compose::audible::{change, profile, THRESHOLD};
use luminesce::compose::stream::{
    apply, def_count, diff, family_of, layers, lead_median, normalise, parse_equation, plan_arc, tidy, Composer, Move, Role, Seed,
    DEFAULT_SEED_SONG, MAX_DEFS, MAX_LAYERS, STAGE_BEATS,
};
use luminesce::render::{render, VoiceTrace};
use luminesce::song::{Song, VoiceKind};
use luminesce::stream::{gain_at, save_text, stage_of, stage_start, Renderer, Stream};
use std::collections::BTreeSet;

fn chain(seed: Seed, stages: u64) -> Composer {
    let mut c = Composer::new(seed).expect("the seed composes");
    c.ensure(stages - 1);
    c
}

// ------------------------------------------------------------------------------------------------ the chain

#[test]
fn a_seed_always_writes_the_same_chain() {
    let a = chain(Seed::default_song(), 20);
    let b = chain(Seed::default_song(), 20);
    let ta: Vec<&str> = a.stages().iter().map(|s| s.text.as_str()).collect();
    let tb: Vec<&str> = b.stages().iter().map(|s| s.text.as_str()).collect();
    assert_eq!(ta, tb);
    let other = chain(Seed::Song { text: DEFAULT_SEED_SONG.to_string(), seed: 2 }, 20);
    assert_ne!(ta, other.stages().iter().map(|s| s.text.as_str()).collect::<Vec<_>>(), "another seed evolves differently");
    let g1 = chain(Seed::Grammar(5), 6);
    let g2 = chain(Seed::Grammar(5), 6);
    assert_eq!(g1.stages()[5].text, g2.stages()[5].text);
}

#[test]
fn every_stage_parses_and_renders() {
    for seed in [Seed::default_song(), Seed::Grammar(3), Seed::Grammar(11)] {
        let c = chain(seed, 10);
        for s in c.stages() {
            let song = Song::parse(&s.text).unwrap_or_else(|e| panic!("stage {} does not parse: {e}\n{}", s.index + 1, s.text));
            assert_eq!(song.beats, STAGE_BEATS);
            let mix = render(&song).unwrap_or_else(|e| panic!("stage {} does not render: {e}", s.index + 1));
            assert!(mix.peak > 0.0, "stage {} is silent", s.index + 1);
        }
    }
}

#[test]
fn tempo_never_changes_and_the_key_only_by_a_modulation() {
    let c = chain(Seed::default_song(), 60);
    let bpm0: Vec<&String> = c.stages()[0].eq.settings.iter().filter(|s| s.starts_with("bpm")).collect();
    let mut modulations = 0;
    for w in c.stages().windows(2) {
        let (a, b) = (&w[0], &w[1]);
        let bpm: Vec<&String> = b.eq.settings.iter().filter(|s| s.starts_with("bpm")).collect();
        assert_eq!(bpm, bpm0, "stage {} changed the tempo", b.index + 1);
        assert_eq!(a.eq.settings, b.eq.settings);
        let key = |s: &luminesce::compose::stream::Stage| s.eq.def("K").map(|d| d.line.clone());
        if key(a) != key(b) {
            modulations += 1;
            assert!(b.turn && b.summary.contains("modulates"), "stage {} changed K outside a modulation: {}", b.index + 1, b.summary);
        }
        // The lens's own form changes only when K joins it.
        if a.eq.def("h") != b.eq.def("h") {
            assert!(b.changed.contains(&"K".to_string()));
        }
    }
    assert!(modulations >= 1, "60 stages should hold at least one modulation");
}

#[test]
fn arcs_run_intro_builds_peak_breakdown_turn_return() {
    for seed in [1u64, 2, 3, 77, 1234] {
        for arc in 0..30 {
            let p = plan_arc(seed, arc);
            let (lo, hi) = if arc == 0 { (7, 8) } else { (6, 7) };
            assert!((lo..=hi).contains(&p.len()), "arc {arc} of seed {seed} is {} stages: {p:?}", p.len());
            assert_eq!(p[0], Role::Intro);
            assert_eq!(&p[p.len() - 4..], &[Role::Peak, Role::Strip, Role::Turn, Role::Return]);
            assert!(p.len() >= 6 && p[1..p.len() - 4].iter().all(|r| *r == Role::Build), "{p:?}");
        }
    }
    // A composed chain follows its plans, arc after arc.
    let c = chain(Seed::Grammar(3), 40);
    let mut at = 0usize;
    let mut arc = 0u64;
    while at < c.stages().len() {
        for role in plan_arc(c.seed(), arc) {
            let Some(s) = c.stages().get(at) else { break };
            assert_eq!((s.role, s.arc), (role, arc), "stage {}", at + 1);
            assert!(at == 0 || s.summary.starts_with(role.word()), "stage {}: {}", at + 1, s.summary);
            assert_eq!(s.turn, role != Role::Build, "stage {}", at + 1);
            at += 1;
        }
        arc += 1;
    }
    assert!(arc >= 5);
}

/// v1's mean stage change by the same measure, over the same 250 changes (stages 1-51 of the kept song with seeds 1
/// and 2, and grammar seeds 3, 11 and 42), measured on 2026-11-03 on the v1 composer (commit 979b532) before v2
/// replaced its moves: mean 0.822, median 0.434, 169 of 250 (68%) below the threshold.
const V1_MEAN: f64 = 0.822;

#[test]
fn every_stage_is_audible_and_the_changes_are_far_larger_than_v1s() {
    let mut all = Vec::new();
    for seed in [
        Seed::default_song(),
        Seed::Song { text: DEFAULT_SEED_SONG.to_string(), seed: 2 },
        Seed::Grammar(3),
        Seed::Grammar(11),
        Seed::Grammar(42),
    ] {
        let c = chain(seed, 51);
        let profiles: Vec<_> = c.stages().iter().map(|s| profile(&Song::parse(&s.text).unwrap())).collect();
        for (i, w) in profiles.windows(2).enumerate() {
            let s = &c.stages()[i + 1];
            let measured = change(&w[0], &w[1]).total();
            assert!((measured - s.audibility).abs() < 1e-9, "stage {} reports {} but measures {measured}", s.index + 1, s.audibility);
            assert!(measured >= THRESHOLD, "stage {} of {} changes by only {measured:.3}: {}", s.index + 1, c.origin, s.summary);
            all.push(measured);
        }
    }
    let mean = all.iter().sum::<f64>() / all.len() as f64;
    let mut sorted = all.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!(
        "v2: {} changes, mean {mean:.3}, median {:.3}, min {:.3} (v1: mean {V1_MEAN}, median 0.434, 68% below {THRESHOLD})",
        all.len(),
        sorted[sorted.len() / 2],
        sorted[0]
    );
    assert_eq!(all.len(), 250);
    assert!(mean >= 3.0 * V1_MEAN, "mean change {mean:.3} is not far above v1's {V1_MEAN}");
}

/// The kept song in stream form with every voice playing.
fn full_seed() -> luminesce::compose::stream::Equation {
    let mut eq = normalise(parse_equation(DEFAULT_SEED_SONG).unwrap());
    for b in std::mem::take(&mut eq.stash) {
        eq.voices.push(b);
    }
    eq
}

fn measure(a: &luminesce::compose::stream::Equation, b: &luminesce::compose::stream::Equation) -> f64 {
    let p = |e: &luminesce::compose::stream::Equation| profile(&Song::parse(&e.song_text("m")).unwrap());
    change(&p(a), &p(b)).total()
}

#[test]
fn the_measure_hears_a_layer_or_a_progression_but_not_a_v1_nudge() {
    let eq = full_seed();
    assert_eq!(measure(&eq, &eq), 0.0, "a stage against itself");
    // v1's commonest edits: a melody phase moved by a quarter, one chord-root digit by one.
    let mut phase = eq.clone();
    let m = phase.defs.iter_mut().find(|d| d.name == "M").unwrap();
    assert!(m.line.contains("k / 8 + 0.25"));
    m.line = m.line.replace("k / 8 + 0.25", "k / 8 + 0.5");
    let mut digit = eq.clone();
    let n = digit.defs.iter_mut().find(|d| d.name == "N").unwrap();
    n.line = "N = 57465756".into();
    for (what, e) in [("a phase", &phase), ("a root digit", &digit)] {
        let x = measure(&eq, e);
        println!("v1 nudge, {what}: {x:.3}");
        assert!(x > 0.0 && x < THRESHOLD, "{what}: {x}");
    }
    // A layer leaving, and a whole new progression.
    let mut no_kick = eq.clone();
    no_kick.voices.retain(|b| b.name != "kick");
    assert!(measure(&eq, &no_kick) >= THRESHOLD);
    let mut r = Rng::new(3);
    let prog = apply(Move::Progression, &eq, &mut r).expect("a progression applies");
    let x = measure(&eq, &prog.eq);
    println!("a new progression ({}): {x:.3}", prog.words);
    assert!(x >= THRESHOLD);
}

#[test]
fn the_budget_caps_layers_and_definitions_and_something_leaves() {
    for seed in [Seed::default_song(), Seed::Grammar(3), Seed::Grammar(11), Seed::Grammar(42)] {
        let c = chain(seed, 60);
        let start = def_count(&c.stages()[0].eq);
        for s in c.stages() {
            assert!(layers(&s.eq).len() <= MAX_LAYERS, "stage {}: {:?}", s.index + 1, layers(&s.eq));
            assert!(def_count(&s.eq) <= MAX_DEFS.max(start), "stage {} has {} definitions", s.index + 1, def_count(&s.eq));
        }
    }
    // At the budget, a layer coming in pushes another out.
    let mut eq = full_seed();
    let here = layers(&eq);
    assert!(here.len() > MAX_LAYERS);
    let extra = here.iter().rev().find(|l| *l != "lead").unwrap().clone();
    let blocks: Vec<_> = eq.voices.iter().filter(|b| luminesce::compose::audible::layer_of(&b.name) == extra).cloned().collect();
    eq.voices.retain(|b| luminesce::compose::audible::layer_of(&b.name) != extra);
    eq.stash.extend(blocks);
    assert_eq!(layers(&eq).len(), MAX_LAYERS);
    let e = apply(Move::Enter, &eq, &mut Rng::new(1)).unwrap();
    assert_eq!(layers(&e.eq).len(), MAX_LAYERS, "{}", e.words);
    assert!(e.words.contains("leave"), "{}", e.words);
    assert!(layers(&e.eq).contains(&"lead".to_string()), "the lead never leaves to make room");
    // A definition nothing reads is tidied away.
    let mut stray = full_seed();
    stray.defs.insert(0, luminesce::compose::stream::Def { name: "zz".into(), line: "zz = 3".into() });
    tidy(&mut stray);
    assert!(stray.def("zz").is_none() && stray.def("M").is_some());
}

#[test]
fn a_stage_never_returns_to_any_of_the_last_eight_states() {
    let c = chain(Seed::default_song(), 50);
    let st = c.stages();
    for (i, s) in st.iter().enumerate() {
        for back in 1..=8.min(i) {
            let o = &st[i - back];
            assert!(o.eq.defs != s.eq.defs || o.eq.voices != s.eq.voices, "stage {} repeats stage {}", i + 1, i + 1 - back);
        }
    }
}

#[test]
fn a_new_melody_is_another_family_on_the_same_bars_in_the_same_register() {
    let eq = full_seed();
    let before = lead_median(&eq).unwrap();
    let mut families = BTreeSet::new();
    for k in 0..60 {
        let mut r = Rng::new(k);
        let e = apply(Move::Melody, &eq, &mut r).expect("the melody move applies");
        let m = e.eq.def("M").unwrap().rhs().to_string();
        assert!(m.starts_with("R(floor(k / 16))"), "the bar index is kept: {m}");
        let fam = family_of(&m);
        assert_ne!(fam, "squares", "{m}");
        families.insert(fam);
        let after = lead_median(&e.eq).unwrap();
        assert!((after - before).abs() <= 6.0, "{}: the lead moved from {before} to {after}", e.words);
        Song::parse(&e.eq.song_text("m")).unwrap_or_else(|err| panic!("{}: {err}", e.words));
    }
    assert!(families.len() >= 6, "{families:?}");
}

#[test]
fn each_move_applies_parses_and_reports_its_changes() {
    // The book's stage 1 too: Swap (and the book's own forms of the other moves) act only on a book stream.
    let seeds = [
        full_seed(),
        chain(Seed::Grammar(3), 1).stages()[0].eq.clone(),
        chain(Seed::default_song(), 1).stages()[0].eq.clone(),
        chain(Seed::Book(3), 4).stages()[3].eq.clone(),
    ];
    let mut applied = BTreeSet::new();
    for eq in &seeds {
        // From the seed and from a few evolved states, so every move finds something to act on.
        let mut states = vec![eq.clone()];
        let mut r = Rng::new(99);
        for _ in 0..12 {
            let last = states.last().unwrap().clone();
            let m = Move::ALL[r.range(0, Move::ALL.len() as i64 - 1) as usize];
            if let Some(e) = apply(m, &last, &mut r) {
                states.push(e.eq);
            }
        }
        for st in &states {
            for &m in &Move::ALL {
                for k in 0..10 {
                    let mut r = Rng::new(1000 + k);
                    let Some(e) = apply(m, st, &mut r) else { continue };
                    applied.insert(format!("{m:?}"));
                    let got: BTreeSet<String> = diff(st, &e.eq).into_iter().collect();
                    let want: BTreeSet<String> = e.touched.iter().cloned().collect();
                    assert_eq!(got, want, "{m:?}: {}", e.words);
                    assert!(!got.is_empty(), "{m:?} changed nothing");
                    assert!(!e.words.is_empty());
                    assert_eq!(e.eq.settings, st.settings, "{m:?} changed a setting");
                    Song::parse(&e.eq.song_text("check"))
                        .unwrap_or_else(|err| panic!("{m:?} wrote a song that does not parse: {err}\n{}", e.eq.song_text("x")));
                }
            }
        }
    }
    for m in &Move::ALL {
        assert!(applied.contains(&format!("{m:?}")), "{m:?} never applied");
    }
}

#[test]
fn stage_one_opens_with_a_low_voice_and_a_pulse_playing_the_seeds_functions() {
    let c = chain(Seed::default_song(), 1);
    let s0 = &c.stages()[0];
    let stage = Song::parse(&s0.text).unwrap();
    let seed = Song::parse(DEFAULT_SEED_SONG).unwrap();
    for name in ["c", "N"] {
        assert!(s0.eq.def(name).is_some(), "no {name}");
    }
    assert_eq!(s0.eq.def("N").unwrap().line, "N = 57465746");
    let present: Vec<&str> = s0.eq.voices.iter().map(|b| b.name.as_str()).collect();
    assert_eq!(present, ["bass", "hat"]);
    // The lead waits in the stash with the pluck.
    let lead = s0.eq.stash.iter().find(|b| b.name == "lead").unwrap();
    assert_eq!(lead.get("partials"), Some("0, -4, -6, -12, -10, -16, -15, -21"));
    // The bass plays the seed's own functions (from the seed's second section, where it plays).
    let (VoiceKind::Tonal(a), VoiceKind::Tonal(b)) =
        (&stage.voices.iter().find(|v| v.name == "bass").unwrap().kind, &seed.voices.iter().find(|v| v.name == "bass").unwrap().kind)
    else {
        panic!("the bass is tonal")
    };
    for i in (16 * 48)..(32 * 48) {
        let t = i as f64 / 48.0 + 1e-7;
        let (ga, gb) = (a.gate.eval(&stage.program, t), b.gate.eval(&seed.program, t));
        assert_eq!(ga > 0.0, gb > 0.0, "bass gate at beat {t}");
        if gb > 0.0 {
            assert!((a.pitch.eval(&stage.program, t) - b.pitch.eval(&seed.program, t)).abs() < 1e-9, "bass pitch at {t}");
        }
    }
    // Every grammar seed opens thin too.
    for g in [3, 11, 42] {
        let c = chain(Seed::Grammar(g), 1);
        assert!(layers(&c.stages()[0].eq).len() <= 2, "grammar seed {g}: {:?}", layers(&c.stages()[0].eq));
    }
}

// ------------------------------------------------------------------------------------------------ the renderer

/// A song whose every function repeats every 32 beats, so two stages of it are one 64-beat song: a pad held across
/// the seam with a gliding pitch and an amp reading its note_age, a pluck, a dash drum hit just before the seam (still ringing over it), reverb with a
/// predelay, and a delay.
fn periodic(beats: u32) -> String {
    format!(
        "bpm 120\nbeats {beats}\nreverb_size 0.8\nreverb_predelay 15\ndelay_beats 0.75\ndelay_feedback 0.4\n\
         w = sin(tau t / 32)\n\
         voice pad {{\n  wave = additive\n  partials = 0, -6, -10\n  pitch = -14 + 3 w\n  gate = 1\n  amp = 0.8 + 0.2 sin(note_age)\n  gain = 0.1\n  attack = 300\n  release = 400\n  reverb = 0.5\n}}\n\
         voice pluck {{\n  wave = triangle\n  pitch = 3 mod(floor(2t), 4)\n  gate = 0.4 - frac(2t)\n  gain = 0.08\n  decay = 200\n  sustain = 0.3\n  release = 300\n  delay = 0.4\n  cutoff = 1500 + 800 w\n}}\n\
         drum hit {{\n  kind = dash\n  trigger = (frac(t / 8) > 0.95)(1)\n  from = 400\n  to = 60\n  length = 600\n  decay = 200\n  gain = 0.3\n  reverb = 0.2\n}}\n"
    )
}

#[test]
fn a_stream_renders_stage_after_stage_exactly_as_one_long_render() {
    let stage = Song::parse(&periodic(32)).unwrap();
    let whole = Song::parse(&periodic(64)).unwrap();
    let roster: Vec<String> = stage.voices.iter().map(|v| v.name.clone()).collect();
    let drums = stage.voices.iter().map(|v| matches!(v.kind, VoiceKind::Drum(_))).collect();
    let mut r = Renderer::new(&stage, roster, drums, 48);
    let a = r.render(&stage).unwrap();
    let b = r.render(&stage).unwrap();
    assert_eq!(a.start, 0);
    assert_eq!(b.start, 32 * 24_000);
    let mix = render(&whole).unwrap();
    assert_eq!(mix.gain, 1.0, "the reference must not be normalised");
    let mut worst = 0.0f64;
    for (k, st) in [&a, &b].iter().enumerate() {
        for i in 0..st.frames() {
            let j = st.start as usize + i;
            worst = worst.max((st.audio[2 * i] as f64 - mix.left[j]).abs()).max((st.audio[2 * i + 1] as f64 - mix.right[j]).abs());
            let _ = k;
        }
    }
    println!("the stream against one long render: worst sample difference {worst:e}");
    assert!(worst < 1e-6, "the stream differs from one long render by {worst}");
    // The voice record too: drum hits at the same samples.
    let (_, trace) = luminesce::render::render_traced(&whole, 48).unwrap();
    let VoiceTrace::Drum(hw) = &trace.voices[2] else { panic!("drum") };
    let (VoiceTrace::Drum(h1), VoiceTrace::Drum(h2)) = (&a.traces[2], &b.traces[2]) else { panic!("drum") };
    let streamed: Vec<u64> = h1.iter().chain(h2).map(|h| h.sample).collect();
    assert_eq!(streamed, hw.iter().map(|h| h.sample).collect::<Vec<_>>());
}

/// The roughest moment at a seam (the largest second difference within two samples of it) against the music's own
/// roughest moment in the quarter second before it.
fn seam_ratio(audio: &[f32], seam: usize, rate: usize) -> (f64, f64) {
    let d2 = |i: usize, ch: usize| (audio[2 * (i + 1) + ch] - 2.0 * audio[2 * i + ch] + audio[2 * (i - 1) + ch]).abs() as f64;
    let at = (seam - 2..=seam + 2).flat_map(|i| [d2(i, 0), d2(i, 1)]).fold(0.0, f64::max);
    let before = (seam - rate / 4..seam - 3).flat_map(|i| [d2(i, 0), d2(i, 1)]).fold(0.0, f64::max);
    (at, before)
}

#[test]
fn seams_are_no_rougher_than_the_music_and_would_be_without_the_carried_state() {
    let mut c = Composer::new(Seed::default_song()).unwrap();
    c.ensure(7);
    let first = Song::parse(&c.stages()[0].text).unwrap();
    let roster = c.roster().to_vec();
    let rs = Song::parse(&c.roster_song_text()).unwrap();
    let drums: Vec<bool> = roster.iter().map(|n| rs.voices.iter().any(|v| &v.name == n && matches!(v.kind, VoiceKind::Drum(_)))).collect();
    let run = |reset: bool| -> Vec<f32> {
        let mut r = Renderer::new(&first, roster.clone(), drums.clone(), 48);
        r.reset_at_seams = reset;
        let mut audio = Vec::new();
        for s in &c.stages()[..8] {
            audio.extend(r.render(&Song::parse(&s.text).unwrap()).unwrap().audio);
        }
        audio
    };
    let (carried, reset) = (run(false), run(true));
    let mut clicks_without = 0;
    for k in 1..8u64 {
        let seam = stage_start(k, first.bpm, first.rate) as usize;
        let (at, before) = seam_ratio(&carried, seam, 48_000);
        println!("seam {k}: carried {at:.5} against {before:.5} before it ({:.2}x)", at / before);
        assert!(at <= before, "seam {k}: {at} is rougher than the music before it ({before})");
        let (at_r, before_r) = seam_ratio(&reset, seam, 48_000);
        println!("        without the carried state {at_r:.5} against {before_r:.5} ({:.2}x)", at_r / before_r);
        if at_r > before_r {
            clicks_without += 1;
        }
    }
    assert!(clicks_without >= 3, "the measure should catch the seams of a renderer that drops its state ({clicks_without} of 7)");
}

#[test]
fn the_streamed_audio_is_seamless_and_never_clips() {
    let s = Stream::start(Seed::default_song(), 1, 0).unwrap();
    let k = 6u64;
    let frames = s.stage_start(k) as usize + 4800;
    let mut audio = vec![0.0f32; 2 * frames];
    assert_eq!(s.pull(&mut audio).unwrap(), frames);
    let peak = audio.iter().fold(0.0f32, |a, x| a.max(x.abs()));
    assert!(peak as f64 <= luminesce::render::CEILING + 1e-6, "peak {peak}");
    assert!(peak > 0.2, "the stream is near silent: {peak}");
    for j in 1..k {
        let seam = s.stage_start(j) as usize;
        let (at, before) = seam_ratio(&audio, seam, 48_000);
        println!("pulled seam {j}: {at:.5} against {before:.5} ({:.2}x)", at / before);
        assert!(at <= before, "seam {j}");
    }
    // Pulled again from a fresh stream in odd-sized blocks, the samples are identical.
    let s2 = Stream::start(Seed::default_song(), 1, 0).unwrap();
    let mut again = vec![0.0f32; 2 * frames];
    let mut at = 0;
    for (n, block) in [1usize, 777, 48_000, 12_345].iter().cycle().enumerate() {
        if at >= frames || n > 10_000 {
            break;
        }
        let take = (*block).min(frames - at);
        assert_eq!(s2.pull(&mut again[2 * at..2 * (at + take)]).unwrap(), take);
        at += take;
    }
    assert!(audio == again, "pulling in other block sizes changed the samples");
}

#[test]
fn the_gain_moves_between_stages_without_leaving_the_safe_range() {
    let rate = 48_000;
    let (s0, s1) = (1_000_000u64, 1_900_000u64);
    for (g, before, after) in [(0.8, Some(0.5), Some(0.6)), (0.5, Some(0.9), Some(0.7)), (0.7, None, None), (0.6, Some(0.6), Some(0.6))] {
        let mut prev = gain_at(s0, s0, s1, g, before, after, rate);
        let lo = [Some(g), before, after].iter().flatten().fold(f64::INFINITY, |a: f64, &b| a.min(b));
        for f in s0..s1 {
            let x = gain_at(f, s0, s1, g, before, after, rate);
            assert!(x <= g + 1e-12 && x >= lo - 1e-12, "{x} outside [{lo}, {g}]");
            assert!((x - prev).abs() < 1e-4, "a step of {} at {f}", (x - prev).abs());
            prev = x;
        }
        // In the middle the stage plays at its own gain; at its end it meets the next stage's start.
        assert_eq!(gain_at((s0 + s1) / 2, s0, s1, g, before, after, rate), g);
        if let Some(a) = after {
            if a < g {
                assert!((gain_at(s1, s0, s1, g, before, after, rate) - a).abs() < 1e-9);
            }
        }
    }
}

#[test]
fn stage_boundaries_tile_the_samples() {
    for (bpm, rate) in [(106.0, 48_000u32), (120.0, 48_000), (97.3, 44_100)] {
        assert_eq!(stage_start(0, bpm, rate), 0);
        for k in 0..50u64 {
            let (a, b) = (stage_start(k, bpm, rate), stage_start(k + 1, bpm, rate));
            assert!(b > a);
            // Its first sample is at or after its first beat, and the sample before is before it.
            let beat = |i: u64| i as f64 * bpm / (60.0 * rate as f64);
            assert!(beat(a) >= k as f64 * 32.0 - 1e-6 && (a == 0 || beat(a - 1) < k as f64 * 32.0));
            assert_eq!(stage_of(a, bpm, rate), k);
            assert_eq!(stage_of(b - 1, bpm, rate), k);
        }
    }
}

// ------------------------------------------------------------------------------------------------ saving

#[test]
fn a_saved_stretch_plays_the_streams_functions() {
    let c = chain(Seed::default_song(), 14);
    let stages = &c.stages()[9..14];
    let text = save_text(stages, c.roster(), &c.origin, c.seed()).unwrap();
    let saved = Song::parse(&text).unwrap();
    assert_eq!(saved.beats, 32.0 * 5.0);
    render(&saved).expect("the saved song renders");
    for (i, st) in stages.iter().enumerate() {
        let song = Song::parse(&st.text).unwrap();
        for v in &song.voices {
            let sv = saved.voices.iter().find(|x| x.name == v.name).unwrap_or_else(|| panic!("{} is saved", v.name));
            match (&v.kind, &sv.kind) {
                (VoiceKind::Tonal(a), VoiceKind::Tonal(b)) => {
                    for g in 0..(32 * 24) {
                        let t = g as f64 / 24.0 + 1e-7;
                        let ga = a.gate.eval(&song.program, t);
                        let gb = b.gate.eval(&saved.program, 32.0 * i as f64 + t);
                        assert_eq!(ga > 0.0, gb > 0.0, "{} gate, stage {} beat {t}", v.name, st.index + 1);
                        if ga > 0.0 {
                            let (pa, pb) = (a.pitch.eval(&song.program, t), b.pitch.eval(&saved.program, 32.0 * i as f64 + t));
                            assert!((pa - pb).abs() < 1e-9, "{} pitch, stage {} beat {t}: {pa} vs {pb}", v.name, st.index + 1);
                        }
                    }
                }
                (VoiceKind::Drum(a), VoiceKind::Drum(b)) => {
                    for g in 0..(32 * 48) {
                        let t = g as f64 / 48.0 + 1e-7;
                        let (ta, tb) = (a.trigger.eval(&song.program, t), b.trigger.eval(&saved.program, 32.0 * i as f64 + t));
                        assert_eq!(ta > 0.0, tb > 0.0, "{} trigger, stage {} beat {t}", v.name, st.index + 1);
                    }
                }
                _ => panic!("{} changed kind", v.name),
            }
        }
        // A voice absent in this stage is silent in the saved song there.
        for sv in &saved.voices {
            if song.voices.iter().any(|v| v.name == sv.name) {
                continue;
            }
            if let VoiceKind::Tonal(b) = &sv.kind {
                assert!(b.gate.eval(&saved.program, 32.0 * i as f64 + 5.3) <= 0.0, "{} sounds in stage {}", sv.name, st.index + 1);
            }
        }
    }
    assert!(text.starts_with("# Saved from a Dreamride stream"));
}

// ------------------------------------------------------------------------------------------------ the markup

#[test]
fn the_lens_is_typeset_as_a_floor_over_a_fraction() {
    let m = display::def_markup("h(d) = floor((12d + c) / 7)", None);
    assert_eq!(m, "\\it{h}\\paren{\\it{d}} = \\floor{\\frac{12\\it{d} + \\it{c}}{7}}");
    let m = display::def_markup("R(i) = mod(floor(N / 10^(7 - mod(i, 8))), 10) - 5", None);
    assert!(m.contains("\\frac{\\it{N}}{10\\sup{7 − \\rm{mod}\\paren{\\it{i}, 8}}}"), "{m}");
    assert!(m.ends_with(", 10}\\brk − 5"), "a top-level minus is a place to break: {m}");
    // The world's layout tests (world/tests/math_layout_test.cpp) use these exact strings.
    assert_eq!(m, "\\it{R}\\paren{\\it{i}} = \\rm{mod}\\paren{\\floor{\\frac{\\it{N}}{10\\sup{7 − \\rm{mod}\\paren{\\it{i}, 8}}}}, 10}\\brk − 5");
    let m = display::def_markup(
        "M(k) = R(floor(k / 16)) + 1 sq(k / 4 + 0.25) + 2 sq(k / 16) + 4 sq(k / 2 + 0.5)",
        Some("M(k) = R(floor(k / 16)) + 1 sq(k / 8 + 0.25) + 2 sq(k / 16) + 4 sq(k / 2 + 0.5)"),
    );
    assert_eq!(
        m,
        "\\it{M}\\paren{\\it{k}} = \\it{R}\\paren{\\floor{\\frac{\\it{k}}{16}}}\\brk + 1\\,\\rm{sq}\\paren{\\frac{\\it{k}}{\\hl{4}} + 0.25}\\brk + 2\\,\\rm{sq}\\paren{\\frac{\\it{k}}{16}}\\brk + 4\\,\\rm{sq}\\paren{\\frac{\\it{k}}{2} + 0.5}"
    );
    let m = display::def_markup("x = (t >= 16)(0.5 - frac(4t)) + tau", None);
    assert!(m.contains("\\paren{\\it{t} ≥ 16}") && m.contains("\\it{τ}"), "{m}");
}

#[test]
fn only_the_changed_terms_are_highlighted() {
    let old = "M(k) = R(floor(k / 16)) + 1 sq(k / 8 + 0.25) + 2 sq(k / 16)";
    let new = "M(k) = R(floor(k / 16)) + 1 sq(k / 4 + 0.25) + 2 sq(k / 16)";
    let m = display::def_markup(new, Some(old));
    assert_eq!(m.matches("\\hl{").count(), 1, "{m}");
    assert!(m.contains("\\hl{4}"), "{m}");
    let unchanged = display::def_markup(new, Some(new));
    assert!(!unchanged.contains("\\hl{"));
}

#[test]
fn the_markup_braces_always_balance() {
    let c = chain(Seed::Grammar(7), 12);
    for (i, s) in c.stages().iter().enumerate() {
        let before = if i > 0 { Some(&c.stages()[i - 1].eq) } else { None };
        for line in s.display(before) {
            let (head, markup) = line.split_once('\t').expect("kind, flag, tab");
            assert!(matches!(head, "G0" | "G1" | "V0" | "V1" | "L0" | "L1" | "T0" | "T1"), "{line}");
            let mut depth = 0i32;
            for ch in markup.chars() {
                depth += (ch == '{') as i32 - (ch == '}') as i32;
                assert!(depth >= 0, "{markup}");
            }
            assert_eq!(depth, 0, "{markup}");
        }
    }
}

// ------------------------------------------------------------------------------------------------ the C ABI

mod c_abi {
    use luminesce::compose::stream::Seed;
    use luminesce::ffi::*;
    use luminesce::stream::Stream;
    use std::ffi::{c_char, CStr, CString};
    use std::ptr::{null, null_mut};

    fn cstr(p: *const c_char) -> String {
        assert!(!p.is_null());
        unsafe { CStr::from_ptr(p) }.to_str().unwrap().to_string()
    }

    #[test]
    fn a_stream_starts_pulls_describes_windows_and_saves_through_the_c_abi() {
        let mut err = [0 as c_char; 512];
        let s = unsafe { lm_stream_start(null(), 1, 0, 1, 0, err.as_mut_ptr(), err.len()) };
        assert!(!s.is_null(), "{}", cstr(err.as_ptr()));
        let mut info = LmStreamInfo::default();
        assert_eq!(unsafe { lm_stream_info(s, &mut info) }, 1);
        assert_eq!((info.bpm, info.stage_beats, info.rate, info.trace_step), (106.0, 32.0, 48_000, 48));
        assert!(info.stages_rendered >= 1 && info.stages_composed >= 2, "{info:?}");
        let roster: Vec<String> = (0..info.roster_count).map(|i| cstr(unsafe { lm_stream_roster(s, i) })).collect();
        assert!(roster.contains(&"lead".to_string()) && roster.contains(&"fill".to_string()), "{roster:?}");
        assert!(unsafe { lm_stream_roster(s, info.roster_count) }.is_null());

        // The same samples as the Rust stream.
        let frames = 30_000u64;
        let mut a = vec![0f32; 2 * frames as usize];
        assert_eq!(unsafe { lm_stream_pull(s, a.as_mut_ptr(), frames) }, frames);
        let r = Stream::start(Seed::default_song(), 1, 0).unwrap();
        let mut b = vec![0f32; 2 * frames as usize];
        r.pull(&mut b).unwrap();
        assert!(a == b);

        // Stage 2 is composed ahead: its equation, voices and changes.
        let mut st = std::mem::MaybeUninit::<LmStageInfo>::zeroed();
        assert_eq!(unsafe { lm_stream_stage(s, 1, st.as_mut_ptr()) }, 1);
        let st = unsafe { st.assume_init() };
        assert_eq!((st.index, st.beats), (1, 32.0));
        assert_eq!(unsafe { lm_stream_stage_at(s, st.start_frame) }, 1);
        assert_eq!(unsafe { lm_stream_stage_at(s, st.start_frame - 1) }, 0);
        let eq = cstr(st.equation);
        assert!(eq.lines().all(|l| l.len() > 3 && l.as_bytes()[2] == b'\t'), "{eq}");
        assert!(!cstr(st.changed).is_empty());
        assert!(cstr(st.text).contains("beats 32"));
        // The voices playing, by roster name (v2 opens without the lead, so stage 2 need not hold it).
        let voices = cstr(st.voices);
        assert!(!voices.is_empty() && voices.split(' ').all(|v| roster.iter().any(|r| r == v)), "{voices}");
        let mut none = std::mem::MaybeUninit::<LmStageInfo>::zeroed();
        assert_eq!(unsafe { lm_stream_stage(s, 999, none.as_mut_ptr()) }, 0);

        // A window over stage 1: every roster voice, frames on the trace grid.
        let mut w = u64::MAX;
        let win = unsafe { lm_stream_window(s, 0, 0, &mut w, err.as_mut_ptr(), err.len()) };
        assert!(!win.is_null(), "{}", cstr(err.as_ptr()));
        assert_eq!(w, 0);
        let mut wi = LmInfo::default();
        assert_eq!(unsafe { lm_info(win, &mut wi) }, 1);
        assert_eq!(wi.voice_count, info.roster_count);
        assert_eq!(wi.looped, 0);
        assert!((wi.beats - 32.0).abs() < 0.01, "{}", wi.beats);
        let mut lead = std::mem::MaybeUninit::<LmVoice>::zeroed();
        let li = roster.iter().position(|n| n == "lead").unwrap() as u32;
        assert_eq!(unsafe { lm_voice(win, li, lead.as_mut_ptr()) }, 1);
        let lead = unsafe { lead.assume_init() };
        assert_eq!(lead.frame_count, (wi.frames + 47) / 48);
        unsafe { lm_free(win) };
        // A stage not rendered yet is an error, not a crash.
        let far = unsafe { lm_stream_window(s, 40, 41, &mut w, err.as_mut_ptr(), err.len()) };
        assert!(far.is_null());
        assert!(cstr(err.as_ptr()).contains("not rendered"), "{}", cstr(err.as_ptr()));

        // Save stages 1 and 2 into a song file.
        let dir = std::env::temp_dir().join(format!("lm_stream_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("saved.txt");
        let cp = CString::new(path.to_str().unwrap()).unwrap();
        assert_eq!(unsafe { lm_stream_save(s, 1, 4, cp.as_ptr(), err.as_mut_ptr(), err.len()) }, 1, "{}", cstr(err.as_ptr()));
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(luminesce::song::Song::parse(&text).is_ok());
        assert!(text.contains("beats 64"), "two stages: 1 and 2");
        let bad = CString::new(dir.join("no").join("such").join("x.txt").to_str().unwrap()).unwrap();
        assert_eq!(unsafe { lm_stream_save(s, 1, 2, bad.as_ptr(), err.as_mut_ptr(), err.len()) }, 0);
        assert!(cstr(err.as_ptr()).contains("cannot write"));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(unsafe { lm_stream_error(s, err.as_mut_ptr(), err.len()) }, 0);
        unsafe { lm_stream_free(s) };
    }

    #[test]
    fn a_grammar_seed_and_a_given_song_start_too_and_a_bad_song_is_an_error() {
        let mut err = [0 as c_char; 512];
        let g = unsafe { lm_stream_start(null(), 4, LM_STREAM_GRAMMAR, 1, 0, err.as_mut_ptr(), err.len()) };
        assert!(!g.is_null(), "{}", cstr(err.as_ptr()));
        unsafe { lm_stream_free(g) };
        let song = CString::new(std::fs::read_to_string("examples/graph_takagi.txt").unwrap()).unwrap();
        let t = unsafe { lm_stream_start(song.as_ptr(), 2, 0, 1, 0, err.as_mut_ptr(), err.len()) };
        assert!(!t.is_null(), "{}", cstr(err.as_ptr()));
        unsafe { lm_stream_free(t) };
        let bad = CString::new("bpm 100\nbeats 8\nvoice x {\n wave = sine\n pitch = nope(t)\n gate = 1\n}\n").unwrap();
        let b = unsafe { lm_stream_start(bad.as_ptr(), 1, 0, 1, 0, err.as_mut_ptr(), err.len()) };
        assert!(b.is_null());
        assert!(cstr(err.as_ptr()).contains("nope"), "{}", cstr(err.as_ptr()));
    }

    /// The world draws every stream through its windows: a move-book stream's window must hold every roster voice,
    /// the seam voices (`fill`, `rise`) included, with the kind they play as.
    #[test]
    fn a_book_streams_window_holds_every_roster_voice_through_the_c_abi() {
        let mut err = [0 as c_char; 512];
        let s = unsafe { lm_stream_start(null(), 4821, LM_STREAM_GRAMMAR, 1, 0, err.as_mut_ptr(), err.len()) };
        assert!(!s.is_null(), "{}", cstr(err.as_ptr()));
        let mut info = LmStreamInfo::default();
        assert_eq!(unsafe { lm_stream_info(s, &mut info) }, 1);
        let roster: Vec<String> = (0..info.roster_count).map(|i| cstr(unsafe { lm_stream_roster(s, i) })).collect();
        assert!(roster.contains(&"rise".to_string()), "{roster:?}");
        let mut w = u64::MAX;
        let win = unsafe { lm_stream_window(s, 0, 0, &mut w, err.as_mut_ptr(), err.len()) };
        assert!(!win.is_null(), "{}", cstr(err.as_ptr()));
        let mut wi = LmInfo::default();
        assert_eq!(unsafe { lm_info(win, &mut wi) }, 1);
        assert_eq!(wi.voice_count, info.roster_count);
        for (i, name) in roster.iter().enumerate() {
            let mut v = std::mem::MaybeUninit::<LmVoice>::zeroed();
            assert_eq!(unsafe { lm_voice(win, i as u32, v.as_mut_ptr()) }, 1);
            let v = unsafe { v.assume_init() };
            assert_eq!(&cstr(v.name), name);
            let want = if name == "rise" { LM_TONAL } else if name == "fill" { LM_DRUM } else { v.kind };
            assert_eq!(v.kind, want, "{name}");
        }
        unsafe { lm_free(win) };
        unsafe { lm_stream_free(s) };
    }

    #[test]
    fn null_streams_and_null_buffers_return_nothing() {
        let mut info = LmStreamInfo::default();
        let mut buf = [0f32; 8];
        let mut st = std::mem::MaybeUninit::<LmStageInfo>::zeroed();
        let mut err = [0 as c_char; 64];
        unsafe {
            assert_eq!(lm_stream_info(null(), &mut info), 0);
            assert_eq!(lm_stream_pull(null(), buf.as_mut_ptr(), 4), 0);
            assert_eq!(lm_stream_stage(null(), 0, st.as_mut_ptr()), 0);
            assert_eq!(lm_stream_stage_at(null(), 100), 0);
            assert!(lm_stream_roster(null(), 0).is_null());
            assert!(lm_stream_window(null(), 0, 0, null_mut(), err.as_mut_ptr(), err.len()).is_null());
            assert_eq!(lm_stream_save(null(), 0, 1, null(), null_mut(), 0), 0);
            assert_eq!(lm_stream_error(null(), null_mut(), 0), 0);
            lm_stream_free(null_mut());
        }
        let mut e = [0 as c_char; 256];
        let s = unsafe { lm_stream_start(null(), 1, 0, 1, 0, e.as_mut_ptr(), e.len()) };
        assert!(!s.is_null());
        unsafe {
            assert_eq!(lm_stream_info(s, null_mut()), 0);
            assert_eq!(lm_stream_pull(s, null_mut(), 4), 0);
            assert_eq!(lm_stream_pull(s, buf.as_mut_ptr(), 0), 0);
            assert_eq!(lm_stream_stage(s, 0, null_mut()), 0);
            assert_eq!(lm_stream_save(s, 0, 1, null(), e.as_mut_ptr(), e.len()), 0);
            lm_stream_free(s);
        }
    }
}
