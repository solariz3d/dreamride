//! The move book: hand-written, graph-native modules the stream arranges (the text is `book.txt`, beside this file).
//!
//! Listening to stream v2 (2026-11-03): the kick was good and the way it changes intelligent, but the instruments and
//! melodies lost the feel and were still in the style of MIDI rather than abstract graph music like the reference video;
//! the content itself was flat. The design settled: **the brain writes the material, the algorithm only arranges it.** v2's
//! arc planner and audibility rule stay; the content they arrange now comes from this book, written by judgment, instead
//! of being sampled from the grammar's generators (which remain a fallback for the lead).
//!
//! A module fills one **slot** (a progression, the lead, a counter-line, the arpeggio, the bass, the pads, a drum, or a
//! seam), with a one-line description of its shape and tags (energy and density 1-3, mood words, and whether it glides)
//! the stream uses to choose by need: an intro wants low energy, a peak density, a breakdown space. The contract that
//! lets any module sit on any progression is stated at the top of `book.txt`.

use super::critic;
use super::rng::Rng;
use super::stream::{tidy, Block, Def, Equation};
use crate::song::Song;
use std::sync::OnceLock;

/// The book's text, as written: the first species, `lens` (12-tone modes through the scale lens `h(d)`).
pub const BOOK_TEXT: &str = include_str!("book.txt");

/// Every species' book, in order: the lens first, then the species written on 2026-11-03 after every stream's
/// function was found to look nearly the same (`src/compose/species/`). Each is a book in the same
/// format with its own helpers, frame and modules, so its equation is its own (see [`Species`]).
pub const SPECIES_TEXTS: [&str; 10] = [
    BOOK_TEXT,
    include_str!("species/ratios.txt"),
    include_str!("species/chaos.txt"),
    include_str!("species/automaton.txt"),
    include_str!("species/loops.txt"),
    include_str!("species/tritave.txt"),
    include_str!("species/bounce.txt"),
    include_str!("species/oneline.txt"),
    include_str!("species/dialogue.txt"),
    include_str!("species/maqam.txt"),
];

/// What a module fills.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Slot {
    Progression,
    Lead,
    Counter,
    Arp,
    Bass,
    Pads,
    Kick,
    Hat,
    Clap,
    Perc,
    Seam,
}

impl Slot {
    pub const ALL: [Slot; 11] =
        [Slot::Progression, Slot::Lead, Slot::Counter, Slot::Arp, Slot::Bass, Slot::Pads, Slot::Kick, Slot::Hat, Slot::Clap, Slot::Perc, Slot::Seam];

    pub fn name(self) -> &'static str {
        match self {
            Slot::Progression => "progression",
            Slot::Lead => "lead",
            Slot::Counter => "counter",
            Slot::Arp => "arp",
            Slot::Bass => "bass",
            Slot::Pads => "pads",
            Slot::Kick => "kick",
            Slot::Hat => "hat",
            Slot::Clap => "clap",
            Slot::Perc => "perc",
            Slot::Seam => "seam",
        }
    }

    pub fn from_name(s: &str) -> Option<Slot> {
        Slot::ALL.into_iter().find(|x| x.name() == s)
    }

    /// The slot a stream layer belongs to (layers are named as in `audible::layer_of`).
    pub fn of_layer(layer: &str) -> Option<Slot> {
        match layer {
            "progression" | "seam" => None,
            l => Slot::from_name(l),
        }
    }

    /// A melodic line, measured against the corpus's melody bands.
    pub fn melodic(self) -> bool {
        matches!(self, Slot::Lead | Slot::Counter)
    }

    pub fn drum(self) -> bool {
        matches!(self, Slot::Kick | Slot::Hat | Slot::Clap | Slot::Perc)
    }
}

/// One module of the book.
#[derive(Debug, Clone)]
pub struct Module {
    pub id: String,
    pub slot: Slot,
    pub energy: u8,
    pub density: u8,
    pub mood: Vec<String>,
    /// The line bends or glides continuously (tagged by hand; `tests/book.rs` measures it).
    pub glide: bool,
    /// One line: the shape, in words.
    pub describe: String,
    pub defs: Vec<Def>,
    pub blocks: Vec<Block>,
}

