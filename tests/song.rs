//! The song file format: settings, blocks, defaults, the exact length, and every rejection.

use luminesce::song::{DrumKind, Song, VoiceKind, Wave};
use luminesce::Error;

const MINIMAL: &str = "bpm 120\nbeats 4\nvoice a {\n  wave = sine\n  pitch = 0\n  gate = 1\n}\n";

fn err(src: &str) -> Error {
    match Song::parse(src) {
        Ok(_) => panic!("should have been rejected:\n{src}"),
        Err(e) => e,
    }
}

#[test]
fn a_minimal_song_parses_with_defaults() {
    let s = Song::parse(MINIMAL).unwrap();
    assert_eq!(s.bpm, 120.0);
    assert_eq!(s.beats, 4.0);
    assert_eq!(s.rate, 48_000);
    assert_eq!(s.reference_hz, 440.0);
    assert_eq!(s.voices.len(), 1);
    let v = &s.voices[0];
    assert_eq!(v.name, "a");
    assert_eq!(v.gain, 1.0);
    assert_eq!(v.pan, 0.0);
    match &v.kind {
        VoiceKind::Tonal(t) => {
            assert_eq!(t.wave, Wave::Sine);
            assert_eq!(t.attack_ms, 5.0);
            assert_eq!(t.release_ms, 30.0);
            assert!(t.amp.is_none());
        }
        _ => panic!("expected a tonal voice"),
    }
}

#[test]
fn a_full_song_parses_every_field() {
    let src = "\
# comment line
bpm 96   # trailing comment
beats 8
rate 44100
reference 432
h(d) = floor((12d + 5) / 7)
voice lead {
  wave = square
  pitch = h(floor(t))
  gate = 0.8 - frac(t)
  amp = 0.5
  gain = 0.3
  pan = -0.5
  attack = 2
  release = 50
}
drum k {
  kind = kick
  trigger = 0.25 - frac(t)
  amp = 1
  gain = 0.9
  pan = 0.1
}
";
    let s = Song::parse(src).unwrap();
    assert_eq!((s.bpm, s.beats, s.rate, s.reference_hz), (96.0, 8.0, 44_100, 432.0));
    assert_eq!(s.voices.len(), 2);
    match &s.voices[0].kind {
        VoiceKind::Tonal(t) => {
            assert_eq!(t.wave, Wave::Square);
            assert_eq!((t.attack_ms, t.release_ms), (2.0, 50.0));
            assert_eq!(t.pitch.eval(&s.program, 2.5), 4.0);
        }
        _ => panic!(),
    }
    assert_eq!((s.voices[0].gain, s.voices[0].pan), (0.3, -0.5));
    match &s.voices[1].kind {
        VoiceKind::Drum(d) => assert_eq!(d.kind, DrumKind::Kick),
        _ => panic!(),
    }
}

#[test]
fn the_sample_count_is_rounded_exactly() {
    let s = Song::parse(&MINIMAL.replace("beats 4", "beats 3")).unwrap();
    assert_eq!(s.sample_count(), 72_000, "3 beats at 120 bpm is 1.5 s");
    // 1 beat at 7 bpm at 48 kHz = 411428.571... samples: rounds to nearest, not down.
    let s = Song::parse(&MINIMAL.replace("bpm 120", "bpm 7").replace("beats 4", "beats 1")).unwrap();
    assert_eq!(s.sample_count(), 411_429);
    // 1 beat at 9 bpm at 44.1 kHz = 294000 exactly.
    let s = Song::parse(&format!("rate 44100\n{}", MINIMAL.replace("bpm 120", "bpm 9").replace("beats 4", "beats 1"))).unwrap();
    assert_eq!(s.sample_count(), 294_000);
}

#[test]
fn an_empty_song_is_rejected() {
    assert!(err("").message.contains("missing `bpm`"));
    assert!(err("# only a comment\n\n").message.contains("missing `bpm`"));
}

#[test]
fn missing_settings_are_rejected() {
    assert!(err(&MINIMAL.replace("bpm 120\n", "")).message.contains("missing `bpm`"));
    assert!(err(&MINIMAL.replace("beats 4\n", "")).message.contains("missing `beats`"));
}

