//! `luminesce-measure [repo]`: measure every judged stretch against its own song (docs/what_makes_music_bad.md).
//!
//! Good stretches are the keeper's S saves in `examples\`, bad ones his B marks in `judged\bad\`: files named
//! `stream_s<seed>_<first>-<last>.txt` beside the seed file `stream_s<seed>_v<version>.seed`. Each stretch is
//! regenerated from its seed at its version, from stage 1 to its last stage, and must equal the saved file byte for
//! byte, or it is not what was heard and is left out (and said so). The stages before it are its history. With at least
//! 8 bad and 8 good stretches it scores the two registered families.

use luminesce::compose::measure::{auc, measure, Measures, NAMES};
use luminesce::compose::stream::{Composer, Seed, GENERATOR_VERSIONS};
use luminesce::song::Song;
use luminesce::stream::{read_seed_file, save_text};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Bad and good SONGS each needed before the registration is scored. Several stretches of one song are one judgment
/// (the keeper pressed B four times through seed 85749), so a song counts once, as the median of its stretches
/// (amended 2026-11-05 after the first marks, before any scoring; docs/what_makes_music_bad.md).
const ENOUGH: usize = 8;

/// One value per song: the median of its stretches, measure by measure (NaN left out).
fn per_song(rows: &[(u64, [f64; 6])]) -> Vec<[f64; 6]> {
    let mut seeds: Vec<u64> = rows.iter().map(|r| r.0).collect();
    seeds.sort();
    seeds.dedup();
    seeds
        .iter()
        .map(|s| {
            let mut out = [f64::NAN; 6];
            for (k, slot) in out.iter_mut().enumerate() {
                let mut xs: Vec<f64> = rows.iter().filter(|r| r.0 == *s).map(|r| r.1[k]).filter(|x| x.is_finite()).collect();
                xs.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
                if !xs.is_empty() {
                    *slot = xs[xs.len() / 2];
                }
            }
            out
        })
        .collect()
}

fn main() -> ExitCode {
    let repo = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| ".".into()));
    match run(&repo) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

struct Judged {
    file: String,
    seed: u64,
    /// From the seed file; None for a save older than seed files (2026-11-04), whose version is found by trying each.
    version: Option<u32>,
    first: u64,
    last: u64,
}

/// `stream_s<seed>_<first>-<last>.txt` (a numbered copy, `_<first>-<last>_2.txt`, too, and `_heard`: a stretch re-saved
/// as it was heard after the save fix of 2026-11-04); anything else is not a stretch.
fn parse_name(name: &str) -> Option<(u64, u64, u64)> {
    let rest = name.strip_prefix("stream_s")?.strip_suffix(".txt")?;
    let mut parts = rest.split('_');
    let seed = parts.next()?.parse().ok()?;
    let (a, b) = parts.next()?.split_once('-')?;
    match parts.next() {
        None => {}
        Some(n) if (n == "heard" || n.parse::<u32>().is_ok()) && parts.next().is_none() => {}
        Some(_) => return None,
    }
    Some((seed, a.parse().ok()?, b.parse().ok()?))
}

fn judged_in(dir: &Path) -> Result<Vec<Judged>, String> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else { return Ok(out) };
    let mut names: Vec<String> = entries.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    for name in &names {
        let Some((seed, first, last)) = parse_name(name) else { continue };
        let prefix = format!("stream_s{seed}_v");
        let seeds: Vec<&String> = names.iter().filter(|n| n.starts_with(&prefix) && n.ends_with(".seed")).collect();
        // A plain save that has a re-saved `_heard` twin is the pre-fix file; the twin is what was heard.
        if !name.contains("_heard") && names.contains(&name.replace(".txt", "_heard.txt")) {
            continue;
        }
        let version = match seeds.last() {
            Some(seed_name) => {
                let text = std::fs::read_to_string(dir.join(seed_name)).map_err(|e| format!("{seed_name}: {e}"))?;
                Some(read_seed_file(&text).map_err(|e| format!("{seed_name}: {e}"))?.1)
            }
            None => None,
        };
        out.push(Judged { file: name.clone(), seed, version, first, last });
    }
    Ok(out)
}

