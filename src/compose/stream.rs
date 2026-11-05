//! Stream mode's composer: music as a chain of eight-bar equations, each an evolution of the one before.
//!
//! The idea, 2026-11-02, after the first composer: show the equation in real time, writing an eight-bar equation,
//! then stitching a new one after it, and so on, instead of a premade song in the conventional sense. That is how @measure_plan plays a graph: one equation sounds, he edits a line, it plays
//! on. Here the program makes the edits.
//!
//! **Version 2 (2026-11-03).** Listening to v1 live: the equation did not change enough every eight bars, and its
//! subtle changes stopped working as it grew more complex, though the bones were right. v1 edited a number at a time (a melody phase by a quarter, one root digit, one rhythm bit) and let voices
//! pile up. v2 makes every stage an event a listener hears, the way a graphing-calculator musician's stages are (a bass
//! and a pulse, then the kick, then the melody, then harmony, a breakdown, a new idea, a return):
//!
//! - **Arcs, not a random walk** ([`plan_arc`]): stages come in phrases of six to eight, each with a [`Role`]: intro,
//!   one to three builds, peak, breakdown, turn, return. The next arc opens on a thinned stage with a new generator
//!   idea, a modulation or a new progression, so the stream neither stagnates nor piles up.
//! - **Layer-level moves** ([`Move`]): a layer enters or leaves (the pads move as one layer); the melody's generator is
//!   replaced by another family's ([`grammar::generator`], plus two of its own: two waves, and residues `ak mod m`);
//!   the lead's rhythm integer or the kick's pattern becomes a clearly different groove (at least 5 or 3 of 16 steps
//!   differ); the chord digits become a new progression (at least 4 of 8 bars differ); the melody moves an octave;
//!   half-time or double-time; the lens moves two or three modes; a modulation; a breakdown to two layers; the drop.
//! - **Audibility is measured and required** ([`audible`]): a candidate stage below [`audible::THRESHOLD`] is not
//!   taken while one above it exists; the critic picks among those that clear it, and a stage with none is boosted
//!   (another layer, a new melody, a progression, a groove) until one does.
//! - **A complexity budget**: at most [`MAX_LAYERS`] layers and [`MAX_DEFS`] definitions (time names not counted). A
//!   layer entering at the budget pushes another out, and definitions nothing reads any more are removed ([`tidy`]).
//!
//! **Stage 1** is a seed song (a grammar song from a seed, or the kept song G, `s7-c38`, embedded below) put in
//! *stream form* ([`normalise`]: chord roots as eight digits `N`, the rhythm mask as a sixteen-bit `P`, the mode as a
//! term `c`, the lead plucked), then opened down to a low voice and a pulse ([`opening`]); the rest arrive by the arc.
//!
//! Each stage tries eight candidates for its role. Tempo never changes, the key only by a modulation, and nothing
//! returns to any of the last eight states. A seed always gives the same chain.

use super::audible;
use super::book::{self, Need, Slot};
use super::critic;
use super::display;
use super::grammar;
use super::harmony;
use super::rng::Rng;
use crate::song::Song;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// The length of one stage, in beats: eight bars of four.
pub const STAGE_BEATS: f64 = 32.0;

/// The seed kept from the blind test (generated/20261103-004208/5_s7-c38_squares.txt), liked for its odd simplicity
/// and its high plucked lead.
pub const DEFAULT_SEED_SONG: &str = include_str!("seeds/kept_s7-c38_squares.txt");

/// The time names the grammar writes (`n = floor(4t)` and the like): shown on one line of their own.
pub const TIME_NAMES: [&str; 8] = ["n", "f", "e", "u", "b", "j", "s", "tr"];

/// Candidate edits tried per stage.
const CANDIDATES: u64 = 8;
/// A stage never returns to any of this many recent states.
const RECENT: usize = 8;

/// The plucked lead timbre that was chosen (the kept song's `voice lead`).
const PLUCK_PARTIALS: &str = "0, -4, -6, -12, -10, -16, -15, -21";

/// One definition: its name and its whole line, `name(a) = rhs`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Def {
    pub name: String,
    pub line: String,
}

impl Def {
    pub fn new(name: &str, line: String) -> Def {
        Def { name: name.to_string(), line }
    }
    pub fn rhs(&self) -> &str {
        self.line.split_once('=').map_or("", |(_, r)| r.trim())
    }
    fn lhs(&self) -> &str {
        self.line.split_once('=').map_or("", |(l, _)| l.trim())
    }
    fn with_rhs(&self, rhs: &str) -> Def {
        Def { name: self.name.clone(), line: format!("{} = {rhs}", self.lhs()) }
    }
}

/// One voice or drum block: `voice name { key = value ... }`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Block {
    /// `voice` or `drum`.
    pub kind: String,
    pub name: String,
    pub fields: Vec<(String, String)>,
}

impl Block {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.fields.iter().find(|f| f.0 == key).map(|f| f.1.as_str())
    }
    pub(crate) fn set(&mut self, key: &str, value: &str) {
        match self.fields.iter_mut().find(|f| f.0 == key) {
            Some(f) => f.1 = value.to_string(),
            None => self.fields.push((key.to_string(), value.to_string())),
        }
    }
    pub fn is_drum(&self) -> bool {
        self.kind == "drum"
    }
    pub fn text(&self) -> String {
        let mut s = format!("{} {} {{\n", self.kind, self.name);
        for (k, v) in &self.fields {
            s += &format!("  {k} = {v}\n");
        }
        s + "}"
    }
}

/// A stage's equation: the settings, the definitions in order, the voices present, and the voices that are not
/// (kept so they can come back as they were).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Equation {
    /// Setting lines (`bpm 106`), without `beats` and `loop`.
    pub settings: Vec<String>,
    pub defs: Vec<Def>,
    pub voices: Vec<Block>,
    pub stash: Vec<Block>,
    /// Every voice name the stream can ever have, in a fixed order (the world's roster).
    pub roster: Vec<String>,
    /// In a stream from the move book: the module playing in each slot, as (slot, module id). Empty otherwise.
    pub book: Vec<(String, String)>,
    /// The species whose book the modules come from ([`book::Species`]); empty for the lens (the first book).
    pub species: String,
}

impl Equation {
    pub fn def(&self, name: &str) -> Option<&Def> {
        self.defs.iter().find(|d| d.name == name)
    }
    fn def_mut(&mut self, name: &str) -> Option<&mut Def> {
        self.defs.iter_mut().find(|d| d.name == name)
    }
    pub fn voice(&self, name: &str) -> Option<&Block> {
        self.voices.iter().find(|b| b.name == name)
    }
    fn has_name(&self, name: &str) -> bool {
        self.def(name).is_some()
    }
    fn fresh(&self, base: &str) -> String {
        if !self.has_name(base) {
            return base.to_string();
        }
        (2..).map(|k| format!("{base}{k}")).find(|n| !self.has_name(n)).expect("a free name")
    }

    /// The song file for this stage, `STAGE_BEATS` long, starting with a comment line.
    pub fn song_text(&self, comment: &str) -> String {
        let mut s = format!("# {comment}\n");
        for st in &self.settings {
            s += st;
            s += "\n";
        }
        s += &format!("beats {}\n\n", STAGE_BEATS);
        for d in &self.defs {
            s += &d.line;
            s += "\n";
        }
        for b in &self.voices {
            s += "\n";
            s += &b.text();
            s += "\n";
        }
        s
    }

    fn state_hash(&self) -> u64 {
        let mut h = DefaultHasher::new();
        self.defs.hash(&mut h);
        self.voices.hash(&mut h);
        h.finish()
    }

    /// Put a block among the voices in roster order.
    pub fn insert_voice(&mut self, b: Block) {
        let pos = |n: &str| self.roster.iter().position(|r| r == n).unwrap_or(usize::MAX);
        let at = self.voices.iter().position(|v| pos(&v.name) > pos(&b.name)).unwrap_or(self.voices.len());
        self.voices.insert(at, b);
    }
}

const SETTINGS: [&str; 15] = [
    "bpm", "beats", "rate", "reference", "reverb_size", "reverb_damp", "reverb_width", "reverb_predelay", "delay_beats",
    "delay_feedback", "delay_tone", "wow", "wow_rate", "warmth", "loop",
];

/// Read a song file into an equation (the song must parse; it is checked first).
pub fn parse_equation(src: &str) -> Result<Equation, String> {
    Song::parse(src).map_err(|e| e.to_string())?;
    let mut eq = Equation { settings: Vec::new(), defs: Vec::new(), voices: Vec::new(), stash: Vec::new(), roster: Vec::new(), book: Vec::new(), species: String::new() };
    let mut open: Option<Block> = None;
    for raw in src.lines() {
        let text = raw.split('#').next().unwrap_or("").trim();
        if text.is_empty() {
            continue;
        }
        if let Some(b) = open.as_mut() {
            if text == "}" {
                eq.voices.push(open.take().expect("open block"));
            } else if let Some((k, v)) = text.split_once('=') {
                b.fields.push((k.trim().to_string(), v.trim().to_string()));
            }
            continue;
        }
        let first = text.split_whitespace().next().unwrap_or("");
        if SETTINGS.contains(&first) {
            if first != "beats" && first != "loop" {
                eq.settings.push(text.split_whitespace().collect::<Vec<_>>().join(" "));
            }
        } else if first == "voice" || first == "drum" {
            let name = text[first.len()..].trim().trim_end_matches('{').trim().to_string();
            open = Some(Block { kind: first.to_string(), name, fields: Vec::new() });
        } else if let Some((lhs, rhs)) = text.split_once('=') {
            let name = lhs.split('(').next().unwrap_or("").trim().to_string();
            let lhs: String = lhs.trim().chars().filter(|c| !c.is_whitespace()).collect::<String>().replace(',', ", ");
            eq.defs.push(Def::new(&name, format!("{lhs} = {}", rhs.trim())));
        }
    }
    eq.roster = eq.voices.iter().map(|b| b.name.clone()).collect();
    Ok(eq)
}

// ------------------------------------------------------------------------------------------------ stream form

/// Remove `lay(dddd)` factors from an expression, returning it and the first digit (section 1) if there was one.
fn strip_lay(e: &str) -> (String, Option<char>) {
    let Some(i) = e.find("lay(") else { return (e.to_string(), None) };
    let Some(close) = e[i..].find(')') else { return (e.to_string(), None) };
    let digits = &e[i + 4..i + close];
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return (e.to_string(), None);
    }
    let rest = format!("{}{}", &e[..i], &e[i + close + 1..]);
    let rest = rest.trim().to_string();
    (if rest.is_empty() { "1".into() } else { rest }, digits.chars().next())
}

fn program_of(eq: &Equation) -> Option<crate::expr::Program> {
    let src: Vec<&str> = eq.defs.iter().map(|d| d.line.as_str()).collect();
    crate::expr::Program::from_source(&src.join("\n")).ok()
}

/// Put an equation in stream form (see the module header). Each step applies only when its pattern is there, so any
/// song can seed a stream; one that has none of them still streams, with fewer kinds of move.
pub fn normalise(mut eq: Equation) -> Equation {
    // Section switches: a voice silent in the song's first section starts out absent.
    let mut keep = Vec::new();
    for mut b in std::mem::take(&mut eq.voices) {
        let mut first = None;
        for key in ["gate", "trigger"] {
            if let Some(v) = b.get(key).map(str::to_string) {
                let (stripped, d) = strip_lay(&v);
                if d.is_some() {
                    first = d;
                    b.set(key, &stripped);
                }
            }
        }
        if first == Some('0') && b.name != "lead" {
            eq.stash.push(b);
        } else {
            keep.push(b);
        }
    }
    eq.voices = keep;
    if eq.voices.iter().chain(&eq.stash).all(|b| !b.fields.iter().any(|f| f.1.contains("lay("))) {
        eq.defs.retain(|d| d.name != "lay");
    }

    // The lens's mode as its own term.
    if !eq.has_name("c") {
        if let Some(h) = eq.def("h").cloned() {
            let r = h.rhs().replace(' ', "");
            if let Some(rest) = r.strip_prefix("floor((12d+") {
                if let Some(num) = rest.strip_suffix(")/7)") {
                    if num.parse::<i64>().is_ok() {
                        let at = eq.defs.iter().position(|d| d.name == "h").unwrap_or(0);
                        eq.defs[at] = h.with_rhs("floor((12d + c) / 7)");
                        eq.defs.insert(at, Def::new("c", format!("c = {num}")));
                    }
                }
            }
        }
    }

    // Chord roots as eight digits.
    if let (Some(r), Some(p)) = (eq.def("R").cloned(), program_of(&eq)) {
        let vals: Option<Vec<i64>> = (0..8).map(|i| p.call("R", &[i as f64]).ok().filter(|v| v.fract() == 0.0).map(|v| v as i64)).collect();
        if let (Some(vals), true) = (vals, r.lhs().replace(' ', "") == "R(i)") {
            let lo = *vals.iter().min().unwrap_or(&0);
            let hi = *vals.iter().max().unwrap_or(&0);
            if hi - lo <= 9 {
                let off = if lo >= -5 && hi <= 4 { 5 } else { -lo };
                let digits: String = vals.iter().map(|v| char::from(b'0' + (v + off) as u8)).collect();
                let n = eq.fresh("N");
                let at = eq.defs.iter().position(|d| d.name == "R").unwrap_or(0);
                eq.defs[at] = r.with_rhs(&format!("mod(floor({n} / 10^(7 - mod(i, 8))), 10) - {off}"));
                eq.defs.insert(at, Def::new(&n, format!("{n} = {digits}")));
            }
        }
    }

    // The rhythm mask as sixteen bits.
    if let (Some(on), Some(p)) = (eq.def("on").cloned(), program_of(&eq)) {
        if on.lhs().replace(' ', "") == "on(k)" {
            let bits: Option<Vec<bool>> = (0..16).map(|k| p.call("on", &[k as f64]).ok().map(|v| v > 0.0)).collect();
            if let Some(bits) = bits {
                let v: u32 = bits.iter().enumerate().map(|(k, b)| (*b as u32) << k).sum();
                let name = eq.fresh("P");
                let at = eq.defs.iter().position(|d| d.name == "on").unwrap_or(0);
                eq.defs[at] = on.with_rhs(&format!("mod(floor({name} / 2^mod(k, 16)), 2)"));
                eq.defs.insert(at, Def::new(&name, format!("{name} = {v}")));
            }
        }
    }

    // The lead takes the pluck.
    for b in eq.voices.iter_mut().chain(eq.stash.iter_mut()) {
        if b.name == "lead" && b.get("wave") == Some("additive") {
            b.set("partials", PLUCK_PARTIALS);
            b.set("attack", "5");
            b.set("decay", "300");
            b.set("sustain", "0.55");
            b.set("release", "150");
        }
    }
    eq.roster = eq.voices.iter().chain(&eq.stash).map(|b| b.name.clone()).collect();
    eq.roster.push(FILL_VOICE.to_string());
    let order: Vec<String> = eq.roster.clone();
    eq.voices.sort_by_key(|b| order.iter().position(|n| *n == b.name));
    eq
}

