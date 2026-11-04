//! The progression layer: harmony written by judgment, never generated (the text is `harmony.txt`, beside this file).
//!
//! 2026-11-03: a loops stream whose chord root cycled by `mod(c + m, 5)` out of step with the bar did not sound good
//! most of the time, and not because the sound was cheap: the progression was the problem.
//! The one generated piece judged alright ran on real functional harmony. So the harmony of every 12-tone species is
//! one of the progressions in `harmony.txt`, chosen by the stream like any module, and the species' own maths writes
//! only the melodies, rhythms and shapes on top of it.
//!
//! A progression is eight bars (one stage) of Roman numerals. This module parses them, voices each chord for three pad
//! voices with the smallest movement from the chord before (no crossing, inside fixed registers), and writes the result
//! as four digit tables the song language reads as functions of the half bar (`Hb` the bass on the circle of fifths,
//! `Hx Hy Hz` the pads low to high), plus the melody's mode `c`. Everything is relative to the key `K`, which the stream
//! sets once per song, so the tables do not depend on the key.

use super::book::{Module, Slot};
use super::stream::Def;
use std::sync::OnceLock;

/// The text, as written.
pub const TEXT: &str = include_str!("harmony.txt");

/// Harmonic slots in a stage: one every half bar (two beats), eight bars.
pub const SLOTS: usize = 16;
/// Pad voice m lives in `K + BAND_LOW + BAND_STEP m` .. `+ BAND_SPAN` (semitones from A4): three overlapping registers
/// a tritone apart, from a tenth below the tonic to a third above it, each nine semitones wide so its table needs one
/// digit a slot (0..8). Chosen by measuring every layout against every chord written: the one where every chord has a
/// voicing without a minor-second cluster and the voices move least (4.2 semitones a change, all three together).
pub const BAND_LOW: i32 = -16;
pub const BAND_STEP: i32 = 6;
pub const BAND_SPAN: i32 = 8;
/// Adjacent pad voices stay at least this many semitones apart (no minor-second clusters).
pub const MIN_GAP: i32 = 2;
/// The names a progression writes (its module's definitions).
pub const TABLE_NAMES: [&str; 6] = ["Hm", "Ho", "Hb", "Hx", "Hy", "Hz"];

/// The frame every harmony species shares: the key (set by the stream) and the scale lens in the progression's mode.
pub const FRAME: &str = "K = 0\nh(d) = K + floor((12d + c) / 7)\n";

/// The vocabulary every harmony species shares (helpers: not counted against the budget).
///
/// - `hd(N, i)`: digit i of a table, read left to right, one a half bar.
/// - `Hr(i)`: the bass note of half bar i (a pitch class, as a pitch in `K .. K + 11`): `Hb` holds it as steps of
///   `Hm` semitones from the origin `Ho`, fifths (7) on the circle of fifths, or half steps (1) for a chromatic line.
/// - `Hv(m, i)`: pad voice m (0 low, 2 high) of half bar i.
/// - `Hc(m, i)`: chord tone m counted upward from the bass's pitch class just under the pads (an arpeggio's ladder).
/// - `nr(x, p)`: p's pitch class nearest x; `mn(x, a, b)`: whichever of a and b is nearer x.
/// - `sc(x, i)`: x pulled to the nearest chord tone of half bar i (the bass or a pad's pitch class).
/// - `hs(d, k, p)`: scale degree d as a pitch, pulled to the chord when step k falls on a half bar (p steps to one).
pub const HELPERS: &str = "hd(N, i) = mod(floor(N / 10^(15 - mod(i, 16))), 10)
Hr(i) = K + mod((hd(Hb, i) + Ho) Hm, 12)
Hv(m, i) = K + 6m - 16 + hd(if(m == 0, Hx, if(m == 1, Hy, Hz)), i)
Hc(m, i) = 12 floor(m / 4) + if(mod(m, 4) == 0, Hv(0, i) - 1 - mod(Hv(0, i) - 1 - Hr(i), 12), Hv(mod(m, 4) - 1, i))
nr(x, p) = p + 12 floor((x - p) / 12 + 0.5)
mn(x, a, b) = if(abs(x - a) <= abs(x - b), a, b)
sc(x, i) = mn(x, mn(x, nr(x, Hr(i)), nr(x, Hv(0, i))), mn(x, nr(x, Hv(1, i)), nr(x, Hv(2, i))))
hs(d, k, p) = if(mod(k, p) == 0, sc(h(d), floor(k / p)), h(d))
";

