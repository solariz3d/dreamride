//! Voices that listen (generator version 3; docs/dialogue.md, registered point 1). Each kind of listening is checked
//! against the played song, evaluated by the engine, on lens stages that play both a lead and a counter. The imitation
//! check has an independent oracle: the ORIGINAL song's lead, evaluated by the engine at `t - d`, plus the interval,
//! snapped by the engine's own `sc`. It does not use the copied, retimed definitions it is checking.

use luminesce::compose::dialogue::{apply, Kind, ANSWER_DUTY};
use luminesce::compose::stream::{Composer, Equation, Seed};
use luminesce::song::{Song, Tonal, VoiceKind};

/// Lens stages (the move book) that play both a lead and a counter, from a few seeds at version 2.
fn duets() -> Vec<Equation> {
    let mut out = Vec::new();
    for seed in 1..=12u64 {
        let mut c = Composer::with_version(Seed::Book(seed), 2).expect("version 2");
        c.ensure(23);
        for s in c.stages() {
            if s.eq.voice("lead").is_some() && s.eq.voice("counter").is_some() {
                out.push(s.eq.clone());
            }
        }
    }
    assert!(out.len() >= 10, "only {} lens stages play lead and counter; the test needs more", out.len());
    out
}

fn song(eq: &Equation) -> Song {
    let text = eq.song_text("dialogue test");
    Song::parse(&text).unwrap_or_else(|e| panic!("does not parse: {e}\n{text}"))
}

fn tonal<'a>(song: &'a Song, name: &str) -> &'a Tonal {
    match &song.voices.iter().find(|v| v.name == name).unwrap_or_else(|| panic!("no voice {name}")).kind {
        VoiceKind::Tonal(t) => t,
        _ => panic!("{name} is not tonal"),
    }
}

/// Every 16th of the stage, a little after the grid (as the engine's own reading of a gate is).
fn frames(song: &Song) -> impl Iterator<Item = f64> {
    let n = (song.beats * 4.0) as usize;
    (0..n).map(|k| k as f64 / 4.0 + 0.01)
}

fn open_share(song: &Song, voice: &str) -> f64 {
    let v = tonal(song, voice);
    let n = frames(song).count().max(1);
    frames(song).filter(|&t| v.gate.eval(&song.program, t) > 0.0).count() as f64 / n as f64
}

#[test]
fn imitation_plays_the_lead_d_beats_earlier_an_interval_below_snapped_to_the_chord() {
    let mut checked = 0;
    for eq in duets() {
        for (delay, interval) in [(1.0, -12.0), (2.0, -9.0), (4.0, -8.0)] {
            let before = song(&eq);
            let after = song(&apply(&eq, &Kind::Imitation { delay, interval }));
            let (lead_then, lead_now, counter) = (tonal(&before, "lead"), tonal(&after, "lead"), tonal(&after, "counter"));
            for t in frames(&after) {
                if lead_now.gate.eval(&after.program, t) <= 0.0 || counter.gate.eval(&after.program, t) <= 0.0 {
                    continue;
                }
                let x = lead_then.pitch.eval(&before.program, t - delay) + interval;
                let want = before.program.compile_expr(&format!("sc({x}, floor(t / 2))")).expect("sc compiles").eval(&before.program, t);
                let got = counter.pitch.eval(&after.program, t);
                if (got - want).abs() >= 1e-6 {
                    let e = apply(&eq, &Kind::Imitation { delay, interval });
                    panic!(
                        "d {delay}, interval {interval}, t {t}: counter {got}, lead then snapped {want}
lead was: {:?}
counter now: {:?}
{}",
                        eq.voice("lead").map(|v| v.text()),
                        e.voice("counter").map(|v| v.text()),
                        e.song_text("failing")
                    );
                }
                checked += 1;
            }
        }
    }
    assert!(checked >= 200, "only {checked} frames where both sound; the check is too thin");
}

