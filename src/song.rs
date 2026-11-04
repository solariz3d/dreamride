//! The song file format. See README.md for the full description.
//!
//! A song is plain text: settings (`bpm 100`), definitions (`h(d) = ...`), and blocks
//! (`voice name { ... }`, `drum name { ... }`) whose fields are `key = value`, one per line.
//! Definitions may appear anywhere outside blocks and in any order.

use crate::error::Error;
use crate::expr::{parse_definition, strip_comment, Expr, Program};

/// Longest song accepted, in seconds (keeps a typo in `beats` from allocating gigabytes).
pub const MAX_SECONDS: f64 = 3600.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wave {
    Sine,
    Triangle,
    Saw,
    Square,
    /// A sum of sine partials at whole multiples of the pitch (graph-native: what a list of Desmos
    /// `tone()` sines adds up to). Levels from the voice's `partials`; partials at or above Nyquist
    /// are left out.
    Additive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrumKind {
    Kick,
    Snare,
    Hat,
    /// A sine (and optional harmonics) whose frequency dashes down exponentially: see [`Dash`].
    Dash,
}

/// A graph-native percussive hit: `f(tau) = to + (from - to) e^(-tau / dash)`, its phase in closed
/// form, its level held for `hold` and then falling by 1/e per `decay`, cut at `length` with a fade.
#[derive(Debug, Clone, PartialEq)]
pub struct Dash {
    pub from_hz: f64,
    pub to_hz: f64,
    pub dash_ms: f64,
    pub hold_ms: f64,
    pub decay_ms: f64,
    pub length_ms: f64,
    /// Linear amplitudes of harmonics 1, 2, 3, ... of the swept sine.
    pub partials: Vec<f64>,
    /// The Desmos gain cap as a roll-off: each partial is scaled by min(1, rolloff / its frequency)
    /// (0 is off).
    pub rolloff_hz: f64,
}

#[derive(Debug, Clone)]
pub struct Tonal {
    pub wave: Wave,
    /// Semitones relative to the reference pitch (0 = `reference`, default A4 = 440 Hz).
    pub pitch: Expr,
    /// The voice sounds while this is > 0.
    pub gate: Expr,
    /// Amplitude multiplier, evaluated per sample (default 1).
    pub amp: Option<Expr>,
    pub attack_ms: f64,
    /// Time from full level down to the sustain level.
    pub decay_ms: f64,
    /// Level held while the gate is open, 0..1 (default 1: no decay).
    pub sustain: f64,
    pub release_ms: f64,
    /// Low-pass cutoff in Hz, evaluated per sample (None: no filter).
    pub cutoff: Option<Expr>,
    /// 0..0.95; 0 is a Butterworth response (-3 dB at the cutoff).
    pub resonance: f64,
    /// Octaves added to the cutoff at each note onset, decaying with `filter_decay_ms`.
    pub filter_env: f64,
    pub filter_decay_ms: f64,
    /// Soft saturation, 0 (off) .. 10.
    pub drive: f64,
    /// Two oscillators this many cents apart (0: one oscillator).
    pub detune_cents: f64,
    /// For `Additive`: linear amplitudes of harmonics 1, 2, 3, ... (empty for the other waves).
    pub partials: Vec<f64>,
    /// For `Sine` and `Additive`: each partial is scaled by min(1, rolloff / its frequency): the
    /// Desmos gain cap min(10, 660 / f), normalised to 1 below the corner (0 is off).
    pub rolloff_hz: f64,
}

#[derive(Debug, Clone)]
pub struct Drum {
    pub kind: DrumKind,
    /// A hit starts on every sample where this goes from <= 0 to > 0.
    pub trigger: Expr,
    /// Hit velocity, evaluated at the onset (default 1).
    pub amp: Option<Expr>,
    /// The sweep's shape, for `DrumKind::Dash` only.
    pub dash: Option<Dash>,
    /// A noise drum's shape (`kind = hat` or `kind = snare`) when `decay` or `length` is given; else the built-in one.
    pub noise: Option<Noise>,
}

/// The shape of a noise drum when its block gives `decay` or `length` (since 2026-11-04, for kits drawn per song): the
/// noise falls by 1/e per `decay_ms` and the hit is cut at `length_ms`. A field it omits keeps the built-in value
/// (hat 25 ms and 120 ms, snare 70 ms and 300 ms), so a block with neither sounds exactly as before.
#[derive(Debug, Clone, PartialEq)]
pub struct Noise {
    pub decay_ms: f64,
    pub length_ms: f64,
}

#[derive(Debug, Clone)]
pub enum VoiceKind {
    Tonal(Tonal),
    Drum(Drum),
}

#[derive(Debug, Clone)]
pub struct Voice {
    pub name: String,
    pub kind: VoiceKind,
    pub gain: f64,
    /// -1 is hard left, 1 hard right.
    pub pan: f64,
    /// Send to the reverb, 0..1 (pre-pan, post-envelope).
    pub reverb: f64,
    /// Send to the stereo delay, 0..1.
    pub delay: f64,
    /// The line of the block header, for error messages.
    pub line: usize,
}

impl Voice {
    /// "voice 'lead'" or "drum 'kick'", for messages.
    pub fn label(&self) -> String {
        match self.kind {
            VoiceKind::Tonal(_) => format!("voice '{}'", self.name),
            VoiceKind::Drum(_) => format!("drum '{}'", self.name),
        }
    }
}

/// The song's send effects and master colour. See README.md, "Space".
#[derive(Debug, Clone, PartialEq)]
pub struct Effects {
    /// Freeverb room size, 0..1 (comb feedback 0.7 + 0.28 size).
    pub reverb_size: f64,
    /// Freeverb damping, 0..1 (the comb's one-pole coefficient is 0.4 damp).
    pub reverb_damp: f64,
    /// 0 is mono, 1 full stereo.
    pub reverb_width: f64,
    pub reverb_predelay_ms: f64,
    /// The stereo delay's time, in beats.
    pub delay_beats: f64,
    pub delay_feedback: f64,
    /// Low-pass in the delay's feedback path, Hz: each repeat is darker.
    pub delay_tone_hz: f64,
    /// Tape wow: peak pitch deviation of the whole mix, in cents (0 is off).
    pub wow_cents: f64,
    pub wow_rate_hz: f64,
    /// Master saturation and top roll-off, 0 (off) .. 1.
    pub warmth: f64,
    /// Treat the song as a loop: effect tails from the end ring over the start, and wow completes
    /// whole cycles, so the file repeats seamlessly. Otherwise tails past the end are cut.
    pub looped: bool,
}

#[derive(Debug, Clone)]
pub struct Song {
    pub bpm: f64,
    pub beats: f64,
    pub rate: u32,
    pub reference_hz: f64,
    pub program: Program,
    pub voices: Vec<Voice>,
    pub effects: Effects,
}

impl Song {
    /// Samples per channel: round(beats * 60 / bpm * rate).
    pub fn sample_count(&self) -> usize {
        (self.beats * 60.0 / self.bpm * self.rate as f64).round() as usize
    }

    pub fn duration_secs(&self) -> f64 {
        self.beats * 60.0 / self.bpm
    }

    pub fn parse(src: &str) -> Result<Song, Error> {
        parse(src)
    }
}

// ---------------------------------------------------------------- parsing

#[derive(Clone, Copy, PartialEq)]
enum BlockKind {
    Voice,
    Drum,
}

struct Field {
    key: String,
    value: String,
    line: usize,
    col: usize,
}

struct Block {
    kind: BlockKind,
    name: String,
    line: usize,
    fields: Vec<Field>,
}

impl Block {
    fn label(&self) -> String {
        match self.kind {
            BlockKind::Voice => format!("voice '{}'", self.name),
            BlockKind::Drum => format!("drum '{}'", self.name),
        }
    }
}

struct Setting {
    value: f64,
    line: usize,
}

const SETTINGS: [&str; 15] = [
    "bpm", "beats", "rate", "reference", "reverb_size", "reverb_damp", "reverb_width", "reverb_predelay", "delay_beats",
    "delay_feedback", "delay_tone", "wow", "wow_rate", "warmth", "loop",
];

fn is_name(s: &str) -> bool {
    let mut c = s.chars();
    matches!(c.next(), Some(f) if f.is_ascii_alphabetic() || f == '_')
        && c.all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
}

fn col_of(text: &str, byte: usize) -> usize {
    text[..byte].chars().count() + 1
}

fn parse(src: &str) -> Result<Song, Error> {
    let mut settings: [Option<Setting>; 15] = Default::default();
    let mut defs = Vec::new();
    let mut blocks: Vec<Block> = Vec::new();
    let mut open: Option<Block> = None;

    for (n, raw) in src.lines().enumerate() {
        let line = n + 1;
        let text = strip_comment(raw);
        let trimmed = text.trim();
        if trimmed.is_empty() {
            continue;
        }
        let first = trimmed.split_whitespace().next().unwrap_or("");

        if let Some(b) = open.as_mut() {
            if trimmed == "}" {
                blocks.push(open.take().expect("open block"));
                continue;
            }
            if first == "voice" || first == "drum" {
                return Err(Error::line(
                    line,
                    format!("{} opened on line {} is not closed with '}}' before this line", b.label(), b.line),
                ));
            }
            let Some(eq) = text.find('=') else {
                return Err(Error::line(line, format!("expected `key = value` inside {}", b.label())));
            };
            let key = text[..eq].trim();
            if !is_name(key) {
                return Err(Error::line(line, format!("expected `key = value` inside {}", b.label())));
            }
            if b.fields.iter().any(|f| f.key == key) {
                return Err(Error::line(line, format!("`{key}` is given twice in {}", b.label())));
            }
            b.fields.push(Field { key: key.to_string(), value: text[eq + 1..].to_string(), line, col: col_of(text, eq + 1) });
            continue;
        }

        if let Some(k) = SETTINGS.iter().position(|s| *s == first) {
            let rest = trimmed[first.len()..].trim();
            if rest.is_empty() {
                return Err(Error::line(line, format!("`{first}` needs a value, e.g. `{}`", example(first))));
            }
            if rest.starts_with('=') {
                return Err(Error::line(line, format!("write `{}`, without '='", example(first))));
            }
            let value: f64 = rest
                .parse()
                .ok()
                .filter(|v: &f64| v.is_finite())
                .ok_or_else(|| Error::line(line, format!("`{first}` is not a number: '{rest}'")))?;
            if let Some(prev) = &settings[k] {
                return Err(Error::line(line, format!("`{first}` is set twice (first on line {})", prev.line)));
            }
            settings[k] = Some(Setting { value, line });
            continue;
        }

        if first == "voice" || first == "drum" {
            let kind = if first == "voice" { BlockKind::Voice } else { BlockKind::Drum };
            let rest = trimmed[first.len()..].trim();
            let name = rest.strip_suffix('{').map(str::trim).unwrap_or("");
            if !is_name(name) {
                return Err(Error::line(line, format!("expected `{first} <name> {{` (a name, then '{{' on the same line)")));
            }
            if let Some(prev) = blocks.iter().find(|b| b.name == name) {
                return Err(Error::line(line, format!("the name '{name}' is used twice (first on line {})", prev.line)));
            }
            open = Some(Block { kind, name: name.to_string(), line, fields: Vec::new() });
            continue;
        }

        if trimmed == "}" {
            return Err(Error::line(line, "'}' without an open block"));
        }
        if text.contains('=') {
            defs.push(parse_definition(text, line)?);
            continue;
        }
        return Err(Error::line(
            line,
            "unrecognised line: expected a setting (bpm, beats, rate, reference), \
             a definition (`name = ...`), or a block (`voice name {`, `drum name {`)",
        ));
    }
    if let Some(b) = open {
        return Err(Error::line(b.line, format!("{} is never closed with '}}'", b.label())));
    }

    let [bpm, beats, rate, reference, rv_size, rv_damp, rv_width, rv_pre, dl_beats, dl_fb, dl_tone, wow, wow_rate, warmth, looped] =
        settings;
    let bpm = bpm.ok_or_else(|| Error::msg("missing `bpm` (e.g. `bpm 100`)"))?;
    let beats = beats.ok_or_else(|| Error::msg("missing `beats`, the song length in beats (e.g. `beats 32`)"))?;
    if !(bpm.value > 0.0 && bpm.value <= 1000.0) {
        return Err(Error::line(bpm.line, format!("`bpm` must be in (0, 1000], got {}", bpm.value)));
    }
    if beats.value <= 0.0 {
        return Err(Error::line(beats.line, format!("`beats` must be greater than 0, got {}", beats.value)));
    }
    let rate_hz = match rate {
        None => 48_000,
        Some(r) => {
            if r.value.fract() != 0.0 {
                return Err(Error::line(r.line, format!("`rate` must be a whole number of Hz, got {}", r.value)));
            }
            if !(8_000.0..=192_000.0).contains(&r.value) {
                return Err(Error::line(r.line, format!("`rate` must be between 8000 and 192000 Hz, got {}", r.value)));
            }
            r.value as u32
        }
    };
    let reference_hz = match reference {
        None => 440.0,
        Some(r) if r.value > 0.0 => r.value,
        Some(r) => return Err(Error::line(r.line, format!("`reference` must be a frequency above 0 Hz, got {}", r.value))),
    };
    let seconds = beats.value * 60.0 / bpm.value;
    if seconds > MAX_SECONDS {
        return Err(Error::line(
            beats.line,
            format!("the song would be {seconds:.0} s long; the limit is {MAX_SECONDS:.0} s"),
        ));
    }

    // Effects: each setting is optional and range-checked; the defaults leave every old song as it was.
    let ranged = |s: Option<Setting>, name: &str, default: f64, lo: f64, hi: f64| -> Result<f64, Error> {
        match s {
            None => Ok(default),
            Some(s) if (lo..=hi).contains(&s.value) => Ok(s.value),
            Some(s) => Err(Error::line(s.line, format!("`{name}` must be between {lo} and {hi}, got {}", s.value))),
        }
    };
    let effects = Effects {
        reverb_size: ranged(rv_size, "reverb_size", 0.5, 0.0, 1.0)?,
        reverb_damp: ranged(rv_damp, "reverb_damp", 0.5, 0.0, 1.0)?,
        reverb_width: ranged(rv_width, "reverb_width", 1.0, 0.0, 1.0)?,
        reverb_predelay_ms: ranged(rv_pre, "reverb_predelay", 0.0, 0.0, 500.0)?,
        delay_beats: ranged(dl_beats, "delay_beats", 0.75, 0.01, 16.0)?,
        delay_feedback: ranged(dl_fb, "delay_feedback", 0.35, 0.0, 0.95)?,
        delay_tone_hz: ranged(dl_tone, "delay_tone", 3500.0, 100.0, 20000.0)?,
        wow_cents: ranged(wow, "wow", 0.0, 0.0, 100.0)?,
        wow_rate_hz: ranged(wow_rate, "wow_rate", 0.5, 0.01, 20.0)?,
        warmth: ranged(warmth, "warmth", 0.0, 0.0, 1.0)?,
        looped: match looped {
            None => false,
            Some(s) if s.value == 0.0 || s.value == 1.0 => s.value == 1.0,
            Some(s) => return Err(Error::line(s.line, format!("`loop` must be 0 or 1, got {}", s.value))),
        },
    };

    let program = Program::compile(defs)?;
    if blocks.is_empty() {
        return Err(Error::msg("the song has no voices (add a `voice name { ... }` or `drum name { ... }` block)"));
    }
    let voices = blocks.into_iter().map(|b| build_voice(b, &program)).collect::<Result<Vec<_>, _>>()?;

    Ok(Song { bpm: bpm.value, beats: beats.value, rate: rate_hz, reference_hz, program, voices, effects })
}

fn example(setting: &str) -> &'static str {
    match setting {
        "bpm" => "bpm 120",
        "beats" => "beats 32",
        "rate" => "rate 48000",
        "reference" => "reference 440",
        "loop" => "loop 1",
        _ => "reverb_size 0.5",
    }
}