/// The tonic's scale, which also says how Roman numerals count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Major,
    Minor,
    Dorian,
    Lydian,
    Mixolydian,
}

const MAJOR: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];
const NATURAL_MINOR: [i32; 7] = [0, 2, 3, 5, 7, 8, 10];

impl Mode {
    pub fn from_name(s: &str) -> Option<Mode> {
        Some(match s {
            "major" => Mode::Major,
            "minor" => Mode::Minor,
            "dorian" => Mode::Dorian,
            "lydian" => Mode::Lydian,
            "mixolydian" => Mode::Mixolydian,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Mode::Major => "major",
            Mode::Minor => "minor",
            Mode::Dorian => "dorian",
            Mode::Lydian => "lydian",
            Mode::Mixolydian => "mixolydian",
        }
    }

    /// The scale lens's `c` for the melody: `h(d) = K + floor((12d + c) / 7)`.
    pub fn lens(self) -> i32 {
        match self {
            Mode::Major => 5,
            Mode::Minor => 2,
            Mode::Dorian => 3,
            Mode::Lydian => 6,
            Mode::Mixolydian => 4,
        }
    }

    /// A minor tonic (numerals count the natural minor).
    pub fn minor(self) -> bool {
        matches!(self, Mode::Minor | Mode::Dorian)
    }

    fn numerals(self) -> [i32; 7] {
        if self.minor() {
            NATURAL_MINOR
        } else {
            MAJOR
        }
    }
}

/// One chord: where its root is, what it holds, how the pads voice it and what the bass plays.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chord {
    /// As written (`ii9`, `Imaj7/7`, `vi9!`).
    pub symbol: String,
    /// The root's pitch class above the tonic, 0..11.
    pub root: i32,
    /// The chord's tones in semitones above the root (the root itself first).
    pub tones: Vec<i32>,
    /// The three tones the pads play, in semitones above the root (the guide tones and one colour).
    pub pads: [i32; 3],
    /// The bass note's pitch class above the tonic (the root, or the written inversion).
    pub bass: i32,
    /// Marked `!`: a deceptive turn where the tonic was due.
    pub deceptive: bool,
}

impl Chord {
    pub fn is_tonic(&self) -> bool {
        self.root == 0
    }

    /// Dominant function: V of any colour or suspended, viiø7 or vii°7, the tritone substitute bII7, the backdoor bVII7.
    pub fn is_dominant(&self) -> bool {
        let has = |x: i32| self.tones.iter().any(|t| t.rem_euclid(12) == x);
        let dom7 = has(4) && has(10);
        match self.root {
            7 => has(4) || has(5),
            11 => has(3) && has(6),
            1 | 10 => dom7,
            _ => false,
        }
    }

    /// The pitch classes a melody may land on: the bass's and the pads'.
    pub fn pitch_classes(&self) -> Vec<i32> {
        let mut v: Vec<i32> = std::iter::once(self.bass).chain(self.pads.iter().map(|p| (self.root + p).rem_euclid(12))).collect();
        v.sort();
        v.dedup();
        v
    }
}