// ------------------------------------------------------------------------------------------------ the moves

const FILL_VOICE: &str = "fill";
const FILL_DEF: &str = "roll";

/// At most this many layers play at once (the pads count as one layer, the drone pair as one, the roll as none).
pub const MAX_LAYERS: usize = 6;
/// At most this many definitions, time names and the passing roll not counted. A stage that starts above it (a seed
/// can) may not grow.
pub const MAX_DEFS: usize = 13;
/// The same for a stream from the move book, whose modules write their shapes as definitions (a lead's step curve,
/// its glide and its gate) instead of hiding them in the voice blocks; the book's shared helpers are not counted.
pub const MAX_BOOK_DEFS: usize = 20;

/// The definitions shown as the lens (mode, key and the glide lens) and as voice lines (what a voice reads directly);
/// every other definition is a generator or one of its stored integers.
const LENS_NAMES: [&str; 4] = ["c", "h", "K", "g"];
const VOICE_LINE_NAMES: [&str; 5] = ["lead", "on", "W", "roll", "lay"];

fn fill_block() -> Block {
    let f = |k: &str, v: &str| (k.to_string(), v.to_string());
    Block {
        kind: "drum".into(),
        name: FILL_VOICE.into(),
        fields: vec![
            f("kind", "dash"),
            f("trigger", FILL_DEF),
            f("amp", "0.45 + 0.3 (t - 30)"),
            f("from", "320"),
            f("to", "110"),
            f("dash", "12"),
            f("hold", "4"),
            f("decay", "60"),
            f("length", "200"),
            f("partials", "0, -12, -9, -18"),
            f("rolloff", "660"),
            f("gain", "0.45"),
            f("pan", "-0.15"),
            f("reverb", "0.3"),
        ],
    }
}

/// `c = 2`, `c = 2 + s`, `c = 6 - s`: the leading integer and what follows it.
fn lens_parts(rhs: &str) -> Option<(i64, String)> {
    let t = rhs.trim();
    let end = t.find(|c: char| !c.is_ascii_digit()).unwrap_or(t.len());
    let n: i64 = t[..end].parse().ok()?;
    let rest = t[end..].trim().to_string();
    if rest.is_empty() || rest == "+ s" || rest == "- s" {
        Some((n, rest))
    } else {
        None
    }
}

/// The name of the eight-digit roots integer, if the equation has one.
fn roots_name(eq: &Equation) -> Option<String> {
    stored_name(eq, "R")
}

/// The name of the sixteen-bit rhythm integer, if any.
fn rhythm_name(eq: &Equation) -> Option<String> {
    stored_name(eq, "on")
}

/// `X` in `name(..) = mod(floor(X / ...`, when X is defined.
fn stored_name(eq: &Equation, def: &str) -> Option<String> {
    let rhs = eq.def(def)?.rhs();
    let start = rhs.find("mod(floor(")? + "mod(floor(".len();
    let name: String = rhs[start..].chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
    eq.def(&name).map(|_| name)
}

/// Replace every whole-word `old` in `text` by `new`.
fn rename_word(text: &str, old: &str, new: &str) -> String {
    let b = text.as_bytes();
    let word = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    let mut out = String::new();
    let mut i = 0;
    while i < text.len() {
        if text[i..].starts_with(old) && (i == 0 || !word(b[i - 1])) && (i + old.len() >= b.len() || !word(b[i + old.len()])) {
            out += new;
            i += old.len();
        } else {
            let ch = text[i..].chars().next().expect("a char");
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

/// The layers playing, in roster order, without the roll.
pub fn layers(eq: &Equation) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for b in eq.voices.iter().filter(|b| !is_seam_voice(&b.name)) {
        let l = audible::layer_of(&b.name);
        if !out.contains(&l) {
            out.push(l);
        }
    }
    out
}

fn stashed_layers(eq: &Equation) -> Vec<String> {
    let here = layers(eq);
    let mut out: Vec<String> = Vec::new();
    for b in eq.stash.iter().filter(|b| !is_seam_voice(&b.name)) {
        let l = audible::layer_of(&b.name);
        if !out.contains(&l) && !here.contains(&l) {
            out.push(l);
        }
    }
    out
}

/// Definitions that count against the budget: not the time names, not the passing roll.
pub fn def_count(eq: &Equation) -> usize {
    let bk = is_book(eq);
    eq.defs
        .iter()
        .filter(|d| !is_time_name(d) && d.name != FILL_DEF && !book::SEAM_DEFS.contains(&d.name.as_str()))
        // In a book stream the shared helpers (tri, T, bit, the chord pull, the glide) are vocabulary, like the time
        // names: the budget is for the lens, the progression and the modules' own lines.
        .filter(|d| !(bk && book::of(eq).helpers.iter().any(|h| h.name == d.name)))
        // A written progression's tables are its data (the chords, as digits), not generators: the budget counts its
        // mode `c` only, as it counted the lens's `c` before the progressions were written.
        .filter(|d| !(bk && book::of(eq).species.harmony && super::harmony::TABLE_NAMES.contains(&d.name.as_str())))
        .count()
}

/// The roll and a book seam's voices: they pass through one stage and are not layers.
fn is_seam_voice(name: &str) -> bool {
    name == FILL_VOICE || book::SEAM_VOICES.contains(&name)
}

/// A stream whose content comes from the move book.
pub fn is_book(eq: &Equation) -> bool {
    !eq.book.is_empty()
}

fn layer_in(eq: &mut Equation, layer: &str) {
    let blocks: Vec<Block> = eq.stash.iter().filter(|b| audible::layer_of(&b.name) == layer).cloned().collect();
    eq.stash.retain(|b| audible::layer_of(&b.name) != layer);
    for b in blocks {
        eq.insert_voice(b);
    }
}

fn layer_out(eq: &mut Equation, layer: &str) {
    let blocks: Vec<Block> = eq.voices.iter().filter(|b| audible::layer_of(&b.name) == layer).cloned().collect();
    eq.voices.retain(|b| audible::layer_of(&b.name) != layer);
    eq.stash.extend(blocks);
}

/// The order layers are brought in when nothing else decides: the beat, the tune, the harmony, then colour.
const ENTER_ORDER: [&str; 12] = ["kick", "lead", "pads", "bass", "harmony", "hat", "clap", "arp", "drone", "contour", "counter", "perc"];
const TONAL_LAYERS: [&str; 7] = ["pads", "bass", "harmony", "arp", "drone", "contour", "counter"];

fn priority(layer: &str) -> usize {
    ENTER_ORDER.iter().position(|l| *l == layer).unwrap_or(ENTER_ORDER.len())
}

/// `the kick comes in`, `the pads come in`.
fn says(layer: &str, one: &str, many: &str) -> String {
    format!("the {layer} {}", if layer == "pads" { many } else { one })
}

fn list(names: &[String]) -> String {
    match names.len() {
        0 => String::new(),
        1 => format!("the {}", names[0]),
        _ => format!("the {} and {}", names[..names.len() - 1].join(", "), names[names.len() - 1]),
    }
}

/// The kinds of edit. Each changes a layer, a generator, a groove, a progression, a register or a feel: something a
/// listener hears, not a phase moved by a quarter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Move {
    /// A stashed layer comes in (when the layers are at the budget, another leaves).
    Enter,
    /// A layer leaves (never the lead; at least two stay).
    Leave,
    /// The melody's generator is replaced by another family's.
    Melody,
    /// The lead's rhythm integer, or the kick's pattern, becomes a clearly different groove.
    Groove,
    /// The chord-root digits become a new progression.
    Progression,
    /// The melody moves an octave.
    Register,
    /// Half-time or double-time: the melody steps in eighths or in sixteenths.
    Feel,
    /// The lens moves two or three modes brighter or darker.
    Mode,
    /// The key moves up a fourth (or down a fifth).
    Modulate,
    /// Down to two layers.
    Breakdown,
    /// Down to three layers (an arc's opening).
    Thin,
    /// Every stashed layer returns, up to the budget.
    Drop,
    /// A roll in the last two beats, into the seam (in a book stream, a seam module).
    Fill,
    /// In a book stream: a playing bass, pads, arpeggio or counter-line takes another module from the book.
    Swap,
}

impl Move {
    pub const ALL: [Move; 14] = [
        Move::Enter,
        Move::Leave,
        Move::Melody,
        Move::Groove,
        Move::Progression,
        Move::Register,
        Move::Feel,
        Move::Mode,
        Move::Modulate,
        Move::Breakdown,
        Move::Thin,
        Move::Drop,
        Move::Fill,
        Move::Swap,
    ];
}

/// What a move is applied for: the stage's need (book streams choose modules by it) and the modules heard recently.
#[derive(Debug, Clone, Default)]
pub struct Ctx {
    pub need: Need,
    pub avoid: Vec<String>,
    /// Slots whose module has played unchanged for several stages: a swap takes one of these first (the bass most of
    /// all), so a song's bass does not sit through half of it.
    pub stale: Vec<Slot>,
}

/// What a stage of each role wants of a module.
pub fn need_for(role: Role) -> Need {
    let (energy, density) = match role {
        Role::Intro => (1, 1),
        Role::Build => (2, 2),
        Role::Peak => (3, 3),
        Role::Strip => (1, 1),
        Role::Turn => (2, 1),
        Role::Return => (3, 2),
        Role::Coda => (1, 1),
    };
    Need { energy, density }
}

/// An edit: the new equation, the names (definitions and voices) it changed, and what it did in words.
#[derive(Debug, Clone)]
pub struct Edit {
    pub eq: Equation,
    pub touched: Vec<String>,
    pub words: String,
}

fn enter_move(eq: &Equation, r: &mut Rng, swap: bool) -> Option<(Equation, String)> {
    let mut waiting = stashed_layers(eq);
    if waiting.is_empty() {
        return None;
    }
    waiting.sort_by_key(|l| priority(l));
    let l = if r.chance(0.65) { waiting[0].clone() } else { r.pick(&waiting[..waiting.len().min(3)]).clone() };
    let mut e = eq.clone();
    let mut words = says(&l, "comes in", "come in");
    if layers(&e).len() >= MAX_LAYERS {
        if !swap {
            return None; // a second layer in the same stage does not push out the first
        }
        // Something must leave: the layer that matters least, never the lead.
        let out = layers(&e).into_iter().filter(|x| x != "lead").max_by_key(|x| priority(x))?;
        layer_out(&mut e, &out);
        words += &format!(" and {}", says(&out, "leaves", "leave"));
    }
    layer_in(&mut e, &l);
    Some((e, words))
}

fn leave_move(eq: &Equation, r: &mut Rng) -> Option<(Equation, String)> {
    let here = layers(eq);
    let choices: Vec<String> = here.iter().filter(|l| *l != "lead").cloned().collect();
    if here.len() <= 2 || choices.is_empty() {
        return None;
    }
    let l = r.pick(&choices).clone();
    let mut e = eq.clone();
    layer_out(&mut e, &l);
    Some((e, says(&l, "leaves", "leave")))
}

/// Keep `keep` layers: the lead and tonal layers first, then the drums.
fn strip_to(eq: &Equation, r: &mut Rng, keep: usize) -> Option<(Equation, Vec<String>)> {
    let here = layers(eq);
    if here.len() <= keep {
        return None;
    }
    let mut tonal: Vec<String> = here.iter().filter(|l| TONAL_LAYERS.contains(&l.as_str())).cloned().collect();
    let mut rest: Vec<String> = here.iter().filter(|l| *l != "lead" && !TONAL_LAYERS.contains(&l.as_str())).cloned().collect();
    for v in [&mut tonal, &mut rest] {
        for i in (1..v.len()).rev() {
            let j = r.range(0, i as i64) as usize;
            v.swap(i, j);
        }
    }
    // The harmony only doubles the lead: beside it, it is the last tonal choice.
    if let Some(i) = tonal.iter().position(|l| l == "harmony") {
        let h = tonal.remove(i);
        tonal.push(h);
    }
    let mut kept: Vec<String> = Vec::new();
    if here.iter().any(|l| l == "lead") {
        kept.push("lead".into());
    }
    kept.extend(tonal.into_iter().chain(rest));
    kept.truncate(keep);
    let mut e = eq.clone();
    for l in here.iter().filter(|l| !kept.contains(l)) {
        layer_out(&mut e, l);
    }
    let order = layers(&e);
    Some((e, order))
}

fn breakdown_move(eq: &Equation, r: &mut Rng) -> Option<(Equation, String)> {
    let (e, kept) = strip_to(eq, r, 2)?;
    Some((e, format!("only {}", list(&kept))))
}

fn thin_move(eq: &Equation, r: &mut Rng) -> Option<(Equation, String)> {
    let (e, kept) = strip_to(eq, r, 3)?;
    Some((e, format!("it thins to {}", list(&kept))))
}

fn drop_move(eq: &Equation) -> Option<(Equation, String)> {
    let mut waiting = stashed_layers(eq);
    waiting.sort_by_key(|l| priority(l));
    let room = MAX_LAYERS.saturating_sub(layers(eq).len());
    let back: Vec<String> = waiting.into_iter().take(room).collect();
    if back.len() < 2 {
        return None;
    }
    let mut e = eq.clone();
    for l in &back {
        layer_in(&mut e, l);
    }
    Some((e, format!("the drop: {} {}", list(&back), "return")))
}

/// The steps a bar of the melody (`R(floor(k / spb))` in `M`).
fn melody_spb(m_rhs: &str) -> Option<i64> {
    let rest = m_rhs.trim().strip_prefix("R(floor(k / ")?;
    let end = rest.find(')')?;
    rest[..end].trim().parse().ok()
}

/// Which family a melody reads, from the helpers it calls.
pub fn family_of(m_rhs: &str) -> &'static str {
    let has = |n: &str| grammar::uses(m_rhs, n);
    if has("sq") {
        "squares"
    } else if has("T") {
        "fractal"
    } else if has("X") || has("Y") {
        "lissajous"
    } else if has("fm") {
        "fibonacci"
    } else if has("D") {
        "digits"
    } else if has("B") {
        "rule30"
    } else if has("sin") {
        "waves"
    } else if m_rhs.contains("mod(") {
        "residues"
    } else {
        "the seed's"
    }
}