/// A song species: the universe of constraints one book writes in. Two species differ on several axes at once
/// (tuning, meter, the generator family, form, timbre and tempo), and each has its own definitions, so their equations
/// look different, not only their numbers.
#[derive(Debug, Clone, Default)]
pub struct Species {
    /// One word, the key everything else uses (`lens`, `ratios`, ...).
    pub name: String,
    /// One line, what it is.
    pub title: String,
    pub tuning: String,
    pub meter: String,
    pub generator: String,
    pub form: String,
    pub timbre: String,
    /// The stream's tempo (beats of `t` a minute) is drawn from this range, inclusive.
    pub bpm: (i64, i64),
    /// How often a seed lands here, relative to the others.
    pub weight: f64,
    /// The ways a stream of this species may open: each a set of layers, written `bass+hat`.
    pub open: Vec<Vec<String>>,
    /// The melody may move by octaves (a tuning without the octave, Bohlen-Pierce, says no).
    pub register: bool,
    /// Its harmony is the progression layer (`harmony.txt`): its progression slot holds the written progressions, and
    /// its frame and helpers gain the shared harmony vocabulary (`harmony::FRAME`, `harmony::HELPERS`). Every
    /// 12-tone species since 2026-11-03.
    pub harmony: bool,
    /// Behind the Explore option (strange tunings): never chosen by the default stream.
    pub explore: bool,
}

/// The whole book: its species, the shared helpers, the frame (the tuning), and the modules in the order written.
#[derive(Debug, Clone)]
pub struct Book {
    pub species: Species,
    pub helpers: Vec<Def>,
    pub frame: Vec<Def>,
    pub modules: Vec<Module>,
}

impl Book {
    pub fn get(&self, id: &str) -> Option<&Module> {
        self.modules.iter().find(|m| m.id == id)
    }

    pub fn slot(&self, slot: Slot) -> impl Iterator<Item = &Module> {
        self.modules.iter().filter(move |m| m.slot == slot)
    }

    /// Every definition name any module of `slot` writes.
    pub fn slot_names(&self, slot: Slot) -> Vec<String> {
        let mut v: Vec<String> = self.slot(slot).flat_map(|m| m.defs.iter().map(|d| d.name.clone())).collect();
        if slot == Slot::Lead {
            v.extend(["L", "Lm", "Lg", "Lr"].map(String::from)); // the grammar fallback's names
        }
        v.sort();
        v.dedup();
        v
    }
}

fn parse_def(text: &str) -> Option<Def> {
    let (lhs, rhs) = text.split_once('=')?;
    let name = lhs.split('(').next().unwrap_or("").trim().to_string();
    let lhs: String = lhs.trim().chars().filter(|c| !c.is_whitespace()).collect::<String>().replace(',', ", ");
    Some(Def::new(&name, format!("{lhs} = {}", rhs.trim())))
}

/// Read definitions and blocks from lines of the song language.
fn parse_body(lines: &[&str]) -> Result<(Vec<Def>, Vec<Block>), String> {
    let mut defs = Vec::new();
    let mut blocks = Vec::new();
    let mut open: Option<Block> = None;
    for raw in lines {
        let text = raw.split('#').next().unwrap_or("").trim();
        if text.is_empty() {
            continue;
        }
        if let Some(b) = open.as_mut() {
            if text == "}" {
                blocks.push(open.take().expect("open"));
            } else if let Some((k, v)) = text.split_once('=') {
                b.fields.push((k.trim().to_string(), v.trim().to_string()));
            } else {
                return Err(format!("not a field: {text}"));
            }
            continue;
        }
        let first = text.split_whitespace().next().unwrap_or("");
        if first == "voice" || first == "drum" {
            let name = text[first.len()..].trim().trim_end_matches('{').trim().to_string();
            open = Some(Block { kind: first.to_string(), name, fields: Vec::new() });
        } else {
            defs.push(parse_def(text).ok_or_else(|| format!("not a definition: {text}"))?);
        }
    }
    if open.is_some() {
        return Err("a block is not closed".into());
    }
    Ok((defs, blocks))
}