/// A progression: eight bars, its chords per half bar, and their voicing.
#[derive(Debug, Clone)]
pub struct Progression {
    pub name: String,
    pub mode: Mode,
    /// Bars to a phrase: 4 or 8. Each phrase ends on a dominant.
    pub phrase: usize,
    pub energy: u8,
    pub density: u8,
    pub mood: Vec<String>,
    pub describe: String,
    /// The bars as written: one or two chords each (a held bar repeats the chord before, `held` true).
    pub bars: Vec<Bar>,
    /// The chord sounding in each half bar, 16 of them.
    pub slots: Vec<Chord>,
    /// The pads' pitches in each half bar, semitones above the tonic (add `K` for the pitch): low, middle, high.
    pub voicing: Vec<[i32; 3]>,
}

/// One bar as written.
#[derive(Debug, Clone)]
pub struct Bar {
    pub chords: Vec<Chord>,
    /// `%`: the chord before holds through this bar.
    pub held: bool,
}

impl Progression {
    /// Where a chord starts: (half bar, chord), in order. A held bar starts nothing.
    pub fn events(&self) -> Vec<(usize, &Chord)> {
        let mut v = Vec::new();
        for (b, bar) in self.bars.iter().enumerate() {
            if bar.held {
                continue;
            }
            for (k, c) in bar.chords.iter().enumerate() {
                v.push((2 * b + k, c));
            }
        }
        v
    }

    /// Any chord marked as a deceptive turn.
    pub fn deceptive(&self) -> bool {
        self.slots.iter().any(|c| c.deceptive)
    }
}

// ------------------------------------------------------------------------------------------------ parsing

/// The chord qualities: (suffix, upper-case numeral?, tones above the root, the pads' three tones).
const QUALITIES: [(&str, bool, &[i32], [i32; 3]); 30] = [
    ("", true, &[0, 4, 7], [0, 4, 7]),
    ("maj7", true, &[0, 4, 7, 11], [4, 7, 11]),
    ("maj9", true, &[0, 4, 7, 11, 14], [4, 11, 14]),
    ("6", true, &[0, 4, 7, 9], [4, 7, 9]),
    ("69", true, &[0, 4, 7, 9, 14], [4, 9, 14]),
    ("add9", true, &[0, 4, 7, 14], [4, 7, 14]),
    ("7", true, &[0, 4, 7, 10], [4, 7, 10]),
    ("9", true, &[0, 4, 7, 10, 14], [4, 10, 14]),
    ("13", true, &[0, 4, 7, 10, 21], [4, 10, 21]),
    ("7b9", true, &[0, 4, 7, 10, 13], [4, 10, 13]),
    ("7#9", true, &[0, 4, 7, 10, 15], [4, 10, 15]),
    ("7sus", true, &[0, 5, 7, 10], [5, 7, 10]),
    ("9sus", true, &[0, 5, 7, 10, 14], [5, 10, 14]),
    ("+", true, &[0, 4, 8], [0, 4, 8]),
    ("°", true, &[0, 3, 6], [0, 3, 6]),
    ("", false, &[0, 3, 7], [0, 3, 7]),
    ("7", false, &[0, 3, 7, 10], [3, 7, 10]),
    ("m7", false, &[0, 3, 7, 10], [3, 7, 10]),
    ("9", false, &[0, 3, 7, 10, 14], [3, 10, 14]),
    ("11", false, &[0, 3, 7, 10, 14, 17], [3, 10, 17]),
    ("6", false, &[0, 3, 7, 9], [3, 7, 9]),
    ("m6", false, &[0, 3, 7, 9], [3, 7, 9]),
    ("maj7", false, &[0, 3, 7, 11], [3, 7, 11]),
    ("ø7", false, &[0, 3, 6, 10], [3, 6, 10]),
    ("m7b5", false, &[0, 3, 6, 10], [3, 6, 10]),
    ("°7", false, &[0, 3, 6, 9], [3, 6, 9]),
    ("o7", false, &[0, 3, 6, 9], [3, 6, 9]),
    ("°", false, &[0, 3, 6], [0, 3, 6]),
    ("o", false, &[0, 3, 6], [0, 3, 6]),
    ("+", false, &[0, 4, 8], [0, 4, 8]),
];