const MELODY_FAMILIES: [&str; 9] = ["fractal", "squares", "lissajous", "fibonacci", "primes", "collatz", "rule30", "waves", "residues"];

/// A melody of `family`: its helper definitions, `M`'s new right side, and a few words.
fn melody_of(family: &str, r: &mut Rng, spb: i64) -> (Vec<String>, String, String) {
    let rate = (spb / 4).max(1);
    let gf = match family {
        "fractal" => Some(grammar::Family::Fractal),
        "squares" => Some(grammar::Family::Squares),
        "lissajous" => Some(grammar::Family::Lissajous),
        "fibonacci" => Some(grammar::Family::Fibonacci),
        "primes" => Some(grammar::Family::Primes),
        "collatz" => Some(grammar::Family::Collatz),
        "rule30" => Some(grammar::Family::Rule30),
        _ => None,
    };
    if let Some(f) = gf {
        let g = grammar::generator(f, r, spb, rate);
        let what = match f {
            grammar::Family::Fractal => "a Takagi fractal at three time scales",
            grammar::Family::Squares => "square waves (binary counting)",
            grammar::Family::Lissajous => "a Lissajous pair",
            grammar::Family::Fibonacci => "Fibonacci numbers modulo m",
            grammar::Family::Primes => "the primes as digits",
            grammar::Family::Collatz => "a Collatz trajectory as digits",
            grammar::Family::Rule30 => "rule 30's bits",
        };
        return (g.defs, g.melody, what.into());
    }
    if family == "waves" {
        let periods = [spb, 2 * spb, spb / 2, 12, 24, 3 * spb / 2];
        let p1 = *r.pick(&periods);
        let p2 = *r.pick(&periods.iter().copied().filter(|p| *p != p1).collect::<Vec<_>>());
        let a = *r.pick(&[2, 3, 3, 4]);
        let b = *r.pick(&[1, 2]);
        (Vec::new(), format!("R(floor(k / {spb})) + floor({a} sin(tau k / {p1}) + {b} cos(tau k / {p2}) + 0.5)"), format!("two waves, {p1} and {p2} steps long"))
    } else {
        let m = *r.pick(&[5, 7, 7]);
        let a = *r.pick(&[2, 3, 3]);
        let h = m / 2;
        (Vec::new(), format!("R(floor(k / {spb})) + mod({a} k + floor(k / {}), {m}) - {h}", rate), format!("{a}k modulo {m}, turning each beat"))
    }
}

/// `M` without its octave term, and the octave term (a multiple of 7 degrees).
fn split_octave(rhs: &str) -> (String, i64) {
    let t = rhs.trim_end();
    let digits = t.len() - t.trim_end_matches(|c: char| c.is_ascii_digit()).len();
    if digits == 0 {
        return (t.to_string(), 0);
    }
    let n: i64 = t[t.len() - digits..].parse().unwrap_or(0);
    let head = &t[..t.len() - digits];
    for (sign, s) in [(1, " + "), (-1, " - ")] {
        if let Some(core) = head.strip_suffix(s) {
            if n > 0 && n % 7 == 0 {
                return (core.to_string(), sign * n);
            }
        }
    }
    (t.to_string(), 0)
}

fn with_octave(core: &str, oct: i64) -> String {
    match oct.cmp(&0) {
        std::cmp::Ordering::Equal => core.to_string(),
        std::cmp::Ordering::Greater => format!("{core} + {oct}"),
        std::cmp::Ordering::Less => format!("{core} - {}", -oct),
    }
}

/// The lead's steps a beat, from `e = floor(Xt)`.
fn lead_rate(eq: &Equation) -> Option<i64> {
    let r = eq.def("e")?.rhs().replace(' ', "");
    r.strip_prefix("floor(")?.strip_suffix("t)")?.parse().ok()
}

/// The median of the lead's pitch over one stage, from `lead(k)`.
pub fn lead_median(eq: &Equation) -> Option<f64> {
    eq.def("lead")?;
    let p = program_of(eq)?;
    let rate = lead_rate(eq).unwrap_or(4);
    let mut xs: Vec<f64> = (0..(32 * rate))
        .filter_map(|k| p.call_at("lead", &[k as f64], k as f64 / rate as f64 + 1e-6).ok())
        .filter(|v| v.is_finite())
        .collect();
    if xs.is_empty() {
        return None;
    }
    xs.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    Some(xs[xs.len() / 2])
}

/// Put a definition before the time names (or last).
fn insert_def(eq: &mut Equation, d: Def, before: Option<&str>) {
    let at = before
        .and_then(|n| eq.defs.iter().position(|x| x.name == n))
        .or_else(|| eq.defs.iter().position(is_time_name))
        .unwrap_or(eq.defs.len());
    eq.defs.insert(at, d);
}

fn melody_move(eq: &Equation, r: &mut Rng) -> Option<(Equation, String)> {
    if eq.voice("lead").is_none() && eq.voice("harmony").is_none() {
        return None; // nobody would hear it
    }
    let m = eq.def("M")?.clone();
    let spb = melody_spb(m.rhs())?;
    let current = family_of(m.rhs());
    let fams: Vec<&str> = MELODY_FAMILIES
        .iter()
        .copied()
        .filter(|f| *f != current && !(current == "digits" && (*f == "primes" || *f == "collatz")))
        .collect();
    let family = *r.pick(&fams);
    let (helpers, melody, what) = melody_of(family, r, spb);
    let mut e = eq.clone();
    // The new helpers, renamed where a different definition already holds the name.
    let mut new_defs: Vec<Def> = Vec::new();
    let mut melody = melody;
    let mut lines: Vec<String> = helpers;
    let mut i = 0;
    while i < lines.len() {
        let Some((lhs, _)) = lines[i].split_once('=') else { i += 1; continue };
        let name = lhs.split('(').next().unwrap_or("").trim().to_string();
        if let Some(old) = e.def(&name) {
            if old.line == lines[i] {
                i += 1;
                continue;
            }
            let taken = |n: &str| e.has_name(n) || lines.iter().any(|l| l.split('(').next().unwrap_or("").split('=').next().unwrap_or("").trim() == n);
            let fresh = (2..).map(|k| format!("{name}{k}")).find(|n| !taken(n)).expect("a free name");
            for l in lines.iter_mut() {
                *l = rename_word(l, &name, &fresh);
            }
            melody = rename_word(&melody, &name, &fresh);
            continue; // look at the renamed line again
        }
        new_defs.push(Def::new(&name, lines[i].clone()));
        i += 1;
    }
    for d in new_defs {
        insert_def(&mut e, d, Some("M"));
    }
    let before = lead_median(eq);
    *e.def_mut("M")? = m.with_rhs(&melody);
    if let (Some(b), Some(a)) = (before, lead_median(&e)) {
        let o = ((b - a) / 12.0).round() as i64;
        if o != 0 {
            *e.def_mut("M")? = m.with_rhs(&with_octave(&melody, 7 * o));
        }
    }
    Some((e, format!("a new melody: {what}")))
}

fn register_move(eq: &Equation, r: &mut Rng) -> Option<(Equation, String)> {
    eq.voice("lead")?;
    let m = eq.def("M")?.clone();
    let med = lead_median(eq)?;
    let up = if med > 10.0 {
        false
    } else if med < -6.0 {
        true
    } else {
        r.chance(0.5)
    };
    let (core, oct) = split_octave(m.rhs());
    let mut e = eq.clone();
    *e.def_mut("M")? = m.with_rhs(&with_octave(&core, oct + if up { 7 } else { -7 }));
    Some((e, format!("the melody moves {} an octave", if up { "up" } else { "down" })))
}

fn feel_move(eq: &Equation) -> Option<(Equation, String)> {
    eq.voice("lead")?;
    let x = lead_rate(eq)?;
    let u = eq.def("u")?.rhs().replace(' ', "");
    if u != format!("frac({x}t)") || !(x == 2 || x == 4) {
        return None;
    }
    let m = eq.def("M")?.clone();
    let spb = melody_spb(m.rhs())?;
    if spb != 4 * x {
        return None;
    }
    let nx = if x == 4 { 2 } else { 4 };
    let mut e = eq.clone();
    *e.def_mut("e")? = Def::new("e", format!("e = floor({nx}t)"));
    *e.def_mut("u")? = Def::new("u", format!("u = frac({nx}t)"));
    let rhs = m.rhs().replacen(&format!("R(floor(k / {spb}))"), &format!("R(floor(k / {}))", 4 * nx), 1);
    *e.def_mut("M")? = m.with_rhs(&rhs);
    Some((e, if nx < x { "half-time: the melody steps in eighths".into() } else { "double-time: the melody steps in sixteenths".into() }))
}

/// Euclid E(k, 16) rotated by `rot`, as bits (bit i = sixteenth i).
fn euclid16(k: u32, rot: u32) -> u32 {
    (0..16).filter(|i| (k * ((i + rot) % 16)) % 16 < k).map(|i| 1u32 << i).sum()
}

/// Sixteen steps as `x` and `·`.
fn steps(bits: u32) -> String {
    (0..16).map(|i| if bits >> i & 1 == 1 { 'x' } else { '·' }).collect()
}

const KICKS: [&[u32]; 10] =
    [&[0, 4, 8, 12], &[0, 8], &[0, 6, 10], &[0, 3, 8, 11], &[0, 7, 10], &[0, 10], &[0, 6, 8, 14], &[0, 5, 8, 13], &[0, 3, 6, 10, 12], &[0, 4, 7, 10]];

fn groove_move(eq: &Equation, r: &mut Rng) -> Option<(Equation, String)> {
    let lead_ok = eq.voice("lead").is_some()
        && (rhythm_name(eq).is_some() || eq.voice("lead").and_then(|b| b.get("gate")).is_some_and(|g| !g.contains("frac(t / 16)") && !g.starts_with("on(")));
    let kick_ok = eq.voice("kick").is_some();
    let kick = match (lead_ok, kick_ok) {
        (false, false) => return None,
        (true, false) => false,
        (false, true) => true,
        _ => r.chance(0.45),
    };
    let mut e = eq.clone();
    if kick {
        let song = Song::parse(&eq.song_text("groove")).ok()?;
        let prof = audible::profile(&song);
        let cur: u32 = prof.voices.iter().find(|v| v.name == "kick").map_or(0, |v| (0..16).filter(|&i| v.onsets[i]).map(|i| 1u32 << i).sum());
        let mut opts: Vec<u32> = KICKS.iter().map(|s| s.iter().map(|i| 1u32 << i).sum()).filter(|b: &u32| (b ^ cur).count_ones() >= 3).collect();
        if opts.is_empty() {
            return None;
        }
        let q = opts.remove(r.range(0, opts.len() as i64 - 1) as usize);
        // The kick's own pattern integer if it already reads one, else a new one.
        let trig = eq.voice("kick")?.get("trigger").unwrap_or("").to_string();
        let existing = trig.strip_prefix("mod(floor(").and_then(|s| s.split(' ').next()).filter(|n| eq.has_name(n)).map(str::to_string);
        let name = existing.unwrap_or_else(|| eq.fresh("Q"));
        match e.def_mut(&name) {
            Some(d) => *d = Def::new(&name, format!("{name} = {q}")),
            None => insert_def(&mut e, Def::new(&name, format!("{name} = {q}")), None),
        }
        let trig = format!("mod(floor({name} / 2^mod(n, 16)), 2)(0.5 - f)");
        let i = e.voices.iter().position(|b| b.name == "kick")?;
        e.voices[i].set("trigger", &trig);
        return Some((e, format!("the kick plays a new pattern, {}", steps(q))));
    }
    let cur = match rhythm_name(eq) {
        Some(n) => eq.def(&n)?.rhs().parse::<u32>().ok()?,
        None => 0xFFFF,
    };
    let mut pick = None;
    for _ in 0..60 {
        let k = *r.pick(&[4u32, 5, 6, 7, 9, 10, 11, 12]);
        let rot = r.range(0, 15) as u32;
        let b = euclid16(k, rot);
        if (b ^ cur).count_ones() >= 5 {
            pick = Some((b, k));
            break;
        }
    }
    let (bits, k) = pick?;
    match rhythm_name(eq) {
        Some(n) => *e.def_mut(&n)? = Def::new(&n, format!("{n} = {bits}")),
        None => {
            if eq.has_name("on") {
                return None;
            }
            let p = eq.fresh("P");
            insert_def(&mut e, Def::new(&p, format!("{p} = {bits}")), Some("lead"));
            insert_def(&mut e, Def::new("on", format!("on(k) = mod(floor({p} / 2^mod(k, 16)), 2)")), Some("lead"));
            let gate = eq.voice("lead")?.get("gate")?.to_string();
            let new_gate = if gate.starts_with('(') { format!("on(e){gate}") } else { format!("on(e)({gate})") };
            for b in e.voices.iter_mut().chain(e.stash.iter_mut()) {
                if (b.name == "lead" || b.name == "harmony") && b.get("gate") == Some(gate.as_str()) {
                    b.set("gate", &new_gate);
                }
            }
        }
    }
    Some((e, format!("the lead takes a new groove, E({k}, 16): {}", steps(bits))))
}

/// Chord progressions as scale degrees a bar (0 is the tonic).
const PROGRESSIONS: [[i64; 8]; 10] = [
    [0, 0, 5, 5, 3, 3, 4, 4],
    [0, 4, 5, 3, 0, 4, 5, 3],
    [5, 3, 0, 4, 5, 3, 0, 4],
    [0, 3, 4, 0, 0, 3, 4, 4],
    [0, -1, -2, -3, 0, -1, -2, -3],
    [0, 5, 1, 4, 0, 5, 1, 4],
    [3, 4, 2, 5, 3, 4, 0, 0],
    [0, 0, 3, 3, 0, 0, 4, 3],
    [0, 6, 5, 4, 3, 2, 1, 4],
    [1, 4, 0, 0, 1, 4, 0, 5],
];