fn parse_species(lines: &[&str]) -> Result<Species, String> {
    let mut s = Species { register: true, weight: 1.0, ..Species::default() };
    for raw in lines {
        let text = raw.split('#').next().unwrap_or("").trim();
        if text.is_empty() {
            continue;
        }
        let (k, v) = text.split_once(char::is_whitespace).map(|(k, v)| (k, v.trim())).unwrap_or((text, ""));
        match k {
            "name" => s.name = v.to_string(),
            "title" => s.title = v.to_string(),
            "tuning" => s.tuning = v.to_string(),
            "meter" => s.meter = v.to_string(),
            "generator" => s.generator = v.to_string(),
            "form" => s.form = v.to_string(),
            "timbre" => s.timbre = v.to_string(),
            "bpm" => {
                let n: Vec<i64> = v.split_whitespace().filter_map(|x| x.parse().ok()).collect();
                if n.len() != 2 || n[0] > n[1] || n[0] < 40 || n[1] > 240 {
                    return Err(format!("bpm wants two numbers, low and high, in 40..240: '{v}'"));
                }
                s.bpm = (n[0], n[1]);
            }
            "weight" => s.weight = v.parse().ok().filter(|w: &f64| *w > 0.0).ok_or_else(|| format!("weight: '{v}'"))?,
            "open" => {
                s.open = v.split('|').map(|o| o.split('+').map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect()).collect();
                if s.open.iter().any(|o: &Vec<String>| o.is_empty()) {
                    return Err(format!("open: an empty opening in '{v}'"));
                }
            }
            "register" => s.register = v != "no",
            "harmony" => s.harmony = v == "yes",
            "explore" => s.explore = v == "yes",
            _ => return Err(format!("unknown species key '{k}'")),
        }
    }
    if s.name.is_empty() || s.bpm == (0, 0) || s.open.is_empty() {
        return Err("a species needs a name, a bpm range and its openings".into());
    }
    Ok(s)
}

/// Parse the book's text.
pub fn parse(text: &str) -> Result<Book, String> {
    let mut book = Book { species: Species::default(), helpers: Vec::new(), frame: Vec::new(), modules: Vec::new() };
    let mut sections: Vec<(String, Vec<&str>)> = Vec::new();
    for line in text.lines() {
        if let Some(head) = line.strip_prefix("== ") {
            sections.push((head.trim().to_string(), Vec::new()));
        } else if let Some(s) = sections.last_mut() {
            s.1.push(line);
        }
    }
    for (head, lines) in sections {
        if head == "species" {
            book.species = parse_species(&lines)?;
            continue;
        }
        let (defs, blocks) = parse_body(&lines).map_err(|e| format!("{head}: {e}"))?;
        match head.as_str() {
            "helpers" => book.helpers = defs,
            "frame" => book.frame = defs,
            _ => {
                let parts: Vec<&str> = head.split('|').map(str::trim).collect();
                let id = parts[0].to_string();
                let slot = parts.get(1).and_then(|s| Slot::from_name(s)).ok_or_else(|| format!("{id}: no slot"))?;
                let mut m = Module { id: id.clone(), slot, energy: 0, density: 0, mood: Vec::new(), glide: false, describe: String::new(), defs, blocks };
                for p in &parts[2..] {
                    let mut w = p.split_whitespace();
                    match w.next() {
                        Some("energy") => m.energy = w.next().and_then(|x| x.parse().ok()).ok_or_else(|| format!("{id}: energy"))?,
                        Some("density") => m.density = w.next().and_then(|x| x.parse().ok()).ok_or_else(|| format!("{id}: density"))?,
                        Some("mood") => m.mood = w.map(String::from).collect(),
                        Some("glide") => m.glide = true,
                        _ => return Err(format!("{id}: unknown tag '{p}'")),
                    }
                }
                m.describe = lines
                    .iter()
                    .find_map(|l| l.trim().strip_prefix('#').map(|d| d.trim().to_string()))
                    .ok_or_else(|| format!("{id}: no description line"))?;
                if !(1..=3).contains(&m.energy) || !(1..=3).contains(&m.density) {
                    return Err(format!("{id}: energy and density run 1 to 3"));
                }
                book.modules.push(m);
            }
        }
    }
    if book.species.name.is_empty() {
        return Err("no '== species' section".into());
    }
    if book.species.harmony {
        add_harmony(&mut book)?;
    }
    Ok(book)
}

/// A harmony species: the shared frame (the key `K` and the lens `h`) and helpers, and every written progression as a
/// module of its progression slot. Harmony is written, never generated: a harmony species writes no progression of its
/// own.
fn add_harmony(book: &mut Book) -> Result<(), String> {
    if book.slot(Slot::Progression).next().is_some() {
        return Err(format!("{}: a harmony species takes its progressions from harmony.txt", book.species.name));
    }
    let lines: Vec<&str> = super::harmony::FRAME.lines().collect();
    let (frame, _) = parse_body(&lines)?;
    for d in frame.into_iter().rev() {
        if !book.frame.iter().any(|f| f.name == d.name) {
            book.frame.insert(0, d);
        }
    }
    let lines: Vec<&str> = super::harmony::HELPERS.lines().collect();
    let (helpers, _) = parse_body(&lines)?;
    for d in helpers {
        if !book.helpers.iter().any(|f| f.name == d.name) {
            book.helpers.push(d);
        }
    }
    let prefix = if book.species.name == "lens" { String::new() } else { format!("{}/", book.species.name) };
    for p in super::harmony::all() {
        book.modules.push(super::harmony::module(p, &prefix));
    }
    Ok(())
}

