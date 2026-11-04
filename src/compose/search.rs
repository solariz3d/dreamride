//! The search: write N candidates from a seed, score them all from their functions, render the finalists, keep the
//! top K, and write them with a `scores.md` that explains each pick. Deterministic per seed: the songs and their
//! scores do not depend on the thread count or the time (only the folder name and the timing line do).

use super::critic::{self, Report, WEIGHTS};
use super::grammar::{self, Draft};
use super::sound;
use crate::render::render;
use crate::song::Song;
use crate::wav::{self, SampleFormat};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct Options {
    pub seed: u64,
    pub count: usize,
    pub keep: usize,
    /// How many of the best function scores are rendered and given a sound score.
    pub finalists: usize,
    /// Render every candidate (for checking the critic against the registered falsifier).
    pub render_all: bool,
    /// Also write the kept songs as WAV files.
    pub wav: bool,
    /// The base folder; the run goes in `<out>/<UTC timestamp>/`.
    pub out: PathBuf,
    /// Songs to score beside the candidates (the hand-written graph songs).
    pub compare: Vec<PathBuf>,
    pub threads: usize,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            seed: 1,
            count: 200,
            keep: 5,
            finalists: 20,
            render_all: false,
            wav: false,
            out: PathBuf::from("generated"),
            compare: Vec::new(),
            threads: std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4),
        }
    }
}

/// One scored song.
#[derive(Debug, Clone)]
pub struct Scored {
    /// `s<seed>-c<index>` for a candidate, the file name for a compared song.
    pub id: String,
    pub draft: Option<Draft>,
    pub text: String,
    pub report: Report,
}

#[derive(Debug)]
pub struct Outcome {
    pub dir: PathBuf,
    pub candidates: Vec<Scored>,
    /// Indices into `candidates`, best first.
    pub kept: Vec<usize>,
    pub compared: Vec<Scored>,
    pub seconds: f64,
}

/// Run `f` over `0..n` on `threads` threads, results in index order.
pub fn parallel<T: Send, F: Fn(usize) -> T + Sync>(n: usize, threads: usize, f: F) -> Vec<T> {
    let next = AtomicUsize::new(0);
    let out: Mutex<Vec<Option<T>>> = Mutex::new((0..n).map(|_| None).collect());
    std::thread::scope(|s| {
        for _ in 0..threads.max(1).min(n.max(1)) {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::SeqCst);
                if i >= n {
                    break;
                }
                let r = f(i);
                out.lock().unwrap()[i] = Some(r);
            });
        }
    });
    out.into_inner().unwrap().into_iter().map(|x| x.expect("every index ran")).collect()
}

/// Write and score one candidate from its functions alone.
pub fn candidate(seed: u64, index: u64) -> Result<Scored, String> {
    let draft = grammar::compose(seed, index)?;
    let song = Song::parse(&draft.text).map_err(|e| e.to_string())?;
    let report = critic::score(&song);
    Ok(Scored { id: format!("s{seed}-c{index}"), text: draft.text.clone(), draft: Some(draft), report })
}

/// Render a song and add the sound components to its report.
pub fn add_sound(s: &mut Scored) -> Result<crate::render::Mix, String> {
    let song = Song::parse(&s.text).map_err(|e| format!("{}: {e}", s.id))?;
    let mix = render(&song).map_err(|e| format!("{}: {e}", s.id))?;
    for x in mix.left.iter().chain(&mix.right) {
        if !x.is_finite() {
            return Err(format!("{}: the render holds a non-finite sample", s.id));
        }
    }
    s.report.add_sound(&sound::measure(&mix));
    Ok(mix)
}

fn by_score(a: f64, b: f64) -> std::cmp::Ordering {
    b.partial_cmp(&a).unwrap_or(std::cmp::Ordering::Equal)
}

/// The fraction of `pool` that `x` is strictly above.
pub fn percentile(x: f64, pool: &[f64]) -> f64 {
    if pool.is_empty() {
        return f64::NAN;
    }
    pool.iter().filter(|p| x > **p).count() as f64 / pool.len() as f64
}

/// A UTC timestamp `YYYYMMDD-HHMMSS` from the system clock (std has no local time zone).
pub fn utc_stamp() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as i64;
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}{m:02}{d:02}-{:02}{:02}{:02}", rem / 3600, rem % 3600 / 60, rem % 60)
}

