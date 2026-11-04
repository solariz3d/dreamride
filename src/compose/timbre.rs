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
                1 => {
                    fields.extend([
                        f("attack", real(r, 5.0, 25.0)),
                        f("decay", real(r, 200.0, 500.0)),
                        f("sustain", real(r, 0.6, 0.9)),
                        f("release", real(r, 120.0, 300.0)),
                    ]);
                    // a held note varies in what is heard while it holds: an opening filter at each onset, or none
                    if r.chance(0.5) {
                        fields.extend([f("cutoff", real(r, 1800.0, 5000.0)), f("filter_env", real(r, 0.5, 2.0)), f("filter_decay", real(r, 120.0, 400.0))]);
                    } else {
                        fields.push(f("cutoff", real(r, 2500.0, 6000.0)));
                    }
                }
                _ => fields.extend([
                    f("attack", real(r, 1.0, 3.0)),
                    f("decay", real(r, 500.0, 1200.0)),
                    f("sustain", real(r, 0.0, 0.2)),
                    f("release", real(r, 300.0, 600.0)),
                ]),
            }
            if r.chance(0.55) {
                fields.push(f("detune", real(r, 3.0, 14.0)));
            }
            if r.chance(0.45) {
                fields.push(f("drive", real(r, 0.2, 2.5)));
            }
            (["pluck", "held", "bell"][kind as usize], p)
        }
    };
    // The rolloff: the graph calculator's dark cap (660 Hz) about a third of the time, else a brighter one. With 660 at
    // a lead's usual pitch every harmonic is already capped, so all spectra sound nearly alike (measured 2026-11-04:
    // heard brightness 1.02 to 1.24 harmonics over 8 songs, against 1.09 to 2.81 drawn).
    let rolloff = if r.chance(0.35) { 660.0 } else { 1500.0 * 4f64.powf(r.unit()) };
    fields.push(f("rolloff", rolloff));
    Instrument { kind: kind.to_string(), partials, fields }
}

/// The drum voices a kit is drawn for (the book's drum slots).
pub const DRUMS: [&str; 4] = ["kick", "clap", "hat", "perc"];

/// The fields a drawn drum sets; trigger, amp, gain, pan and the sends stay the module's.
pub const DRUM_FIELDS: [&str; 9] = ["kind", "from", "to", "dash", "hold", "decay", "length", "partials", "rolloff"];

/// The most a drawn metallic hat may sweep (from over to). A dash that falls 7600 to 5400 Hz in 5 ms is a chirp, and
/// the keeper heard the hats tweet (2026-11-04).
pub const HAT_SWEEP: f64 = 1.05;

fn dash_fields(from: f64, to: f64, dash: f64, hold: f64, decay: f64, length: f64, partials: &[f64]) -> Vec<(String, String)> {
    let p = partials.iter().map(|d| format!("{}", (d * 10.0).round() / 10.0)).collect::<Vec<_>>().join(", ");
    vec![("kind".into(), "dash".into()), f("from", from), f("to", to), f("dash", dash), f("hold", hold), f("decay", decay), f("length", length), ("partials".into(), p), f("rolloff", 660.0)]
}

fn noise_fields(kind: &str, decay: f64, length: f64) -> Vec<(String, String)> {
    vec![("kind".into(), kind.into()), f("decay", decay), f("length", length)]
}