fn progression_move(eq: &Equation, r: &mut Rng) -> Option<(Equation, String)> {
    let name = roots_name(eq)?;
    let d = eq.def(&name)?.clone();
    let rrhs = eq.def("R")?.rhs().to_string();
    let off: i64 = rrhs.rsplit_once(" - ")?.1.trim().parse().ok()?;
    let mut digits: Vec<i64> = d.rhs().bytes().map(|b| (b - b'0') as i64).collect();
    if digits.len() > 8 || digits.iter().any(|&x| !(0..=9).contains(&x)) {
        return None;
    }
    while digits.len() < 8 {
        digits.insert(0, 0);
    }
    let cur: Vec<i64> = digits.iter().map(|x| x - off).collect();
    let mut order: Vec<usize> = (0..PROGRESSIONS.len()).collect();
    for i in (1..order.len()).rev() {
        let j = r.range(0, i as i64) as usize;
        order.swap(i, j);
    }
    for i in order {
        let p = PROGRESSIONS[i];
        let fit: Vec<i64> = p
            .iter()
            .map(|&x| {
                let mut v = x;
                while v + off > 9 {
                    v -= 7;
                }
                while v + off < 0 {
                    v += 7;
                }
                v
            })
            .collect();
        let differ = fit.iter().zip(&cur).filter(|(a, b)| (*a - *b).rem_euclid(7) != 0).count();
        if differ < 4 {
            continue;
        }
        let s: String = fit.iter().map(|v| char::from(b'0' + (v + off) as u8)).collect();
        let label: Vec<String> = p.iter().map(|x| (x.rem_euclid(7) + 1).to_string()).collect();
        let mut e = eq.clone();
        *e.def_mut(&name)? = d.with_rhs(&s);
        return Some((e, format!("a new progression, {}", label.join(" "))));
    }
    None
}

fn mode_move(eq: &Equation, r: &mut Rng) -> Option<(Equation, String)> {
    if is_book(eq) && book::of(eq).species.harmony {
        return None; // the melody's mode comes with the written progression (harmony is written, never generated)
    }
    let d = eq.def("c")?;
    let (n, rest) = lens_parts(d.rhs())?;
    let (lo, hi) = match rest.as_str() {
        "+ s" => (0, 5),
        "- s" => (1, 6),
        _ => (0, 6),
    };
    let step = *r.pick(&[2, 3]);
    let m = if (r.chance(0.5) && n + step <= hi) || n - step < lo { n + step } else { n - step };
    if !(lo..=hi).contains(&m) {
        return None;
    }
    let mut e = eq.clone();
    let rhs = if rest.is_empty() { m.to_string() } else { format!("{m} {rest}") };
    *e.def_mut("c")? = d.with_rhs(&rhs);
    Some((e, format!("the lens moves {} to {}", grammar::mode_name(n), grammar::mode_name(m))))
}

fn modulate_move(eq: &Equation) -> Option<(Equation, String)> {
    if eq.voices.iter().chain(&eq.stash).any(|b| b.name == "drone" || b.name == "ratio") {
        return None; // a just-ratio drone is not on the lens and would not follow the key
    }
    let h = eq.def("h")?.clone();
    let mut e = eq.clone();
    let k = match e.def("K") {
        Some(d) => d.rhs().parse::<i64>().ok()?,
        None => {
            if h.rhs().contains('K') {
                return None;
            }
            let at = e.defs.iter().position(|d| d.name == "h")?;
            e.defs[at] = h.with_rhs(&format!("{} + K", h.rhs()));
            e.defs.insert(at, Def::new("K", "K = 0".into()));
            0
        }
    };
    let nk = if k + 5 <= 7 { k + 5 } else { k - 7 };
    *e.def_mut("K")? = Def::new("K", format!("K = {nk}"));
    Some((e, format!("modulates {} (K = {nk})", if nk > k { "up a fourth" } else { "down a fifth" })))
}

fn fill_move(eq: &Equation) -> Option<(Equation, String)> {
    if eq.voice(FILL_VOICE).is_some() || eq.has_name(FILL_DEF) {
        return None;
    }
    let mut e = eq.clone();
    e.defs.push(Def::new(FILL_DEF, format!("{FILL_DEF} = (t >= 30)(0.5 - frac(4t))")));
    e.insert_voice(fill_block());
    Some((e, "a roll into the seam".into()))
}

/// Take a fill away again (the stage after it), returning what was removed.
fn remove_fill(eq: &mut Equation) -> Vec<String> {
    let mut gone = Vec::new();
    if eq.voice(FILL_VOICE).is_some() {
        eq.voices.retain(|v| v.name != FILL_VOICE);
        gone.push(FILL_VOICE.to_string());
    }
    if eq.has_name(FILL_DEF) {
        eq.defs.retain(|d| d.name != FILL_DEF);
        gone.push(FILL_DEF.to_string());
    }
    if is_book(eq) {
        let before = eq.clone();
        book::uninstall(eq, Slot::Seam);
        book::finish(eq);
        gone.extend(diff(&before, eq));
    }
    gone
}

/// The time names the grammar writes, for the ones a new generator reads.
const STANDARD_TIMES: [(&str, &str); 5] = [("n", "floor(4t)"), ("f", "frac(4t)"), ("b", "frac(t)"), ("j", "floor(t / 4)"), ("s", "floor(t / 16)")];

/// Drop the definitions nothing reads any more (from the voices playing or waiting, directly or through other
/// definitions), and add any standard time name that is read but missing.
pub fn tidy(eq: &mut Equation) {
    let mut keep: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut frontier: Vec<String> =
        eq.voices.iter().chain(&eq.stash).flat_map(|b| b.fields.iter().map(|f| f.1.clone())).collect();
    while let Some(text) = frontier.pop() {
        for d in &eq.defs {
            if !keep.contains(&d.name) && grammar::uses(&text, &d.name) {
                keep.insert(d.name.clone());
                frontier.push(d.rhs().to_string());
            }
        }
    }
    eq.defs.retain(|d| keep.contains(&d.name));
    for (n, rhs) in STANDARD_TIMES {
        if eq.has_name(n) {
            continue;
        }
        let read = eq.defs.iter().any(|d| grammar::uses(d.rhs(), n))
            || eq.voices.iter().chain(&eq.stash).any(|b| b.fields.iter().any(|f| grammar::uses(&f.1, n)));
        if read {
            eq.defs.push(Def::new(n, format!("{n} = {rhs}")));
        }
    }
}

/// Apply one move. None when it does not apply to this equation (nothing to edit, or it would change nothing).
pub fn apply(m: Move, eq: &Equation, r: &mut Rng) -> Option<Edit> {
    apply_ex(m, eq, r, true, &Ctx::default())
}

/// As [`apply`], for a stage with a need (book streams choose modules by it).
pub fn apply_with(m: Move, eq: &Equation, r: &mut Rng, ctx: &Ctx) -> Option<Edit> {
    apply_ex(m, eq, r, true, ctx)
}

/// As [`apply`]; with `swap` false, an `Enter` at the layer budget does not apply (rather than push another out).
fn apply_ex(m: Move, eq: &Equation, r: &mut Rng, swap: bool, ctx: &Ctx) -> Option<Edit> {
    let bk = is_book(eq);
    let (mut e, words) = match m {
        Move::Enter if bk => book_enter(eq, r, swap, ctx),
        Move::Enter => enter_move(eq, r, swap),
        Move::Leave => leave_move(eq, r),
        Move::Melody if bk => book_melody(eq, ctx, r),
        Move::Melody => melody_move(eq, r),
        Move::Groove if bk => book_groove(eq, ctx, r),
        Move::Groove => groove_move(eq, r),
        Move::Progression if bk => book_progression(eq, ctx, r),
        Move::Progression => progression_move(eq, r),
        Move::Register if bk => book_register(eq, r),
        Move::Register => register_move(eq, r),
        Move::Feel if bk => None,
        Move::Feel => feel_move(eq),
        Move::Mode => mode_move(eq, r),
        Move::Modulate => modulate_move(eq),
        Move::Breakdown => breakdown_move(eq, r),
        Move::Thin => thin_move(eq, r),
        Move::Drop => drop_move(eq),
        Move::Fill if bk => book_fill(eq, "return", r),
        Move::Fill => fill_move(eq),
        Move::Swap if bk => book_swap_any(eq, ctx, r),
        Move::Swap => None,
    }?;
    tidy(&mut e);
    if bk {
        book::finish(&mut e);
    }
    let touched = diff(eq, &e);
    if touched.is_empty() {
        return None;
    }
    Some(Edit { eq: e, touched, words })
}

/// What differs between two equations: definitions added, removed or rewritten, and voices that joined, left or
/// changed, by name, in the new equation's order (removed ones last).
pub fn diff(old: &Equation, new: &Equation) -> Vec<String> {
    let mut out = Vec::new();
    for d in &new.defs {
        if old.def(&d.name).map(|o| &o.line) != Some(&d.line) {
            out.push(d.name.clone());
        }
    }
    for b in &new.voices {
        if old.voice(&b.name) != Some(b) {
            out.push(b.name.clone());
        }
    }
    for d in &old.defs {
        if new.def(&d.name).is_none() {
            out.push(d.name.clone());
        }
    }
    for b in &old.voices {
        if new.voice(&b.name).is_none() {
            out.push(b.name.clone());
        }
    }
    out
}

// ------------------------------------------------------------------------------------------------ book moves

/// How a module coming into a slot is named in a stage's summary.
fn book_words(slot: Slot, m: &book::Module) -> String {
    match slot {
        Slot::Lead => format!("a new melody, {}", m.id),
        Slot::Progression => format!("a new progression, {}", m.id),
        Slot::Pads => format!("the pads take {}", m.id),
        s => format!("the {} takes {}", s.name(), m.id),
    }
}

fn book_swap(eq: &Equation, slot: Slot, ctx: &Ctx, r: &mut Rng) -> Option<(Equation, String)> {
    let m = book::pick(book::of(eq), slot, ctx.need, book::current(eq, slot), &ctx.avoid, r)?;
    Some((book::install(eq, m), book_words(slot, m)))
}

fn playing_slots(eq: &Equation, among: &[Slot]) -> Vec<Slot> {
    let here = layers(eq);
    among.iter().copied().filter(|s| here.iter().any(|l| l == s.name())).collect()
}

/// A layer comes in; half the time it comes with a module chosen for this stage's need.
fn book_enter(eq: &Equation, r: &mut Rng, swap: bool, ctx: &Ctx) -> Option<(Equation, String)> {
    let before = layers(eq);
    let (e, words) = enter_move(eq, r, swap)?;
    let came = layers(&e).into_iter().find(|l| !before.contains(l))?;
    if let Some(slot) = Slot::of_layer(&came) {
        if r.chance(0.5) {
            if let Some(m) = book::pick(book::of(&e), slot, ctx.need, book::current(&e, slot), &ctx.avoid, r) {
                return Some((book::install(&e, m), format!("{words}, playing {}", m.id)));
            }
        }
    }
    Some((e, words))
}

/// A new lead module from the book; when the book has none left to offer, a melody from the grammar's generators.
fn book_melody(eq: &Equation, ctx: &Ctx, r: &mut Rng) -> Option<(Equation, String)> {
    eq.voice("lead")?;
    book_swap(eq, Slot::Lead, ctx, r).or_else(|| grammar_lead(eq, r))
}

/// The fallback: a grammar generator's melody on eighths, gliding, in the lead's slot.
pub fn grammar_lead(eq: &Equation, r: &mut Rng) -> Option<(Equation, String)> {
    if !eq.species.is_empty() {
        return None; // the grammar writes through the lens; another species has no h to read
    }
    let fam = *r.pick(&grammar::Family::ALL);
    let g = grammar::generator(fam, r, 8, 2);
    let mut lines = g.defs.clone();
    let mut melody = g.melody.clone();
    for d in &g.defs {
        let name = d.split(['(', '=']).next().unwrap_or("").trim().to_string();
        if name.is_empty() {
            continue;
        }
        let new = format!("Lx{name}");
        for l in lines.iter_mut() {
            *l = rename_word(l, &name, &new);
        }
        melody = rename_word(&melody, &name, &new);
    }
    let mut defs: Vec<Def> = lines
        .iter()
        .filter_map(|l| l.split_once('=').map(|(lhs, rhs)| (lhs.split('(').next().unwrap_or("").trim().to_string(), lhs.trim(), rhs.trim())))
        .map(|(n, lhs, rhs)| Def::new(&n, format!("{lhs} = {rhs}")))
        .collect();
    // The grammar's melodies stand on a bar's chord root as a scale degree, `R(i)`; written harmony has bass notes, so
    // the fallback reads the degree nearest bar i's bass note instead, and `hs` pulls its strong beats to the chord.
    let melody = rename_word(&melody, "R", "Lr");
    defs.push(Def::new("Lr", "Lr(i) = floor((7 (Hr(2i) - K) + 6) / 12)".into()));
    defs.push(Def::new("Lm", format!("Lm(k) = {melody}")));
    defs.push(Def::new("L", "L = gl(hs(Lm(floor(2t)), floor(2t), 4), hs(Lm(floor(2t) + 1), floor(2t) + 1, 4), frac(2t), 0.3)".into()));
    let mut block = eq.voices.iter().chain(&eq.stash).find(|b| b.name == "lead")?.clone();
    block.set("pitch", "L");
    block.set("gate", "frac(t / 16) < 0.96");
    let m = book::Module {
        id: format!("lead.grammar-{}", fam.name()),
        slot: Slot::Lead,
        energy: 2,
        density: 2,
        mood: Vec::new(),
        glide: true,
        describe: format!("the grammar's {} generator (the fallback)", fam.name()),
        defs,
        blocks: vec![block],
    };
    let mut e = book::install(eq, &m);
    // Into the lead's register: the median about A5.
    if let Some(med) = book_lead_median(&e) {
        let o = ((9.0 - med) / 12.0).round() as i64 * 12;
        if o != 0 {
            let d = e.def("L")?.clone();
            *e.def_mut("L")? = d.with_rhs(&with_octave(d.rhs(), o));
        }
    }
    Some((e, format!("a new melody from the grammar, {}", fam.name())))
}