/// Run the search and write its folder.
pub fn run(o: &Options) -> Result<Outcome, String> {
    if o.count == 0 || o.keep == 0 {
        return Err("count and keep must be at least 1".into());
    }
    let start = Instant::now();
    let results = parallel(o.count, o.threads, |i| candidate(o.seed, i as u64));
    let mut candidates = Vec::with_capacity(o.count);
    for r in results {
        candidates.push(r?);
    }
    let mut order: Vec<usize> = (0..candidates.len()).collect();
    order.sort_by(|&a, &b| by_score(candidates[a].report.function_score(), candidates[b].report.function_score()).then(a.cmp(&b)));
    let n_final = if o.render_all { candidates.len() } else { o.finalists.max(o.keep).min(candidates.len()) };
    let finalists: Vec<usize> = order[..n_final].to_vec();
    // Renders hold a whole song in memory each; keep the render threads to at most 8.
    let sounded = {
        let cands = &candidates;
        parallel(finalists.len(), o.threads.min(8), |k| {
            let mut s = cands[finalists[k]].clone();
            add_sound(&mut s).map(|_| s)
        })
    };
    for (k, s) in sounded.into_iter().enumerate() {
        candidates[finalists[k]] = s?;
    }
    let mut kept = finalists.clone();
    kept.sort_by(|&a, &b| by_score(candidates[a].report.total(), candidates[b].report.total()).then(a.cmp(&b)));
    kept.truncate(o.keep);

    let mut compared = Vec::new();
    for p in &o.compare {
        let text = std::fs::read_to_string(p).map_err(|e| format!("cannot read {}: {e}", p.display()))?;
        let song = Song::parse(&text).map_err(|e| format!("{}: {e}", p.display()))?;
        let id = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let mut s = Scored { id, draft: None, text, report: critic::score(&song) };
        add_sound(&mut s)?;
        compared.push(s);
    }

    let dir = o.out.join(utc_stamp());
    std::fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    for (rank, &i) in kept.iter().enumerate() {
        let c = &candidates[i];
        let fam = c.draft.as_ref().map(|d| d.choices.family.name()).unwrap_or("song");
        let name = format!("{}_{}_{}", rank + 1, c.id, fam);
        let path = dir.join(format!("{name}.txt"));
        std::fs::write(&path, &c.text).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
        if o.wav {
            let song = Song::parse(&c.text).map_err(|e| e.to_string())?;
            let mix = render(&song).map_err(|e| e.to_string())?;
            wav::write(&dir.join(format!("{name}.wav")), &mix, SampleFormat::Pcm16).map_err(|e| e.to_string())?;
        }
    }
    let seconds = start.elapsed().as_secs_f64();
    let outcome = Outcome { dir: dir.clone(), candidates, kept, compared, seconds };
    let md = scores_md(o, &outcome);
    std::fs::write(dir.join("scores.md"), md).map_err(|e| format!("cannot write scores.md: {e}"))?;
    note_ratings(&o.out, &outcome)?;
    Ok(outcome)
}

fn f2(x: f64) -> String {
    if x.is_finite() {
        format!("{x:.2}")
    } else {
        "-".into()
    }
}

fn quantile(xs: &[f64], q: f64) -> f64 {
    let mut s: Vec<f64> = xs.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    if s.is_empty() {
        return f64::NAN;
    }
    s[((s.len() - 1) as f64 * q).round() as usize]
}

