//! **FROZEN: generator version 1's instruments.** Never edit this file. A seed saved at version 1 replays from it, and
//! `tests/versions.rs` holds it to the keeper's saved seed byte for byte. Change instruments in `timbre.rs` (the
//! current version) instead, and give the change a new version (docs/versions.md).
//!
//! This is `timbre.rs` as it stood at 05:45 on 2026-11-04, the build the keeper heard seed 152637 on. It was rebuilt
//! by reversing the later edit, and matched the kept 05:45 engine on that seed's stages 1 and 2, byte for byte.
//!
//! Instruments generated per song (since 2026-11-04). The keeper: "the instrumentation is also always the same, even if
//! the songs are different". Every module carried one fixed sound. A song now draws its own instrument for each tonal
//! role (lead, counter, arp, bass, pads) from its seed, and keeps it for the whole song.
//!
//! What makes it aware rather than random:
//! - **The role.** Each role draws from its own ranges: a bass keeps a strong fundamental, a low filter and a tight
//!   envelope; pads attack slowly with many soft harmonics and detune; a lead is a pluck, a held lead or a bell; the
//!   arp is always a pluck. A spectrum is a slope (dB per octave), an odd/even balance (saw-like to square-like) and one
//!   resonant bump, at 4 to 16 harmonics.
//! - **The balance.** A new instrument's gain is rescaled so it is as loud as the one it replaces (the harmonic levels
//!   with the song's rolloff, at the role's usual pitch), so the mix the modules were written for is kept.
//! - **The references.** [`choose`] renders the song's full band with the instruments and scores its five-band energy
//!   against the two reference recordings (the critic's "sound bands"); an instrument set more than [`MARGIN`] below the
//!   written instruments' score is refused and another drawn ([`TRIES`]), else the best is kept.
//!
//! Only the text a stage plays is dressed ([`dress`]); the stream's own equation, and so how it composes, is unchanged.

use super::rng::Rng;
use super::stream::Equation;
use std::collections::BTreeMap;

/// The tonal roles an instrument is drawn for.
pub const ROLES: [&str; 5] = ["lead", "counter", "arp", "bass", "pad"];
/// How far below the written instruments' band score a drawn set may score and still be kept.
pub const MARGIN: f64 = 0.1;
/// Sets drawn before the best is kept.
pub const TRIES: u64 = 6;

/// The fields an instrument sets on a voice; every other field (pitch, gate, amp, pan, sends) is the module's.
pub const TIMBRE_FIELDS: [&str; 13] =
    ["wave", "partials", "attack", "decay", "sustain", "release", "cutoff", "resonance", "filter_env", "filter_decay", "drive", "detune", "rolloff"];

/// The role a voice plays, by its name.
pub fn role(voice: &str) -> Option<&'static str> {
    match voice {
        "lead" => Some("lead"),
        "counter" => Some("counter"),
        "arp" => Some("arp"),
        "bass" => Some("bass"),
        "pad1" | "pad2" | "pad3" => Some("pad"),
        _ => None,
    }
}

/// One drawn instrument: its harmonic levels (dB, harmonic 1 first) and its other timbre fields.
#[derive(Debug, Clone, PartialEq)]
pub struct Instrument {
    pub kind: String,
    pub partials: Vec<f64>,
    pub fields: Vec<(String, String)>,
}

/// A song's instruments, one per role.
#[derive(Debug, Clone, PartialEq)]
pub struct Palette {
    pub seed: u64,
    pub attempt: u64,
    pub instruments: BTreeMap<&'static str, Instrument>,
}

fn real(r: &mut Rng, lo: f64, hi: f64) -> f64 {
    lo + (hi - lo) * r.unit()
}