/// The lead's median note in a book stream, from its sounding notes.
fn book_lead_median(eq: &Equation) -> Option<f64> {
    let mut e = eq.clone();
    if e.voice("lead").is_none() {
        let b = e.stash.iter().find(|b| b.name == "lead")?.clone();
        e.stash.retain(|b| b.name != "lead");
        e.insert_voice(b);
        book::full(&mut e);
    }
    let song = Song::parse(&e.song_text("median")).ok()?;
    book::line_stats(&song, "lead").map(|s| s.median).filter(|m| m.is_finite())
}

/// The chord roots of bars 0..7, from the equation's `R`.
pub fn roots(eq: &Equation) -> Option<Vec<i64>> {
    let p = program_of(eq)?;
    (0..8).map(|i| p.call("R", &[i as f64]).ok().map(|v| v.round() as i64)).collect()
}

fn book_progression(eq: &Equation, ctx: &Ctx, r: &mut Rng) -> Option<(Equation, String)> {
    // A lens progression must move at least 4 of 8 bars; another species' harmonic source (a ratio set, a map's r)
    // is a different module, and the audibility rule judges the rest.
    let cur = if eq.species.is_empty() { roots(eq) } else { None };
    let mut avoid = ctx.avoid.clone();
    for _ in 0..8 {
        let m = book::pick(book::of(eq), Slot::Progression, ctx.need, book::current(eq, Slot::Progression), &avoid, r)?;
        let e = book::install(eq, m);
        let moved = match (&cur, roots(&e)) {
            (Some(cur), Some(new)) => new.iter().zip(cur).filter(|(a, b)| (*a - *b).rem_euclid(7) != 0).count() >= 4,
            (Some(_), None) => false,
            (None, _) => e != *eq,
        };
        if moved {
            return Some((e, book_words(Slot::Progression, m)));
        }
        avoid.push(m.id.clone());
    }
    None
}

fn book_groove(eq: &Equation, ctx: &Ctx, r: &mut Rng) -> Option<(Equation, String)> {
    let drums = playing_slots(eq, &[Slot::Kick, Slot::Hat, Slot::Clap, Slot::Perc]);
    if drums.is_empty() {
        return None;
    }
    let s = *r.pick(&drums);
    book_swap(eq, s, ctx, r)
}

fn book_swap_any(eq: &Equation, ctx: &Ctx, r: &mut Rng) -> Option<(Equation, String)> {
    let tonal = playing_slots(eq, &[Slot::Bass, Slot::Pads, Slot::Arp, Slot::Counter]);
    if tonal.is_empty() {
        return None;
    }
    // A slot that has sat unchanged goes first, the bass before the others.
    let stale: Vec<Slot> = tonal.iter().copied().filter(|s| ctx.stale.contains(s)).collect();
    let s = if stale.contains(&Slot::Bass) && r.chance(0.8) {
        Slot::Bass
    } else if !stale.is_empty() && r.chance(0.75) {
        *r.pick(&stale)
    } else {
        *r.pick(&tonal)
    };
    book_swap(eq, s, ctx, r)
}

/// The definition the lead's pitch reads (`L` in the lens), when the pitch field is a bare name.
fn lead_line(eq: &Equation) -> Option<String> {
    let b = eq.voices.iter().chain(&eq.stash).find(|b| b.name == "lead")?;
    let p = b.get("pitch")?.trim();
    eq.def(p).map(|d| d.name.clone())
}

/// `rhs` without a trailing octave term (a multiple of 12), and that term.
fn split_twelve(rhs: &str) -> (String, i64) {
    let t = rhs.trim_end();
    for (sign, s) in [(1, " + "), (-1, " - ")] {
        if let Some((head, n)) = t.rsplit_once(s) {
            if let Ok(v) = n.trim().parse::<i64>() {
                if v > 0 && v % 12 == 0 {
                    return (head.to_string(), sign * v);
                }
            }
        }
    }
    (t.to_string(), 0)
}

fn book_register(eq: &Equation, r: &mut Rng) -> Option<(Equation, String)> {
    eq.voice("lead")?;
    if !book::of(eq).species.register {
        return None;
    }
    let line = lead_line(eq)?;
    let d = eq.def(&line)?.clone();
    let med = book_lead_median(eq)?;
    let (core, oct) = split_twelve(d.rhs());
    let up = if med > 14.0 || oct >= 12 {
        false
    } else if med < 2.0 || oct <= -12 {
        true
    } else {
        r.chance(0.5)
    };
    let mut e = eq.clone();
    let no = oct + if up { 12 } else { -12 };
    *e.def_mut(&line)? = d.with_rhs(&with_octave(&core, no));
    Some((e, format!("the melody moves {} an octave", if up { "up" } else { "down" })))
}

/// A seam module into a stage of role `into`.
fn book_fill(eq: &Equation, into: &str, r: &mut Rng) -> Option<(Equation, String)> {
    if book::current(eq, Slot::Seam).is_some() {
        return None;
    }
    let m = book::pick_seam(book::of(eq), into, r)?;
    Some((book::install(eq, m), format!("into the seam, {}", m.id)))
}

/// A book stream's edits per role: v2's arc, with whole modules as the material.
fn book_role_moves(role: Role) -> Vec<(f64, Vec<Move>)> {
    use Move::*;
    match role {
        Role::Intro => vec![
            (2.0, vec![Thin, Melody]),
            (2.0, vec![Thin, Progression]),
            (1.0, vec![Thin, Modulate, Progression]),
            (1.0, vec![Thin, Melody, Mode]),
            (1.0, vec![Thin, Swap, Melody]),
        ],
        Role::Build => vec![
            (3.0, vec![Enter]),
            (2.0, vec![Enter, Groove]),
            (2.0, vec![Enter, Swap]),
            (1.0, vec![Enter, Progression]),
            (1.0, vec![Swap, Progression]),
        ],
        Role::Peak => vec![
            (3.0, vec![Enter, Enter]),
            (1.0, vec![Enter, Progression]),
            (1.0, vec![Enter, Swap]),
            (1.0, vec![Enter, Register]),
            (1.0, vec![Drop]),
        ],
        Role::Strip => vec![(2.0, vec![Breakdown]), (2.0, vec![Breakdown, Swap]), (1.0, vec![Breakdown, Mode])],
        Role::Turn => vec![
            (3.0, vec![Melody]),
            (1.0, vec![Swap, Melody]),
            (1.0, vec![Melody, Enter]),
            (1.0, vec![Progression, Mode]),
            (1.0, vec![Modulate]),
            (1.0, vec![Groove, Swap]),
        ],
        Role::Return => vec![(2.0, vec![Drop]), (2.0, vec![Drop, Progression]), (1.0, vec![Drop, Melody]), (1.0, vec![Drop, Register])],
        Role::Coda => Vec::new(),
    }
}

// ------------------------------------------------------------------------------------------------ the arc

/// A stage's place in its arc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// An arc's first stage: the stream's opening (a low voice and a pulse), or a new idea on a thinned stage.
    Intro,
    /// A layer comes in, with a groove, a progression or a register beside it.
    Build,
    /// The fullest stage: two layers at once, or a layer with a new progression.
    Peak,
    /// The breakdown: two layers.
    Strip,
    /// A new idea while it is bare: a new melody generator, half or double time, a progression, a modulation.
    Turn,
    /// The drop: what left comes back.
    Return,
    /// The ending (a book stream decides when): home, a cadence under a final ritard, a last kick; then it rings out.
    Coda,
}

impl Role {
    pub fn word(self) -> &'static str {
        match self {
            Role::Intro => "intro",
            Role::Build => "build",
            Role::Peak => "peak",
            Role::Strip => "breakdown",
            Role::Turn => "turn",
            Role::Return => "return",
            Role::Coda => "coda",
        }
    }
}

/// The roles of arc `arc`: the opening arc builds in two or three stages (7-8 stages in all), later arcs in one or
/// two (6-7): intro, build..., peak, breakdown, turn, return.
pub fn plan_arc(seed: u64, arc: u64) -> Vec<Role> {
    let mut r = Rng::for_candidate(seed ^ 0x4152_4353, arc); // "ARCS"
    let builds = if arc == 0 { r.range(2, 3) } else { r.range(1, 2) };
    let mut p = vec![Role::Intro];
    p.extend(std::iter::repeat(Role::Build).take(builds as usize));
    p.extend([Role::Peak, Role::Strip, Role::Turn, Role::Return]);
    p
}

/// The edits a role may make, each a list of moves applied in order, with a weight. The first move must apply (a
/// leading `Thin` may not, when the stage is already thin); the others are taken when they apply.
fn role_moves(role: Role, book: bool) -> Vec<(f64, Vec<Move>)> {
    use Move::*;
    if book {
        return book_role_moves(role);
    }
    match role {
        Role::Intro => vec![
            (2.0, vec![Thin, Melody]),
            (1.0, vec![Thin, Modulate, Progression]),
            (1.0, vec![Thin, Progression, Groove]),
            (1.0, vec![Thin, Melody, Mode]),
        ],
        Role::Build => vec![
            (3.0, vec![Enter]),
            (2.0, vec![Enter, Groove]),
            (2.0, vec![Enter, Progression]),
            (1.0, vec![Enter, Register]),
            (1.0, vec![Groove, Progression]),
        ],
        Role::Peak => vec![
            (3.0, vec![Enter, Enter]),
            (2.0, vec![Enter, Progression]),
            (1.0, vec![Enter, Register]),
            (1.0, vec![Enter, Feel]),
            (1.0, vec![Drop]),
        ],
        Role::Strip => vec![(3.0, vec![Breakdown]), (1.0, vec![Breakdown, Mode])],
        Role::Turn => vec![
            (3.0, vec![Melody]),
            (2.0, vec![Feel]),
            (1.0, vec![Melody, Enter]),
            (1.0, vec![Progression, Mode]),
            (1.0, vec![Modulate]),
            (1.0, vec![Feel, Enter]),
            (1.0, vec![Groove, Mode]),
        ],
        Role::Return => vec![(3.0, vec![Drop]), (2.0, vec![Drop, Progression]), (1.0, vec![Drop, Feel]), (1.0, vec![Drop, Register])],
        Role::Coda => Vec::new(),
    }
}

/// Moves tried, in order, on a stage that did not change enough, until it does.
const BOOSTERS: [Move; 6] = [Move::Enter, Move::Melody, Move::Progression, Move::Groove, Move::Leave, Move::Feel];

fn apply_all(base: &Equation, moves: &[Move], r: &mut Rng, ctx: &Ctx) -> Option<Edit> {
    let mut cur: Option<Edit> = None;
    for (i, &m) in moves.iter().enumerate() {
        let from = cur.as_ref().map_or(base, |e| &e.eq);
        match apply_ex(m, from, r, i == 0, ctx) {
            Some(e) => {
                cur = Some(match cur {
                    None => e,
                    Some(prev) => Edit { words: format!("{}; {}", prev.words, e.words), touched: Vec::new(), eq: e.eq },
                })
            }
            None if i == 0 && m != Move::Thin => return None,
            None => {}
        }
    }
    let mut e = cur?;
    e.touched = diff(base, &e.eq);
    Some(e)
}

/// Keep the layers at or under the budget, and the definitions at or under it (or under where the stage started).
pub fn within_budget(eq: &Equation, before: &Equation) -> bool {
    let max_defs = if is_book(eq) { MAX_BOOK_DEFS } else { MAX_DEFS };
    layers(eq).len() <= MAX_LAYERS.max(layers(before).len()) && def_count(eq) <= max_defs.max(def_count(before))
}

/// The opening: one low voice and a pulse (the rest wait to come in). Public for the docs link; `Composer::new`
/// applies it.
pub fn opening(mut eq: Equation) -> Equation {
    for b in std::mem::take(&mut eq.stash) {
        eq.insert_voice(b);
    }
    let here = layers(&eq);
    let low = ["bass", "pads", "drone", "contour", "arp", "harmony"].into_iter().find(|l| here.iter().any(|h| h == l));
    let pulse = ["hat", "arp", "kick", "clap"].into_iter().find(|l| here.iter().any(|h| h == l) && Some(*l) != low);
    let mut keep: Vec<&str> = low.into_iter().chain(pulse).collect();
    if keep.is_empty() {
        keep.push("lead");
    }
    for l in here.iter().filter(|l| !keep.contains(&l.as_str())) {
        layer_out(&mut eq, l);
    }
    if eq.voices.is_empty() {
        // Nothing matched (a seed of other names): play what the seed played.
        let first = eq.stash.remove(0);
        eq.insert_voice(first);
    }
    eq
}

/// Open on the layers of `shape` (a species' opening, [`book::opening_shape`]); the rest wait in the stash. Falls back
/// to [`opening`] when none of them is in the equation.
pub fn opening_as(mut eq: Equation, shape: &[String]) -> Equation {
    for b in std::mem::take(&mut eq.stash) {
        eq.insert_voice(b);
    }
    let here = layers(&eq);
    if !shape.iter().any(|l| here.contains(l)) {
        return opening(eq);
    }
    for l in here.iter().filter(|l| !shape.contains(l)) {
        layer_out(&mut eq, l);
    }
    eq
}

/// A song's key: the tonic in semitones from A4, E to B (-5..2), from its seed. It never changes within a song.
pub fn key_for_seed(seed: u64) -> i64 {
    Rng::for_candidate(seed ^ 0x4B45_59, 0).range(-5, 2) // "KEY"
}

/// Write the key `K` of a harmony species' equation (one written by the progression layer).
pub fn set_key(eq: &mut Equation, k: i64) {
    if let Some(d) = eq.def_mut("K") {
        *d = Def::new("K", format!("K = {k}"));
    }
}

/// The key an equation is in (`K`), if it has one.
pub fn key_of(eq: &Equation) -> Option<i64> {
    eq.def("K").and_then(|d| d.rhs().trim().parse().ok())
}

/// The stage clock of a species whose loops run across seams (`S = k`, read as `t + 32 S`): stage `k`'s value.
pub fn set_clock(eq: &mut Equation, k: u64) {
    if let Some(d) = eq.def_mut("S") {
        *d = Def::new("S", format!("S = {k}"));
    }
}

/// After this many stages on one bass module (while the bass plays), every candidate stage swaps the bass. Since
/// 2026-11-03, when repeated testing kept hearing the same bassline.
pub const BASS_STAYS: usize = 3;