/// The style the lens book's drums are sampled from (groove_model.txt: funk, soul, pop and disco drummers).
pub const GROOVE_STYLE: &str = "citypop";
/// How many grooves the lens book holds; each gives up to four drum modules (kick, clap, hat, perc).
pub const GROOVES: u64 = 24;

/// Every species' book, parsed once, the lens first.
pub fn all() -> &'static [Book] {
    static BOOKS: OnceLock<Vec<Book>> = OnceLock::new();
    BOOKS.get_or_init(|| {
        SPECIES_TEXTS
            .iter()
            .enumerate()
            .map(|(i, t)| {
                // the lens (city pop) plays drums sampled from real drummers, not the written ones (groove.rs)
                let text = if i == 0 { super::groove::replace_drums(t, GROOVE_STYLE, GROOVES) } else { t.to_string() };
                parse(&text).unwrap_or_else(|e| panic!("a species book does not parse (tests/species.rs): {e}"))
            })
            .collect()
    })
}

/// The lens species' book, the move book as first written.
pub fn book() -> &'static Book {
    &all()[0]
}

/// A species' book by name.
pub fn named(name: &str) -> Option<&'static Book> {
    all().iter().find(|b| b.species.name == name)
}

/// The book an equation is written from (the lens when it names none).
pub fn of(eq: &Equation) -> &'static Book {
    if eq.species.is_empty() {
        book()
    } else {
        named(&eq.species).unwrap_or_else(|| panic!("unknown species '{}'", eq.species))
    }
}

/// A module of any species, by id.
pub fn find(id: &str) -> Option<&'static Module> {
    all().iter().find_map(|b| b.get(id))
}

/// The species a seed lands on: weighted by each species' weight, among the default species (never one behind the
/// Explore option).
pub fn species_for_seed(seed: u64) -> &'static Book {
    species_among(seed, false)
}

/// The species an Explore seed lands on: weighted, among the species of strange tunings (`explore yes`).
pub fn explore_for_seed(seed: u64) -> &'static Book {
    species_among(seed, true)
}

fn species_among(seed: u64, explore: bool) -> &'static Book {
    let mut r = Rng::for_candidate(seed ^ 0x5350_4543, 0); // "SPEC"
    let books: Vec<&'static Book> = all().iter().filter(|b| b.species.explore == explore).collect();
    let w: Vec<f64> = books.iter().map(|b| b.species.weight).collect();
    books[r.weighted(&w)]
}

// ------------------------------------------------------------------------------------------------ choosing by need

/// What a stage wants of a module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Need {
    pub energy: u8,
    pub density: u8,
}

impl Default for Need {
    fn default() -> Need {
        Need { energy: 2, density: 2 }
    }
}

/// How well a module fits a need: 1 when its energy and density match, falling off with the distance.
pub fn fit(m: &Module, need: Need) -> f64 {
    let e = [1.0, 0.35, 0.1][(m.energy as i32 - need.energy as i32).unsigned_abs().min(2) as usize];
    let d = [1.0, 0.6, 0.3][(m.density as i32 - need.density as i32).unsigned_abs().min(2) as usize];
    e * d
}

