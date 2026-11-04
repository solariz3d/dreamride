//! The WAV writer, checked by parsing its own output back with an independent reader.

use luminesce::render::Mix;
use luminesce::wav::{encode, write, SampleFormat};

fn u16_at(b: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([b[i], b[i + 1]])
}
fn u32_at(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]])
}

struct Parsed {
    format_tag: u16,
    channels: u16,
    rate: u32,
    byte_rate: u32,
    block_align: u16,
    bits: u16,
    fact_frames: Option<u32>,
    data: Vec<u8>,
}

/// A small RIFF reader: checks the container sizes and walks the chunks.
fn parse(b: &[u8]) -> Parsed {
    assert_eq!(&b[0..4], b"RIFF");
    assert_eq!(u32_at(b, 4) as usize, b.len() - 8, "RIFF size must be the file length minus 8");
    assert_eq!(&b[8..12], b"WAVE");
    let mut i = 12;
    let mut fmt = None;
    let mut fact_frames = None;
    let mut data = None;
    while i < b.len() {
        let id = &b[i..i + 4];
        let size = u32_at(b, i + 4) as usize;
        let body = &b[i + 8..i + 8 + size];
        match id {
            b"fmt " => fmt = Some(body.to_vec()),
            b"fact" => fact_frames = Some(u32_at(body, 0)),
            b"data" => data = Some(body.to_vec()),
            other => panic!("unexpected chunk {:?}", String::from_utf8_lossy(other)),
        }
        i += 8 + size + (size & 1);
    }
    assert_eq!(i, b.len(), "chunks must exactly fill the file");
    let f = fmt.expect("fmt chunk");
    Parsed {
        format_tag: u16_at(&f, 0),
        channels: u16_at(&f, 2),
        rate: u32_at(&f, 4),
        byte_rate: u32_at(&f, 8),
        block_align: u16_at(&f, 12),
        bits: u16_at(&f, 14),
        fact_frames,
        data: data.expect("data chunk"),
    }
}

fn test_mix(frames: usize) -> Mix {
    let left: Vec<f64> = (0..frames).map(|i| ((i as f64) * 0.1).sin() * 0.5).collect();
    let right: Vec<f64> = left.iter().map(|x| -x).collect();
    Mix { rate: 48_000, left, right, raw_peak: 0.5, gain: 1.0, peak: 0.5 }
}

#[test]
fn pcm16_header_parses_back_with_exact_length() {
    let m = test_mix(1001);
    let b = encode(&m, SampleFormat::Pcm16).unwrap();
    let p = parse(&b);
    assert_eq!(p.format_tag, 1, "PCM");
    assert_eq!(p.channels, 2);
    assert_eq!(p.rate, 48_000);
    assert_eq!(p.bits, 16);
    assert_eq!(p.block_align, 4);
    assert_eq!(p.byte_rate, 48_000 * 4);
    assert_eq!(p.fact_frames, None);
    assert_eq!(p.data.len(), 1001 * 4, "exactly frames * channels * 2 bytes");
    assert_eq!(b.len(), 44 + 1001 * 4, "the canonical 44-byte header");
}

#[test]
fn float32_header_parses_back_with_exact_length() {
    let m = test_mix(777);
    let b = encode(&m, SampleFormat::Float32).unwrap();
    let p = parse(&b);
    assert_eq!(p.format_tag, 3, "IEEE float");
    assert_eq!(p.channels, 2);
    assert_eq!(p.rate, 48_000);
    assert_eq!(p.bits, 32);
    assert_eq!(p.block_align, 8);
    assert_eq!(p.byte_rate, 48_000 * 8);
    assert_eq!(p.fact_frames, Some(777));
    assert_eq!(p.data.len(), 777 * 8);
}

#[test]
fn pcm16_samples_are_interleaved_and_scaled() {
    let m = Mix { rate: 44_100, left: vec![0.5, 1.0, 0.0], right: vec![-0.5, -1.0, 0.25], raw_peak: 1.0, gain: 1.0, peak: 1.0 };
    let p = parse(&encode(&m, SampleFormat::Pcm16).unwrap());
    assert_eq!(p.rate, 44_100);
    let s: Vec<i16> = p.data.chunks(2).map(|c| i16::from_le_bytes([c[0], c[1]])).collect();
    assert_eq!(s, vec![16384, -16384, 32767, -32767, 0, 8192]);
}

#[test]
fn float32_samples_round_trip() {
    let m = Mix { rate: 48_000, left: vec![0.5, -0.125], right: vec![0.25, 0.0], raw_peak: 0.5, gain: 1.0, peak: 0.5 };
    let p = parse(&encode(&m, SampleFormat::Float32).unwrap());
    let s: Vec<f32> = p.data.chunks(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect();
    assert_eq!(s, vec![0.5, 0.25, -0.125, 0.0]);
}

#[test]
fn an_empty_mix_is_a_valid_empty_file() {
    let m = test_mix(0);
    let p = parse(&encode(&m, SampleFormat::Pcm16).unwrap());
    assert!(p.data.is_empty());
}

#[test]
fn out_of_range_or_non_finite_samples_are_refused() {
    let mut m = test_mix(4);
    m.left[2] = 1.5;
    assert!(encode(&m, SampleFormat::Pcm16).unwrap_err().message.contains("out of range"));
    m.left[2] = f64::NAN;
    assert!(encode(&m, SampleFormat::Float32).unwrap_err().message.contains("not finite"));
}

#[test]
fn mismatched_channels_are_refused() {
    let mut m = test_mix(4);
    m.right.pop();
    assert!(encode(&m, SampleFormat::Pcm16).is_err());
}

#[test]
fn write_puts_the_encoded_bytes_on_disk_and_replaces_an_old_file() {
    let dir = std::env::temp_dir().join(format!("luminesce-wav-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("out.wav");
    std::fs::write(&path, b"old contents").unwrap();
    let m = test_mix(100);
    write(&path, &m, SampleFormat::Pcm16).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), encode(&m, SampleFormat::Pcm16).unwrap());
    assert!(!dir.join("out.wav.tmp").exists(), "the temporary file is renamed away");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn write_to_a_missing_directory_fails_with_the_path() {
    let path = std::env::temp_dir().join("luminesce-no-such-dir-xyz").join("out.wav");
    let e = write(&path, &test_mix(1), SampleFormat::Pcm16).unwrap_err();
    assert!(e.message.contains("luminesce-no-such-dir-xyz"), "{e}");
}
