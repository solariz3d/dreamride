//! Drum grooves sampled from how real drummers play a style (since 2026-11-04). Nothing here is a written pattern:
//! the statistics in `groove_model.txt` were learned from the Groove MIDI Dataset (Magenta, CC BY 4.0; see CREDITS.md
//! and docs/groove.md), and every hit, loudness and timing offset of a groove is drawn from them.
//!
//! A groove, from a style and a seed:
//! - each instrument walks the 16 steps of a bar with P(hit | the step before hit or not), learned per step;
//! - a bar is kept only when every instrument's hit count and the kick+snare syncopation lie inside the 5th to 95th
//!   percentile of real bars of the style (at most 400 tries, then the try closest to the style's medians);
//! - bars 2 to 8 repeat bar 1 with each step re-drawn at the style's learned bar-to-bar change rate;
//! - bars 4 and 8 end with two beats drawn from the style's fills;
//! - time is kept by the hats or the ride, chosen by how often the style's drummers play each;
//! - timing is each step's learned mean offset with the style's overall lean removed: systematic, never jitter.
//!
//! [`replace_drums`] swaps a book's written kick, hat, clap and perc modules for generated ones (the lens book, at
//! load), so the stream arranges grooves exactly as it arranged the written modules.

use super::rng::Rng;
use std::collections::HashMap;

/// The learned statistics (`docs/groove.md` says how they were made).
pub const MODEL: &str = include_str!("groove_model.txt");

/// The instruments the model knows: kick, snare, closed hat, open hat, ride, toms, crash.
pub const INST: [char; 7] = ['K', 'S', 'H', 'O', 'R', 'T', 'C'];

#[derive(Debug, Clone, Default)]
pub struct Inst {
    /// P(hit at step i | the step before silent), P(hit at step i | the step before hit).
    pub p: [[f64; 16]; 2],
    pub vel_mean: [f64; 16],
    pub vel_sd: [f64; 16],
    /// Mean timing offset at each step, in 16ths.
    pub offset: [f64; 16],
    /// Share of steps that differ from the bar before.
    pub change: f64,
    pub hits: f64,
    /// Hits a bar: 5th percentile, median, 95th percentile of real bars.
    pub dens: (f64, f64, f64),
}

#[derive(Debug, Clone, Default)]
pub struct Part {
    pub bpm: (f64, f64, f64),
    pub sync: (f64, f64),
    pub inst: HashMap<char, Inst>,
}

#[derive(Debug, Clone, Default)]
pub struct Style {
    pub name: String,
    pub beat: Part,
    pub fill: Part,
}

fn nums(rest: &str) -> Result<Vec<f64>, String> {
    rest.split_whitespace().map(|x| x.parse::<f64>().map_err(|_| format!("not a number: '{x}'"))).collect()
}

fn sixteen(rest: &str) -> Result<[f64; 16], String> {
    let v = nums(rest)?;
    v.try_into().map_err(|v: Vec<f64>| format!("wanted 16 numbers, found {}", v.len()))
}

/// Read the model text. Errors name the line.
pub fn parse(text: &str) -> Result<Vec<Style>, String> {
    let mut styles: Vec<Style> = Vec::new();
    let mut part = "beat".to_string();
    for (n, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let err = |e: String| format!("groove_model.txt:{}: {e}", n + 1);
        let (key, rest) = line.split_once(' ').unwrap_or((line, ""));
        if key == "style" {
            styles.push(Style { name: rest.split_whitespace().next().unwrap_or("").to_string(), ..Style::default() });
            continue;
        }
        let style = styles.last_mut().ok_or_else(|| err("a line before any style".into()))?;
        if key == "part" {
            part = rest.trim().to_string();
            continue;
        }
        let p = if part == "fill" { &mut style.fill } else { &mut style.beat };
        match key {
            "bpm" => {
                let v = nums(rest).map_err(err)?;
                p.bpm = (v[0], v[1], v[2]);
            }
            "sync" => {
                let v = nums(rest).map_err(err)?;
                p.sync = (v[0], v[1]);
            }
            "inst" => {
                let w: Vec<&str> = rest.split_whitespace().collect();
                if w.len() != 9 || w[1] != "dens" || w[5] != "change" || w[7] != "hits" {
                    return Err(err(format!("an inst line wants 'X dens a b c change c hits n': '{rest}'")));
                }
                let f = |i: usize| w[i].parse::<f64>().map_err(|_| err(format!("not a number: '{}'", w[i])));
                let k = w[0].chars().next().ok_or_else(|| err("no instrument".into()))?;
                let i = p.inst.entry(k).or_default();
                i.dens = (f(2)?, f(3)?, f(4)?);
                i.change = f(6)?;
                i.hits = f(8)?;
            }
            "p0" | "p1" | "vm" | "vs" | "o" => {
                let (k, v) = rest.split_once(' ').ok_or_else(|| err("no instrument".into()))?;
                let k = k.chars().next().ok_or_else(|| err("no instrument".into()))?;
                let v = sixteen(v).map_err(err)?;
                let i = p.inst.get_mut(&k).ok_or_else(|| err(format!("'{key}' before 'inst {k}'")))?;
                match key {
                    "p0" => i.p[0] = v,
                    "p1" => i.p[1] = v,
                    "vm" => i.vel_mean = v,
                    "vs" => i.vel_sd = v,
                    _ => i.offset = v,
                }
            }
            _ => return Err(err(format!("unknown key '{key}'"))),
        }
    }
    for s in &styles {
        for k in INST {
            if !s.beat.inst.contains_key(&k) || !s.fill.inst.contains_key(&k) {
                return Err(format!("groove_model.txt: style {} has no instrument {k}", s.name));
            }
        }
    }
    Ok(styles)
}