#[test]
fn a_song_without_voices_is_rejected() {
    assert!(err("bpm 120\nbeats 4\n").message.contains("no voices"));
}

#[test]
fn missing_voice_fields_are_rejected_with_the_voice_line() {
    let e = err(&MINIMAL.replace("  pitch = 0\n", ""));
    assert_eq!(e.line, Some(3));
    assert!(e.message.contains("voice 'a'") && e.message.contains("missing `pitch`"), "{e}");
    assert!(err(&MINIMAL.replace("  gate = 1\n", "")).message.contains("missing `gate`"));
    assert!(err(&MINIMAL.replace("  wave = sine\n", "")).message.contains("missing `wave`"));
    let e = err("bpm 120\nbeats 4\ndrum k {\n  kind = kick\n}\n");
    assert!(e.message.contains("missing `trigger`"), "{e}");
    let e = err("bpm 120\nbeats 4\ndrum k {\n  trigger = 1\n}\n");
    assert!(e.message.contains("missing `kind`"), "{e}");
}

#[test]
fn empty_field_values_are_rejected() {
    let e = err(&MINIMAL.replace("pitch = 0", "pitch ="));
    assert!(e.message.contains("`pitch` is empty"), "{e}");
    let e = err(&MINIMAL.replace("bpm 120", "bpm"));
    assert!(e.message.contains("`bpm` needs a value"), "{e}");
}

#[test]
fn bad_setting_values_are_rejected() {
    assert!(err(&MINIMAL.replace("bpm 120", "bpm 0")).message.contains("bpm"));
    assert!(err(&MINIMAL.replace("bpm 120", "bpm -5")).message.contains("bpm"));
    assert!(err(&MINIMAL.replace("bpm 120", "bpm NaN")).message.contains("not a number"));
    assert!(err(&MINIMAL.replace("bpm 120", "bpm inf")).message.contains("not a number"));
    assert!(err(&MINIMAL.replace("bpm 120", "bpm fast")).message.contains("not a number"));
    assert!(err(&MINIMAL.replace("beats 4", "beats 0")).message.contains("beats"));
    assert!(err(&format!("rate 100\n{MINIMAL}")).message.contains("rate"));
    assert!(err(&format!("rate 44100.5\n{MINIMAL}")).message.contains("whole number"));
    assert!(err(&format!("reference 0\n{MINIMAL}")).message.contains("reference"));
    assert!(err(&MINIMAL.replace("beats 4", "beats 100000")).message.contains("limit"));
}

#[test]
fn a_setting_written_with_equals_is_rejected_helpfully() {
    let e = err(&MINIMAL.replace("bpm 120", "bpm = 120"));
    assert!(e.message.contains("write `bpm 120`") || e.message.contains("without '='"), "{e}");
}

#[test]
fn duplicate_settings_voices_and_fields_are_rejected() {
    assert!(err(&MINIMAL.replace("bpm 120", "bpm 120\nbpm 100")).message.contains("twice"));
    assert!(err(&format!("{MINIMAL}{}", &MINIMAL[16..])).message.contains("twice"));
    assert!(err(&MINIMAL.replace("gate = 1", "gate = 1\n  gate = 0")).message.contains("twice"));
}

#[test]
fn unknown_keys_and_words_are_rejected() {
    assert!(err(&MINIMAL.replace("gate = 1", "gate = 1\n  colour = 3")).message.contains("unknown field `colour`"));
    assert!(err(&MINIMAL.replace("wave = sine", "wave = organ")).message.contains("organ"));
    assert!(err("bpm 120\nbeats 4\ndrum k {\n  kind = cowbell\n  trigger = 1\n}\n").message.contains("cowbell"));
    assert!(err(&MINIMAL.replace("gate = 1", "gate = 1\n  trigger = 1")).message.contains("unknown field `trigger`"));
}