/// Read one chord symbol in `mode`.
pub fn parse_chord(symbol: &str, mode: Mode) -> Result<Chord, String> {
    let err = |why: &str| format!("chord '{symbol}': {why}");
    let mut s = symbol.trim();
    let deceptive = s.ends_with('!');
    if deceptive {
        s = &s[..s.len() - 1];
    }
    let (body, slash) = match s.split_once('/') {
        Some((b, x)) => (b, Some(x)),
        None => (s, None),
    };
    let mut shift = 0;
    let mut rest = body;
    while let Some(c) = rest.chars().next() {
        match c {
            'b' => shift -= 1,
            '#' => shift += 1,
            _ => break,
        }
        rest = &rest[1..];
    }
    let letters: String = rest.chars().take_while(|c| matches!(c, 'I' | 'V' | 'i' | 'v')).collect();
    if letters.is_empty() {
        return Err(err("no Roman numeral"));
    }
    let upper = letters.chars().all(|c| c.is_ascii_uppercase());
    if !upper && !letters.chars().all(|c| c.is_ascii_lowercase()) {
        return Err(err("a numeral is all upper or all lower case"));
    }
    let degree = ["I", "II", "III", "IV", "V", "VI", "VII"]
        .iter()
        .position(|n| n.eq_ignore_ascii_case(&letters))
        .ok_or_else(|| err("not a numeral from I to VII"))?;
    let suffix = &rest[letters.len()..];
    let (_, _, tones, pads) = QUALITIES.iter().find(|q| q.0 == suffix && q.1 == upper).ok_or_else(|| err(&format!("unknown quality '{suffix}'")))?;
    let root = (mode.numerals()[degree] + shift).rem_euclid(12);
    let bass_iv = match slash {
        None => 0,
        Some("3") => tones[1],
        Some("5") => tones[2],
        Some("7") => *tones.get(3).ok_or_else(|| err("/7 on a chord without a seventh"))?,
        Some(x) => return Err(err(&format!("a slash names the chord's 3, 5 or 7, not '{x}'"))),
    };
    Ok(Chord { symbol: symbol.trim().to_string(), root, tones: tones.to_vec(), pads: *pads, bass: (root + bass_iv).rem_euclid(12), deceptive })
}