/// Choose a module for `slot` by need, never `current` nor any in `avoid`; None when nothing is left.
pub fn pick(b: &'static Book, slot: Slot, need: Need, current: Option<&str>, avoid: &[String], r: &mut Rng) -> Option<&'static Module> {
    let cands: Vec<&Module> = b.slot(slot).filter(|m| Some(m.id.as_str()) != current && !avoid.contains(&m.id)).collect();
    if cands.is_empty() {
        return None;
    }
    let w: Vec<f64> = cands.iter().map(|m| fit(m, need)).collect();
    Some(cands[r.weighted(&w)])
}

/// A seam module for the seam into a stage of role `into` (`return`, `breakdown`, `peak`, `intro`).
pub fn pick_seam(b: &'static Book, into: &str, r: &mut Rng) -> Option<&'static Module> {
    let tag = format!("into-{}", if into == "breakdown" { "strip" } else { into });
    let cands: Vec<&Module> = b.slot(Slot::Seam).filter(|m| m.mood.contains(&tag)).collect();
    if cands.is_empty() {
        None
    } else {
        Some(*r.pick(&cands))
    }
}

// ------------------------------------------------------------------------------------------------ installing

/// The module playing in `slot`, by id.
pub fn current(eq: &Equation, slot: Slot) -> Option<&str> {
    eq.book.iter().find(|(s, _)| s == slot.name()).map(|(_, id)| id.as_str())
}

fn set_current(eq: &mut Equation, slot: Slot, id: &str) {
    match eq.book.iter_mut().find(|(s, _)| s == slot.name()) {
        Some(e) => e.1 = id.to_string(),
        None => eq.book.push((slot.name().to_string(), id.to_string())),
    }
}

/// Take a slot's module out: its definitions (and, for a seam, its voices) and its entry.
pub fn uninstall(eq: &mut Equation, slot: Slot) {
    let names = of(eq).slot_names(slot);
    eq.defs.retain(|d| !names.contains(&d.name));
    if slot == Slot::Seam {
        eq.voices.retain(|v| !SEAM_VOICES.contains(&v.name.as_str()));
    }
    eq.book.retain(|(s, _)| s != slot.name());
}

/// The voices a seam module plays (they come for one stage and are not layers).
pub const SEAM_VOICES: [&str; 2] = ["fill", "rise"];
/// The definitions a seam module writes (not counted against the budget).
pub const SEAM_DEFS: [&str; 2] = ["roll", "sweep"];

/// Put module `m` in its slot: the slot's old definitions go, the module's come in, its voices replace the slot's
/// voices where they are (playing or waiting), the helpers it reads are added and anything nothing reads is tidied.
pub fn install(eq: &Equation, m: &Module) -> Equation {
    let mut e = eq.clone();
    uninstall(&mut e, m.slot);
    for d in &m.defs {
        match e.defs.iter_mut().find(|x| x.name == d.name) {
            Some(x) => *x = d.clone(),
            None => e.defs.push(d.clone()),
        }
    }
    for b in &m.blocks {
        if let Some(v) = e.voices.iter_mut().find(|v| v.name == b.name) {
            *v = b.clone();
        } else if let Some(v) = e.stash.iter_mut().find(|v| v.name == b.name) {
            *v = b.clone();
        } else if m.slot == Slot::Seam {
            e.insert_voice(b.clone());
        } else {
            e.stash.push(b.clone());
        }
    }
    set_current(&mut e, m.slot, &m.id);
    finish(&mut e);
    e
}

/// Make the equation show what sounds: restore the definitions of every playing slot's module (a layer that comes
/// back from the stash brings its lines back), drop those only waiting layers read, add the helpers that are read, and
/// order them.
pub fn finish(e: &mut Equation) {
    sync(e, false);
}

/// As [`finish`], but every slot's module is restored, waiting or not (for a song holding every voice).
pub fn full(e: &mut Equation) {
    sync(e, true);
}

fn sync(e: &mut Equation, all: bool) {
    let b = of(e);
    let playing = |e: &Equation, m: &Module| m.blocks.iter().any(|bl| e.voices.iter().any(|v| v.name == bl.name));
    for (_, id) in e.book.clone() {
        let Some(m) = b.get(&id) else { continue };
        if m.slot != Slot::Progression && !all && !playing(e, m) {
            continue;
        }
        for d in &m.defs {
            if e.def(&d.name).is_none() {
                e.defs.push(d.clone());
            }
        }
    }
    if !all {
        // A waiting layer's lines are not shown; a fallback module (not in the book) keeps its lines, since nothing
        // could restore them.
        let fallback: Vec<String> = e.book.iter().filter(|(_, id)| b.get(id).is_none()).map(|(s, _)| s.clone()).collect();
        let stash = std::mem::take(&mut e.stash);
        let (keep, away): (Vec<Block>, Vec<Block>) =
            stash.into_iter().partition(|bl| fallback.iter().any(|s| Slot::from_name(s).is_some_and(|sl| slot_voices(sl).contains(&bl.name.as_str()))));
        e.stash = keep;
        finish_defs(e);
        e.stash.extend(away);
    } else {
        finish_defs(e);
    }
}

/// The voice names a slot's modules play.
pub fn slot_voices(slot: Slot) -> &'static [&'static str] {
    match slot {
        Slot::Progression => &[],
        Slot::Lead => &["lead"],
        Slot::Counter => &["counter"],
        Slot::Arp => &["arp"],
        Slot::Bass => &["bass"],
        Slot::Pads => &["pad1", "pad2", "pad3"],
        Slot::Kick => &["kick"],
        Slot::Hat => &["hat"],
        Slot::Clap => &["clap"],
        Slot::Perc => &["perc"],
        Slot::Seam => &["fill", "rise"],
    }
}

fn finish_defs(e: &mut Equation) {
    let b = of(e);
    for f in b.frame.iter().rev() {
        if e.def(&f.name).is_none() {
            e.defs.insert(0, f.clone());
        }
    }
    loop {
        let mut added = false;
        for hd in &b.helpers {
            if e.def(&hd.name).is_some() {
                continue;
            }
            let read = e.defs.iter().any(|d| super::grammar::uses(d.rhs(), &hd.name))
                || e.voices.iter().chain(&e.stash).any(|v| v.fields.iter().any(|f| super::grammar::uses(&f.1, &hd.name)));
            if read {
                e.defs.push(hd.clone());
                added = true;
            }
        }
        if !added {
            break;
        }
    }
    // The progression and the lens (the frame) stay even while nothing that reads them is playing yet.
    let mut stay = b.slot_names(Slot::Progression);
    stay.extend(b.frame.iter().map(|f| f.name.clone()));
    stay.push("K".into());
    let keep: Vec<Def> = e.defs.iter().filter(|d| stay.contains(&d.name)).cloned().collect();
    tidy(e);
    for d in keep {
        if e.def(&d.name).is_none() {
            e.defs.push(d);
        }
    }
    order_defs(e);
}

/// Lens first, then the helpers, the progression, each slot's lines in the roster's order, the seam, the time names.
fn order_defs(e: &mut Equation) {
    let b = of(e);
    let rank = |d: &Def| -> usize {
        if ["c", "K"].contains(&d.name.as_str()) {
            return 0;
        }
        if b.frame.iter().any(|f| f.name == d.name) {
            return 1;
        }
        if let Some(i) = b.helpers.iter().position(|h| h.name == d.name) {
            return 10 + i;
        }
        if super::stream::TIME_NAMES.contains(&d.name.as_str()) {
            return 1000;
        }
        for (k, slot) in Slot::ALL.iter().enumerate() {
            if b.slot_names(*slot).contains(&d.name) {
                return 100 + 10 * k;
            }
        }
        500
    };
    let mut v: Vec<(usize, usize, Def)> = e.defs.drain(..).enumerate().map(|(i, d)| (rank(&d), i, d)).collect();
    v.sort_by_key(|x| (x.0, x.1));
    e.defs = v.into_iter().map(|x| x.2).collect();
}

/// The roster voices that are drums, in every species (`tests/species.rs` checks).
pub const ROSTER_DRUMS: [&str; 5] = ["kick", "clap", "hat", "perc", "fill"];

/// The roster of a book stream: every voice a module can play, in drawing order.
pub const ROSTER: [&str; 13] = ["bass", "pad1", "pad2", "pad3", "arp", "counter", "lead", "kick", "clap", "hat", "perc", "fill", "rise"];

/// The settings every book stream shares; the tempo is drawn from the seed.
fn settings(bpm: i64) -> Vec<String> {
    vec![
        format!("bpm {bpm}"),
        "reverb_size 0.72".into(),
        "reverb_predelay 18".into(),
        "delay_beats 0.75".into(),
        "delay_feedback 0.3".into(),
        "warmth 0.2".into(),
    ]
}

/// A lens stream's first equation (the move book as first written); see [`assemble_in`].
pub fn assemble(seed: u64) -> Equation {
    assemble_in(book(), seed, None)
}

/// A stream's first equation in species `b`, with every slot it has filled (all waiting in the stash): the
/// progression and the hats chosen for an intro, the bass from the **whole** bass set (2026-11-03: repeated
/// testing kept hearing the same bassline; two of the last three streams opened on one bass, which the intro need had
/// made the likeliest), the rest for a build. The tempo is drawn from the species' range unless `bpm` is given (a
/// species change inside a stream keeps the stream's tempo). `stream::opening_as` then brings in the opening layers.
pub fn assemble_in(b: &'static Book, seed: u64, bpm: Option<i64>) -> Equation {
    let mut r = Rng::for_candidate(seed ^ 0x424F_4F4B, 0); // "BOOK"
    let drawn = r.range(b.species.bpm.0, b.species.bpm.1);
    let bpm = bpm.unwrap_or(drawn);
    let mut eq = Equation {
        settings: settings(bpm),
        defs: Vec::new(),
        voices: Vec::new(),
        stash: Vec::new(),
        roster: ROSTER.iter().map(|s| s.to_string()).collect(),
        book: Vec::new(),
        species: if b.species.name == "lens" { String::new() } else { b.species.name.clone() },
    };
    let intro = Need { energy: 1, density: 1 };
    let build = Need { energy: 2, density: 2 };
    for slot in [Slot::Progression, Slot::Bass, Slot::Pads, Slot::Arp, Slot::Counter, Slot::Lead, Slot::Kick, Slot::Clap, Slot::Hat, Slot::Perc] {
        let m = if slot == Slot::Bass {
            let all: Vec<&'static Module> = b.slot(slot).collect();
            if all.is_empty() {
                None
            } else {
                Some(*r.pick(&all))
            }
        } else {
            let need = if matches!(slot, Slot::Progression | Slot::Hat) { intro } else { build };
            pick(b, slot, need, None, &[], &mut r)
        };
        if let Some(m) = m {
            eq = install(&eq, m);
        }
    }
    eq
}

/// How a stream of species `b` opens, from its seed: one of the species' openings (`open` in its header), so streams
/// do not all start on the bass and the hats.
pub fn opening_shape(b: &Book, seed: u64) -> Vec<String> {
    let mut r = Rng::for_candidate(seed ^ 0x4F50_454E, 0); // "OPEN"
    r.pick(&b.species.open).clone()
}

// ------------------------------------------------------------------------------------------------ measuring a line

/// A melodic line measured against the corpus (docs/research/desmos_corpus.md, section 4).
#[derive(Debug, Clone, Copy, Default)]
pub struct LineStats {
    /// Of the beats where the line sounds, the share on a triad tone of the bar's root (0, 3, 4 or 7 semitones above).
    pub chord_all: f64,
    /// The same on the strong beats only (beats 1 and 3 of each bar).
    pub chord_strong: f64,
    /// Of the moves between different pitches, the share of 1-2 semitones.
    pub steps: f64,
    /// 5th to 95th percentile of the notes, semitones.
    pub range: f64,
    /// Mean over bars of the best slot-for-slot match with an earlier bar (sixteenths, pitch and onset).
    pub repetition: f64,
    /// Notes (onsets and glides that land) a beat.
    pub notes_per_beat: f64,
    /// Share of the sounding time spent between semitones (bending or gliding).
    pub glide_share: f64,
    pub median: f64,
}

/// Measure voice `name` of a song. The chord-tone shares need a written progression's tables (a species without
/// written harmony reads NaN there; the rest is measured the same): a beat's note is on a chord tone when its pitch
/// class is the half bar's bass note or one of the pads' (the tones the progression voices, sevenths and colours
/// included; `harmony::chord_tones`).
pub fn line_stats(song: &Song, name: &str) -> Option<LineStats> {
    let tracks = critic::read_tracks(song);
    let tr = tracks.iter().find(|t| t.name == name && t.tonal)?;
    let g = critic::GRID;
    let beats = song.beats.floor() as usize;
    let (mut all, mut all_n, mut strong, mut strong_n) = (0.0, 0.0, 0.0, 0.0);
    let harmony = super::harmony::chord_tones(&song.program, 0).is_some();
    for b in (0..beats).filter(|_| harmony) {
        let i = b * g;
        if i >= tr.open.len() || !tr.open[i] || !tr.pitch[i].is_finite() {
            continue;
        }
        let tones = super::harmony::chord_tones(&song.program, b / 2)?;
        let pc = tr.pitch[i].round().rem_euclid(12.0) as i64;
        let hit = tones.contains(&pc) as i64 as f64;
        all += hit;
        all_n += 1.0;
        if b % 4 == 0 || b % 4 == 2 {
            strong += hit;
            strong_n += 1.0;
        }
    }
    // Notes: each onset, and inside a held note each pitch the line lands on and stays near (within 0.3 semitone) for a
    // fifth of a beat, so a glide counts as the note it reaches, not as every semitone it passes on the way.
    let n = tr.open.len();
    let dwell = g / 5;
    let mut ev: Vec<critic::Event> = Vec::new();
    let mut cur = f64::NAN;
    for i in 0..n {
        if !tr.open[i] || !tr.pitch[i].is_finite() {
            cur = f64::NAN;
            continue;
        }
        let p = tr.pitch[i];
        let r = p.round();
        if i == 0 || !tr.open[i - 1] {
            cur = r;
            ev.push(critic::Event { t: i as f64 / g as f64, pitch: r });
        } else if r != cur && (p - r).abs() <= 0.3 && i + dwell <= n && (i..i + dwell).all(|k| tr.open[k] && (tr.pitch[k] - r).abs() <= 0.3) {
            cur = r;
            ev.push(critic::Event { t: i as f64 / g as f64, pitch: r });
        }
    }
    let ev = &ev;
    let mut moves = Vec::new();
    for w in ev.windows(2) {
        let d = w[1].pitch.round() - w[0].pitch.round();
        if w[1].t - w[0].t <= 4.0 && d != 0.0 {
            moves.push(d);
        }
    }
    let steps = if moves.is_empty() { 0.0 } else { moves.iter().filter(|d| d.abs() <= 2.0).count() as f64 / moves.len() as f64 };
    let mut ps: Vec<f64> = ev.iter().map(|e| e.pitch.round()).collect();
    ps.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    let (range, median) = if ps.len() >= 2 {
        let last = (ps.len() - 1) as f64;
        (ps[(last * 0.95).round() as usize] - ps[(last * 0.05).round() as usize], ps[ps.len() / 2])
    } else {
        (0.0, ps.first().copied().unwrap_or(f64::NAN))
    };
    // Bar repetition, as the critic measures it.
    let bars = beats / 4;
    let slots: Vec<Vec<(Option<i64>, bool)>> = (0..bars)
        .map(|b| {
            (0..16)
                .map(|s| {
                    let i = (b * 16 + s) * g / 4;
                    let p = if tr.open[i] && tr.pitch[i].is_finite() { Some(tr.pitch[i].round() as i64) } else { None };
                    (p, ev.iter().any(|e| ((e.t * 4.0).round() as usize) == b * 16 + s))
                })
                .collect()
        })
        .collect();
    let active: Vec<usize> = (0..bars).filter(|b| slots[*b].iter().any(|s| s.0.is_some())).collect();
    let repetition = if active.len() < 2 {
        0.0
    } else {
        let sim = |a: &[(Option<i64>, bool)], b: &[(Option<i64>, bool)]| a.iter().zip(b).filter(|(x, y)| x == y).count() as f64 / 16.0;
        let mut sum = 0.0;
        for (k, &b) in active.iter().enumerate().skip(1) {
            sum += active[..k].iter().map(|&a| sim(&slots[a], &slots[b])).fold(0.0, f64::max);
        }
        sum / (active.len() - 1) as f64
    };
    let open: Vec<usize> = (0..tr.open.len()).filter(|&i| tr.open[i] && tr.pitch[i].is_finite()).collect();
    let bent = open.iter().filter(|&&i| (tr.pitch[i] - tr.pitch[i].round()).abs() > 0.1).count();
    Some(LineStats {
        chord_all: if all_n > 0.0 { all / all_n } else { f64::NAN },
        chord_strong: if strong_n > 0.0 { strong / strong_n } else { f64::NAN },
        steps,
        range,
        repetition,
        notes_per_beat: ev.len() as f64 / song.beats,
        glide_share: if open.is_empty() { 0.0 } else { bent as f64 / open.len() as f64 },
        median,
    })
}

/// The species name an equation carries for a module (empty for the lens).
fn species_of(m: &Module) -> String {
    all().iter().find(|b| b.modules.iter().any(|x| x.id == m.id)).filter(|b| b.species.name != "lens").map_or(String::new(), |b| b.species.name.clone())
}

/// A song of one stage that plays `module` alone over `progression` (pads, a lead, a drum: whatever it is), at `rate`
/// samples a second. Used by the tests and the measurements.
pub fn solo_song(module: &Module, progression: &Module, rate: u32) -> String {
    let mut eq = Equation {
        settings: settings(100).into_iter().chain([format!("rate {rate}")]).collect(),
        defs: Vec::new(),
        voices: Vec::new(),
        stash: Vec::new(),
        roster: ROSTER.iter().map(|s| s.to_string()).collect(),
        book: Vec::new(),
        species: species_of(module),
    };
    eq = install(&eq, progression);
    eq = install(&eq, module);
    for b in std::mem::take(&mut eq.stash) {
        eq.insert_voice(b);
    }
    full(&mut eq);
    eq.song_text(&format!("{} over {}", module.id, progression.id))
}
