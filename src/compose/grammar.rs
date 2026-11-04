//! The grammar of graph songs: a seed becomes a set of choices, and the choices become a complete song file in our own
//! language, in the style of `examples/graph_*.txt` (one generator, a scale lens, graph instruments).
//!
//! The three hand-written graph songs share one shape, and this module is that shape with its constants opened up:
//!
//! - **a generator** `G`, a function the whole song grows from (a fractal, square waves at several time scales, a
//!   Lissajous pair, or an integer sequence read modulo m: Fibonacci, primes, Collatz, rule 30);
//! - **chord roots** `R(i)`, the generator read slowly, one value a bar (the Takagi song's form, the Pisano song's
//!   roots);
//! - **a melody** `M(k)`, the root plus the generator read quickly, at one or several time scales (the Takagi lead is
//!   the curve at the phrase, the bar and the beat added together);
//! - **a lens** `h(d) = floor((12d + c) / 7)` that puts every degree on a mode, whose `c` may change by section (the
//!   Lissajous song brightens Dorian to Lydian);
//! - **roles**: pads stacked in thirds on the root, a bass on the root, a lead (and optionally a harmony a third or a
//!   fifth above it, an arpeggio, a contour or a just-ratio drone), and drums that are downward pitch dashes, triggered
//!   by a pattern, by bits, or by events in the generator;
//! - **a form**: four sections of four bars, each layer playing in some of them (an order list written as the digits of
//!   one integer, like the reference's chord roots).
//!
//! The output is plain song text: it parses, renders and can be edited by hand like any example.

use super::rng::Rng;
use crate::song::Song;

/// The generator families.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Family {
    Fractal,
    Squares,
    Lissajous,
    Fibonacci,
    Primes,
    Collatz,
    Rule30,
}

impl Family {
    pub const ALL: [Family; 7] = [
        Family::Fractal,
        Family::Squares,
        Family::Lissajous,
        Family::Fibonacci,
        Family::Primes,
        Family::Collatz,
        Family::Rule30,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Family::Fractal => "fractal",
            Family::Squares => "squares",
            Family::Lissajous => "lissajous",
            Family::Fibonacci => "fibonacci",
            Family::Primes => "primes",
            Family::Collatz => "collatz",
            Family::Rule30 => "rule30",
        }
    }
}