/// Parse the whole text into progressions, voiced.
pub fn parse(text: &str) -> Result<Vec<Progression>, String> {
    let mut out: Vec<Progression> = Vec::new();
    let mut head: Option<(String, Vec<String>)> = None;
    let mut sections: Vec<(String, Vec<String>)> = Vec::new();
    for line in text.lines() {
        if let Some(h) = line.strip_prefix("== ") {
            if let Some(s) = head.take() {
                sections.push(s);
            }
            head = Some((h.trim().to_string(), Vec::new()));
        } else if let Some((_, lines)) = head.as_mut() {
            lines.push(line.to_string());
        }
    }
    sections.extend(head);
    for (h, lines) in sections {
        let parts: Vec<&str> = h.split('|').map(str::trim).collect();
        let name = parts[0].to_string();
        let e = |why: String| format!("{name}: {why}");
        if out.iter().any(|p| p.name == name) {
            return Err(e("written twice".into()));
        }
        let mode = parts.get(1).and_then(|m| Mode::from_name(m)).ok_or_else(|| e("no mode".into()))?;
        let (mut phrase, mut energy, mut density, mut mood) = (0usize, 0u8, 0u8, Vec::new());
        for p in &parts[2..] {
            let mut w = p.split_whitespace();
            match w.next() {
                Some("phrase") => phrase = w.next().and_then(|x| x.parse().ok()).unwrap_or(0),
                Some("energy") => energy = w.next().and_then(|x| x.parse().ok()).unwrap_or(0),
                Some("density") => density = w.next().and_then(|x| x.parse().ok()).unwrap_or(0),
                Some("mood") => mood = w.map(String::from).collect(),
                _ => return Err(e(format!("unknown tag '{p}'"))),
            }
        }
        if ![4, 8].contains(&phrase) {
            return Err(e("a phrase is 4 or 8 bars".into()));
        }
        if !(1..=3).contains(&energy) || !(1..=3).contains(&density) || mood.is_empty() {
            return Err(e("energy and density run 1 to 3, and a mood is needed".into()));
        }
        let describe = lines.iter().find_map(|l| l.trim().strip_prefix('#').map(|d| d.trim().to_string())).ok_or_else(|| e("no description".into()))?;
        let bar_line = lines.iter().map(|l| l.trim()).find(|l| l.starts_with('|')).ok_or_else(|| e("no bars".into()))?;
        let mut bars: Vec<Bar> = Vec::new();
        for cell in bar_line.trim_matches('|').split('|').map(str::trim) {
            let syms: Vec<&str> = cell.split_whitespace().collect();
            if syms == ["%"] {
                let last = bars.last().and_then(|b| b.chords.last()).ok_or_else(|| e("% with no chord before it".into()))?.clone();
                bars.push(Bar { chords: vec![last], held: true });
                continue;
            }
            if syms.is_empty() || syms.len() > 2 {
                return Err(e(format!("a bar holds one or two chords: '{cell}'")));
            }
            let chords = syms.iter().map(|s| parse_chord(s, mode)).collect::<Result<Vec<_>, _>>().map_err(e)?;
            bars.push(Bar { chords, held: false });
        }
        if bars.len() != 8 {
            return Err(e(format!("{} bars; a stage is 8", bars.len())));
        }
        let slots: Vec<Chord> = bars.iter().flat_map(|b| if b.chords.len() == 2 { b.chords.clone() } else { vec![b.chords[0].clone(); 2] }).collect();
        let voicing = voice_lead(&slots).map_err(e)?;
        out.push(Progression { name, mode, phrase, energy, density, mood, describe, bars, slots, voicing });
    }
    Ok(out)
}

/// Every progression, parsed and voiced once.
pub fn all() -> &'static [Progression] {
    static ALL: OnceLock<Vec<Progression>> = OnceLock::new();
    ALL.get_or_init(|| parse(TEXT).unwrap_or_else(|e| panic!("harmony.txt does not parse (tests/harmony.rs): {e}")))
}

// ------------------------------------------------------------------------------------------------ voicing

/// The register of pad voice m: (lowest, highest), semitones above the tonic.
pub fn band(m: usize) -> (i32, i32) {
    let lo = BAND_LOW + BAND_STEP * m as i32;
    (lo, lo + BAND_SPAN)
}

/// Every way to voice `chord` on the three pads: low < middle < high, each in its band, adjacent voices at least `gap`
/// apart, the three pitch classes exactly the chord's pad tones.
pub fn candidates(chord: &Chord, gap: i32) -> Vec<[i32; 3]> {
    let mut want: Vec<i32> = chord.pads.iter().map(|p| (chord.root + p).rem_euclid(12)).collect();
    want.sort();
    let mut out = Vec::new();
    let (b0, b1, b2) = (band(0), band(1), band(2));
    for p0 in b0.0..=b0.1 {
        for p1 in (p0 + gap).max(b1.0)..=b1.1 {
            for p2 in (p1 + gap).max(b2.0)..=b2.1 {
                let mut got = [p0.rem_euclid(12), p1.rem_euclid(12), p2.rem_euclid(12)];
                got.sort();
                if got.to_vec() == want {
                    out.push([p0, p1, p2]);
                }
            }
        }
    }
    out
}

/// Total movement from one voicing to the next: the sum of each voice's distance.
pub fn movement(a: &[i32; 3], b: &[i32; 3]) -> i32 {
    a.iter().zip(b).map(|(x, y)| (x - y).abs()).sum()
}

