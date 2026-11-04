//! `luminesce-render <song.txt> <out.wav> [--float]`: render a song to a WAV file.

use luminesce::render::{dbfs, render};
use luminesce::song::Song;
use luminesce::wav::{write, SampleFormat};
use std::path::Path;
use std::process::ExitCode;

const USAGE: &str = "usage: luminesce-render <song.txt> <out.wav> [--float]\n\
                     \x20 --float  write 32-bit float samples instead of 16-bit PCM";

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    let mut format = SampleFormat::Pcm16;
    let mut paths = Vec::new();
    for a in &args {
        match a.as_str() {
            "--float" => format = SampleFormat::Float32,
            "-h" | "--help" => {
                println!("{USAGE}");
                return Ok(());
            }
            s if s.starts_with("--") => return Err(format!("unknown option '{s}'\n{USAGE}")),
            s => paths.push(s),
        }
    }
    let [song_path, out_path] = paths[..] else {
        return Err(format!("expected a song file and an output file\n{USAGE}"));
    };

    let src = std::fs::read_to_string(song_path).map_err(|e| format!("cannot read {song_path}: {e}"))?;
    let song = Song::parse(&src).map_err(|e| format!("{song_path}: {e}"))?;
    let mix = render(&song).map_err(|e| format!("{song_path}: {e}"))?;
    write(Path::new(out_path), &mix, format).map_err(|e| e.to_string())?;

    println!("wrote {out_path}");
    println!(
        "duration: {:.3} s ({} beats at {} bpm)",
        song.duration_secs(),
        song.beats,
        song.bpm
    );
    println!(
        "samples: {} per channel at {} Hz, stereo, {}",
        mix.frames(),
        mix.rate,
        format.describe()
    );
    if mix.peak == 0.0 {
        println!("peak: silent");
    } else if mix.gain < 1.0 {
        println!(
            "peak: {:.2} dBFS (the mix peaked at {:.2} dBFS and was scaled by {:.2} dB to avoid clipping)",
            dbfs(mix.peak),
            dbfs(mix.raw_peak),
            dbfs(mix.gain)
        );
    } else {
        println!("peak: {:.2} dBFS", dbfs(mix.peak));
    }
    Ok(())
}