/// The model's styles, parsed once.
pub fn styles() -> &'static [Style] {
    static S: std::sync::OnceLock<Vec<Style>> = std::sync::OnceLock::new();
    S.get_or_init(|| parse(MODEL).unwrap_or_else(|e| panic!("{e}")))
}

pub fn style(name: &str) -> Option<&'static Style> {
    styles().iter().find(|s| s.name == name)
}

/// Longuet-Higgins and Lee syncopation of one bar of onsets: a note on a weaker step followed by silence on a stronger.
pub fn syncopation(on: &[bool; 16]) -> f64 {
    const W: [i32; 16] = [0, -4, -3, -4, -2, -4, -3, -4, -1, -4, -3, -4, -2, -4, -3, -4];
    let mut s = 0;
    for i in 0..16 {
        if !on[i] {
            continue;
        }
        for j in i + 1..=i + 4 {
            let jj = j % 16;
            if on[jj] {
                break;
            }
            if W[jj] > W[i] {
                s += W[jj] - W[i];
                break;
            }
        }
    }
    s as f64
}

/// One bar: per instrument, a MIDI velocity (1..127) or 0 at each 16th.
pub type Bar = HashMap<char, [u8; 16]>;

#[derive(Debug, Clone)]
pub struct Groove {
    pub style: String,
    pub seed: u64,
    /// 'H' (hats) or 'R' (ride).
    pub keep: char,
    pub bars: Vec<Bar>,
    /// Each instrument's timing offset at each step, in 16ths (the style's lean removed), within -0.4..0.4.
    pub offset: HashMap<char, [f64; 16]>,
}

fn gauss(r: &mut Rng) -> f64 {
    (-2.0 * r.unit().max(1e-12).ln()).sqrt() * (std::f64::consts::TAU * r.unit()).cos()
}

fn velocity(i: &Inst, step: usize, r: &mut Rng) -> u8 {
    (i.vel_mean[step] + i.vel_sd[step] * gauss(r)).round().clamp(1.0, 127.0) as u8
}

/// Walk the steps `from..16` of `out`, each drawn given the step before (`prev_last`: was step 15 of the bar before hit).
fn walk(i: &Inst, r: &mut Rng, from: usize, prev_last: bool, out: &mut [u8; 16]) {
    let mut prev = if from > 0 { out[from - 1] > 0 } else { prev_last };
    for step in from..16 {
        let hit = r.unit() < i.p[prev as usize][step];
        out[step] = if hit { velocity(i, step, r) } else { 0 };
        prev = hit;
    }
}

fn count(a: &[u8; 16]) -> f64 {
    a.iter().filter(|v| **v > 0).count() as f64
}

