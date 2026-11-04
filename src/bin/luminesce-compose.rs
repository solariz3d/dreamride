//! `luminesce-compose --seed S --count N --keep K --out DIR`: write N graph songs from a seed, score them, keep the
//! best K in `DIR/<UTC timestamp>/` with a `scores.md`. See docs/composer.md.

use luminesce::compose::search::{run, Options};
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "usage: luminesce-compose [--seed S] [--count N] [--keep K] [--out DIR] [--finalists F]\n\
                     \x20                        [--render-all] [--wav] [--threads T] [--compare song.txt ...]\n\
                     \x20 --seed S        the run's seed (default 1); the same seed writes the same songs\n\
                     \x20 --count N       candidates to write and score (default 200)\n\
                     \x20 --keep K        songs to keep (default 5)\n\
                     \x20 --out DIR       base folder (default generated); the run goes in DIR/<UTC timestamp>/\n\
                     \x20 --finalists F   how many of the best function scores to render for the sound score (default 20)\n\
                     \x20 --render-all    render every candidate (to check the critic against random samples)\n\
                     \x20 --wav           also write the kept songs as WAV\n\
                     \x20 --compare ...   score these song files beside the candidates (the rest of the arguments)";

fn main() -> ExitCode {
    match parse(std::env::args().skip(1).collect()) {
        Ok(None) => ExitCode::SUCCESS,
        Ok(Some(o)) => match run(&o) {
            Ok(r) => {
                println!("wrote {}", r.dir.display());
                println!("{} candidates in {:.1} s; kept:", r.candidates.len(), r.seconds);
                for &i in &r.kept {
                    let c = &r.candidates[i];
                    println!("  {}  total {:.3}  function {:.3}", c.id, c.report.total(), c.report.function_score());
                }
                for s in &r.compared {
                    println!("  compared {}  total {:.3}  function {:.3}", s.id, s.report.total(), s.report.function_score());
                }
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            }
        },
        Err(e) => {
            eprintln!("error: {e}\n{USAGE}");
            ExitCode::FAILURE
        }
    }
}

fn parse(args: Vec<String>) -> Result<Option<Options>, String> {
    let mut o = Options::default();
    let mut it = args.into_iter();
    let num = |it: &mut std::vec::IntoIter<String>, flag: &str| -> Result<u64, String> {
        let v = it.next().ok_or_else(|| format!("{flag} needs a value"))?;
        v.parse::<u64>().map_err(|_| format!("{flag}: '{v}' is not a whole number"))
    };
    while let Some(a) = it.next() {
        match a.as_str() {
            "--seed" => o.seed = num(&mut it, "--seed")?,
            "--count" => o.count = num(&mut it, "--count")? as usize,
            "--keep" => o.keep = num(&mut it, "--keep")? as usize,
            "--finalists" => o.finalists = num(&mut it, "--finalists")? as usize,
            "--threads" => o.threads = (num(&mut it, "--threads")? as usize).max(1),
            "--out" => o.out = PathBuf::from(it.next().ok_or("--out needs a folder")?),
            "--render-all" => o.render_all = true,
            "--wav" => o.wav = true,
            "--compare" => o.compare.extend(it.by_ref().map(PathBuf::from)),
            "-h" | "--help" => {
                println!("{USAGE}");
                return Ok(None);
            }
            s => return Err(format!("unknown argument '{s}'")),
        }
    }
    if o.count == 0 || o.keep == 0 {
        return Err("--count and --keep must be at least 1".into());
    }
    Ok(Some(o))
}