#[test]
fn imitation_is_not_the_lead_now() {
    // The falsifier for the test above: the same oracle WITHOUT the delay must disagree somewhere, or the check
    // above could pass with a counter that copies the lead in unison time.
    let mut differs = 0;
    for eq in duets() {
        let before = song(&eq);
        let after = song(&apply(&eq, &Kind::Imitation { delay: 2.0, interval: -12.0 }));
        let (lead, counter) = (tonal(&before, "lead"), tonal(&after, "counter"));
        for t in frames(&after) {
            if counter.gate.eval(&after.program, t) <= 0.0 {
                continue;
            }
            let x = lead.pitch.eval(&before.program, t) - 12.0;
            let now = before.program.compile_expr(&format!("sc({x}, floor(t / 2))")).expect("sc").eval(&before.program, t);
            if (counter.pitch.eval(&after.program, t) - now).abs() > 1e-6 {
                differs += 1;
            }
        }
    }
    assert!(differs > 0, "the delayed counter equals the undelayed lead everywhere");
}

#[test]
fn an_answer_never_sounds_while_the_lead_does() {
    let (mut answered, mut fell_back) = (0, 0);
    for eq in duets() {
        let kind = Kind::Answer { delay: 2.0, interval: -12.0 };
        let after = song(&apply(&eq, &kind));
        if open_share(&song(&eq), "lead") >= ANSWER_DUTY {
            fell_back += 1;
            continue;
        }
        let (lead, counter) = (tonal(&after, "lead"), tonal(&after, "counter"));
        let mut spoke = 0;
        for t in frames(&after) {
            if counter.gate.eval(&after.program, t) > 0.0 {
                assert!(lead.gate.eval(&after.program, t) <= 0.0, "t {t}: the counter answers over the lead");
                spoke += 1;
            }
        }
        if spoke > 0 {
            answered += 1;
        }
    }
    assert!(answered >= 3, "only {answered} stages where the counter answered ({fell_back} fell back to imitation)");
}

#[test]
fn contrary_motion_moves_against_the_lead_more_often_than_with_it() {
    let mut stages = 0;
    for eq in duets() {
        let after = song(&apply(&eq, &Kind::Contrary));
        let (lead, counter) = (tonal(&after, "lead"), tonal(&after, "counter"));
        let both: Vec<(f64, f64)> = frames(&after)
            .filter(|&t| lead.gate.eval(&after.program, t) > 0.0 && counter.gate.eval(&after.program, t) > 0.0)
            .map(|t| (lead.pitch.eval(&after.program, t), counter.pitch.eval(&after.program, t)))
            .collect();
        let (mut against, mut with) = (0, 0);
        for w in both.windows(2) {
            let (dl, dc) = (w[1].0 - w[0].0, w[1].1 - w[0].1);
            if dl.abs() < 1e-9 || dc.abs() < 1e-9 {
                continue;
            }
            if dl.signum() != dc.signum() {
                against += 1;
            } else {
                with += 1;
            }
        }
        if against + with == 0 {
            continue;
        }
        assert!(against > with, "a contrary stage moved with the lead {with} times and against it {against}");
        stages += 1;
    }
    assert!(stages >= 5, "only {stages} contrary stages had moving steps");
}

#[test]
fn version_3_lens_stages_listen_and_version_2_stages_do_not() {
    let mut changed = 0;
    for seed in 1..=12u64 {
        let mut v2 = Composer::with_version(Seed::Book(seed), 2).expect("v2");
        let mut v3 = Composer::with_version(Seed::Book(seed), 3).expect("v3");
        v2.ensure(23);
        v3.ensure(23);
        for (a, b) in v2.stages().iter().zip(v3.stages()) {
            assert!(a.eq == b.eq, "version 3 composed seed {seed} differently; only the played text may change");
            let duet = a.eq.voice("lead").is_some() && a.eq.voice("counter").is_some();
            if !duet {
                assert_eq!(a.text, b.text, "seed {seed} stage {}: no duet, yet version 3 plays it differently", a.index + 1);
            } else if a.text != b.text {
                changed += 1;
            }
        }
    }
    assert!(changed >= 5, "only {changed} duet stages changed at version 3");
}