/// Distance from the middle of each voice's band.
pub fn off_centre(v: &[i32; 3]) -> i32 {
    (0..3).map(|m| (2 * v[m] - band(m).0 - band(m).1).abs()).sum()
}

/// A chord's candidates, relaxing the gap to a semitone only when nothing fits with the usual one.
pub fn options(chord: &Chord) -> Result<Vec<[i32; 3]>, String> {
    let c = candidates(chord, MIN_GAP);
    if !c.is_empty() {
        return Ok(c);
    }
    let c = candidates(chord, 1);
    if c.is_empty() {
        Err(format!("no voicing of {} fits the pads' registers", chord.symbol))
    } else {
        Ok(c)
    }
}

/// Voice the chords of each half bar: the first as near the middle of the registers as it can be, every next one with
/// the smallest total movement from the one before (ties: nearer the middle, then the lowest).
pub fn voice_lead(slots: &[Chord]) -> Result<Vec<[i32; 3]>, String> {
    let mut out: Vec<[i32; 3]> = Vec::new();
    for c in slots {
        let opts = options(c)?;
        let best = match out.last() {
            None => opts.iter().min_by_key(|v| (off_centre(v), **v)),
            Some(prev) => opts.iter().min_by_key(|v| (movement(prev, v), off_centre(v), **v)),
        };
        out.push(*best.expect("options are never empty"));
    }
    Ok(out)
}

// ------------------------------------------------------------------------------------------------ the tables

/// The bass as one digit a half bar: the step (7, fifths on the circle of fifths; or 1, half steps, for a chromatic line
/// the circle cannot hold in ten digits), the origin, and the digits, with `pitch class = step (digit + origin) mod 12`.
/// Fifths first; the origin is the one that keeps the largest digit smallest.
pub fn bass_digits(p: &Progression) -> Result<(i32, i32, Vec<i64>), String> {
    for step in [7, 1] {
        // 7 and 1 are their own inverses mod 12, so the position of a pitch class is step times it.
        let pos: Vec<i32> = p.slots.iter().map(|c| (step * c.bass).rem_euclid(12)).collect();
        let mut best: Option<(i32, Vec<i64>)> = None;
        for o in -6..6 {
            let d: Vec<i64> = pos.iter().map(|f| (f - o).rem_euclid(12) as i64).collect();
            let max = *d.iter().max().expect("16 slots");
            if max > 9 || table(&d) >= 1 << 53 {
                continue;
            }
            if best.as_ref().map_or(true, |b| max < *b.1.iter().max().expect("16")) {
                best = Some((o, d));
            }
        }
        if let Some((o, d)) = best {
            return Ok((step, o, d));
        }
    }
    Err(format!("{}: its bass spans more than ten fifths and more than ten half steps", p.name))
}

/// Sixteen digits as one integer, the first slot leftmost.
pub fn table(digits: &[i64]) -> i64 {
    digits.iter().fold(0i64, |n, d| n * 10 + d)
}

/// Pad voice m's digits: its pitch above the bottom of its band, one a half bar.
pub fn pad_digits(p: &Progression, m: usize) -> Vec<i64> {
    p.voicing.iter().map(|v| (v[m] - band(m).0) as i64).collect()
}

/// The definitions a progression's module writes: the melody's mode `c`, the bass's step, origin and table, the pads'
/// tables.
pub fn defs(p: &Progression) -> Result<Vec<Def>, String> {
    let (step, o, bass) = bass_digits(p)?;
    let mut d = vec![
        Def::new("c", format!("c = {}", p.mode.lens())),
        Def::new("Hm", format!("Hm = {step}")),
        Def::new("Ho", format!("Ho = {o}")),
        Def::new("Hb", format!("Hb = {}", table(&bass))),
    ];
    for (m, n) in ["Hx", "Hy", "Hz"].iter().enumerate() {
        d.push(Def::new(n, format!("{n} = {}", table(&pad_digits(p, m)))));
    }
    Ok(d)
}