fn build_voice(b: Block, program: &Program) -> Result<Voice, Error> {
    let label = b.label();
    let allowed: &[&str] = match b.kind {
        BlockKind::Voice => &[
            "wave", "pitch", "gate", "amp", "gain", "pan", "attack", "decay", "sustain", "release", "cutoff",
            "resonance", "filter_env", "filter_decay", "drive", "detune", "reverb", "delay", "partials", "rolloff",
        ],
        BlockKind::Drum => &[
            "kind", "trigger", "amp", "gain", "pan", "reverb", "delay", "from", "to", "dash", "hold", "decay", "length",
            "partials", "rolloff",
        ],
    };
    for f in &b.fields {
        if !allowed.contains(&f.key.as_str()) {
            return Err(Error::line(
                f.line,
                format!("unknown field `{}` in {label} (allowed: {})", f.key, allowed.join(", ")),
            ));
        }
    }
    let get = |key: &str| -> Option<&Field> { b.fields.iter().find(|f| f.key == key) };
    let need = |key: &str| -> Result<&Field, Error> {
        get(key).ok_or_else(|| Error::line(b.line, format!("{label} is missing `{key}`")))
    };
    let nonempty = |f: &Field| -> Result<(), Error> {
        if f.value.trim().is_empty() {
            return Err(Error::line(f.line, format!("`{}` is empty in {label}", f.key)));
        }
        Ok(())
    };
    let expr = |f: &Field| -> Result<Expr, Error> {
        nonempty(f)?;
        program.compile_expr_at(&f.value, f.line, f.col)
    };
    // Gates, triggers and drum velocities decide where notes are, so they cannot read the note.
    let timing = |f: &Field| -> Result<Expr, Error> {
        let e = expr(f)?;
        if e.uses_note() {
            return Err(Error::line(
                f.line,
                format!("`{}` in {label} reads note_age or note_left, which only pitch, amp and cutoff of a voice may read", f.key),
            ));
        }
        Ok(e)
    };
    let number = |key: &str, default: f64, ok: &dyn Fn(f64) -> bool, range: &str| -> Result<f64, Error> {
        let Some(f) = get(key) else { return Ok(default) };
        nonempty(f)?;
        let v: f64 = f
            .value
            .trim()
            .parse()
            .ok()
            .filter(|v: &f64| v.is_finite())
            .ok_or_else(|| Error::line(f.line, format!("`{key}` must be a number, found '{}'", f.value.trim())))?;
        if !ok(v) {
            return Err(Error::line(f.line, format!("`{key}` must be {range}, got {v}")));
        }
        Ok(v)
    };

    let rolloff_ok = |v: f64| v == 0.0 || (1.0..=20_000.0).contains(&v);
    let gain = number("gain", 1.0, &|v| (0.0..=10.0).contains(&v), "between 0 and 10")?;
    let pan = number("pan", 0.0, &|v| (-1.0..=1.0).contains(&v), "between -1 and 1")?;
    let reverb = number("reverb", 0.0, &|v| (0.0..=1.0).contains(&v), "between 0 and 1")?;
    let delay = number("delay", 0.0, &|v| (0.0..=1.0).contains(&v), "between 0 and 1")?;
    let amp = get("amp").map(if b.kind == BlockKind::Drum { &timing as &dyn Fn(&Field) -> Result<Expr, Error> } else { &expr }).transpose()?;

    let kind = match b.kind {
        BlockKind::Voice => {
            let w = need("wave")?;
            nonempty(w)?;
            let wave = match w.value.trim() {
                "sine" => Wave::Sine,
                "triangle" => Wave::Triangle,
                "saw" => Wave::Saw,
                "square" => Wave::Square,
                "additive" => Wave::Additive,
                other => {
                    return Err(Error::line(
                        w.line,
                        format!("unknown wave '{other}' in {label} (sine, triangle, saw, square, additive)"),
                    ))
                }
            };
            let partials = match (wave, get("partials")) {
                (Wave::Additive, Some(f)) => db_list(f, &label)?,
                (Wave::Additive, None) => {
                    return Err(Error::line(
                        b.line,
                        format!("{label} is additive and needs `partials` (dB per harmonic, e.g. `partials = 0, -22, -14`)"),
                    ))
                }
                (_, Some(f)) => return Err(Error::line(f.line, format!("`partials` in {label} needs `wave = additive`"))),
                (_, None) => Vec::new(),
            };
            if let Some(f) = get("rolloff") {
                if !matches!(wave, Wave::Sine | Wave::Additive) {
                    return Err(Error::line(f.line, format!("`rolloff` in {label} needs `wave = sine` or `wave = additive`")));
                }
            }
            let env_ok = |v: f64| (0.1..=10_000.0).contains(&v);
            VoiceKind::Tonal(Tonal {
                wave,
                pitch: expr(need("pitch")?)?,
                gate: timing(need("gate")?)?,
                amp,
                attack_ms: number("attack", 5.0, &env_ok, "between 0.1 and 10000 ms")?,
                decay_ms: number("decay", 100.0, &env_ok, "between 0.1 and 10000 ms")?,
                sustain: number("sustain", 1.0, &|v| (0.0..=1.0).contains(&v), "between 0 and 1")?,
                release_ms: number("release", 30.0, &env_ok, "between 0.1 and 10000 ms")?,
                cutoff: get("cutoff").map(expr).transpose()?,
                resonance: number("resonance", 0.0, &|v| (0.0..=0.95).contains(&v), "between 0 and 0.95")?,
                filter_env: number("filter_env", 0.0, &|v| (-8.0..=8.0).contains(&v), "between -8 and 8 octaves")?,
                filter_decay_ms: number("filter_decay", 200.0, &env_ok, "between 0.1 and 10000 ms")?,
                drive: number("drive", 0.0, &|v| (0.0..=10.0).contains(&v), "between 0 and 10")?,
                detune_cents: number("detune", 0.0, &|v| (0.0..=100.0).contains(&v), "between 0 and 100 cents")?,
                partials,
                rolloff_hz: number("rolloff", 0.0, &rolloff_ok, "0 (off) or between 1 and 20000 Hz")?,
            })
        }
        BlockKind::Drum => {
            let k = need("kind")?;
            nonempty(k)?;
            let kind = match k.value.trim() {
                "kick" => DrumKind::Kick,
                "snare" => DrumKind::Snare,
                "hat" => DrumKind::Hat,
                "dash" => DrumKind::Dash,
                other => {
                    return Err(Error::line(k.line, format!("unknown drum kind '{other}' in {label} (kick, snare, hat, dash)")))
                }
            };
            let dash = if kind == DrumKind::Dash {
                let hz_ok = |v: f64| (1.0..=20_000.0).contains(&v);
                let ms_ok = |v: f64| (0.1..=10_000.0).contains(&v);
                Some(Dash {
                    from_hz: number("from", 450.0, &hz_ok, "between 1 and 20000 Hz")?,
                    to_hz: number("to", 60.0, &hz_ok, "between 1 and 20000 Hz")?,
                    dash_ms: number("dash", 23.0, &ms_ok, "between 0.1 and 10000 ms")?,
                    hold_ms: number("hold", 0.0, &|v| (0.0..=10_000.0).contains(&v), "between 0 and 10000 ms")?,
                    decay_ms: number("decay", 150.0, &ms_ok, "between 0.1 and 10000 ms")?,
                    length_ms: number("length", 400.0, &|v| (1.0..=10_000.0).contains(&v), "between 1 and 10000 ms")?,
                    partials: match get("partials") {
                        Some(f) => db_list(f, &label)?,
                        None => vec![1.0],
                    },
                    rolloff_hz: number("rolloff", 0.0, &rolloff_ok, "0 (off) or between 1 and 20000 Hz")?,
                })
            } else {
                // a noise drum (hat, snare) may shape its decay and length; nothing else here takes a dash's fields
                let noisy = matches!(kind, DrumKind::Hat | DrumKind::Snare);
                for key in ["from", "to", "dash", "hold", "decay", "length", "partials", "rolloff"] {
                    if noisy && (key == "decay" || key == "length") {
                        continue;
                    }
                    if let Some(f) = get(key) {
                        return Err(Error::line(f.line, format!("`{key}` in {label} needs `kind = dash`")));
                    }
                }
                None
            };
            let noise = if matches!(kind, DrumKind::Hat | DrumKind::Snare) && (get("decay").is_some() || get("length").is_some()) {
                let (d, l) = if kind == DrumKind::Hat { (25.0, 120.0) } else { (70.0, 300.0) };
                Some(Noise {
                    decay_ms: number("decay", d, &|v| (0.1..=10_000.0).contains(&v), "between 0.1 and 10000 ms")?,
                    length_ms: number("length", l, &|v| (1.0..=10_000.0).contains(&v), "between 1 and 10000 ms")?,
                })
            } else {
                None
            };
            VoiceKind::Drum(Drum { kind, trigger: timing(need("trigger")?)?, amp, dash, noise })
        }
    };
    Ok(Voice { name: b.name, kind, gain, pan, reverb, delay, line: b.line })
}