#[test]
fn bad_numeric_fields_are_rejected() {
    assert!(err(&MINIMAL.replace("gate = 1", "gate = 1\n  gain = loud")).message.contains("`gain` must be a number"));
    assert!(err(&MINIMAL.replace("gate = 1", "gate = 1\n  pan = 2")).message.contains("pan"));
    assert!(err(&MINIMAL.replace("gate = 1", "gate = 1\n  gain = -1")).message.contains("gain"));
    assert!(err(&MINIMAL.replace("gate = 1", "gate = 1\n  attack = 0")).message.contains("attack"));
}

#[test]
fn an_unclosed_block_is_rejected() {
    let e = err("bpm 120\nbeats 4\nvoice a {\n  wave = sine\n");
    assert_eq!(e.line, Some(3));
    assert!(e.message.contains("never closed"), "{e}");
    let e = err("bpm 120\nbeats 4\nvoice a {\nvoice b {\n}\n");
    assert!(e.message.contains("not closed"), "{e}");
}

#[test]
fn a_stray_line_is_rejected() {
    assert!(err(&format!("{MINIMAL}}}\n")).message.contains("without an open block"));
    assert!(err(&format!("{MINIMAL}hello there\n")).message.contains("unrecognised line"));
    assert!(err(&format!("{MINIMAL}voice {{\n}}\n")).message.contains("expected `voice <name> {`"));
}

#[test]
fn an_expression_error_in_a_voice_points_at_its_line_and_column() {
    let e = err(&MINIMAL.replace("pitch = 0", "pitch = 1 + nope"));
    assert_eq!((e.line, e.col), (Some(5), Some(15)), "{e}");
    assert!(e.message.contains("unknown name 'nope'"), "{e}");
}

#[test]
fn a_definition_error_points_at_its_line() {
    let e = err(&format!("bpm 120\nbeats 4\nf(x) = x +\n{}", &MINIMAL[16..]));
    assert_eq!(e.line, Some(3), "{e}");
}

#[test]
fn the_filter_and_envelope_fields_parse_with_defaults() {
    let s = Song::parse(MINIMAL).unwrap();
    let VoiceKind::Tonal(t) = &s.voices[0].kind else { panic!() };
    assert_eq!((t.decay_ms, t.sustain), (100.0, 1.0));
    assert!(t.cutoff.is_none());
    assert_eq!((t.resonance, t.filter_env, t.filter_decay_ms, t.drive), (0.0, 0.0, 200.0, 0.0));

    let src = MINIMAL.replace(
        "  gate = 1\n",
        "  gate = 1\n  decay = 80\n  sustain = 0.3\n  cutoff = 800 + 400 sin(t)\n  resonance = 0.5\n  filter_env = 2\n  filter_decay = 120\n  drive = 1.5\n",
    );
    let s = Song::parse(&src).unwrap();
    let VoiceKind::Tonal(t) = &s.voices[0].kind else { panic!() };
    assert_eq!((t.decay_ms, t.sustain, t.resonance, t.filter_env, t.filter_decay_ms, t.drive), (80.0, 0.3, 0.5, 2.0, 120.0, 1.5));
    assert_eq!(t.cutoff.as_ref().unwrap().eval(&s.program, 0.0), 800.0);
}

#[test]
fn out_of_range_filter_and_envelope_fields_are_rejected() {
    for (field, want) in [
        ("sustain = 1.5", "`sustain` must be between 0 and 1"),
        ("sustain = -0.1", "`sustain` must be between 0 and 1"),
        ("resonance = 1", "`resonance` must be between 0 and 0.95"),
        ("filter_env = 9", "`filter_env` must be between -8 and 8"),
        ("decay = 0", "`decay` must be between 0.1 and 10000"),
        ("drive = 11", "`drive` must be between 0 and 10"),
        ("cutoff = nope", "unknown name 'nope'"),
    ] {
        let e = err(&MINIMAL.replace("  gate = 1\n", &format!("  gate = 1\n  {field}\n")));
        assert!(e.message.contains(want), "{field}: {e}");
    }
}

#[test]
fn drums_do_not_take_tonal_fields() {
    let e = err("bpm 100\nbeats 4\ndrum k {\n kind = kick\n trigger = 1\n cutoff = 100\n}\n");
    assert!(e.message.contains("unknown field `cutoff`"), "{e}");
}
