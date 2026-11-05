//! Voices that listen: generator version 3 (2026-11-05; docs/dialogue.md). Before it, no lens counter-line read the
//! lead; both read only the chords, two voices on one topic that never respond to each other. Here the counter reads the
//! lead's own functions, so its line is made of what the lead does. A song draws one way of listening:
//! - **imitation**: the lead's pitch and gate read `d` beats earlier (every definition they read that depends on `t` is
//!   copied with `t` as `(t - d)`), an interval below, snapped to the chord now sounding: a canon;
//! - **contrary**: the lead mirrored about its own median, a fifth below, snapped to the chord;
//! - **answer**: the counter sounds only where the lead's gate is closed, drawn only for a stage whose lead is open less
//!   than [`ANSWER_DUTY`] of the time (else imitation: an answer to a voice that never pauses is silence).
//!
//! Only the played text is changed ([`apply`], through `Composer::dressed`), so how the stream composes is unchanged.

use super::display::{tokens, Tok};
use super::rng::Rng;
use super::stream::{Def, Equation};
use std::collections::{BTreeMap, BTreeSet};

/// The share of a stage the lead's gate may be open for the counter to answer in its gaps.
pub const ANSWER_DUTY: f64 = 0.8;

/// A song's way of listening (lead and counter).
#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    /// The counter plays the lead `delay` beats later, `interval` semitones from it, snapped to the chord.
    Imitation { delay: f64, interval: f64 },
    /// The counter mirrors the lead about the lead's median, a fifth below, snapped to the chord.
    Contrary,
    /// The counter sounds only where the lead is silent; a stage whose lead hardly pauses imitates instead.
    Answer { delay: f64, interval: f64 },
}

/// A song's way of listening, from its seed.
pub fn draw(seed: u64) -> Kind {
    let mut r = Rng::for_candidate(seed ^ 0x4449_414C_4F47, 0); // "DIALOG"
    let delay = *r.pick(&[1.0, 2.0, 4.0]);
    let interval = *r.pick(&[-12.0, -9.0, -8.0]);
    match r.weighted(&[0.45, 0.25, 0.30]) {
        0 => Kind::Imitation { delay, interval },
        1 => Kind::Contrary,
        _ => Kind::Answer { delay, interval },
    }
}

/// `expr` with each identifier `map` gives a replacement for replaced (by tokens, so `4t` and `Lm(` are safe).
fn substitute(expr: &str, map: &dyn Fn(&str) -> Option<String>) -> String {
    let mut out = String::new();
    let mut at = 0;
    for tk in tokens(expr) {
        if let Tok::Ident(n) = &tk.tok {
            if let Some(r) = map(n) {
                out += &expr[at..tk.start];
                out += &r;
                at = tk.end;
            }
        }
    }
    out + &expr[at..]
}

fn idents(expr: &str) -> BTreeSet<String> {
    tokens(expr).into_iter().filter_map(|t| if let Tok::Ident(n) = t.tok { Some(n) } else { None }).collect()
}

/// A function definition's parameters (`mn(x, a, b) = …` has x, a, b); none for a plain definition.
fn params(d: &Def) -> BTreeSet<String> {
    let lhs = d.line.split_once('=').map_or("", |(l, _)| l);
    match lhs.split_once('(') {
        Some((_, ps)) => idents(ps),
        None => BTreeSet::new(),
    }
}

/// The names a definition's right side reads from outside itself: its identifiers less its own parameters, which
/// shadow any definition of the same name (`mn(x, a, b)` reads its argument `b`, not a song's `b = frac(t)`).
fn free(d: &Def) -> BTreeSet<String> {
    let ps = params(d);
    idents(d.rhs()).into_iter().filter(|n| !ps.contains(n)).collect()
}

/// The definitions that read `t`, directly or through another definition.
fn reading_t(defs: &BTreeMap<String, Def>) -> BTreeSet<String> {
    let mut read: BTreeSet<String> = defs.iter().filter(|(_, d)| free(d).contains("t")).map(|(n, _)| n.clone()).collect();
    loop {
        let more: Vec<String> =
            defs.iter().filter(|(n, d)| !read.contains(*n) && free(d).iter().any(|i| read.contains(i))).map(|(n, _)| n.clone()).collect();
        if more.is_empty() {
            return read;
        }
        read.extend(more);
    }
}