/// The stretch measured against its song, with the version that plays it, or why it cannot be. Without a seed file, a
/// species seed at each generator version is tried, and only a byte-for-byte match is accepted.
fn measure_one(dir: &Path, j: &Judged) -> Result<(u32, Measures), String> {
    let tries: Vec<(Seed, u32)> = match j.version {
        Some(v) => {
            let text = std::fs::read_to_string(dir.join(format!("stream_s{}_v{v}.seed", j.seed))).map_err(|e| e.to_string())?;
            vec![read_seed_file(&text)?]
        }
        None => GENERATOR_VERSIONS.iter().map(|&v| (Seed::Species(j.seed), v)).collect(),
    };
    let mut why = Vec::new();
    for (seed, version) in tries {
        match measure_at(dir, j, seed, version) {
            Ok(m) => return Ok((version, m)),
            Err(e) => why.push(format!("v{version}: {e}")),
        }
    }
    Err(why.join("; "))
}

fn measure_at(dir: &Path, j: &Judged, seed: Seed, version: u32) -> Result<Measures, String> {
    let mut c = Composer::with_version(seed, version)?;
    c.ensure(j.last - 1);
    if (c.stages().len() as u64) < j.last {
        return Err(format!("the song ends at stage {}", c.stages().len()));
    }
    let (a, b) = ((j.first - 1) as usize, j.last as usize);
    let heard = std::fs::read_to_string(dir.join(&j.file)).map_err(|e| e.to_string())?.replace("\r\n", "\n");
    let now = save_text(&c.stages()[a..b], c.roster(), &c.origin, c.seed())?;
    if now.replace("\r\n", "\n") != heard {
        return Err("does not play it as saved; not what was heard".into());
    }
    let whole = save_text(&c.stages()[..b], c.roster(), &c.origin, c.seed())?;
    let song = Song::parse(&whole).map_err(|e| e.to_string())?;
    Ok(measure(&song, 32.0 * a as f64))
}

fn run(repo: &Path) -> Result<(), String> {
    let groups = [("good", repo.join("examples")), ("bad", repo.join("judged").join("bad"))];
    let mut values: Vec<Vec<(u64, [f64; 6])>> = vec![Vec::new(), Vec::new()];
    println!("{:<34} {:>4} {:>8} {:>9} {:>10} {:>12} {:>10} {:>8}", "stretch", "v", "surprise", "roughness", "repetition", "macroharmony", "centricity", "conjunct");
    for (g, (label, dir)) in groups.iter().enumerate() {
        println!("-- {label} ({})", dir.display());
        for j in judged_in(dir)? {
            match measure_one(dir, &j) {
                Ok((version, m)) => {
                    let v = m.values();
                    println!(
                        "{:<34} {:>4} {:>8.2} {:>9.4} {:>10.2} {:>12.1} {:>10.3} {:>8.2}",
                        j.file, version, v[0], v[1], v[2], v[3], v[4], v[5]
                    );
                    values[g].push((j.seed, v));
                }
                Err(e) => println!("  skip {}: {e}", j.file),
            }
        }
    }
    let (good, bad) = (per_song(&values[0]), per_song(&values[1]));
    println!("\n{} good stretches from {} songs, {} bad stretches from {} songs.", values[0].len(), good.len(), values[1].len(), bad.len());
    let (good, bad) = (&good, &bad);
    if good.len() < ENOUGH || bad.len() < ENOUGH {
        println!("The registration is scored at {ENOUGH} songs on each side (docs/what_makes_music_bad.md); not yet.");
        return Ok(());
    }
    let mut aucs = [0.0; 6];
    for (k, (name, dir)) in NAMES.iter().enumerate() {
        let col = |xs: &Vec<[f64; 6]>| xs.iter().map(|v| v[k]).collect::<Vec<_>>();
        aucs[k] = auc(&col(bad), &col(good), *dir);
        println!("  {name:<14} AUC {:.3} (bad {} than good, as registered)", aucs[k], if *dir > 0.0 { "higher" } else { "lower" });
    }
    for (family, idx) in [("first family (surprise, roughness, repetition)", [0, 1, 2]), ("theory family (macroharmony, centricity, conjunct)", [3, 4, 5])] {
        let pass = idx.iter().filter(|&&i| aucs[i] >= 0.70).count();
        let dead = idx.iter().all(|&i| !(aucs[i] >= 0.60));
        let verdict = if pass >= 2 { "HOLDS" } else if dead { "FALSIFIED" } else { "neither: some separation, below the bar" };
        println!("{family}: {verdict} ({pass} of 3 at AUC >= 0.70)");
    }
    Ok(())
}