/// `e` with its bass swapped for another module of the book (when the bass plays in it and has not already changed).
fn refresh_bass(e: Edit, base: &Equation, ctx: &Ctx, r: &mut Rng) -> Edit {
    if !layers(&e.eq).iter().any(|l| l == "bass") || book::current(&e.eq, Slot::Bass) != book::current(base, Slot::Bass) {
        return e;
    }
    match book_swap(&e.eq, Slot::Bass, ctx, r) {
        Some((mut eq, words)) => {
            tidy(&mut eq);
            book::finish(&mut eq);
            Edit { touched: diff(base, &eq), words: format!("{}; {words}", e.words), eq }
        }
        None => e,
    }
}

/// The progressions a song may not choose again: once the deceptive turn has played (a progression whose mood says
/// `deceptive`), every deceptive progression of the playing species is spent; it is colour, used once in a song.
pub fn spent_progressions(stages: &[Stage]) -> Vec<String> {
    let deceptive = |m: &book::Module| m.mood.iter().any(|w| w == "deceptive");
    let played = stages.iter().any(|s| book::current(&s.eq, Slot::Progression).and_then(|id| book::of(&s.eq).get(id)).is_some_and(deceptive));
    match stages.last() {
        Some(last) if played => book::of(&last.eq).slot(Slot::Progression).filter(|m| deceptive(m)).map(|m| m.id.clone()).collect(),
        _ => Vec::new(),
    }
}

/// How often a species stream changes species at the start of a later arc.
pub const SPECIES_TURN: f64 = 0.3;

// ------------------------------------------------------------------------------------------------ the chain

/// One stage of the stream.
#[derive(Debug, Clone)]
pub struct Stage {
    /// From 0.
    pub index: u64,
    pub eq: Equation,
    /// The stage as a song file (`STAGE_BEATS` long).
    pub text: String,
    /// Names changed since the stage before (empty for stage 0).
    pub changed: Vec<String>,
    /// What changed, in words.
    pub summary: String,
    /// Not an ordinary build step: an arc's intro, its peak, breakdown, turn or return.
    pub turn: bool,
    /// The critic's score of this stage from its functions (0..1).
    pub score: f64,
    /// Its place in its arc, and which arc (from 0).
    pub role: Role,
    pub arc: u64,
    /// How audible the change from the stage before is ([`audible::change`]); 0 for stage 0.
    pub audibility: f64,
    /// How ready the song is to end after this stage ([`Composer::closure`]), 0..1; 0 for a stream that never ends.
    pub closure: f64,
}

impl Stage {
    /// The equation as display lines for the overlay: a kind (`G` a generator or a stored integer, `V` a line a
    /// voice reads, `L` the lens, `T` a time name), then `1` if it changed at this stage, a tab, and its markup (see
    /// [`display`]).
    pub fn display(&self, before: Option<&Equation>) -> Vec<String> {
        self.eq
            .defs
            .iter()
            .map(|d| {
                let kind = if is_time_name(d) && d.lhs() == d.name {
                    'T'
                } else if LENS_NAMES.contains(&d.name.as_str()) {
                    'L'
                } else if VOICE_LINE_NAMES.contains(&d.name.as_str()) {
                    'V'
                } else {
                    'G'
                };
                let changed = self.changed.contains(&d.name);
                let old = if changed { before.and_then(|b| b.def(&d.name)).map(|o| o.line.as_str()) } else { None };
                format!("{kind}{}\t{}", if changed { '1' } else { '0' }, display::def_markup(&d.line, old))
            })
            .collect()
    }
}

fn is_time_name(d: &Def) -> bool {
    TIME_NAMES.contains(&d.name.as_str())
}

/// The function score without the structure components (they assume a 64-beat song of four sections).
pub fn stage_score(song: &Song) -> f64 {
    let r = critic::score(song);
    let comps: Vec<_> = r.components.iter().filter(|c| !c.name.starts_with("structure") && !c.name.starts_with("sound")).collect();
    let w: f64 = comps.iter().map(|c| c.weight).sum();
    if w <= 0.0 {
        return 0.0;
    }
    comps.iter().map(|c| c.weight * c.score).sum::<f64>() / w
}

/// Where a stream starts.
#[derive(Debug, Clone)]
pub enum Seed {
    /// A song file's text (its first 32 beats, in stream form), evolved by `seed`.
    Song { text: String, seed: u64 },
    /// The best of a few grammar songs from `seed`, evolved by the same seed.
    Grammar(u64),
    /// Modules from the move book (the lens species), chosen and arranged by `seed`.
    Book(u64),
    /// A song species chosen by `seed` ([`book::species_for_seed`]), its modules arranged by the same seed; it may
    /// change species, rarely, at the start of a later arc (the default stream since the species, 2026-11-03).
    Species(u64),
    /// The named species, arranged by `seed`, never changing species.
    Named { species: String, seed: u64 },
    /// A species of strange tunings (behind the Explore option: 19-tone, Bohlen-Pierce, quarter tones, just ratios),
    /// chosen by `seed`; it may change, rarely, to another of them at a later arc.
    Explore(u64),
}

impl Seed {
    /// The kept song G, evolved by seed 1 (the library's "Stream from G").
    pub fn default_song() -> Seed {
        Seed::Song { text: DEFAULT_SEED_SONG.to_string(), seed: 1 }
    }
}

/// A candidate stage: the edit, its song, how audible it is and how the critic scores it.
struct Candidate {
    edit: Edit,
    audibility: f64,
    score: f64,
}

/// The composer of one stream: it writes stage after stage, each from the last, along a plan of arcs.
#[derive(Debug, Clone)]
pub struct Composer {
    seed: u64,
    stages: Vec<Stage>,
    recent: Vec<u64>,
    plan: Vec<Role>,
    /// The index in `plan` of the next stage's role.
    pos: usize,
    arc: u64,
    /// The seed's name, for comments and saved files.
    pub origin: String,
    bpm: f64,
    /// 0 playing; 1 the coda is written (the ring-out comes next); 2 the song is over.
    ending: u8,
    /// The opening's progression and the first lead heard (the home the ending returns to).
    opening_prog: Option<String>,
    first_lead: Option<String>,
    /// This arc's return brings the opening progression and lead back (closure is high enough to head home).
    homing: bool,
    /// The stream may change species at a new arc.
    wander: bool,
    /// The song's instruments, drawn once by its generator version; none for a stream that keeps its written ones.
    instruments: Instruments,
    /// The generator version this stream was composed by ([`GENERATOR_VERSION`] unless asked for an older one).
    version: u32,
    /// The song's way of listening, lead to counter, in a lens stage (version 3 on; docs/dialogue.md).
    dialogue: Option<super::dialogue::Kind>,
}

/// The generators' version: a seed plus its version is a song, forever. Any change to how a stream is composed or
/// dressed that alters what an existing version plays is a NEW version, and the old one is kept frozen beside it
/// (docs/versions.md); `tests/versions.rs` holds version 1 to the keeper's saved seed byte for byte.
/// - 1: the instruments of 2026-11-04 05:45 (`timbre_v1.rs`), the build seed 152637 was heard on.
/// - 2: drawn kits, a drawn rolloff, held-lead filters (`timbre.rs`).
/// - 3: version 2, and in a lens stage the counter listens to the lead (`dialogue.rs`, docs/dialogue.md).
pub const GENERATOR_VERSION: u32 = 3;
/// Every version a stream can be started at.
pub const GENERATOR_VERSIONS: [u32; 3] = [1, 2, 3];

/// A song's drawn instruments, by the generator version that drew them.
#[derive(Debug, Clone)]
enum Instruments {
    None,
    V1(super::timbre_v1::Palette),
    V2(super::timbre::Palette),
}

impl Composer {
    pub fn new(seed: Seed) -> Result<Composer, String> {
        Composer::with_version(seed, GENERATOR_VERSION)
    }

