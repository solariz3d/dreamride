//! `luminesce-stream --seed N --out FILE.wav [--from SECONDS] [--seconds S] [--species NAME | --book | --explore |
//! --grammar]`: play a stream headless (no window, no audio device) and write a stretch of it to a WAV file, exactly the
//! samples the world would play. The default is a species stream (the seed picks one of the default species, which
//! carry written harmony); `--species NAME` names one, `--book` plays the lens species alone, `--explore` a species of
//! strange tunings (the Explore option), `--grammar` stream v2's grammar seed.

use luminesce::compose::stream::Seed;
use luminesce::render::Mix;
use luminesce::stream::Stream;
use luminesce::wav::{write, SampleFormat};
use std::path::Path;
use std::process::ExitCode;

const USAGE: &str = "usage: luminesce-stream --seed N --out FILE.wav [--from SECONDS] [--seconds S] [--grammar]\n\
                     \x20 --from S      skip the first S seconds (default 0)\n\
                     \x20 --seconds S   how much to write (default 150), or until the stream ends\n\
                     \x20 --species N   that species only (lens, dialogue, chaos, automaton, bounce, oneline; or an Explore one)\n\
                     \x20 --book        the lens species only\n\
                     \x20 --explore     a species of strange tunings (loops, tritave, maqam, ratios)\n\
                     \x20 --grammar     stream v2's grammar seed instead of the move book";

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}\n{USAGE}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    let (mut seed, mut out, mut from, mut secs, mut grammar, mut lens) = (1u64, None, 0.0f64, 150.0f64, false, false);
    let mut explore = false;
    let mut species: Option<String> = None;
    let mut it = args.into_iter();
    let num = |v: Option<String>, f: &str| -> Result<f64, String> {
        let v = v.ok_or_else(|| format!("{f} needs a value"))?;
        v.parse::<f64>().ok().filter(|x| x.is_finite() && *x >= 0.0).ok_or_else(|| format!("{f}: '{v}' is not a number of seconds"))
    };
    while let Some(a) = it.next() {
        match a.as_str() {
            "--seed" => seed = it.next().and_then(|v| v.parse().ok()).ok_or("--seed needs a whole number")?,
            "--out" => out = Some(it.next().ok_or("--out needs a file")?),
            "--from" => from = num(it.next(), "--from")?,
            "--seconds" => secs = num(it.next(), "--seconds")?,
            "--grammar" => grammar = true,
            "--book" => lens = true,
            "--explore" => explore = true,
            "--species" => species = Some(it.next().ok_or("--species needs a name")?),
            s => return Err(format!("unknown argument '{s}'")),
        }
    }
    let out = out.ok_or("--out is required")?;
    let start = match (grammar, lens, species) {
        (true, _, _) => Seed::Grammar(seed),
        (_, true, _) => Seed::Book(seed),
        (_, _, Some(species)) => Seed::Named { species, seed },
        _ if explore => Seed::Explore(seed),
        _ => Seed::Species(seed),
    };
    let stream = Stream::start(start, 1, 0)?;
    let rate = stream.rate as usize;
    let mut skip = (from * rate as f64) as usize;
    let want = (secs * rate as f64) as usize;
    let mut buf = vec![0f32; 2 * 4800];
    while skip > 0 {
        let n = skip.min(4800);
        let got = stream.pull(&mut buf[..2 * n])?;
        skip -= got;
        if got < n {
            return Err("the stream ended before --from".into());
        }
    }
    let (mut left, mut right) = (Vec::with_capacity(want), Vec::with_capacity(want));
    while left.len() < want {
        let n = (want - left.len()).min(4800);
        let got = stream.pull(&mut buf[..2 * n])?;
        for j in 0..got {
            left.push(buf[2 * j] as f64);
            right.push(buf[2 * j + 1] as f64);
        }
        if got < n {
            break; // the stream ended
        }
    }
    let peak = left.iter().chain(&right).fold(0.0f64, |a, x| a.max(x.abs()));
    let mix = Mix { rate: stream.rate, left, right, raw_peak: peak, gain: 1.0, peak };
    write(Path::new(&out), &mix, SampleFormat::Pcm16).map_err(|e| e.to_string())?;
    println!("wrote {out}: {:.1} s of {} (seed {seed}), {:.1} bpm", mix.frames() as f64 / rate as f64, stream.origin, stream.bpm);
    Ok(())
}
