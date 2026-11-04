//! City pop v4 claims to be v3's music revoiced with graph instruments. Checked here: every note,
//! chord, bass step and drum hit is v3's, step for step; every tonal voice is a sum of sines under
//! the Desmos gain cap and every drum is a dash; and the file renders finite and exactly as long.

use luminesce::render::render;
use luminesce::song::{DrumKind, Song, VoiceKind, Wave};

const V3: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/citypop_v3.txt");
const V4: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/citypop_v4.txt");

fn load(path: &str) -> Song {
    Song::parse(&std::fs::read_to_string(path).unwrap()).unwrap_or_else(|e| panic!("{e}"))
}

fn f(s: &Song, name: &str, args: &[f64]) -> f64 {
    s.program.call(name, args).unwrap_or_else(|e| panic!("{name}{args:?}: {e}"))
}

#[test]
fn v4_plays_v3s_music_step_for_step() {
    let (a, b) = (load(V3), load(V4));
    for st in 0..256 {
        let s = st as f64;
        for name in ["deg", "qual", "lr", "lp", "la", "bassd", "bassat", "kickd", "snared", "hatv", "keyd"] {
            assert_eq!(f(&a, name, &[s]), f(&b, name, &[s]), "{name} differs at step {st}");
        }
        if f(&a, "lr", &[s]) != 0.0 {
            let on = f(&a, "lon", &[s]);
            assert_eq!(f(&a, "lnote", &[on]), f(&b, "lnote", &[on]), "the lead's note differs at step {st}");
        }
        for k in 0..4 {
            let k = k as f64;
            assert_eq!(f(&a, "voicing", &[k, s, -6.0]), f(&b, "voicing", &[k, s, -6.0]), "voicing {k} differs at step {st}");
        }
    }
    // And the lead's phrase time, gates and dynamics: the same expression, sampled through the loop.
    for i in 0..640 {
        let x = i as f64 * 0.1;
        for name in ["lx", "lgate", "padgate", "keygate"] {
            assert_eq!(f(&a, name, &[x]), f(&b, name, &[x]), "{name} differs at beat {x}");
        }
    }
}

#[test]
fn v4_voices_are_graph_instruments() {
    let s = load(V4);
    for v in &s.voices {
        match &v.kind {
            VoiceKind::Tonal(t) => {
                assert_eq!(t.wave, Wave::Additive, "{} is not additive", v.name);
                assert_eq!(t.rolloff_hz, 660.0, "{} is not under the Desmos cap", v.name);
                assert!(t.cutoff.is_none() && t.detune_cents == 0.0 && t.drive == 0.0, "{} uses a synth stage", v.name);
            }
            VoiceKind::Drum(d) => {
                assert_eq!(d.kind, DrumKind::Dash, "{} is not a dash", v.name);
                let dash = d.dash.as_ref().unwrap();
                assert!(dash.from_hz > dash.to_hz, "{}'s pitch does not fall", v.name);
            }
        }
    }
    let kick = s.voices.iter().find(|v| v.name == "kick").unwrap();
    let VoiceKind::Drum(d) = &kick.kind else { panic!("kick is not a drum") };
    let k = d.dash.as_ref().unwrap();
    // The measured dash: several hundred Hz down to the bass range in about 23 ms.
    assert!(k.from_hz >= 400.0 && k.to_hz <= 80.0 && (k.dash_ms - 23.0).abs() < 1.0);
}

#[test]
fn v4_renders_finite_and_exactly_as_long_as_asked() {
    let s = load(V4);
    let m = render(&s).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(m.frames(), s.sample_count());
    assert_eq!(m.frames(), (64.0f64 * 60.0 / 94.0 * 48_000.0).round() as usize);
    assert!(m.left.iter().chain(&m.right).all(|x| x.is_finite()));
    assert!(m.peak <= luminesce::render::CEILING + 1e-12);
}