/// Harmonic levels: `slope` dB per octave down, `odd` dB off every even harmonic, a bump of `bump` dB at harmonic
/// `centre` (`width` octaves wide), `n` harmonics; clamped to the engine's -120..20 dB.
fn spectrum(n: usize, slope: f64, odd: f64, bump: f64, centre: f64, width: f64) -> Vec<f64> {
    (1..=n)
        .map(|k| {
            let o = (k as f64).log2();
            let b = bump * (-(o - centre.log2()).powi(2) / (2.0 * width * width)).exp();
            let even = if k % 2 == 0 { odd } else { 0.0 };
            let db = -slope * o - even + b;
            (db * 10.0).round() / 10.0
        })
        .map(|d| d.clamp(-120.0, 20.0))
        .collect()
}

fn f(k: &str, v: f64) -> (String, String) {
    (k.to_string(), format!("{}", (v * 100.0).round() / 100.0))
}

fn draw(role: &str, r: &mut Rng) -> Instrument {
    let mut fields = Vec::new();
    let (kind, partials) = match role {
        "bass" => {
            let p = spectrum(r.range(4, 8) as usize, real(r, 6.0, 14.0), real(r, 0.0, 10.0), real(r, 0.0, 6.0), real(r, 2.0, 3.0), 0.5);
            fields.extend([
                f("attack", real(r, 2.0, 6.0)),
                f("decay", real(r, 150.0, 420.0)),
                f("sustain", real(r, 0.4, 0.8)),
                f("release", real(r, 50.0, 130.0)),
                f("cutoff", real(r, 280.0, 950.0)),
                f("resonance", real(r, 0.0, 0.4)),
                f("filter_env", real(r, 0.5, 2.5)),
                f("filter_decay", real(r, 60.0, 220.0)),
                f("drive", real(r, 0.0, 3.0)),
            ]);
            ("bass", p)
        }
        "pad" => {
            let p = spectrum(r.range(10, 16) as usize, real(r, 5.0, 10.0), real(r, 0.0, 6.0), real(r, 0.0, 6.0), real(r, 2.0, 6.0), 0.8);
            fields.extend([
                f("attack", real(r, 60.0, 500.0)),
                f("decay", real(r, 300.0, 900.0)),
                f("sustain", real(r, 0.6, 0.95)),
                f("release", real(r, 300.0, 1100.0)),
                f("cutoff", real(r, 1200.0, 4200.0)),
                f("detune", real(r, 4.0, 14.0)),
            ]);
            ("pad", p)
        }
        _ => {
            // lead, counter, arp: the arp is always a pluck; a lead or counter is a pluck, a held lead or a bell
            let darker = if role == "counter" { 2.0 } else { 0.0 };
            let kind = if role == "arp" { 0 } else { r.range(0, 2) };
            let p = spectrum(
                r.range(6, 12) as usize,
                real(r, 4.0, 9.0) + darker,
                real(r, 0.0, 14.0),
                real(r, 0.0, 8.0),
                if kind == 2 { real(r, 3.0, 7.0) } else { real(r, 2.0, 5.0) },
                0.6,
            );
            match kind {
                0 => fields.extend([
                    f("attack", real(r, 2.0, 6.0)),
                    f("decay", real(r, 110.0, 340.0)),
                    f("sustain", real(r, 0.1, 0.45)),
                    f("release", real(r, 80.0, 200.0)),
                    f("cutoff", real(r, 900.0, 2600.0)),
                    f("filter_env", real(r, 1.5, 3.5)),
                    f("filter_decay", real(r, 80.0, 220.0)),
                ]),
                1 => fields.extend([
                    f("attack", real(r, 5.0, 25.0)),
                    f("decay", real(r, 200.0, 500.0)),
                    f("sustain", real(r, 0.6, 0.9)),
                    f("release", real(r, 120.0, 300.0)),
                    f("cutoff", real(r, 2500.0, 6000.0)),
                ]),
                _ => fields.extend([
                    f("attack", real(r, 1.0, 3.0)),
                    f("decay", real(r, 500.0, 1200.0)),
                    f("sustain", real(r, 0.0, 0.2)),
                    f("release", real(r, 300.0, 600.0)),
                ]),
            }
            if r.chance(0.4) {
                fields.push(f("detune", real(r, 3.0, 10.0)));
            }
            if r.chance(0.3) {
                fields.push(f("drive", real(r, 0.2, 1.5)));
            }
            (["pluck", "held", "bell"][kind as usize], p)
        }
    };
    Instrument { kind: kind.to_string(), partials, fields }
}