/// What a generator family hands the role mappings: its definitions and, as expressions, each thing the roles read.
#[derive(Debug, Clone, Default)]
pub struct Gen {
    /// Helper definitions, written first.
    pub defs: Vec<String>,
    /// The chord root degree of bar `i`.
    pub root: String,
    /// The melody degree of lead step `k` (it may call `R`).
    pub melody: String,
    /// A loudness in about 0..1 for the hat at sixteenth `n`.
    pub hat_amp: String,
    /// The hat's own trigger, if the family has one (the Lissajous ratio).
    pub hat_trigger: Option<String>,
    /// The clap's own trigger, if the family has one.
    pub clap_trigger: Option<String>,
    /// A rest mask over lead steps `k` (1 = play), if the family has one.
    pub rest: Option<String>,
    /// A kick condition over sixteenths `n` drawn from the generator.
    pub kick_seq: String,
    /// An index 0..3 over sixteenths `n`, for the arpeggio.
    pub arp_index: String,
    /// 0..2 over beats `x`, for a walking bass (root, third, fifth).
    pub walk: String,
    /// A slow degree curve over `t`, for a contour voice, if the family is continuous.
    pub contour: Option<String>,
    /// Whether the family sounds its ratio as a just interval (Lissajous).
    pub ratio_drone: bool,
    /// The lens this family prefers, if any.
    pub lens: Option<Lens>,
    /// One sentence each: what the generator is and how the roles read it.
    pub notes: Vec<String>,
    /// The family's parameters, for scores.md.
    pub params: Vec<(String, String)>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Lens {
    /// One mode for the whole song: c in 0..6 (2 Aeolian, 3 Dorian, 4 Mixolydian, 5 Ionian, 6 Lydian).
    Fixed(i64),
    /// c = c0 + s: one sharp brighter each section (the Lissajous song).
    Brighten(i64),
    /// c = 6 - s: one flat darker each section.
    Darken,
    /// c read per section from four digits.
    Table([i64; 4]),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Rest {
    None,
    /// Euclid E(k, 8) over lead steps, rotated.
    Euclid(i64, i64),
    /// The family's own mask.
    Family,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Bass {
    Beats,
    Walk,
    Octaves,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kick {
    Four,
    Half,
    /// Euclid E(k, 16), rotated.
    Euclid(i64, i64),
    /// Where the melody falls by this many degrees or more (the Takagi kick).
    Fall(i64),
    /// From the generator's own sequence.
    Seq,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Clap {
    /// A tonal voice whose pitch dashes down, on beats 2 and 4.
    BackbeatTone,
    /// A dash drum on beats 2 and 4.
    BackbeatDash,
    /// The family's own trigger.
    Family,
    None,
}

/// Every choice that makes a song. A seed samples one; `write` turns it into text.
#[derive(Debug, Clone)]
pub struct Choices {
    pub seed: u64,
    pub index: u64,
    pub family: Family,
    pub gen: Gen,
    pub bpm: f64,
    /// Semitones added to every pitch: the key, relative to A.
    pub key: i64,
    pub lens: Lens,
    /// Lead steps per beat: 2 (eighths) or 4 (sixteenths).
    pub rate: i64,
    pub legato: bool,
    pub gate_len: f64,
    /// Where in the step the glide into the next note starts, or none.
    pub glide: Option<f64>,
    pub rest: Rest,
    pub lead_partials: usize,
    pub lead_gain: f64,
    /// Octave placement of the lead (semitones, a multiple of 12), set so the lead sits around A4 to A5.
    pub lead_octave: i64,
    /// Degrees above the lead for a harmony voice: 2 (a third) or 4 (a fifth).
    pub harmony: Option<i64>,
    pub arp: bool,
    pub pads: usize,
    pub contour: bool,
    pub bass: Bass,
    pub kick: Kick,
    pub clap: Clap,
    pub hat_eighths: bool,
    pub form: usize,
    pub reverb_size: f64,
    pub predelay: f64,
    pub delay_beats: f64,
    pub feedback: f64,
    pub warmth: f64,
}

/// A written song and what made it.
#[derive(Debug, Clone)]
pub struct Draft {
    pub text: String,
    pub choices: Choices,
}

impl Draft {
    /// The choices as name = value pairs, for scores.md.
    pub fn params(&self) -> Vec<(String, String)> {
        let c = &self.choices;
        let mut p = vec![
            ("family".to_string(), c.family.name().to_string()),
            ("bpm".into(), num(c.bpm)),
            ("key".into(), c.key.to_string()),
            ("lens".into(), format!("{:?}", c.lens)),
            ("lead steps per beat".into(), c.rate.to_string()),
            ("legato".into(), c.legato.to_string()),
            ("glide".into(), format!("{:?}", c.glide)),
            ("rest".into(), format!("{:?}", c.rest)),
            ("harmony".into(), format!("{:?}", c.harmony)),
            ("arp".into(), c.arp.to_string()),
            ("pads".into(), c.pads.to_string()),
            ("contour".into(), c.contour.to_string()),
            ("bass".into(), format!("{:?}", c.bass)),
            ("kick".into(), format!("{:?}", c.kick)),
            ("clap".into(), format!("{:?}", c.clap)),
            ("hats".into(), if c.hat_eighths { "eighths" } else { "sixteenths" }.into()),
            ("form".into(), FORMS[c.form].0.to_string()),
        ];
        p.extend(c.gen.params.iter().cloned());
        p
    }
}

/// A number as the song language writes it: no exponent, at most four decimals, no trailing zeros.
pub fn num(x: f64) -> String {
    let r = (x * 10_000.0).round() / 10_000.0;
    let r = if r == 0.0 { 0.0 } else { r };
    let s = format!("{r:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    s.to_string()
}

/// ` + x` or ` - x`, so a written expression never holds `+ -x`.
fn plus(x: f64) -> String {
    if x < 0.0 {
        format!(" - {}", num(-x))
    } else {
        format!(" + {}", num(x))
    }
}

// ------------------------------------------------------------------------------------------------ the instruments
// Taken from the hand-written graph songs, which took them from measuring the reference recordings
// (docs/research/graph_sound.md).

const LEAD_PARTIALS: [&str; 3] = [
    "0, -16, -4.5, -15, -11, -18.4, -16.1, -22.9, -22.9, -28",
    "0, -12, -5, -11, -9, -12, -11, -13",
    "0, -4, -6, -12, -10, -16, -15, -21",
];
const PAD_PARTIALS: &str = "0, -12, -5, -11, -9, -12, -11, -13, -12, -14, -13, -15, -14, -15, -15, -16";
const BASS_PARTIALS: &str = "-11, 0, -26, -12, -28, -20, -32, -21";

/// The forms: a name, and for each layer the sections (of four bars) it plays in, as four digits.
/// Layers: pads, bass, lead, harmony, arp, kick, clap, hat.
pub const FORMS: [(&str, [&str; 8]); 4] = [
    ("build: one layer more each section", ["1111", "0111", "0111", "0001", "0011", "0111", "0011", "1111"]),
    (
        "build, break, return: the drums and bass drop out for section 3",
        ["1111", "1101", "0111", "0001", "0010", "0101", "0101", "1101"],
    ),
    ("the hand songs' shape: all in, the lead from bar 5", ["1111", "1111", "0111", "0011", "0111", "1111", "1111", "1111"]),
    ("an arch: thin, full, full, thinning", ["1111", "0111", "1110", "0100", "0111", "0110", "0110", "0111"]),
];

// ------------------------------------------------------------------------------------------------ the families

fn fractal(r: &mut Rng, spb: i64, rate: i64) -> Gen {
    let w: f64 = *r.pick(&[0.5, 0.5, 0.6, 0.7]);
    let terms = r.range(5, 6);
    let mut t = String::from("T(u) = tri(u)");
    for k in 1..terms {
        let m = 1i64 << k;
        t += &format!(" + {} tri({m}u)", num(w.powi(k as i32)));
    }
    let ra = r.range(6, 8);
    let rp = *r.pick(&[0.0, 0.0, 0.25]);
    let root = if rp == 0.0 { format!("floor({ra} T(i / 16))") } else { format!("floor({ra} T(i / 16 + {}))", num(rp)) };
    // Three time scales from: four bars, two bars, a bar, half a bar, a beat (in lead steps).
    let scales_all = [4 * spb, 2 * spb, spb, spb / 2, rate];
    let mut idx: Vec<usize> = (0..5).collect();
    for i in (1..idx.len()).rev() {
        let j = r.range(0, i as i64) as usize;
        idx.swap(i, j);
    }
    let mut pick: Vec<i64> = idx[..3].iter().map(|&i| scales_all[i]).collect();
    pick.sort_unstable_by(|a, b| b.cmp(a));
    pick.dedup();
    let amps: Vec<i64> = pick.iter().map(|_| r.range(3, 7)).collect();
    let sum: Vec<String> = pick.iter().zip(&amps).map(|(s, a)| format!("{a} T(k / {s})")).collect();
    let melody = format!("R(floor(k / {spb})) + floor({})", sum.join(" + "));
    let ca = *r.pick(&[10, 12, 14]);
    Gen {
        defs: vec!["tri(v) = abs(v - floor(v + 0.5))".into(), t],
        root,
        melody,
        hat_amp: "2 T(n / 8)".into(),
        kick_seq: "floor(6 T(n / 16)) > floor(6 T((n - 1) / 16))".into(),
        arp_index: "floor(4 T(n / 8))".into(),
        walk: "floor(3 T(x / 8))".into(),
        contour: Some(format!("{ca} T(t / 64)")),
        notes: vec![
            format!(
                "T(u) sums {terms} tents at halving sizes, each {} the height of the last (the Takagi-Landsberg curve), so every stretch looks like the whole.",
                num(w)
            ),
            format!("The chord roots are T across the whole song, one sample a bar, times {ra}."),
            format!(
                "The melody adds T at {} time scales ({} lead steps), so each short figure has the shape of the longer ones.",
                pick.len(),
                pick.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(", ")
            ),
        ],
        params: vec![
            ("takagi weight".into(), num(w)),
            ("takagi terms".into(), terms.to_string()),
            ("root scale".into(), ra.to_string()),
            ("melody scales".into(), format!("{pick:?}")),
            ("melody amplitudes".into(), format!("{amps:?}")),
        ],
        ..Gen::default()
    }
}

fn squares(r: &mut Rng, spb: i64) -> Gen {
    // Roots: two square waves at bar scales.
    let mut periods = vec![16, 8, 4, 2];
    let p1 = periods.remove(r.range(0, 3) as usize);
    let p2 = periods.remove(r.range(0, 2) as usize);
    let c1 = *r.pick(&[1, 2, 3, -2]);
    let c2 = *r.pick(&[1, 2, -1, -2, 3]);
    let root = format!("{} sq(i / {p1}){}", c1, if c2 < 0 { format!(" - {} sq(i / {p2})", -c2) } else { format!(" + {c2} sq(i / {p2})") });
    let n = r.range(3, 4) as usize;
    let mut scales = vec![2i64, 4, 8, 16, 32, 64];
    let mut terms = Vec::new();
    let mut desc = Vec::new();
    for _ in 0..n {
        let s = scales.remove(r.range(0, scales.len() as i64 - 1) as usize);
        let c = *r.pick(&[-3, -2, -1, 1, 2, 2, 3, 4]);
        let ph = *r.pick(&[0.0, 0.0, 0.25, 0.5]);
        let arg = if ph == 0.0 { format!("k / {s}") } else { format!("k / {s} + {}", num(ph)) };
        terms.push(if terms.is_empty() {
            format!("{c} sq({arg})")
        } else if c < 0 {
            format!(" - {} sq({arg})", -c)
        } else {
            format!(" + {c} sq({arg})")
        });
        desc.push(format!("{c} x period {s}"));
    }
    let melody = format!("R(floor(k / {spb})) + {}", terms.concat());
    Gen {
        defs: vec!["sq(v) = floor(2 frac(v))".into()],
        root,
        melody,
        hat_amp: "(1 + sq(n / 4) + sq(n / 16 + 0.5)) / 3".into(),
        kick_seq: "sq(n / 16) + sq(n / 4 + 0.25) == 0".into(),
        arp_index: "sq(n / 2) + 2 sq(n / 8)".into(),
        walk: "sq(x / 4) + sq(x / 2)".into(),
        notes: vec![
            "sq(v) is a square wave, 0 for the first half of each cycle and 1 for the second: sums of them at time scales an octave apart are binary counting, read as music (Walsh functions).".into(),
            format!("The chord roots are two square waves {p1} and {p2} bars long."),
            format!("The melody is the root plus {n} square waves: {}.", desc.join(", ")),
        ],
        params: vec![("root periods (bars)".into(), format!("{p1}, {p2}")), ("melody terms".into(), desc.join("; "))],
        ..Gen::default()
    }
}

fn lissajous(r: &mut Rng, spb: i64, rate: i64) -> Gen {
    let climb = r.chance(0.6);
    let (p, q, ratios, lens) = if climb {
        let p0 = r.range(2, 3);
        let ratios = (0..4).map(|s| format!("{}:{}", p0 + s, p0 + s - 1)).collect::<Vec<_>>().join(", ");
        (format!("s + {p0}"), format!("s + {}", p0 - 1), ratios, Lens::Brighten(*r.pick(&[2, 3])))
    } else {
        ("5 - s".to_string(), "4 - s".to_string(), "5:4, 4:3, 3:2, 2:1".to_string(), Lens::Darken)
    };
    let ra = r.range(0, 2);
    let root = if ra == 0 { "0".to_string() } else { format!("floor({ra} Y(4 i) + 0.5)") };
    let ma = *r.pick(&[2.5, 3.0, 3.5]);
    let melody = format!("R(floor(k / {spb})) + floor({} (X(k / {rate}) + Y(k / {rate})) + 0.5)", num(ma));
    Gen {
        defs: vec![
            format!("p = {p}"),
            format!("q = {q}"),
            "X(x) = sin(p tau x / 16)".into(),
            "Y(x) = cos(q tau x / 16)".into(),
            "z = (p - q) / (p + q)".into(),
            "ji = 34.6241 (z + z^3/3 + z^5/5 + z^7/7)".into(),
        ],
        root,
        melody,
        hat_amp: "0.6 + 0.4 (frac(p t / 2) < 0.5)".into(),
        hat_trigger: Some("0.5 - frac(p t)".into()),
        clap_trigger: Some("0.5 - frac(q t / 4 + 0.5)".into()),
        kick_seq: "X(n / 4) > X((n - 1) / 4) + 0.3".into(),
        arp_index: "floor(2 + 1.9 Y(n / 4))".into(),
        walk: "mod(floor(1.5 + 1.4 X(x)), 3)".into(),
        contour: Some("3 + 3 Y(t)".into()),
        ratio_drone: true,
        lens: Some(lens),
        notes: vec![
            format!("A Lissajous figure X = sin(p th), Y = cos(q th), th turning once a section, whose ratio p:q goes {ratios}."),
            "The ratio is also the harmony (a drone sounds it as a just interval, 12 log2(p/q), from the log series) and the rhythm (p hats a beat, q claps a bar).".into(),
            format!("The melody plucks {} (X + Y), so its range swells and shrinks where the two curves reinforce and cancel.", num(ma)),
        ],
        params: vec![("ratios".into(), ratios), ("root amplitude".into(), ra.to_string()), ("melody amplitude".into(), num(ma))],
        ..Gen::default()
    }
}

/// The Pisano period of m: the period of the Fibonacci numbers modulo m.
pub fn pisano(m: u64) -> u64 {
    let (mut a, mut b) = (0u64, 1u64);
    for i in 1..=m * m * 6 {
        let c = (a + b) % m;
        a = b;
        b = c;
        if a == 0 && b == 1 {
            return i;
        }
    }
    unreachable!("every modulus has a Pisano period of at most 6m")
}

fn fibonacci(r: &mut Rng, spb: i64) -> Gen {
    let m = *r.pick(&[5u64, 7, 7, 8]);
    let p = pisano(m);
    let h = m / 2;
    Gen {
        defs: vec![
            "phi = (1 + sqrt(5)) / 2".into(),
            "F(k) = floor(phi^k / sqrt(5) + 0.5)".into(),
            "fm(k, m, P) = mod(F(mod(k, P)), m)".into(),
            format!("sg(v) = mod(v + {h}, {m}) - {h}"),
        ],
        root: format!("sg(fm(i, {m}, {p}))"),
        melody: format!("R(floor(k / {spb})) + sg(fm(k, {m}, {p}))"),
        hat_amp: "(1 + fm(n, 5, 20)) / 5".into(),
        rest: Some("fm(k, 2, 3)".into()),
        kick_seq: "fm(mod(n, 16), 8, 12) == 0".into(),
        arp_index: "fm(n, 4, 6)".into(),
        walk: "fm(x, 3, 8)".into(),
        notes: vec![
            format!("F(k) is Binet's formula, exact to k = 70, read modulo {m}, where it repeats every {p} (its Pisano period)."),
            format!("The chord roots are F of the bar number mod {m}, and the melody adds F of the step mod {m}, as signed degrees."),
            "Other voices read F modulo other numbers, whose periods (3, 6, 8, 12, 20) cycle against each other.".into(),
        ],
        params: vec![("modulus".into(), m.to_string()), ("pisano period".into(), p.to_string())],
        ..Gen::default()
    }
}

/// The first `count` primes.
pub fn primes(count: usize) -> Vec<u64> {
    let mut out = Vec::new();
    let mut n = 2u64;
    while out.len() < count {
        if (2..n).take_while(|d| d * d <= n).all(|d| n % d != 0) {
            out.push(n);
        }
        n += 1;
    }
    out
}

/// The Collatz trajectory of `n0`, `count` terms long (after 1 it cycles 4, 2, 1).
pub fn collatz(n0: u64, count: usize) -> Vec<u64> {
    let mut out = Vec::with_capacity(count);
    let mut n = n0;
    for _ in 0..count {
        out.push(n);
        n = if n % 2 == 0 { n / 2 } else { 3 * n + 1 };
    }
    out
}

/// The centre column of Wolfram's rule 30 from a single live cell, `count` bits.
pub fn rule30(count: usize) -> Vec<u8> {
    let w = 2 * count + 3;
    let mut cells = vec![0u8; w];
    cells[w / 2] = 1;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        out.push(cells[w / 2]);
        let mut next = vec![0u8; w];
        for i in 1..w - 1 {
            let pat = (cells[i - 1] << 2) | (cells[i] << 1) | cells[i + 1];
            next[i] = (30u8 >> pat) & 1;
        }
        cells = next;
    }
    out
}

/// Fifteen values (each 0..9) as the digits of one integer, read back by `D(i)` with period 15.
pub fn digits(values: &[u64]) -> u64 {
    assert_eq!(values.len(), 15);
    values.iter().fold(0u64, |acc, v| {
        assert!(*v < 10);
        acc * 10 + v
    })
}

fn digit_family(r: &mut Rng, spb: i64, family: Family) -> Gen {
    let m = *r.pick(&[5u64, 7, 7]);
    let h = m / 2;
    let (values, what, param) = match family {
        Family::Primes => {
            let start = r.range(0, 6) as usize;
            let ps = primes(start + 15);
            let v: Vec<u64> = ps[start..].iter().map(|p| p % m).collect();
            (v, format!("the primes from {} on", ps[start]), ("first prime".to_string(), ps[start].to_string()))
        }
        _ => {
            let mut n0;
            loop {
                n0 = r.range(6, 99) as u64;
                if collatz(n0, 40).iter().position(|&x| x == 1).unwrap_or(40) >= 10 {
                    break;
                }
            }
            let v: Vec<u64> = collatz(n0, 15).iter().map(|x| x % m).collect();
            (v, format!("the Collatz trajectory of {n0}"), ("start".to_string(), n0.to_string()))
        }
    };
    let n = digits(&values);
    let ro = r.range(0, 14);
    Gen {
        defs: vec![
            format!("D(i) = mod(floor({n} / 10^(14 - mod(i, 15))), 10)"),
            format!("sg(v) = mod(v + {h}, {m}) - {h}"),
        ],
        root: if ro == 0 { "sg(D(i))".into() } else { format!("sg(D(i + {ro}))") },
        melody: format!("R(floor(k / {spb})) + sg(D(k))"),
        hat_amp: format!("(1 + D(n)) / {m}"),
        rest: Some("mod(D(k + 1), 2) + (D(k) == D(k + 1))".into()),
        kick_seq: "mod(D(mod(n, 16)), 3) == 0".into(),
        arp_index: "mod(D(n), 4)".into(),
        walk: "mod(D(x), 3)".into(),
        notes: vec![
            format!("D(i) reads {what}, each modulo {m}, stored as the fifteen digits of {n} (the reference's chord-root trick)."),
            format!("The chord roots are D a bar at a time{}; the melody adds D a step at a time, as signed degrees.", if ro == 0 { String::new() } else { format!(", offset by {ro}") }),
        ],
        params: vec![param, ("modulus".into(), m.to_string()), ("digits".into(), n.to_string())],
        ..Gen::default()
    }
}

fn rule30_family(r: &mut Rng, spb: i64) -> Gen {
    let bits = rule30(48);
    let n = bits.iter().fold(0u64, |acc, b| acc * 2 + *b as u64);
    let ko = r.range(1, 20);
    Gen {
        defs: vec![format!("B(i) = mod(floor({n} / 2^(47 - mod(i, 48))), 2)")],
        root: "B(3i) + 2 B(3i + 1) + 4 B(3i + 2) - 3".into(),
        melody: format!("R(floor(k / {spb})) + B(k) + 2 B(k + 1) + 4 B(k + 2) - 3"),
        hat_amp: "(1 + B(n) + B(n + 1)) / 3".into(),
        rest: Some("max(B(k + 5), B(k + 6))".into()),
        kick_seq: format!("max(min(B(n + {ko}), B(n + {})), mod(n, 16) == 0)", ko + 1),
        arp_index: "B(n) + 2 B(n + 9)".into(),
        walk: "B(x) + B(x + 17)".into(),
        notes: vec![
            format!("B(i) is the centre column of Wolfram's rule 30 grown from one cell, 48 bits stored as the integer {n}."),
            "The melody reads three bits at a time as a binary number, and the chord roots read them three bars apart.".into(),
        ],
        params: vec![("bits".into(), n.to_string()), ("kick offset".into(), ko.to_string())],
        ..Gen::default()
    }
}

// ------------------------------------------------------------------------------------------------ sampling

/// A generator of `family`, for a lead of `rate` steps a beat (`spb` steps a bar). Stream mode uses it to give a
/// melody a new generator.
pub fn generator(family: Family, r: &mut Rng, spb: i64, rate: i64) -> Gen {
    match family {
        Family::Fractal => fractal(r, spb, rate),
        Family::Squares => squares(r, spb),
        Family::Lissajous => lissajous(r, spb, rate),
        Family::Fibonacci => fibonacci(r, spb),
        Family::Primes | Family::Collatz => digit_family(r, spb, family),
        Family::Rule30 => rule30_family(r, spb),
    }
}

/// Sample a song's choices from a run seed and a candidate index.
pub fn sample(seed: u64, index: u64) -> Choices {
    let mut r = Rng::for_candidate(seed, index);
    let family = *r.pick(&Family::ALL);
    let rate = if r.chance(0.5) { 2 } else { 4 };
    let spb = 4 * rate;
    let gen = generator(family, &mut r, spb, rate);
    let lens = match gen.lens {
        Some(l) => l,
        None => match r.weighted(&[3.0, 1.0, 1.0]) {
            0 => Lens::Fixed(*r.pick(&[2, 3, 4, 4, 5, 5, 5, 6])),
            1 => Lens::Brighten(*r.pick(&[2, 3])),
            _ => {
                let base = *r.pick(&[3, 4, 5]);
                let mut t = [base; 4];
                let at = r.range(1, 3) as usize;
                t[at] = base + if r.chance(0.5) { 1 } else { -1 };
                Lens::Table(t)
            }
        },
    };
    let legato = r.chance(0.25);
    let rest = if legato {
        Rest::None
    } else {
        match r.weighted(&[1.0, 1.5, if gen.rest.is_some() { 1.5 } else { 0.0 }]) {
            0 => Rest::None,
            1 => Rest::Euclid(r.range(4, 7), r.range(0, 7)),
            _ => Rest::Family,
        }
    };
    let kick = match r.weighted(&[1.0, 1.0, 1.5, 1.0, 1.5]) {
        0 => Kick::Four,
        1 => Kick::Half,
        2 => Kick::Euclid(*r.pick(&[3, 5, 5, 7]), *r.pick(&[0, 0, 2, 3])),
        3 => Kick::Fall(r.range(2, 3)),
        _ => Kick::Seq,
    };
    let clap = match r.weighted(&[1.0, 1.0, if gen.clap_trigger.is_some() { 1.5 } else { 0.0 }, 0.4]) {
        0 => Clap::BackbeatTone,
        1 => Clap::BackbeatDash,
        2 => Clap::Family,
        _ => Clap::None,
    };
    Choices {
        seed,
        index,
        family,
        bpm: r.range(88, 112) as f64,
        key: r.range(-5, 2),
        lens,
        rate,
        legato,
        gate_len: *r.pick(&[0.5, 0.7, 0.85]),
        glide: if legato || r.chance(0.5) { Some(*r.pick(&[0.6, 0.75])) } else { None },
        rest,
        lead_partials: r.range(0, 2) as usize,
        lead_gain: r.real(0.15, 0.22, 0.01),
        lead_octave: 0,
        harmony: match r.weighted(&[1.0, 1.0, 0.5]) {
            0 => None,
            1 => Some(2),
            _ => Some(4),
        },
        arp: r.chance(0.5),
        pads: if gen.ratio_drone { 0 } else { r.range(3, 4) as usize },
        contour: gen.contour.is_some() && r.chance(0.6),
        bass: *r.pick(&[Bass::Beats, Bass::Walk, Bass::Octaves]),
        kick,
        clap,
        hat_eighths: r.chance(0.35),
        form: r.range(0, FORMS.len() as i64 - 1) as usize,
        reverb_size: r.real(0.6, 0.8, 0.05),
        predelay: r.range(12, 25) as f64,
        delay_beats: *r.pick(&[0.75, 0.75, 1.5]),
        feedback: r.real(0.25, 0.35, 0.05),
        warmth: r.real(0.2, 0.3, 0.05),
        gen,
    }
    .with_family_fixups()
}

impl Choices {
    fn with_family_fixups(mut self) -> Choices {
        if self.gen.ratio_drone {
            self.pads = 0;
        }
        self
    }
}

// ------------------------------------------------------------------------------------------------ writing

fn lens_defs(lens: Lens) -> (Vec<String>, String) {
    let h = "h(d) = floor((12d + c) / 7)".to_string();
    match lens {
        Lens::Fixed(c) => (vec![format!("h(d) = floor((12d + {c}) / 7)")], format!("The lens h is the {} mode throughout.", mode_name(c))),
        Lens::Brighten(c0) => (
            vec![format!("c = {c0} + s"), h],
            format!("The lens brightens one sharp a section, {} to {}.", mode_name(c0), mode_name(c0 + 3)),
        ),
        Lens::Darken => (vec!["c = 6 - s".into(), h], "The lens darkens one flat a section, Lydian to Dorian.".into()),
        Lens::Table(t) => (
            vec![format!("c = mod(floor({}{}{}{} / 10^(3 - s)), 10)", t[0], t[1], t[2], t[3]), h],
            format!(
                "The lens changes mode by section: {}.",
                t.iter().map(|c| mode_name(*c)).collect::<Vec<_>>().join(", ")
            ),
        ),
    }
}

pub fn mode_name(c: i64) -> &'static str {
    ["Locrian", "Phrygian", "Aeolian", "Dorian", "Mixolydian", "Ionian", "Lydian"][c.clamp(0, 6) as usize]
}

/// `lay(0111)` for a layer that does not play in every section, or nothing.
fn lay(mask: &str) -> String {
    if mask == "1111" {
        String::new()
    } else {
        format!("lay({mask})")
    }
}

/// Is `name` used as a whole word in `text`?
pub fn uses(text: &str, name: &str) -> bool {
    let b = text.as_bytes();
    let word = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    let mut start = 0;
    while let Some(p) = text[start..].find(name) {
        let i = start + p;
        let j = i + name.len();
        // A name right after a number is read too: `2v` is 2 times v (implicit multiplication), so the word run
        // before the name counts as a boundary when it is all digits.
        let mut k = i;
        while k > 0 && word(b[k - 1]) {
            k -= 1;
        }
        let after_number = k < i && b[k..i].iter().all(|c| c.is_ascii_digit());
        if (i == 0 || !word(b[i - 1]) || after_number) && (j >= b.len() || !word(b[j])) {
            return true;
        }
        start = i + 1;
    }
    false
}

/// Write a song file from its choices.
pub fn write(c: &Choices) -> String {
    let g = &c.gen;
    let form = FORMS[c.form].1;
    let [m_pads, m_bass, m_lead, m_harm, m_arp, m_kick, m_clap, m_hat] = form;
    let lead_const = c.key + c.lead_octave;
    let mut defs: Vec<String> = g.defs.clone();
    let (lens_defs, lens_note) = lens_defs(c.lens);
    defs.extend(lens_defs);
    let glens = c.contour;
    if glens {
        defs.push("g(u) = h(floor(u)) + (h(floor(u) + 1) - h(floor(u))) smooth((frac(u) - 0.7) / 0.3)".into());
    }
    defs.push(format!("R(i) = {}", g.root));
    defs.push(format!("M(k) = {}", g.melody));
    defs.push(format!("lead(k) = h(M(k)){}", plus(lead_const as f64)));
    let any_layer = form.iter().any(|m| *m != "1111");
    if any_layer {
        defs.push("lay(N) = mod(floor(N / 10^(3 - s)), 10)".into());
    }
    if c.rest == Rest::Family {
        defs.push(format!("on(k) = {}", g.rest.as_deref().unwrap_or("1")));
    } else if let Rest::Euclid(k, rot) = c.rest {
        defs.push(if rot == 0 { format!("on(k) = mod({k} mod(k, 8), 8) < {k}") } else { format!("on(k) = mod({k} mod(k + {rot}, 8), 8) < {k}") });
    }
    if c.bass == Bass::Walk {
        defs.push(format!("W(x) = {}", g.walk));
    }

    let mut blocks = Vec::new();
    let mut notes = g.notes.clone();
    notes.push(lens_note);

    // The lead.
    let pitch = match c.glide {
        Some(gs) => format!("lead(e) + (lead(e + 1) - lead(e)) smooth((u - {}) / {})", num(gs), num(1.0 - gs)),
        None => "lead(e)".into(),
    };
    let rest = if c.rest == Rest::None { String::new() } else { "on(e)".into() };
    let lead_gate = |mask: &str| {
        if c.legato {
            join_product(&[lay(mask), "(frac(t / 16) < 0.96)".into()])
        } else {
            join_product(&[lay(mask), rest.clone(), format!("({} - u)", num(c.gate_len))])
        }
    };
    let gate = lead_gate(m_lead);
    let step_name = if c.rate == 2 { "eighth" } else { "sixteenth" };
    notes.push(format!(
        "The lead is h(M) on every {step_name}{}{}{}.",
        if c.legato { ", held legato and gliding into each step" } else { ", plucked" },
        match c.rest {
            Rest::None => String::new(),
            Rest::Euclid(k, _) => format!(", sounding on a Euclidean E({k}, 8)"),
            Rest::Family => ", resting where the generator's own mask says".into(),
        },
        if c.glide.is_some() && !c.legato { ", gliding into the next note" } else { "" }
    ));
    blocks.push(format!(
        "voice lead {{\n  wave = additive\n  partials = {}\n  rolloff = 660\n  pitch = {pitch}\n  gate = {gate}\n  gain = {}\n  pan = 0.1\n  attack = {}\n  decay = 300\n  sustain = {}\n  release = {}\n  reverb = 0.3\n  delay = 0.25\n}}",
        LEAD_PARTIALS[c.lead_partials],
        num(c.lead_gain),
        if c.legato { 30 } else { 5 },
        if c.legato { "1" } else { "0.55" },
        if c.legato { 300 } else { 150 },
    ));
    if let Some(deg) = c.harmony {
        let hp = match c.glide {
            Some(gs) => format!(
                "h(M(e) + {deg}){} + (h(M(e + 1) + {deg}) - h(M(e) + {deg})) smooth((u - {}) / {})",
                plus(lead_const as f64),
                num(gs),
                num(1.0 - gs)
            ),
            None => format!("h(M(e) + {deg}){}", plus(lead_const as f64)),
        };
        let hg = lead_gate(m_harm);
        notes.push(format!("A harmony doubles the lead a diatonic {} above.", if deg == 2 { "third" } else { "fifth" }));
        blocks.push(format!(
            "voice harmony {{\n  wave = additive\n  partials = {}\n  rolloff = 660\n  pitch = {hp}\n  gate = {hg}\n  gain = {}\n  pan = -0.25\n  attack = 8\n  decay = 300\n  sustain = 0.55\n  release = 150\n  reverb = 0.35\n}}",
            LEAD_PARTIALS[1],
            num(c.lead_gain * 0.45)
        ));
    }

    // Pads: a stack of thirds on the root, re-struck each bar.
    let pad_const = c.key - 12;
    for i in 0..c.pads {
        let pan = [-0.3, 0.3, 0.0, 0.45][i];
        let gate = join_product(&[lay(m_pads), "(0.97 - frac(t / 4))".into()]);
        blocks.push(format!(
            "voice pad{} {{\n  wave = additive\n  partials = {PAD_PARTIALS}\n  rolloff = 660\n  pitch = h(R(j){}){}\n  gate = {}\n  gain = {}\n  pan = {}\n  attack = 120\n  release = 500\n  reverb = 0.4\n}}",
            i + 1,
            if i == 0 { String::new() } else { format!(" + {}", 2 * i) },
            plus(pad_const as f64),
            gate,
            num(0.09 - 0.007 * i as f64),
            num(pan)
        ));
    }
    if c.pads > 0 {
        notes.push(format!("{} pads stack thirds on the root, struck each bar.", c.pads));
    }
    if c.gen.ratio_drone {
        let d = c.key - 24;
        notes.push("Two drones sound the ratio p:q as a just interval over the key's root.".into());
        for (name, p, pan) in [("drone", format!("{d}"), -0.2), ("ratio", format!("{d} + ji"), 0.2)] {
            blocks.push(format!(
                "voice {name} {{\n  wave = additive\n  partials = {PAD_PARTIALS}\n  rolloff = 660\n  pitch = {p}\n  gate = frac(t / 16) < 0.98\n  gain = 0.11\n  pan = {}\n  attack = 400\n  release = 600\n  reverb = 0.5\n}}",
                num(pan)
            ));
        }
    }
    if c.contour {
        let curve = g.contour.as_deref().unwrap_or("0");
        notes.push("A contour voice draws the generator across the whole song, held on scale tones by the glide lens g.".into());
        blocks.push(format!(
            "voice contour {{\n  wave = additive\n  partials = 0, -12, -5, -11, -9, -12\n  rolloff = 660\n  pitch = g({curve}){}\n  gate = 1\n  gain = 0.05\n  pan = -0.4\n  attack = 2000\n  reverb = 0.6\n}}",
            plus((c.key - 2) as f64)
        ));
    }
    if c.arp {
        let gate = join_product(&[lay(m_arp), "(0.45 - f)".into()]);
        notes.push("An arpeggio walks the chord's tones by the generator, every sixteenth.".into());
        blocks.push(format!(
            "voice arp {{\n  wave = additive\n  partials = {}\n  rolloff = 660\n  pitch = h(R(j) + 2 mod({}, 4)){}\n  gate = {gate}\n  gain = 0.06\n  pan = 0.25\n  attack = 2\n  decay = 120\n  sustain = 0.3\n  release = 80\n  delay = 0.2\n}}",
            LEAD_PARTIALS[2],
            g.arp_index,
            plus((c.key - 2) as f64)
        ));
    }

    // Bass.
    let bc = c.key - 36;
    let (bp, bg, bnote) = match c.bass {
        Bass::Beats => (format!("h(R(j)){}", plus(bc as f64)), "(0.85 - b)".to_string(), "The bass plays the root on every beat."),
        Bass::Walk => (
            format!("h(R(j) + 2 mod(W(floor(t)), 3)){}", plus(bc as f64)),
            "(0.9 - b)".to_string(),
            "The bass walks the chord's root, third and fifth by the generator, one a beat.",
        ),
        Bass::Octaves => (
            format!("h(R(j)){} + 12 mod(floor(2t), 2)", plus(bc as f64)),
            "(0.8 - frac(2t))".to_string(),
            "The bass jumps octaves on the root in eighths.",
        ),
    };
    notes.push(bnote.into());
    blocks.push(format!(
        "voice bass {{\n  wave = additive\n  partials = {BASS_PARTIALS}\n  rolloff = 660\n  pitch = {bp}\n  gate = {}\n  gain = 0.27\n  attack = 3\n  decay = 250\n  sustain = 0.6\n}}",
        join_product(&[lay(m_bass), bg])
    ));

    // Drums: every one a downward pitch dash.
    let (kt, knote) = match c.kick {
        Kick::Four => ("(0.5 - b)".to_string(), "The kick is on every beat.".to_string()),
        Kick::Half => ("(0.5 - frac(t / 2))".to_string(), "The kick is on beats 1 and 3.".to_string()),
        Kick::Euclid(k, rot) => (
            if rot == 0 { format!("(mod({k} mod(n, 16), 16) < {k})(0.5 - f)") } else { format!("(mod({k} mod(n + {rot}, 16), 16) < {k})(0.5 - f)") },
            format!("The kick is a Euclidean E({k}, 16){}.", if rot == 0 { String::new() } else { format!(" rotated by {rot}") }),
        ),
        Kick::Fall(d) => (
            format!("(M(e - 1) - M(e) >= {d})(0.5 - u)"),
            format!("The kick lands wherever the melody falls by {d} degrees or more."),
        ),
        Kick::Seq => (format!("({})(0.5 - f)", g.kick_seq), "The kick is drawn from the generator's own sequence.".to_string()),
    };
    notes.push(knote);
    blocks.push(format!(
        "drum kick {{\n  kind = dash\n  trigger = {}\n  from = 450\n  to = 58\n  dash = 23\n  hold = 40\n  decay = 90\n  length = 380\n  partials = 0, -24, -12, -20, -19\n  rolloff = 660\n  gain = 0.5\n}}",
        join_product(&[lay(m_kick), kt])
    ));
    match c.clap {
        Clap::BackbeatTone => {
            notes.push("The clap is a pitch line dashing down on 2 and 4.".into());
            blocks.push(format!(
                "voice clap {{\n  wave = additive\n  partials = 0, -6, -9, -12\n  rolloff = 660\n  pitch = 24 - 60 b\n  gate = {}\n  gain = 0.6\n  attack = 1\n  release = 20\n  reverb = 0.3\n}}",
                join_product(&[lay(m_clap), "(mod(floor(t), 2) == 1)(b < 0.09)".into()])
            ));
        }
        Clap::BackbeatDash | Clap::Family => {
            let trig = if c.clap == Clap::Family {
                notes.push("The clap comes q times a bar.".into());
                g.clap_trigger.clone().unwrap_or_default()
            } else {
                notes.push("The clap is a dash on 2 and 4.".into());
                "(mod(floor(t), 2) == 1)(0.5 - b)".into()
            };
            blocks.push(format!(
                "drum clap {{\n  kind = dash\n  trigger = {}\n  from = 820\n  to = 190\n  dash = 9\n  hold = 6\n  decay = 55\n  length = 260\n  partials = 0, -18, -9, -16, -15, -21, -21, -25\n  rolloff = 660\n  gain = 0.4\n  reverb = 0.3\n}}",
                join_product(&[lay(m_clap), wrap(&trig)])
            ));
        }
        Clap::None => {}
    }
    let (ht, hnote) = match (&g.hat_trigger, c.hat_eighths) {
        (Some(tr), _) => (tr.clone(), "The hats come p to a beat"),
        (None, true) => ("0.5 - frac(2t)".to_string(), "The hats are eighths"),
        (None, false) => ("0.5 - f".to_string(), "The hats are sixteenths"),
    };
    notes.push(format!("{hnote}, their loudness read from the generator."));
    blocks.push(format!(
        "drum hat {{\n  kind = dash\n  trigger = {}\n  amp = {}\n  from = 7600\n  to = 5400\n  dash = 5\n  decay = 12\n  length = 70\n  partials = 0, -10\n  rolloff = 660\n  gain = 1.5\n  pan = 0.3\n}}",
        join_product(&[lay(m_hat), wrap(&ht)]),
        g.hat_amp
    ));
    notes.push(format!("Form: {}.", FORMS[c.form].0));

    // Time names, only those used.
    let body = format!("{}\n{}", defs.join("\n"), blocks.join("\n"));
    let mut time_names = Vec::new();
    for (name, def) in [
        ("n", "n = floor(4t)".to_string()),
        ("f", "f = frac(4t)".to_string()),
        ("e", format!("e = floor({}t)", c.rate)),
        ("u", format!("u = frac({}t)", c.rate)),
        ("b", "b = frac(t)".to_string()),
        ("j", "j = floor(t / 4)".to_string()),
        ("s", "s = floor(t / 16)".to_string()),
    ] {
        if uses(&body, name) {
            time_names.push(def);
        }
    }

    let mut out = String::new();
    out += &format!(
        "# Composed by luminesce-compose, seed {} candidate {}: a {} song, grown from one generator through a scale lens.\n#\n",
        c.seed,
        c.index,
        c.family.name()
    );
    for line in wrap_comment(&notes.join(" "), 116) {
        out += &format!("# {line}\n");
    }
    out += &format!("# {} definitions and {} time names.\n\n", defs.len(), time_names.len());
    out += &format!(
        "bpm {}\nbeats 64\nloop 1\nreverb_size {}\nreverb_predelay {}\ndelay_beats {}\ndelay_feedback {}\nwarmth {}\n\n",
        num(c.bpm),
        num(c.reverb_size),
        num(c.predelay),
        num(c.delay_beats),
        num(c.feedback),
        num(c.warmth)
    );
    out += &defs.join("\n");
    out += "\n";
    out += &time_names.join("\n");
    out += "\n\n";
    out += &blocks.join("\n\n");
    out += "\n";
    out
}

/// Parenthesise an expression unless it already is one bracket.
fn wrap(e: &str) -> String {
    let t = e.trim();
    if t.starts_with('(') && t.ends_with(')') && balanced_outer(t) {
        t.to_string()
    } else {
        format!("({t})")
    }
}

fn balanced_outer(t: &str) -> bool {
    let mut depth = 0;
    for (i, ch) in t.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 && i != t.len() - 1 {
                    return false;
                }
            }
            _ => {}
        }
    }
    true
}

/// Multiply factors by juxtaposition, dropping empty ones: `lay(0111)(0.85 - b)`.
fn join_product(parts: &[String]) -> String {
    let parts: Vec<&String> = parts.iter().filter(|p| !p.is_empty()).collect();
    if parts.len() == 1 {
        let p = parts[0];
        return if p.starts_with('(') && balanced_outer(p) { p[1..p.len() - 1].to_string() } else { p.to_string() };
    }
    parts.iter().map(|p| if p.starts_with("lay(") || p.starts_with("on(") { p.to_string() } else { wrap(p) }).collect::<Vec<_>>().join("")
}

fn wrap_comment(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut cur = String::new();
    for w in text.split_whitespace() {
        if !cur.is_empty() && cur.len() + 1 + w.len() > width {
            lines.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur += w;
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

/// The median pitch of the lead's note onsets, read from the parsed song (semitones above A4).
fn lead_median(song: &Song) -> Option<f64> {
    let v = song.voices.iter().find(|v| v.name == "lead")?;
    let crate::song::VoiceKind::Tonal(tn) = &v.kind else { return None };
    let mut ps: Vec<f64> = (0..(song.beats as usize * 4))
        .map(|i| tn.pitch.eval(&song.program, i as f64 / 4.0 + 1e-6))
        .filter(|p| p.is_finite())
        .collect();
    if ps.is_empty() {
        return None;
    }
    ps.sort_by(|a, b| a.partial_cmp(b).unwrap());
    Some(ps[ps.len() / 2])
}

/// Sample and write one candidate, with the lead placed in its register (median between about E4 and E5).
pub fn compose(seed: u64, index: u64) -> Result<Draft, String> {
    let mut c = sample(seed, index);
    let text = write(&c);
    let song = Song::parse(&text).map_err(|e| format!("seed {seed} candidate {index}: {e}\n{text}"))?;
    if let Some(med) = lead_median(&song) {
        c.lead_octave = -12 * ((med - 1.0) / 12.0).floor() as i64;
    }
    let text = write(&c);
    Song::parse(&text).map_err(|e| format!("seed {seed} candidate {index}: {e}\n{text}"))?;
    Ok(Draft { text, choices: c })
}