    /// A stream composed by generator `version` (one of [`GENERATOR_VERSIONS`]): the same seed and version always
    /// compose and play the same song.
    pub fn with_version(seed: Seed, version: u32) -> Result<Composer, String> {
        if !GENERATOR_VERSIONS.contains(&version) {
            return Err(format!("no generator version {version}; this build has {GENERATOR_VERSIONS:?}"));
        }
        let (text, s, origin) = match seed.clone() {
            Seed::Song { text, seed } => {
                let first = text.lines().next().unwrap_or("").trim_start_matches('#').trim().to_string();
                (text, seed, if first.is_empty() { "a song".to_string() } else { first })
            }
            Seed::Grammar(seed) => {
                let mut best: Option<(f64, String)> = None;
                for i in 0..6 {
                    let d = grammar::compose(seed, i)?;
                    let song = Song::parse(&d.text).map_err(|e| e.to_string())?;
                    let sc = critic::score(&song).function_score();
                    if best.as_ref().map_or(true, |b| sc > b.0) {
                        best = Some((sc, d.text));
                    }
                }
                (best.expect("six candidates").1, seed, format!("grammar seed {seed}"))
            }
            Seed::Book(seed) => (String::new(), seed, format!("the move book, seed {seed}")),
            Seed::Species(seed) | Seed::Explore(seed) => (String::new(), seed, String::new()),
            Seed::Named { species, seed } => {
                book::named(&species).ok_or_else(|| format!("no species '{species}'"))?;
                (String::new(), seed, String::new())
            }
        };
        let (species, wander) = match &seed {
            Seed::Book(_) => (Some(book::book()), false),
            Seed::Species(_) => (Some(book::species_for_seed(s)), true),
            Seed::Explore(_) => (Some(book::explore_for_seed(s)), true),
            Seed::Named { species, .. } => (book::named(species), false),
            _ => (None, false),
        };
        let origin = match (&seed, species) {
            (Seed::Species(_) | Seed::Named { .. } | Seed::Explore(_), Some(b)) => format!("the {} species, seed {s}", b.species.name),
            _ => origin,
        };
        let mut eq = match species {
            Some(b) => opening_as(book::assemble_in(b, s, None), &book::opening_shape(b, s)),
            None => opening(normalise(parse_equation(&text)?)),
        };
        tidy(&mut eq);
        if is_book(&eq) {
            book::finish(&mut eq);
        }
        if is_book(&eq) && book::of(&eq).species.harmony {
            set_key(&mut eq, key_for_seed(s));
        }
        // A species whose frame holds a motif number (`Mn`, the phrase species) gets its song's motif, drawn once like the
        // key. No other species has `Mn`, so nothing else changes.
        if let Some(d) = eq.def_mut("Mn") {
            *d = Def::new("Mn", format!("Mn = {}", Rng::for_candidate(s ^ 0x4D4F_5449_46, 0).range(0, 63))); // "MOTIF"
        }
        let plan = plan_arc(s, 0);
        let bpm = eq.settings.iter().find_map(|l| l.strip_prefix("bpm ").and_then(|v| v.trim().parse::<f64>().ok())).unwrap_or(100.0);
        let mut c = Composer {
            seed: s,
            stages: Vec::new(),
            recent: Vec::new(),
            plan,
            pos: 1,
            arc: 0,
            origin,
            bpm,
            ending: 0,
            opening_prog: None,
            first_lead: None,
            homing: false,
            wander,
            instruments: Instruments::None,
            version,
            dialogue: if version >= 3 { Some(super::dialogue::draw(s)) } else { None },
        };
        set_clock(&mut eq, 0);
        let summary = format!("the seed opens with {}", list(&layers(&eq)));
        let text = eq.song_text(&c.comment(0, &summary));
        let song = Song::parse(&text).map_err(|e| format!("stage 1 does not parse: {e}\n{text}"))?;
        let score = stage_score(&song);
        c.recent.push(eq.state_hash());
        c.opening_prog = book::current(&eq, Slot::Progression).map(str::to_string);
        c.stages.push(Stage { index: 0, eq, text, changed: Vec::new(), summary, turn: true, score, role: Role::Intro, arc: 0, audibility: 0.0, closure: 0.0 });
        c.stages[0].closure = c.closure();
        if c.dresses(&c.stages[0].eq) {
            let full = c.roster_eq();
            c.instruments = match version {
                1 => Instruments::V1(super::timbre_v1::choose(s, &full).0),
                _ => Instruments::V2(super::timbre::choose(s, &full).0),
            };
            let text = c.dressed(&c.stages[0].eq).song_text(&c.comment(0, &c.stages[0].summary));
            c.stages[0].text = text;
        }
        Ok(c)
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    fn comment(&self, index: u64, summary: &str) -> String {
        format!("Dreamride stream (from {}), stage {}: {summary}", self.origin, index + 1)
    }

    pub fn stages(&self) -> &[Stage] {
        &self.stages
    }

    pub fn stage(&self, index: u64) -> Option<&Stage> {
        self.stages.get(index as usize)
    }

    /// Every voice name the stream can have, in its fixed order.
    pub fn roster(&self) -> &[String] {
        &self.stages[0].eq.roster
    }

    /// A song holding every roster voice (each as stage 1 or its stash has it), for the voices' fixed settings.
    pub fn roster_song_text(&self) -> String {
        self.dressed(&self.roster_eq()).song_text("roster")
    }

    /// Whether a stage of this equation wears the song's drawn instruments: a book species, never an Explore tuning
    /// (their sound is part of the tuning's idea).
    fn dresses(&self, eq: &Equation) -> bool {
        is_book(eq) && !book::of(eq).species.explore
    }

    /// `eq` in the song's instruments, when it has them and the species takes them; else `eq` as it is.
    pub fn dressed(&self, eq: &Equation) -> Equation {
        if !self.dresses(eq) {
            return eq.clone();
        }
        match &self.dialogue {
            Some(k) if book::of(eq).species.name == "lens" => self.instrumented(&super::dialogue::apply(eq, k)),
            _ => self.instrumented(eq),
        }
    }

    /// `eq` in the song's drawn instruments only (no listening between voices).
    fn instrumented(&self, eq: &Equation) -> Equation {
        match &self.instruments {
            Instruments::V1(p) => super::timbre_v1::dress(eq, p),
            Instruments::V2(p) => super::timbre::dress(eq, p),
            Instruments::None => eq.clone(),
        }
    }

    /// The text the composer judges a stage by: what it plays, except that from version 3 the voices' listening
    /// (dialogue.rs) is left out, so that a seed composes the same moves at version 3 as at version 2 and only the
    /// counter's line differs. Versions 1 and 2 read the played text, as they always did.
    pub fn judged_text(&self, st: &Stage) -> String {
        match self.dialogue {
            None => st.text.clone(),
            Some(_) if !self.dresses(&st.eq) => st.text.clone(),
            Some(_) => self.instrumented(&st.eq).song_text(&self.comment(st.index, &st.summary)),
        }
    }

    /// The generator version this stream was composed by.
    pub fn version(&self) -> u32 {
        self.version
    }

    /// An equation holding every roster voice (each as stage 1 or its stash has it), in its written instruments.
    pub fn roster_eq(&self) -> Equation {
        let mut eq = self.stages[0].eq.clone();
        for b in eq.stash.clone() {
            eq.insert_voice(b);
        }
        if eq.voice(FILL_VOICE).is_none() {
            eq.defs.push(Def::new(FILL_DEF, format!("{FILL_DEF} = (t >= 30)(0.5 - frac(4t))")));
            eq.insert_voice(fill_block());
        }
        if is_book(&eq) {
            book::full(&mut eq);
            // The seam voices come only with a seam module, for one stage: the roster holds them all the same, so the
            // window (every roster voice) needs one of each (the book's first; a seam's own settings play it).
            // Every roster voice (a species without an arpeggio, a bass or pads still has them in its roster).
            for name in book::ROSTER {
                if eq.voice(name).is_none() {
                    if let Some(m) = book::of(&eq).slot(Slot::Seam).find(|m| m.blocks.iter().any(|b| b.name == name)) {
                        for d in &m.defs {
                            if eq.def(&d.name).is_none() {
                                eq.defs.push(d.clone());
                            }
                        }
                        eq.insert_voice(m.blocks.iter().find(|b| b.name == name).expect("found above").clone());
                    } else {
                        // A species without this voice: a silent one of the roster's kind holds its place.
                        let kind = if book::ROSTER_DRUMS.contains(&name) { "drum" } else { "voice" };
                        let fields: Vec<(String, String)> = if kind == "drum" {
                            vec![("kind".into(), "dash".into()), ("trigger".into(), "0".into()), ("from".into(), "300".into()), ("to".into(), "100".into())]
                        } else {
                            vec![("wave".into(), "sine".into()), ("pitch".into(), "0".into()), ("gate".into(), "0".into())]
                        };
                        eq.insert_voice(Block { kind: kind.into(), name: name.into(), fields });
                    }
                }
            }
        }
        eq.stash.clear();
        eq
    }

    /// The role the next stage takes, and the one after it (a new arc is planned when one ends).
    fn next_role(&mut self) -> (Role, u64, Role) {
        if self.pos >= self.plan.len() {
            self.arc += 1;
            self.plan = plan_arc(self.seed, self.arc);
            self.pos = 0;
        }
        let role = self.plan[self.pos];
        self.pos += 1;
        let after = self.plan.get(self.pos).copied().unwrap_or(Role::Intro);
        (role, self.arc, after)
    }

    /// How many stages in a row, back from the last, the bass has played the same module (stages without it skipped).
    fn bass_run(&self) -> usize {
        let mut run = 0;
        let mut id: Option<&str> = None;
        for s in self.stages.iter().rev() {
            if !layers(&s.eq).iter().any(|l| l == "bass") {
                continue;
            }
            let cur = book::current(&s.eq, Slot::Bass);
            if id.is_none() {
                id = cur;
            } else if cur != id {
                break;
            }
            run += 1;
        }
        run
    }

    /// The tonal slots whose module has played unchanged through the last three stages.
    fn stale_slots(&self) -> Vec<Slot> {
        let st = &self.stages;
        if st.len() < 3 {
            return Vec::new();
        }
        let last = &st[st.len() - 3..];
        [Slot::Bass, Slot::Pads, Slot::Arp, Slot::Counter]
            .into_iter()
            .filter(|&sl| {
                let ids: Vec<Option<&str>> = last.iter().map(|s| book::current(&s.eq, sl)).collect();
                ids[0].is_some() && ids.iter().all(|x| *x == ids[0]) && last.iter().all(|s| layers(&s.eq).iter().any(|l| l == sl.name()))
            })
            .collect()
    }

    /// At the start of a later arc, rarely, a species stream changes species: a compatible one (the stream's tempo
    /// inside its range), opened as its own first stage would be. None when it stays.
    fn species_turn(&self, base: &Equation) -> Option<(Equation, String)> {
        if !self.wander || self.arc == 0 {
            return None;
        }
        let mut r = Rng::for_candidate(self.seed ^ 0x5455_524E, self.arc); // "TURN"
        if !r.chance(SPECIES_TURN) {
            return None;
        }
        let bpm = self.bpm.round() as i64;
        let here = book::of(base).species.name.clone();
        // A species fits the stream's tempo when it lies within an eighth of its range (the tempo never changes).
        let explore = book::of(base).species.explore;
        let fits: Vec<&'static book::Book> = book::all()
            .iter()
            .filter(|b| b.species.name != here && b.species.explore == explore && b.species.drawn)
            .filter(|b| bpm as f64 >= b.species.bpm.0 as f64 * 0.88 && bpm as f64 <= b.species.bpm.1 as f64 * 1.12)
            .collect();
        if fits.is_empty() {
            return None;
        }
        let w: Vec<f64> = fits.iter().map(|b| b.species.weight).collect();
        let b = fits[r.weighted(&w)];
        let seed = self.seed ^ (self.arc << 32);
        let mut eq = opening_as(book::assemble_in(b, seed, Some(bpm)), &book::opening_shape(b, seed));
        tidy(&mut eq);
        book::finish(&mut eq);
        // The song keeps its key across a change of species.
        set_key(&mut eq, key_of(base).unwrap_or_else(|| key_for_seed(self.seed)));
        Some((eq, format!("a new species, {}: {}", b.species.name, b.species.title)))
    }

    /// The book modules heard in the last four stages (a stream does not go straight back to one).
    fn recent_modules(&self) -> Vec<String> {
        let mut v: Vec<String> = self.stages.iter().rev().take(4).flat_map(|s| s.eq.book.iter().map(|(_, id)| id.clone())).collect();
        v.sort();
        v.dedup();
        v
    }

    fn judge(&self, base: &Equation, edit: Edit, prev: &audible::Profile) -> Option<Candidate> {
        if !within_budget(&edit.eq, base) || self.recent.contains(&edit.eq.state_hash()) {
            return None;
        }
        let song = Song::parse(&edit.eq.song_text("candidate")).ok()?;
        let audibility = audible::change(prev, &audible::profile(&song)).total();
        Some(Candidate { edit, audibility, score: stage_score(&song) })
    }

    /// Write the next stage and return it. Once a book stream has ended, it returns its last stage.
    pub fn next(&mut self) -> &Stage {
        let k = self.stages.len() as u64;
        match self.ending {
            2 => return self.stages.last().expect("stage 0 exists"),
            1 => return self.push_ring(k),
            _ => {}
        }
        if is_book(&self.stages[0].eq) && self.should_end(k) {
            return self.push_coda(k);
        }
        let prev = self.stages.last().expect("stage 0 exists").clone();
        let mut r = Rng::for_candidate(self.seed, k);
        let starts_arc = self.pos >= self.plan.len();
        let (role, arc, after) = self.next_role();
        if starts_arc {
            self.homing = is_book(&prev.eq) && prev.closure >= HOMING_AT;
        }
        let mut base = prev.eq.clone();
        remove_fill(&mut base);
        let prev_song = Song::parse(&self.judged_text(&prev)).expect("every stage parses");
        let prev_prof = audible::profile(&prev_song);

        let bk = is_book(&base);
        let options = role_moves(role, bk);
        let weights: Vec<f64> = options.iter().map(|o| o.0).collect();
        let mut avoid = self.recent_modules();
        avoid.extend(spent_progressions(&self.stages));
        let ctx = Ctx { need: need_for(role), avoid, stale: self.stale_slots() };
        let mut cands: Vec<Candidate> = Vec::new();
        let turned = if starts_arc && bk { self.species_turn(&base) } else { None };
        if let Some((eq, words)) = &turned {
            let edit = Edit { touched: diff(&base, eq), eq: eq.clone(), words: words.clone() };
            let song = Song::parse(&edit.eq.song_text("candidate"));
            if let Ok(song) = song {
                let audibility = audible::change(&prev_prof, &audible::profile(&song)).total();
                cands.push(Candidate { edit, audibility, score: stage_score(&song) });
            }
        }
        // A bass that has played one module through BASS_STAYS stages takes another in every candidate.
        let bass_due = bk && self.bass_run() >= BASS_STAYS;
        for _ in 0..if turned.is_some() { 0 } else { CANDIDATES } {
            let moves = &options[r.weighted(&weights)].1;
            let edit = apply_all(&base, moves, &mut r, &ctx).map(|e| if bass_due { refresh_bass(e, &base, &ctx, &mut r) } else { e });
            if let Some(c) = edit.and_then(|e| self.judge(&base, e, &prev_prof)) {
                cands.push(c);
            }
        }
        // Too quiet a change: add to the loudest candidates, then try each strong move alone.
        if !cands.iter().any(|c| c.audibility >= audible::THRESHOLD) {
            let mut seeds: Vec<Edit> = {
                let mut v: Vec<&Candidate> = cands.iter().collect();
                v.sort_by(|a, b| b.audibility.partial_cmp(&a.audibility).expect("finite"));
                v.into_iter().take(3).map(|c| c.edit.clone()).collect()
            };
            seeds.push(Edit { eq: base.clone(), touched: Vec::new(), words: String::new() });
            'boost: for s in seeds {
                for m in BOOSTERS {
                    let Some(e) = apply_with(m, &s.eq, &mut r, &ctx) else { continue };
                    let words = if s.words.is_empty() { e.words.clone() } else { format!("{}; {}", s.words, e.words) };
                    let edit = Edit { touched: diff(&base, &e.eq), eq: e.eq, words };
                    // A due bass is swapped here too: a boosted base would otherwise carry the old bass past its stay.
                    let edit = if bass_due { refresh_bass(edit, &base, &ctx, &mut r) } else { edit };
                    if let Some(c) = self.judge(&base, edit, &prev_prof) {
                        let loud = c.audibility >= audible::THRESHOLD;
                        cands.push(c);
                        if loud {
                            break 'boost;
                        }
                    }
                }
            }
        }
        let loud = cands.iter().any(|c| c.audibility >= audible::THRESHOLD);
        let best = cands
            .into_iter()
            .filter(|c| !loud || c.audibility >= audible::THRESHOLD)
            .fold(None::<Candidate>, |b, c| match b {
                Some(b) if (loud && b.score >= c.score) || (!loud && b.audibility >= c.audibility) => Some(b),
                _ => Some(c),
            });
        let (mut eq, mut summary, mut audibility, mut score) = match best {
            Some(c) => (c.edit.eq, c.edit.words, c.audibility, c.score),
            None => {
                let song = Song::parse(&base.song_text("fallback")).ok();
                (base.clone(), "plays on".to_string(), 0.0, song.as_ref().map_or(0.0, stage_score))
            }
        };
        // Heading home: the return of an arc that starts with the song ready to close brings back the opening's
        // progression and its first lead, the bookend the ending will reprise (kept only if still audible).
        if bk && self.homing && role == Role::Return {
            if let Some((home, words)) = self.home(&eq) {
                if within_budget(&home, &base) {
                    if let Ok(song) = Song::parse(&home.song_text("home")) {
                        let a = audible::change(&prev_prof, &audible::profile(&song)).total();
                        if a >= audible::THRESHOLD {
                            eq = home;
                            audibility = a;
                            score = stage_score(&song);
                            summary = format!("{summary}; {words}");
                        }
                    }
                }
            }
        }
        // Into a drop, a roll. A book stream also marks the seams into a breakdown and into the next arc, and half of
        // those into a peak, each with a seam module chosen for where it leads.
        if bk {
            let into_next_arc = self.pos >= self.plan.len();
            let seam = match after {
                Role::Return | Role::Strip => true,
                Role::Peak => r.chance(0.5),
                _ => into_next_arc,
            };
            let into = if into_next_arc { "intro" } else { after.word() };
            if seam {
                if let Some((e, w)) = book_fill(&eq, into, &mut r) {
                    eq = e;
                    summary = format!("{summary}; {w}");
                }
            }
        } else if after == Role::Return {
            if let Some((e, w)) = fill_move(&eq) {
                eq = e;
                summary = format!("{summary}; {w}");
            }
        }
        if turned.is_some() && eq.species != base.species {
            // A new species: its home is its own opening.
            self.opening_prog = book::current(&eq, Slot::Progression).map(str::to_string);
            self.first_lead = None;
        }
        set_clock(&mut eq, k);
        let summary = format!("{}: {summary}", role.word());
        let changed = diff(&prev.eq, &eq);
        let text = self.dressed(&eq).song_text(&self.comment(k, &summary));
        self.recent.push(eq.state_hash());
        if self.recent.len() > RECENT {
            self.recent.remove(0);
        }
        if self.first_lead.is_none() && eq.voice("lead").is_some() {
            self.first_lead = book::current(&eq, Slot::Lead).map(str::to_string);
        }
        self.push(Stage { index: k, eq, text, changed, summary, turn: role != Role::Build, score, role, arc, audibility, closure: 0.0 })
    }

    /// Add a stage, with its closure.
    fn push(&mut self, st: Stage) -> &Stage {
        self.stages.push(st);
        let c = self.closure();
        let last = self.stages.last_mut().expect("just pushed");
        last.closure = c;
        last
    }

    /// Make sure stages 0..=index exist (or as many as the song has, once it has ended).
    pub fn ensure(&mut self, index: u64) {
        while (self.stages.len() as u64) <= index && self.ending != 2 {
            self.next();
        }
    }
}

// ------------------------------------------------------------------------------------------------ the ending

/// A book stream never ends before this many seconds (with its coda): at least one whole arc and then some, a little
/// under the 3-4 minutes of a pop song, so it never stops before it has gone somewhere and come back.
pub const MIN_SONG_SECS: f64 = 150.0;
/// And always by this many seconds, its ring-out included: the length of an extended city-pop mix or a long
/// electronic track. By then about four arcs have used most of the book (each arc brings eight to twelve modules into
/// play, of 55), so novelty is spent anyway.
pub const MAX_SONG_SECS: f64 = 420.0;
/// Time kept for the ring-out after the coda when checking the upper bound.
pub const RING_RESERVE_SECS: f64 = 12.0;
/// The closure at which a song that has finished an arc ends.
pub const CLOSURE_THRESHOLD: f64 = 0.7;
/// The closure at which the next arc's return heads home (the opening's progression and first lead come back).
pub const HOMING_AT: f64 = 0.5;