/// The run's report.
pub fn scores_md(o: &Options, r: &Outcome) -> String {
    let mut m = String::new();
    m += &format!("# Composer run, seed {}\n\n", o.seed);
    m += &format!(
        "{} candidates written and scored from their functions; {} rendered and given a sound score; {} kept. {:.1} s on {} threads.\n\n",
        r.candidates.len(),
        r.candidates.iter().filter(|c| c.report.has_sound()).count(),
        r.kept.len(),
        r.seconds,
        o.threads
    );
    m += "Scores are 0..1. The critic, its components, weights and citations are in `docs/composer.md`. The function \
          score leaves out the sound (which needs a render); the total includes it.\n\n";
    let fs: Vec<f64> = r.candidates.iter().map(|c| c.report.function_score()).collect();
    m += &format!(
        "Function scores over all candidates: min {}, median {}, 80th percentile {}, max {}.\n\n",
        f2(quantile(&fs, 0.0)),
        f2(quantile(&fs, 0.5)),
        f2(quantile(&fs, 0.8)),
        f2(quantile(&fs, 1.0))
    );
    let names: Vec<&str> = WEIGHTS.iter().map(|w| w.0).collect();
    m += "## The kept songs\n\n| rank | song | family | total | function | ";
    m += &names.join(" | ");
    m += " |\n|---|---|---|---|---|";
    m += &"---|".repeat(names.len());
    m += "\n";
    let row = |label: String, s: &Scored, fam: &str| {
        let mut line = format!("{label} | {fam} | {} | {} |", f2(s.report.total()), f2(s.report.function_score()));
        for n in &names {
            line += &format!(" {} |", s.report.get(n).map(|c| f2(c.score)).unwrap_or_else(|| "-".into()));
        }
        line + "\n"
    };
    for (rank, &i) in r.kept.iter().enumerate() {
        let c = &r.candidates[i];
        let fam = c.draft.as_ref().map(|d| d.choices.family.name()).unwrap_or("");
        m += &row(format!("| {} | {}", rank + 1, c.id), c, fam);
    }
    if !r.compared.is_empty() {
        m += "\n## The hand-written songs, scored the same way\n\n| | song | | total | function | ";
        m += &names.join(" | ");
        m += " |\n|---|---|---|---|---|";
        m += &"---|".repeat(names.len());
        m += "\n";
        for s in &r.compared {
            m += &row(format!("| | {}", s.id), s, "hand");
        }
        let all_sound = r.candidates.iter().all(|c| c.report.has_sound());
        let pool_total: Vec<f64> = r.candidates.iter().filter(|c| c.report.has_sound()).map(|c| c.report.total()).collect();
        m += "\nWhere each hand-written song ranks among this run's random grammar samples (the fraction of samples it \
              scores above):\n\n";
        for s in &r.compared {
            m += &format!(
                "- **{}**: above {:.0}% by function score{}\n",
                s.id,
                100.0 * percentile(s.report.function_score(), &fs),
                if all_sound {
                    format!(", above {:.0}% by total (every sample rendered)", 100.0 * percentile(s.report.total(), &pool_total))
                } else {
                    String::new()
                }
            );
        }
    }
    m += "\n## Why each was kept\n\n";
    for (rank, &i) in r.kept.iter().enumerate() {
        let c = &r.candidates[i];
        let mut comps = c.report.components.clone();
        comps.sort_by(|a, b| by_score(a.score, b.score));
        m += &format!("### {}. {} (total {})\n\n", rank + 1, c.id, f2(c.report.total()));
        m += &format!(
            "Strongest: {}. Weakest: {}.\n\n",
            comps.iter().take(3).map(|x| format!("{} {} (measured {})", x.name, f2(x.score), f2(x.value))).collect::<Vec<_>>().join("; "),
            comps.iter().rev().take(2).map(|x| format!("{} {} (measured {})", x.name, f2(x.score), f2(x.value))).collect::<Vec<_>>().join("; ")
        );
        if let Some(d) = &c.draft {
            let notes: Vec<&str> = d.text.lines().take_while(|l| l.starts_with('#')).map(|l| l.trim_start_matches('#').trim()).collect();
            m += &format!("{}\n\n", notes.join(" ").trim());
            m += "Parameters: ";
            m += &d.params().iter().map(|(k, v)| format!("{k} = {v}")).collect::<Vec<_>>().join("; ");
            m += "\n\n";
        }
    }
    m
}

/// The ratings file: `generated/ratings.txt`. Lines are `<song id> <keep|skip|1-5> [note]`; `#` starts a comment.
pub const RATINGS_HEADER: &str = "# Ratings of composed songs, by ear. One line per song:\n\
# <song id> <keep | skip | 1-5> [a note]\n\
# e.g.  s7-c12 keep  the glide in bar 9\n\
# The song id is the s<seed>-c<index> in the file name. Lines starting with # are ignored. Each run appends a comment\n\
# listing its kept songs, so they can be rated in place. Nothing reads these yet: see docs/composer.md, \"Learning\n\
# from ratings\", for the plan.\n";

/// A rating: keep = 1 and skip = 0 on the keep scale; 1-5 as given.
#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    Keep,
    Skip,
    Stars(u8),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rating {
    pub id: String,
    pub verdict: Verdict,
    pub note: String,
}

/// Parse a ratings file. A malformed line is an error naming its line number, not silently skipped.
pub fn parse_ratings(src: &str) -> Result<Vec<Rating>, String> {
    let mut out = Vec::new();
    for (n, line) in src.lines().enumerate() {
        let l = line.trim();
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        let mut parts = l.splitn(3, char::is_whitespace);
        let id = parts.next().unwrap_or("").to_string();
        let v = parts.next().ok_or_else(|| format!("ratings line {}: expected '<id> <keep|skip|1-5>'", n + 1))?;
        let verdict = match v {
            "keep" => Verdict::Keep,
            "skip" => Verdict::Skip,
            s => match s.parse::<u8>() {
                Ok(k) if (1..=5).contains(&k) => Verdict::Stars(k),
                _ => return Err(format!("ratings line {}: '{s}' is not keep, skip or 1-5", n + 1)),
            },
        };
        out.push(Rating { id, verdict, note: parts.next().unwrap_or("").trim().to_string() });
    }
    Ok(out)
}

/// Create the ratings file if it is missing, and append a comment naming this run's kept songs. Only appends.
fn note_ratings(base: &Path, r: &Outcome) -> Result<(), String> {
    use std::io::Write;
    let path = base.join("ratings.txt");
    let fresh = !path.exists();
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("cannot open {}: {e}", path.display()))?;
    let mut s = String::new();
    if fresh {
        s += RATINGS_HEADER;
    }
    let dir = r.dir.file_name().map(|d| d.to_string_lossy().to_string()).unwrap_or_default();
    s += &format!("# run {dir}: {}\n", r.kept.iter().map(|&i| r.candidates[i].id.as_str()).collect::<Vec<_>>().join(" "));
    f.write_all(s.as_bytes()).map_err(|e| format!("cannot write {}: {e}", path.display()))
}