/// A song's instruments, drawn from its seed (`attempt` draws again).
pub fn palette(seed: u64, attempt: u64) -> Palette {
    let mut r = Rng::for_candidate(seed ^ 0x5449_4D42_5245, attempt); // "TIMBRE"
    let instruments = ROLES.iter().map(|role| (*role, draw(role, &mut r))).collect();
    Palette { seed, attempt, instruments }
}

/// The usual pitch of a role, in Hz, for the loudness estimate.
fn usual_hz(role: &str) -> f64 {
    match role {
        "bass" => 70.0,
        "pad" => 260.0,
        "lead" => 830.0,
        _ => 620.0,
    }
}

/// The loudness of a voice's sound at its role's usual pitch: the root of the summed squared harmonic amplitudes,
/// with the rolloff where the voice has one. Simple waves are read as their harmonic series.
fn loudness(fields: &[(String, String)], role: &str) -> Option<f64> {
    let get = |k: &str| fields.iter().find(|x| x.0 == k).map(|x| x.1.trim());
    let hz = usual_hz(role);
    let roll: Option<f64> = get("rolloff").and_then(|v| v.parse().ok());
    let levels: Vec<f64> = match get("wave").unwrap_or("sine") {
        "additive" => get("partials")?.split(',').map(|x| x.trim().parse::<f64>().ok().map(|d| 10f64.powf(d / 20.0))).collect::<Option<_>>()?,
        "sine" => vec![1.0],
        "saw" => (1..=16).map(|k| 1.0 / k as f64).collect(),
        "square" => (1..=16).map(|k| if k % 2 == 1 { 1.0 / k as f64 } else { 0.0 }).collect(),
        "triangle" => (1..=16).map(|k| if k % 2 == 1 { 1.0 / (k * k) as f64 } else { 0.0 }).collect(),
        _ => return None,
    };
    // The low-pass filter (a two-pole response above the cutoff), and the envelope: a held note sits at the sustain
    // level, so a sound that decays to a low sustain is heard quieter than its harmonics say (half onset, half sustain).
    let cutoff: Option<f64> = get("cutoff").and_then(|v| v.parse().ok());
    let sustain: f64 = get("sustain").and_then(|v| v.parse().ok()).unwrap_or(1.0);
    let s: f64 = levels
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let f = (i + 1) as f64 * hz;
            let c = roll.map_or(1.0, |r| (r / f).min(1.0));
            let lp = cutoff.map_or(1.0, |fc| 1.0 / (1.0 + (f / fc).powi(4)).sqrt());
            (a * c * lp).powi(2)
        })
        .sum();
    Some(s.sqrt() * (0.5 + 0.5 * sustain.clamp(0.0, 1.0))).filter(|l| *l > 0.0)
}

/// Whether the module playing `voice` in `eq` is a gliding line (tagged `glide` in its book).
fn glides(eq: &Equation, voice: &str) -> bool {
    use super::book::{self, Slot};
    let slot = match voice {
        "lead" => Slot::Lead,
        "counter" => Slot::Counter,
        "arp" => Slot::Arp,
        "bass" => Slot::Bass,
        "pad1" | "pad2" | "pad3" => Slot::Pads,
        _ => return false,
    };
    if eq.book.is_empty() {
        return false;
    }
    book::current(eq, slot).and_then(|id| book::of(eq).get(id)).is_some_and(|m| m.glide)
}