/// The ending's clock: the coda's score time `tr` against the stage's own `t`. Score beats 0-16 run in time; beats
/// 16-24 slow by the Friberg-Sundberg final ritard (tempo (1 + (w^q - 1) x)^(1/q) at position x of the ritard, q = 3,
/// the runners' braking, falling to w = 1/2), integrated and inverted in closed form so the score's last two bars take
/// 10.29 beats of the clock; then it runs on at half speed while everything rings.
pub const RITARD_TIME: &str =
    "tr = if(t < 16, t, if(t < 26.2857, 16 + 9.142857 (1 - (1 - 0.0729167 (min(t, 26.2857) - 16))^1.5), 24 + 0.5 (t - 26.2857)))";

/// Rename the time `t` to `tr` in an expression (by tokens, so `4t` becomes `4tr`, a product).
fn retime(expr: &str) -> String {
    let mut out = String::new();
    let mut at = 0;
    for tk in display::tokens(expr) {
        if let display::Tok::Ident(n) = &tk.tok {
            if n == "t" {
                out += &expr[at..tk.start];
                out += "tr";
                at = tk.end;
            }
        }
    }
    out += &expr[at..];
    out
}

/// The reverse of [`retime`]: the ending's clock `tr` back to `t`.
fn untime(expr: &str) -> String {
    let mut out = String::new();
    let mut at = 0;
    for tk in display::tokens(expr) {
        if let display::Tok::Ident(n) = &tk.tok {
            if n == "tr" {
                out += &expr[at..tk.start];
                out += "t";
                at = tk.end;
            }
        }
    }
    out += &expr[at..];
    out
}

/// What makes up a song's readiness to end, each 0..1.
#[derive(Debug, Clone, Copy, Default)]
pub struct Closure {
    /// Arcs completed (returns heard), of two: three arcs and a coda do not fit in [`MAX_SONG_SECS`] at 92-106 bpm
    /// (about 20 stages of 18-21 s), so a song that needed three to feel finished would always be cut by the clock.
    pub arcs: f64,
    /// Book modules heard, of half the book: the novelty spent.
    pub novelty: f64,
    /// How far the changes have tired: the last arc's mean audibility against the liveliest arc's.
    pub fatigue: f64,
    /// Home re-established: the opening's progression is playing again.
    pub home: f64,
    /// Full peaks reached (five or more layers), of two.
    pub energy: f64,
}

impl Closure {
    pub fn score(&self) -> f64 {
        0.3 * self.arcs + 0.25 * self.novelty + 0.15 * self.fatigue + 0.15 * self.home + 0.15 * self.energy
    }
}

impl Composer {
    /// Whether the song has ended (its ring-out is composed).
    pub fn ended(&self) -> bool {
        self.ending == 2
    }

    /// Seconds of one stage.
    pub fn stage_secs(&self) -> f64 {
        STAGE_BEATS * 60.0 / self.bpm
    }

    /// The song's readiness to end after its last stage, by part.
    pub fn closure_parts(&self) -> Closure {
        let st = &self.stages;
        if !is_book(&st[0].eq) {
            return Closure::default();
        }
        let returns = st.iter().filter(|s| s.role == Role::Return).count();
        // Novelty counts families, not modules: a slot's generated grooves are takes of one idea (groove::family), so
        // 24 of them do not make the book 24 ideas bigger (2026-11-04, when the lens drums became generated).
        let heard: std::collections::BTreeSet<String> = st
            .iter()
            .flat_map(|s| s.eq.book.iter().filter(|(sl, _)| sl != "seam").map(|(_, id)| super::groove::family(id)))
            .collect();
        let families: std::collections::BTreeSet<String> = book::of(&st[st.len() - 1].eq)
            .modules
            .iter()
            .filter(|m| m.slot != Slot::Seam)
            .map(|m| super::groove::family(&m.id))
            .collect();
        let total = families.len() as f64;
        let mut arcs: Vec<(u64, f64, f64)> = Vec::new();
        for s in st.iter().skip(1) {
            match arcs.last_mut() {
                Some(a) if a.0 == s.arc => {
                    a.1 += s.audibility;
                    a.2 += 1.0;
                }
                _ => arcs.push((s.arc, s.audibility, 1.0)),
            }
        }
        let means: Vec<f64> = arcs.iter().map(|a| a.1 / a.2).collect();
        let fatigue = match (means.last(), means.iter().cloned().fold(f64::NAN, f64::max)) {
            (Some(&last), max) if means.len() >= 2 && max > 0.0 => (1.5 - last / max).clamp(0.0, 1.0),
            _ => 0.5,
        };
        let cur = book::current(&st[st.len() - 1].eq, Slot::Progression);
        let home = if cur.is_some() && cur == self.opening_prog.as_deref() && st.len() > 1 { 1.0 } else { 0.0 };
        let peaks = st.iter().filter(|s| s.role == Role::Peak && layers(&s.eq).len() >= 5).count();
        Closure {
            arcs: (returns as f64 / 2.0).min(1.0),
            novelty: (heard.len() as f64 / (0.5 * total)).min(1.0),
            fatigue,
            home,
            energy: (peaks as f64 / 2.0).min(1.0),
        }
    }

    /// The song's readiness to end after its last stage, 0..1 (see [`Closure`]).
    pub fn closure(&self) -> f64 {
        self.closure_parts().score()
    }

    /// Whether stage `k` is the coda: at the end of an arc, once the song is long enough and has said what it has to
    /// say (or the next arc would not fit); at any stage, if one more would break the upper bound.
    fn should_end(&self, k: u64) -> bool {
        let t = self.stage_secs();
        let ends = |stages: u64| stages as f64 * t + RING_RESERVE_SECS;
        if ends(k + 2) > MAX_SONG_SECS {
            return true; // a coda one stage later would end past the bound
        }
        if self.pos < self.plan.len() || ((k + 1) as f64 * t) < MIN_SONG_SECS {
            return false;
        }
        let closure = self.stages.last().map_or(0.0, |s| s.closure);
        // By its own closure only after two arcs: a species with a small book spends its novelty in one, and a song
        // that has gone somewhere and come back once has not yet said it twice.
        let returns = self.stages.iter().filter(|s| s.role == Role::Return).count();
        (closure >= CLOSURE_THRESHOLD && returns >= 2) || ends(k + 1 + 6) > MAX_SONG_SECS
    }

    /// `eq` with the opening's progression and first lead back, and what that says; None if both are already there.
    fn home(&self, eq: &Equation) -> Option<(Equation, String)> {
        let mut home = eq.clone();
        let mut words = Vec::new();
        let b = book::of(eq);
        if let Some(p) = self.opening_prog.as_deref().and_then(|id| b.get(id)) {
            if book::current(&home, Slot::Progression) != Some(p.id.as_str()) {
                home = book::install(&home, p);
                words.push(format!("home to {}", p.id));
            }
        }
        if let Some(l) = self.first_lead.as_deref().and_then(|id| b.get(id)) {
            if home.voice("lead").is_some() && book::current(&home, Slot::Lead) != Some(l.id.as_str()) {
                home = book::install(&home, l);
                words.push(format!("the first melody, {}, again", l.id));
            }
        }
        if words.is_empty() {
            None
        } else {
            Some((home, words.join(", ")))
        }
    }

    /// The coda: the opening's progression for four bars, then IV V into the tonic, under the final ritard; the first
    /// lead an octave down over the bass, the pads and the kick; one last kick dash on the tonic, and everything closes
    /// a bar and a half later and rings into the reverb.
    fn push_coda(&mut self, k: u64) -> &Stage {
        let prev = self.stages.last().expect("stage 0 exists").clone();
        let mut e = prev.eq.clone();
        remove_fill(&mut e);
        // The ending's texture: bass, pads, lead and kick.
        let keep = ["bass", "pads", "lead", "kick"];
        for l in layers(&e) {
            if !keep.contains(&l.as_str()) {
                layer_out(&mut e, &l);
            }
        }
        for l in keep {
            if !layers(&e).iter().any(|x| x == l) {
                layer_in(&mut e, l);
            }
        }
        let mut words = Vec::new();
        if let Some((h, w)) = self.home(&e) {
            e = h;
            words.push(w);
        }
        // The last chord is a held, struck triad (in the lens; another species ends in its own sound).
        let lens = e.species.is_empty();
        if let Some(p) = book::of(&e).get("pads.struck") {
            if book::current(&e, Slot::Pads) != Some(p.id.as_str()) {
                e = book::install(&e, p);
            }
        }
        book::finish(&mut e);
        // The cadence (a harmony species): the progression playing (home, by now) for four bars, then IV, V7 and the
        // tonic for the last two (`harmony::coda`), in the song's own key.
        if book::of(&e).species.harmony {
            let playing = book::current(&e, Slot::Progression).and_then(|id| id.rsplit("prog.").next()).and_then(|n| harmony::all().iter().find(|p| p.name == n));
            if let Some(p) = playing {
                let prefix = if e.species.is_empty() { String::new() } else { format!("{}/", e.species) };
                e = book::install(&e, &harmony::module(&harmony::coda(p), &prefix));
            }
        }
        // The lead an octave down (when it sits high enough to go).
        if let (true, Some(med), Some(d)) = (lens, book_lead_median(&e), e.def("L").cloned()) {
            if med > 6.0 {
                let (core, oct) = split_twelve(d.rhs());
                *e.def_mut("L").expect("L") = d.with_rhs(&with_octave(&core, oct - 12));
                words.push("the lead an octave down".into());
            }
        }
        // The song's home key (a modulation at a turn may have moved it), and nothing unread; `finish` keeps the
        // progression's tables, which the frame's lens reads even where no voice does.
        if let Some(home) = key_of(&self.stages[0].eq) {
            set_key(&mut e, home);
        }
        book::finish(&mut e);
        // Every function runs on the ending's clock.
        for d in e.defs.iter_mut() {
            let lhs = d.lhs().to_string();
            d.line = format!("{lhs} = {}", retime(d.rhs()));
        }
        e.defs.insert(0, Def::new("tr", RITARD_TIME.to_string()));
        for b in e.voices.iter_mut() {
            for key in ["pitch", "gate", "amp", "trigger", "cutoff"] {
                if let Some(v) = b.get(key).map(str::to_string) {
                    b.set(key, &retime(&v));
                }
            }
            if b.is_drum() {
                let trig = b.get("trigger").unwrap_or("0").to_string();
                let last = if b.name == "kick" { "(tr >= 24)(tr < 24.2)" } else { "0" };
                b.set("trigger", &format!("if(tr < 23.75, {trig}, {last})"));
            } else {
                let gate = b.get("gate").unwrap_or("1").to_string();
                b.set("gate", &format!("if(tr < 24, {gate}, tr < 25.5)"));
                let pitch = b.get("pitch").unwrap_or("0").to_string();
                match b.name.as_str() {
                    "bass" if lens => b.set("pitch", &format!("if(tr < 24, {pitch}, lo(h(0)))")),
                    // The lead's last note: the tonic in the octave its line was in.
                    "lead" if lens => b.set("pitch", &format!("if(tr < 24, {pitch}, K + 12 floor(({pitch} - K) / 12 + 0.04))")),
                    _ => {}
                }
            }
        }
        book::finish(&mut e);
        let mut summary = "coda: ".to_string();
        if !words.is_empty() {
            summary += &format!("{}; ", words.join("; "));
        }
        summary += if lens {
            "the opening's bars, then IV V7 I under a final ritard, a last kick on the tonic, and it rings out"
        } else if book::of(&e).species.harmony {
            "its opening again, then IV V7 I under a final ritard, a last kick, and it rings out"
        } else {
            "its opening again under a final ritard, a last kick, and it rings out"
        };
        self.finish_stage(k, prev, e, summary)
    }

    /// The ring-out: every voice closed, its pitch held where the coda left it, while releases and reverb decay.
    fn push_ring(&mut self, k: u64) -> &Stage {
        let prev = self.stages.last().expect("the coda").clone();
        let song = Song::parse(&prev.text).expect("the coda parses");
        let mut e = prev.eq.clone();
        let t_end = STAGE_BEATS - 1e-3;
        for b in e.voices.iter_mut() {
            let v = song.voices.iter().find(|v| v.name == b.name).expect("the coda's voice");
            match &v.kind {
                crate::song::VoiceKind::Tonal(tn) => {
                    let p = tn.pitch.eval(&song.program, t_end);
                    b.set("pitch", &grammar::num(if p.is_finite() { p } else { 0.0 }));
                    b.set("gate", "0");
                    if let Some(a) = &tn.amp {
                        let a = a.eval(&song.program, t_end);
                        b.set("amp", &grammar::num(if a.is_finite() { a } else { 1.0 }));
                    }
                }
                crate::song::VoiceKind::Drum(_) => {
                    b.set("trigger", "0");
                    if b.get("amp").is_some() {
                        b.set("amp", "1");
                    }
                }
            }
        }
        e.defs.retain(|d| d.name != "tr");
        // What stays (a species' frame, its stream clock `w = tr + 32 S`) goes back to the stage's own time.
        for d in e.defs.iter_mut() {
            let lhs = d.lhs().to_string();
            d.line = format!("{lhs} = {}", untime(d.rhs()));
        }
        book::finish(&mut e);
        self.finish_stage(k, prev, e, "coda: it rings out".into())
    }

    fn finish_stage(&mut self, k: u64, prev: Stage, mut eq: Equation, summary: String) -> &Stage {
        set_clock(&mut eq, k);
        let text = self.dressed(&eq).song_text(&self.comment(k, &summary));
        let song = Song::parse(&text).unwrap_or_else(|e| panic!("the ending does not parse: {e}\n{text}"));
        let prev_song = Song::parse(&self.judged_text(&prev)).expect("every stage parses");
        let audibility = audible::change(&audible::profile(&prev_song), &audible::profile(&song)).total();
        let score = stage_score(&song);
        let changed = diff(&prev.eq, &eq);
        self.ending += 1;
        let arc = self.arc;
        self.push(Stage { index: k, eq, text, changed, summary, turn: true, score, role: Role::Coda, arc, audibility, closure: 0.0 })
    }
}