/// The most partials a voice or dash may list.
pub const MAX_PARTIALS: usize = 64;

/// `partials = 0, -22, -14`: levels in dB of harmonics 1, 2, 3, ..., returned as linear
/// amplitudes. Each is a number from -120 to 20 dB; at least one and at most [`MAX_PARTIALS`].
fn db_list(f: &Field, label: &str) -> Result<Vec<f64>, Error> {
    let text = f.value.trim();
    if text.is_empty() {
        return Err(Error::line(f.line, format!("`partials` is empty in {label}")));
    }
    let mut out = Vec::new();
    for (i, item) in text.split(',').enumerate() {
        let item = item.trim();
        let db: f64 = item.parse().ok().filter(|v: &f64| v.is_finite()).ok_or_else(|| {
            Error::line(
                f.line,
                format!("partial {} in {label} is not a number of dB: '{item}' (write e.g. `partials = 0, -22, -14`)", i + 1),
            )
        })?;
        if !(-120.0..=20.0).contains(&db) {
            return Err(Error::line(f.line, format!("partial {} in {label} must be between -120 and 20 dB, got {db}", i + 1)));
        }
        out.push(10f64.powf(db / 20.0));
    }
    if out.len() > MAX_PARTIALS {
        return Err(Error::line(f.line, format!("{label} lists {} partials; the limit is {MAX_PARTIALS}", out.len())));
    }
    Ok(out)
}