/// A groove of eight bars in `style`, from `seed`. The same seed always gives the same groove.
pub fn groove(style: &Style, seed: u64) -> Groove {
    let mut r = Rng::for_candidate(seed ^ 0x4752_4F4F_5645, 0); // "GROOVE"
    let b = &style.beat;
    let hats = b.inst[&'H'].hits;
    let keep = if r.unit() < hats / (hats + b.inst[&'R'].hits).max(1.0) { 'H' } else { 'R' };
    let used: Vec<char> = INST.iter().copied().filter(|k| *k != 'C' && *k != if keep == 'H' { 'R' } else { 'H' }).collect();
    let in_band = |bar: &Bar| {
        used.iter().all(|k| {
            let n = count(&bar[k]);
            let d = b.inst[k].dens;
            n >= d.0 && n <= d.2
        }) && {
            let mut on = [false; 16];
            for s in 0..16 {
                on[s] = bar[&'K'][s] > 0 || bar[&'S'][s] > 0;
            }
            let s = syncopation(&on);
            s >= b.sync.0 && s <= b.sync.1
        } && count(&bar[&'K']) > 0.0
            && count(&bar[&'S']) > 0.0
    };
    let mut first: Option<Bar> = None;
    let mut best: Option<(f64, Bar)> = None;
    for _ in 0..400 {
        let mut bar: Bar = HashMap::new();
        for k in INST {
            let mut a = [0u8; 16];
            if used.contains(&k) {
                let i = &b.inst[&k];
                let prev_last = r.unit() < i.p[0][15];
                walk(i, &mut r, 0, prev_last, &mut a);
            }
            bar.insert(k, a);
        }
        if in_band(&bar) {
            first = Some(bar);
            break;
        }
        let err: f64 = used.iter().map(|k| (count(&bar[k]) - b.inst[k].dens.1).abs()).sum();
        if best.as_ref().is_none_or(|(e, _)| err < *e) {
            best = Some((err, bar));
        }
    }
    let first = first.or(best.map(|(_, bar)| bar)).expect("400 tries make at least one bar");
    let mut bars = vec![first];
    for n in 1..8 {
        let mut bar: Bar = HashMap::new();
        for k in INST {
            let mut a = bars[0][&k];
            if used.contains(&k) {
                let i = &b.inst[&k];
                for step in 0..16 {
                    if r.unit() < i.change {
                        let prev = if step > 0 { a[step - 1] > 0 } else { bars[n - 1][&k][15] > 0 };
                        a[step] = if r.unit() < i.p[prev as usize][step] { velocity(i, step, &mut r) } else { 0 };
                    }
                }
            }
            bar.insert(k, a);
        }
        if n == 3 || n == 7 {
            for k in ['K', 'S', 'T', keep, 'O'] {
                let a = bar.get_mut(&k).expect("every instrument");
                walk(&style.fill.inst[&k], &mut r, 8, false, a);
            }
        }
        bars.push(bar);
    }
    let mut offset = HashMap::new();
    for k in INST {
        let o = &b.inst[&k].offset;
        let lean = (o[0] + o[4] + o[8] + o[12]) / 4.0;
        let mut a = [0.0; 16];
        for s in 0..16 {
            a[s] = (o[s] - lean).clamp(-0.4, 0.4);
        }
        offset.insert(k, a);
    }
    Groove { style: style.name.clone(), seed, keep, bars, offset }
}

// ------------------------------------------------------------------------------------------------ the song language

/// MIDI velocity to the song language's digit 1..9 (0 for silence).
fn digit(v: u8) -> u8 {
    if v == 0 { 0 } else { ((v as f64 / 127.0 * 9.0).round() as u8).clamp(1, 9) }
}

/// Half a bar as an 8-digit integer: a 16-digit table starting with 9 would pass a double's exact range (2^53).
fn half(a: &[u8; 16], h: usize) -> String {
    a[8 * h..8 * h + 8].iter().map(|v| (b'0' + digit(*v)) as char).collect()
}

/// The shared timing: `Gs` the 16th step nearest the beat position, `Gu` how far from it, `Gb` the bar of the 8-bar
/// stage, and `Gt(v, o)` a hit when the step's digit `v` is set, `o` sixteenths from the grid.
pub const SHARED: &str = "Gn = floor(4t + 0.5)
Gu = 4t - Gn
Gs = mod(Gn, 16)
Gb = mod(floor(Gn / 16), 8)
Gt(v, o) = (v > 0)(Gu >= o)(0.3 - Gu + o)";

/// The lines that play instrument `k` of groove `g` under the prefix `p`: its two half-bar tables (one per bar of
/// eight), its timing offsets, and its digit at the current step.
fn tables(g: &Groove, k: char, p: &str) -> String {
    let mut out = String::new();
    for h in 0..2 {
        let mut e = String::new();
        for (i, bar) in g.bars.iter().enumerate() {
            if i < 7 {
                e.push_str(&format!("if(Gb == {i}, {}, ", half(&bar[&k], h)));
            } else {
                e.push_str(&half(&bar[&k], h));
            }
        }
        e.push_str(&")".repeat(7));
        out.push_str(&format!("{p}{h} = {e}\n"));
    }
    let offs: String = g.offset[&k].iter().map(|o| char::from(b'0' + ((o * 10.0).round() as i64 + 4).clamp(0, 8) as u8)).collect();
    out.push_str(&format!("{p}o = (mod(floor({offs} / 10^(15 - Gs)), 10) - 4) / 10\n"));
    out.push_str(&format!("{p}v = if(Gs < 8, mod(floor({p}0 / 10^(7 - Gs)), 10), mod(floor({p}1 / 10^(15 - Gs)), 10))\n"));
    out
}