fn draw_drum(voice: &str, r: &mut Rng) -> Instrument {
    let (kind, fields) = match voice {
        "kick" => {
            let p = [0.0, -real(r, 12.0, 30.0), -real(r, 8.0, 20.0), -real(r, 14.0, 26.0)];
            ("kick", dash_fields(real(r, 180.0, 620.0), real(r, 40.0, 65.0), real(r, 10.0, 40.0), real(r, 0.0, 50.0), real(r, 60.0, 180.0), real(r, 250.0, 500.0), &p))
        }
        "clap" => {
            if r.chance(0.55) {
                ("noise snare", noise_fields("snare", real(r, 40.0, 140.0), real(r, 150.0, 400.0)))
            } else {
                let p = [0.0, -real(r, 10.0, 22.0), -real(r, 6.0, 14.0), -real(r, 12.0, 20.0), -real(r, 12.0, 20.0)];
                ("dash clap", dash_fields(real(r, 700.0, 1200.0), real(r, 150.0, 260.0), real(r, 5.0, 14.0), real(r, 2.0, 10.0), real(r, 40.0, 90.0), real(r, 180.0, 320.0), &p))
            }
        }
        "hat" => {
            if r.chance(0.6) {
                ("noise hat", noise_fields("hat", real(r, 8.0, 45.0), real(r, 60.0, 200.0)))
            } else {
                let to = real(r, 4500.0, 9000.0);
                let p = [0.0, -real(r, 4.0, 12.0), -real(r, 6.0, 16.0)];
                ("metal hat", dash_fields(to * real(r, 1.0, HAT_SWEEP), to, real(r, 1.0, 4.0), 0.0, real(r, 6.0, 25.0), real(r, 50.0, 140.0), &p))
            }
        }
        _ => {
            if r.chance(0.6) {
                ("noise open hat", noise_fields("hat", real(r, 60.0, 200.0), real(r, 200.0, 500.0)))
            } else {
                let to = real(r, 4000.0, 8000.0);
                let p = [0.0, -real(r, 3.0, 10.0), -real(r, 5.0, 14.0)];
                ("metal open hat", dash_fields(to * real(r, 1.0, HAT_SWEEP), to, real(r, 2.0, 6.0), 0.0, real(r, 60.0, 180.0), real(r, 200.0, 450.0), &p))
            }
        }
    };
    Instrument { kind: kind.to_string(), partials: vec![0.0], fields }
}

/// A song's instruments, drawn from its seed (`attempt` draws again).
pub fn palette(seed: u64, attempt: u64) -> Palette {
    let mut r = Rng::for_candidate(seed ^ 0x5449_4D42_5245, attempt); // "TIMBRE"
    let mut instruments: BTreeMap<&'static str, Instrument> = ROLES.iter().map(|role| (*role, draw(role, &mut r))).collect();
    for d in DRUMS {
        instruments.insert(d, draw_drum(d, &mut r));
    }
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
            dress_drum(b, p);
            continue;
        }
        let Some(role) = role(&b.name) else { continue };
        let Some(ins) = p.instruments.get(role) else { continue };
        let before = loudness(&b.fields, role);
        b.fields.retain(|(k, _)| !TIMBRE_FIELDS.contains(&k.as_str()));
        b.set("wave", "additive");
        b.set("partials", &ins.partials.iter().map(|d| format!("{d}")).collect::<Vec<_>>().join(", "));
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

/// The energy of one hit of a drum's sound alone (at full level, gain 1), for matching a drawn drum's loudness to the
/// written one's. Cached by the sound, since a song dresses every stage.
fn hit_energy(fields: &[(String, String)]) -> Option<f64> {
    use std::sync::{Mutex, OnceLock};
    static CACHE: OnceLock<Mutex<BTreeMap<String, Option<f64>>>> = OnceLock::new();
    let key: String = fields.iter().filter(|(k, _)| DRUM_FIELDS.contains(&k.as_str())).map(|(k, v)| format!("  {k} = {v}\n")).collect();
    let cache = CACHE.get_or_init(|| Mutex::new(BTreeMap::new()));
    if let Some(e) = cache.lock().expect("the hit cache").get(&key) {
        return *e;
    }
    let text = format!("bpm 60\nbeats 1\nrate 16000\ndrum x {{\n  trigger = 0.5 - t\n{key}}}\n");
    let e = crate::song::Song::parse(&text)
        .ok()
        .and_then(|song| crate::render::render(&song).ok())
        .map(|m| m.left.iter().chain(&m.right).map(|x| x * x).sum::<f64>() / (m.gain * m.gain).max(1e-18))
        .filter(|e| *e > 0.0);
    cache.lock().expect("the hit cache").insert(key, e);
    e
}

/// A drum voice wearing the palette's drum for its slot: its sound fields replaced, its trigger, amp, pan and sends
/// kept, its gain rescaled so one hit carries the written sound's energy.
fn dress_drum(b: &mut super::stream::Block, p: &Palette) {
    let Some(ins) = DRUMS.iter().find(|d| **d == b.name).and_then(|d| p.instruments.get(d)) else { return };
    let before = hit_energy(&b.fields);
    b.fields.retain(|(k, _)| !DRUM_FIELDS.contains(&k.as_str()));
    for (k, v) in &ins.fields {
        b.set(k, v);
    }
    if let (Some(before), Some(after)) = (before, hit_energy(&b.fields)) {
        let g: f64 = b.get("gain").and_then(|g| g.trim().parse().ok()).unwrap_or(1.0);
        b.set("gain", &format!("{:.4}", (g * (before / after).sqrt()).clamp(0.0, 10.0)));
    }
}
