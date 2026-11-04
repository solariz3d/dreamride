//! The example song end to end: determinism, finiteness, length, and the command-line tool.

use luminesce::render::{render, CEILING};
use luminesce::song::Song;
use luminesce::wav::{encode, SampleFormat};
use std::process::Command;

const EXAMPLE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/citypop_sketch.txt");

fn example() -> Song {
    Song::parse(&std::fs::read_to_string(EXAMPLE).unwrap()).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn two_renders_are_byte_identical() {
    let a = encode(&render(&example()).unwrap(), SampleFormat::Pcm16).unwrap();
    let b = encode(&render(&example()).unwrap(), SampleFormat::Pcm16).unwrap();
    assert!(a == b, "renders differ");
    let a = encode(&render(&example()).unwrap(), SampleFormat::Float32).unwrap();
    let b = encode(&render(&example()).unwrap(), SampleFormat::Float32).unwrap();
    assert!(a == b, "float renders differ");
}

#[test]
fn the_example_is_finite_in_range_and_exactly_as_long_as_asked() {
    let s = example();
    let m = render(&s).unwrap();
    assert_eq!(m.frames(), 921_600, "32 beats at 100 bpm at 48 kHz is 19.2 s");
    assert!(m.left.iter().chain(&m.right).all(|x| x.is_finite()));
    assert!(m.peak <= CEILING + 1e-12, "peak {} above the ceiling", m.peak);
    assert!(m.peak > 0.1, "the example should not be near-silent ({})", m.peak);
}

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("luminesce-cli-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn the_cli_renders_and_reports_duration_samples_and_peak() {
    let dir = temp_dir("ok");
    let out = dir.join("x.wav");
    let o = Command::new(env!("CARGO_BIN_EXE_luminesce-render")).arg(EXAMPLE).arg(&out).output().unwrap();
    let text = String::from_utf8_lossy(&o.stdout);
    assert!(o.status.success(), "stderr: {}", String::from_utf8_lossy(&o.stderr));
    assert!(text.contains("duration: 19.200 s"), "{text}");
    assert!(text.contains("samples: 921600 per channel at 48000 Hz"), "{text}");
    assert!(text.contains("peak: "), "{text}");
    assert_eq!(std::fs::metadata(&out).unwrap().len(), 44 + 921_600 * 4);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_cli_fails_loudly_on_bad_input() {
    let dir = temp_dir("bad");
    let song = dir.join("bad.txt");
    std::fs::write(&song, "bpm 100\nbeats 4\nvoice a {\n wave = sine\n pitch = nope\n gate = 1\n}\n").unwrap();
    let out = dir.join("x.wav");
    let o = Command::new(env!("CARGO_BIN_EXE_luminesce-render")).arg(&song).arg(&out).output().unwrap();
    assert!(!o.status.success());
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(err.contains("line 5, column 10: unknown name 'nope'"), "{err}");
    assert!(!out.exists(), "no output file on failure");

    let o = Command::new(env!("CARGO_BIN_EXE_luminesce-render")).arg(&song).output().unwrap();
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("usage"));

    let o = Command::new(env!("CARGO_BIN_EXE_luminesce-render")).arg(dir.join("missing.txt")).arg(&out).output().unwrap();
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("cannot read"));
    std::fs::remove_dir_all(&dir).unwrap();
}