/// The pitch classes of the chord at half bar `slot` of a song with written harmony (its bass note and the pads'), read
/// from the song's own tables in exact integers, so it works whether or not the song's voices read `Hr` and `Hv`. None
/// when the song carries no progression.
pub fn chord_tones(program: &crate::expr::Program, slot: usize) -> Option<Vec<i64>> {
    let c = |n: &str| program.call(n, &[]).ok().filter(|v| v.is_finite() && v.fract() == 0.0).map(|v| v as i64);
    let (k, step, origin) = (c("K")?, c("Hm")?, c("Ho")?);
    let digit = |n: i64| (n / 10i64.pow(15 - (slot % SLOTS) as u32)) % 10;
    let mut v = vec![(k + ((digit(c("Hb")?) + origin) * step).rem_euclid(12)).rem_euclid(12)];
    for (m, n) in ["Hx", "Hy", "Hz"].iter().enumerate() {
        v.push((k + (BAND_LOW + BAND_STEP * m as i32) as i64 + digit(c(n)?)).rem_euclid(12));
    }
    Some(v)
}

/// The chords as written, bar by bar (`Imaj9 | IVmaj7 V7 | ...`).
pub fn chart(p: &Progression) -> String {
    p.bars.iter().map(|b| if b.held { "%".to_string() } else { b.chords.iter().map(|c| c.symbol.as_str()).collect::<Vec<_>>().join(" ") }).collect::<Vec<_>>().join(" | ")
}

/// The progression as a module of the progression slot, with id `prefix` + `prog.` + its name. Its mood carries
/// `minor` or `major` and, when it holds a deceptive turn, `deceptive` (the stream uses one such once a song).
pub fn module(p: &Progression, prefix: &str) -> Module {
    let mut mood = p.mood.clone();
    mood.push(if p.mode.minor() { "minor".into() } else { "major".into() });
    if p.deceptive() {
        mood.push("deceptive".into());
    }
    Module {
        id: format!("{prefix}prog.{}", p.name),
        slot: Slot::Progression,
        energy: p.energy,
        density: p.density,
        mood,
        glide: false,
        describe: format!("{} ({}: {})", p.describe, p.mode.name(), chart(p)),
        defs: defs(p).unwrap_or_else(|e| panic!("{e}")),
        blocks: Vec::new(),
    }
}

/// The coda's progression: the first four bars of `p`, then IV (or iv), V and the tonic held for the last two bars, so
/// the ending's final ritard lands on home at bar 6. When `p`'s fourth bar is already a dominant, bar 5 opens on the
/// tonic before the IV, so the V never falls back to IV.
pub fn coda(p: &Progression) -> Progression {
    let minor = p.mode.minor();
    let ch = |s: &str| parse_chord(s, p.mode).expect("the coda's chords parse");
    let four = if minor { ch("iv9") } else { ch("IVmaj7") };
    let tonic = if minor { ch("i9") } else { ch("Imaj9") };
    let dom = if minor { vec![ch("V7b9")] } else { vec![ch("V7sus"), ch("V7")] };
    let mut bars: Vec<Bar> = p.bars[..4].to_vec();
    let fifth_bar = if p.bars[3].chords.last().is_some_and(|c| c.is_dominant()) { vec![tonic.clone(), four] } else { vec![four] };
    bars.push(Bar { chords: fifth_bar, held: false });
    bars.push(Bar { chords: dom, held: false });
    bars.push(Bar { chords: vec![tonic.clone()], held: false });
    bars.push(Bar { chords: vec![tonic], held: true });
    let slots: Vec<Chord> = bars.iter().flat_map(|b| if b.chords.len() == 2 { b.chords.clone() } else { vec![b.chords[0].clone(); 2] }).collect();
    let voicing = voice_lead(&slots).expect("the coda's chords voice");
    Progression { name: format!("{}.coda", p.name), phrase: 8, describe: format!("the ending of {}: its first four bars, then home", p.name), bars, slots, voicing, ..p.clone() }
}