/// `eq` with every tonal voice (playing or waiting) wearing the palette's instrument for its role. The module's own
/// pitch, gate, amp, pan and sends stay; its gain is rescaled to keep its loudness.
pub fn dress(eq: &Equation, p: &Palette) -> Equation {
    let mut e = eq.clone();
    for b in e.voices.iter_mut().chain(e.stash.iter_mut()) {
        if b.is_drum() {
            continue;
        }
        let Some(role) = role(&b.name) else { continue };
        let Some(ins) = p.instruments.get(role) else { continue };
        let before = loudness(&b.fields, role);
        let rolloff = b.get("rolloff").map(str::to_string);
        b.fields.retain(|(k, _)| !TIMBRE_FIELDS.contains(&k.as_str()));
        b.set("wave", "additive");
        b.set("partials", &ins.partials.iter().map(|d| format!("{d}")).collect::<Vec<_>>().join(", "));
        b.set("rolloff", rolloff.as_deref().unwrap_or("660"));
        for (k, v) in &ins.fields {
            b.set(k, v);
        }
        // A gliding line (its module is tagged `glide`) holds its notes while it bends: a pluck or a bell would ring
        // once and fade under it. It keeps the drawn spectrum and filter, with a held envelope.
        if glides(eq, &b.name) {
            let num = |k: &str, d: f64| b.get(k).and_then(|v| v.trim().parse::<f64>().ok()).unwrap_or(d);
            let (sustain, decay) = (num("sustain", 1.0), num("decay", 300.0));
            b.set("sustain", &format!("{}", sustain.max(0.65)));
            b.set("decay", &format!("{}", decay.max(300.0)));
        }
        if let (Some(before), Some(after)) = (before, loudness(&b.fields, role)) {
            let g: f64 = b.get("gain").and_then(|g| g.trim().parse().ok()).unwrap_or(1.0);
            b.set("gain", &format!("{:.4}", (g * before / after).clamp(0.0, 10.0)));
        }
    }
    e
}

/// The critic's "sound bands" score of a rendered song (0..1): its five-band energy against the two references.
pub fn band_score(text: &str) -> Option<f64> {
    let song = crate::song::Song::parse(text).ok()?;
    let mix = crate::render::render(&song).ok()?;
    let mut rep = super::critic::Report { components: Vec::new() };
    rep.add_sound(&super::sound::measure(&mix));
    rep.get("sound bands").map(|c| c.score)
}

/// The probe a palette is judged on: `text` (a song holding the full band) cut to 16 beats at 16 kHz, so a check costs
/// a fraction of a stage's render.
fn probe(text: &str) -> String {
    let mut out = String::new();
    for l in text.lines() {
        let t = l.trim_start();
        if t.starts_with("beats ") {
            out.push_str("beats 16\n");
        } else if t.starts_with("rate ") {
            continue;
        } else {
            out.push_str(l);
            out.push('\n');
        }
    }
    out.replacen("beats 16\n", "beats 16\nrate 16000\n", 1)
}

/// The song's palette: the first of [`TRIES`] draws whose full band scores within [`MARGIN`] of the written
/// instruments' band score, else the best-scoring draw. `full_band` is an equation holding every roster voice.
/// Returns the palette with its score and the written instruments' score.
pub fn choose(seed: u64, full_band: &Equation) -> (Palette, f64, f64) {
    let plain = band_score(&probe(&full_band.song_text("instruments: written"))).unwrap_or(0.0);
    let mut best: Option<(Palette, f64)> = None;
    for attempt in 0..TRIES {
        let p = palette(seed, attempt);
        let s = band_score(&probe(&dress(full_band, &p).song_text("instruments: drawn"))).unwrap_or(0.0);
        if s >= plain - MARGIN {
            return (p, s, plain);
        }
        if best.as_ref().is_none_or(|(_, b)| s > *b) {
            best = Some((p, s));
        }
    }
    let (p, s) = best.expect("TRIES > 0");
    (p, s, plain)
}