/// The lead's expressions read `delay` beats earlier: (pitch, gate, the copied definitions to add).
fn delayed(eq: &Equation, pitch: &str, gate: &str, delay: f64) -> (String, String, Vec<Def>) {
    let defs: BTreeMap<String, Def> = eq.defs.iter().map(|d| (d.name.clone(), d.clone())).collect();
    let timed = reading_t(&defs);
    // the definitions the lead reaches that depend on t
    let mut want: BTreeSet<String> = BTreeSet::new();
    let mut todo: Vec<String> = idents(pitch).into_iter().chain(idents(gate)).collect();
    while let Some(n) = todo.pop() {
        if let Some(d) = defs.get(&n) {
            if timed.contains(&n) && want.insert(n.clone()) {
                todo.extend(free(d));
            }
        }
    }
    let suffix = format!("_d{}", delay as u64);
    let map = |n: &str| -> Option<String> {
        if n == "t" {
            Some(format!("(t - {delay})"))
        } else if want.contains(n) {
            Some(format!("{n}{suffix}"))
        } else {
            None
        }
    };
    let copies = want
        .iter()
        .map(|n| {
            let d = &defs[n];
            let (lhs, rhs) = d.line.split_once('=').expect("a definition has =");
            let new_name = format!("{n}{suffix}");
            let lhs = substitute(lhs.trim(), &|x| if x == n { Some(new_name.clone()) } else { None });
            let ps = params(d);
            let inside = |x: &str| if ps.contains(x) { None } else { map(x) };
            Def::new(&new_name, format!("{lhs} = {}", substitute(rhs.trim(), &inside)))
        })
        .collect();
    (substitute(pitch, &map), substitute(gate, &map), copies)
}

/// The lead's gate duty (share of 16ths open) and its median pitch over the stage, read from the played song.
fn lead_shape(eq: &Equation) -> Option<(f64, f64)> {
    let song = crate::song::Song::parse(&eq.song_text("listening")).ok()?;
    let lead = song.voices.iter().find(|v| v.name == "lead")?;
    let crate::song::VoiceKind::Tonal(tn) = &lead.kind else { return None };
    let steps = (song.beats * 4.0) as usize;
    let (mut open, mut pitches) = (0usize, Vec::new());
    for k in 0..steps {
        let t = k as f64 / 4.0 + 0.01;
        if tn.gate.eval(&song.program, t) > 0.0 {
            open += 1;
            let p = tn.pitch.eval(&song.program, t);
            if p.is_finite() {
                pitches.push(p);
            }
        }
    }
    pitches.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    let median = if pitches.is_empty() { 0.0 } else { pitches[pitches.len() / 2] };
    Some((open as f64 / steps.max(1) as f64, median))
}

/// `eq` with its counter listening to its lead the song's way, when both play; else `eq` unchanged.
pub fn apply(eq: &Equation, kind: &Kind) -> Equation {
    let (Some(lead), Some(counter)) = (eq.voice("lead"), eq.voice("counter")) else { return eq.clone() };
    let (Some(lp), Some(lg)) = (lead.get("pitch").map(str::to_string), lead.get("gate").map(str::to_string)) else { return eq.clone() };
    let Some(cg) = counter.get("gate").map(str::to_string) else { return eq.clone() };
    if cg.trim() == "0" {
        return eq.clone(); // a closed voice (the ring-out holds its played pitch): nothing to listen with
    }
    let Some((duty, median)) = lead_shape(eq) else { return eq.clone() };
    let mut e = eq.clone();
    let (pitch, gate) = match kind {
        Kind::Contrary => (format!("sc({} - ({lp}), floor(t / 2))", 2.0 * median - 7.0), cg),
        Kind::Answer { .. } if duty < ANSWER_DUTY => (counter.get("pitch").unwrap_or("0").to_string(), format!("({cg}) * (({lg}) <= 0)")),
        Kind::Answer { delay, interval } | Kind::Imitation { delay, interval } => {
            let (p, g, copies) = delayed(eq, &lp, &lg, *delay);
            for d in copies {
                if e.def(&d.name).is_none() {
                    e.defs.push(d);
                }
            }
            (format!("sc(({p}) + {interval}, floor(t / 2))"), g)
        }
    };
    if let Some(b) = e.voices.iter_mut().find(|v| v.name == "counter") {
        b.set("pitch", &pitch);
        b.set("gate", &gate);
    }
    e
}