const KICK: &str = "kind = dash\n  from = 450\n  to = 58\n  dash = 23\n  hold = 40\n  decay = 90\n  length = 380\n  partials = 0, -24, -12, -20, -19\n  rolloff = 660\n  gain = 0.68";
const SNARE: &str = "kind = snare\n  gain = 0.5\n  pan = -0.05\n  reverb = 0.25";
const HAT: &str = "kind = dash\n  from = 7600\n  to = 5400\n  dash = 5\n  decay = 12\n  length = 70\n  partials = 0, -10\n  rolloff = 660\n  gain = 2.4\n  pan = 0.3";
const RIDE: &str = "kind = dash\n  from = 5400\n  to = 4600\n  dash = 8\n  decay = 170\n  length = 650\n  partials = 0, -6, -11, -15\n  rolloff = 660\n  gain = 1.1\n  pan = 0.3";
const OPEN: &str = "kind = dash\n  from = 7000\n  to = 5200\n  dash = 6\n  decay = 85\n  length = 320\n  partials = 0, -8, -14\n  rolloff = 660\n  gain = 1.7\n  pan = -0.3";

/// 1..3 from where `n` falls against a style's band: below the median, up to midway to the 95th percentile, above.
fn level(n: f64, d: (f64, f64, f64)) -> u8 {
    if n < d.1 { 1 } else if n <= (d.1 + d.2) / 2.0 { 2 } else { 3 }
}

/// Four book modules from one groove, in the book's text: kick (the kick), clap (the snare), hat (the hats or the ride)
/// and perc (the open hat), each tagged with energy and density from how busy it is against the style.
pub fn modules_text(g: &Groove) -> String {
    let st = style(&g.style).expect("the groove's style");
    let mean = |k: char| g.bars.iter().map(|b| count(&b[&k])).sum::<f64>() / g.bars.len() as f64;
    let mut out = String::new();
    let parts: [(&str, char, &str, &str, &str); 4] = [
        ("kick", 'K', "kick", KICK, "the kick"),
        ("clap", 'S', "clap", SNARE, "the snare, ghost notes included"),
        ("hat", g.keep, "hat", if g.keep == 'H' { HAT } else { RIDE }, if g.keep == 'H' { "the hats" } else { "the ride" }),
        ("perc", 'O', "perc", OPEN, "the open hat"),
    ];
    for (slot, k, voice, sound, what) in parts {
        if g.bars.iter().all(|b| count(&b[&k]) == 0.0) {
            continue;
        }
        let d = st.beat.inst[&k].dens;
        let n = mean(k);
        let p = format!("G{}", k);
        out.push_str(&format!(
            "== {slot}.{}{} | {slot} | energy {} | density {} | mood groove\n# {what} of a {} groove (seed {}), sampled from how the dataset's {} drummers play: about {:.1} hits a bar\n{SHARED}\n{}\ndrum {voice} {{\n  trigger = Gt({p}v, {p}o)\n  amp = {p}v / 9\n  {sound}\n}}\n\n",
            g.style, g.seed, level(n, d), level(n, d), g.style, g.seed, g.style, n, tables(g, k, &p)
        ));
    }
    out
}

/// What a module counts as when a song measures the novelty it has spent: a generated groove module
/// (`kick.citypop7`) belongs to its slot's family (`kick.citypop`), since the grooves of one style are takes of one
/// idea, not new ideas; every other module is its own family (its id).
pub fn family(id: &str) -> String {
    if let Some((slot, rest)) = id.split_once('.') {
        for s in styles() {
            if let Some(seed) = rest.strip_prefix(s.name.as_str()) {
                if !seed.is_empty() && seed.bytes().all(|b| b.is_ascii_digit()) {
                    return format!("{slot}.{}", s.name);
                }
            }
        }
    }
    id.to_string()
}

/// A book text with its written kick, hat, clap and perc modules replaced by `count` generated grooves of `style`.
pub fn replace_drums(book_text: &str, style_name: &str, count: u64) -> String {
    let st = style(style_name).unwrap_or_else(|| panic!("groove_model.txt has no style '{style_name}'"));
    let mut out = String::new();
    let mut skip = false;
    for line in book_text.lines() {
        if let Some(head) = line.strip_prefix("== ") {
            let slot = head.split('|').nth(1).map(str::trim).unwrap_or("");
            skip = matches!(slot, "kick" | "hat" | "clap" | "perc");
        }
        if !skip {
            out.push_str(line);
            out.push('\n');
        }
    }
    for seed in 0..count {
        out.push_str(&modules_text(&groove(st, seed)));
    }
    out
}
