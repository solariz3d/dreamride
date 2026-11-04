//! The C ABI the world (the native host in `world/`) loads songs through. Built as `luminesce.dll`
//! (the crate is also a `cdylib`); the header is `world/src/luminesce.h`.
//!
//! One call renders a song and records what the renderer used ([`render_traced`]): the host plays
//! the audio buffer and draws each voice from the trace, so the picture and the sound come from
//! the same numbers. All functions are safe to call with null pointers (they return 0 or null),
//! and none of them panic across the boundary.
//!
//! Lifetime: everything a function returns a pointer to belongs to the song and lives until
//! [`lm_free`].

use crate::render::{render_traced, DrumHit, TonalFrame, VoiceTrace};
use crate::song::{DrumKind, Song, VoiceKind};
use std::ffi::{c_char, CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};

/// A loaded, rendered song. Opaque to C.
pub struct LmSong {
    song: Song,
    /// The mix, interleaved stereo f32 (left, right, left, ...).
    audio: Vec<f32>,
    frames: usize,
    gain: f64,
    step: usize,
    voices: Vec<VoiceData>,
}

struct VoiceData {
    name: CString,
    trace: VoiceTrace,
}

/// The song's settings and sizes.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct LmInfo {
    pub bpm: f64,
    pub beats: f64,
    /// Seconds: `beats * 60 / bpm`.
    pub duration: f64,
    /// The frequency of pitch 0 in Hz.
    pub reference_hz: f64,
    /// The normalisation gain the renderer applied to the mix (1 when none).
    pub mix_gain: f64,
    pub rate: u32,
    /// Stereo frames in the audio buffer.
    pub frames: u64,
    pub voice_count: u32,
    /// Samples between tonal frames: frame `k` is at sample `k * trace_step`.
    pub trace_step: u32,
    /// 1 if the song is a loop (`loop 1`).
    pub looped: u32,
}

pub const LM_TONAL: u32 = 0;
pub const LM_DRUM: u32 = 1;

/// One voice: its name, kind and the size of its trace. For drums, the shape of one hit's pitch
/// sweep, `f(tau) = to + (from - to) e^(-tau / dash)` (the built-in kick's own sweep; the
/// built-in snare and hat do not sweep, and give `from == to`).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LmVoice {
    /// Nul-terminated UTF-8, owned by the song.
    pub name: *const c_char,
    /// LM_TONAL or LM_DRUM.
    pub kind: u32,
    /// For drums: 0 kick, 1 snare, 2 hat, 3 dash. 0 for tonal voices.
    pub drum_kind: u32,
    pub gain: f64,
    pub pan: f64,
    /// Tonal voices: the number of frames (`lm_frames`). Drums: 0.
    pub frame_count: u64,
    /// Drums: the number of hits (`lm_hits`). Tonal voices: 0.
    pub hit_count: u64,
    pub drum_from_hz: f64,
    pub drum_to_hz: f64,
    pub drum_dash_ms: f64,
    /// The length of one hit in ms.
    pub drum_length_ms: f64,
}

/// A drum hit, with its time in the two units the host needs.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LmHit {
    pub sample: u64,
    pub seconds: f64,
    pub beat: f64,
    /// The `amp` expression's value at the onset, before the voice's gain.
    pub amp: f64,
}

/// `TonalFrame` crosses the boundary as is: gate, pitch (semitones, NaN where not evaluated), amp,
/// level (envelope x amp x gain), four f64.
pub type LmFrame = TonalFrame;

fn write_error(buf: *mut c_char, cap: usize, message: &str) {
    if buf.is_null() || cap == 0 {
        return;
    }
    // Truncate on a character boundary so the C side always gets valid UTF-8.
    let mut end = message.len().min(cap - 1);
    while !message.is_char_boundary(end) {
        end -= 1;
    }
    // SAFETY: the caller promises `buf` holds `cap` bytes; `end < cap`.
    unsafe {
        std::ptr::copy_nonoverlapping(message.as_ptr(), buf as *mut u8, end);
        *buf.add(end) = 0;
    }
}

fn load(src: &str, trace_step: u32) -> Result<LmSong, String> {
    let song = Song::parse(src).map_err(|e| e.to_string())?;
    let step = if trace_step == 0 { (song.rate as usize / 1000).max(1) } else { trace_step as usize };
    let (mix, trace) = render_traced(&song, step).map_err(|e| e.to_string())?;
    let frames = mix.frames();
    let mut audio = Vec::with_capacity(frames * 2);
    for i in 0..frames {
        audio.push(mix.left[i] as f32);
        audio.push(mix.right[i] as f32);
    }
    let voices = song
        .voices
        .iter()
        .zip(trace.voices)
        .map(|(v, t)| VoiceData {
            // Names are validated identifiers in the song format, so they hold no nul byte.
            name: CString::new(v.name.as_str()).unwrap_or_default(),
            trace: t,
        })
        .collect();
    Ok(LmSong { song, audio, frames, gain: mix.gain, step, voices })
}

fn load_from(src: Result<String, String>, trace_step: u32, err: *mut c_char, cap: usize) -> *mut LmSong {
    let result = catch_unwind(AssertUnwindSafe(|| src.and_then(|s| load(&s, trace_step))));
    match result {
        Ok(Ok(song)) => Box::into_raw(Box::new(song)),
        Ok(Err(message)) => {
            write_error(err, cap, &message);
            std::ptr::null_mut()
        }
        Err(_) => {
            write_error(err, cap, "internal error: the engine panicked while loading the song");
            std::ptr::null_mut()
        }
    }
}

/// # Safety
/// `p` must be null or a nul-terminated string.
unsafe fn read_str(p: *const c_char, what: &str) -> Result<String, String> {
    if p.is_null() {
        return Err(format!("{what} is null"));
    }
    CStr::from_ptr(p).to_str().map(str::to_owned).map_err(|_| format!("{what} is not UTF-8"))
}

/// Load a song file, render it, and record its trace every `trace_step` samples (0: one frame per
/// millisecond). Returns null on any error and writes the message (nul-terminated, truncated to
/// `err_cap`) into `err`, which may be null.
///
/// # Safety
/// `path` must be a nul-terminated UTF-8 path; `err` null or `err_cap` writable bytes.
#[no_mangle]
pub unsafe extern "C" fn lm_load_file(path: *const c_char, trace_step: u32, err: *mut c_char, err_cap: usize) -> *mut LmSong {
    let src = read_str(path, "the path").and_then(|p| std::fs::read_to_string(&p).map_err(|e| format!("cannot read {p}: {e}")));
    load_from(src, trace_step, err, err_cap)
}

/// As [`lm_load_file`], from the song's text.
///
/// # Safety
/// `source` must be nul-terminated UTF-8; `err` null or `err_cap` writable bytes.
#[no_mangle]
pub unsafe extern "C" fn lm_load_source(source: *const c_char, trace_step: u32, err: *mut c_char, err_cap: usize) -> *mut LmSong {
    load_from(read_str(source, "the source"), trace_step, err, err_cap)
}

/// Free a song. Null is ignored.
///
/// # Safety
/// `song` must come from `lm_load_*` and not have been freed.
#[no_mangle]
pub unsafe extern "C" fn lm_free(song: *mut LmSong) {
    if !song.is_null() {
        drop(Box::from_raw(song));
    }
}

/// # Safety
/// `song` null or live; `out` null or writable.
#[no_mangle]
pub unsafe extern "C" fn lm_info(song: *const LmSong, out: *mut LmInfo) -> i32 {
    let (Some(s), Some(out)) = (song.as_ref(), out.as_mut()) else { return 0 };
    *out = LmInfo {
        bpm: s.song.bpm,
        beats: s.song.beats,
        duration: s.song.duration_secs(),
        reference_hz: s.song.reference_hz,
        mix_gain: s.gain,
        rate: s.song.rate,
        frames: s.frames as u64,
        voice_count: s.voices.len() as u32,
        trace_step: s.step as u32,
        looped: s.song.effects.looped as u32,
    };
    1
}

/// The mix as interleaved stereo f32, `2 * frames` values; null for a null song.
///
/// # Safety
/// `song` null or live.
#[no_mangle]
pub unsafe extern "C" fn lm_audio(song: *const LmSong) -> *const f32 {
    song.as_ref().map_or(std::ptr::null(), |s| s.audio.as_ptr())
}

/// Describe voice `index`. Returns 0 if the song is null or the index is out of range.
///
/// # Safety
/// `song` null or live; `out` null or writable.
#[no_mangle]
pub unsafe extern "C" fn lm_voice(song: *const LmSong, index: u32, out: *mut LmVoice) -> i32 {
    let (Some(s), Some(out)) = (song.as_ref(), out.as_mut()) else { return 0 };
    let (Some(v), Some(d)) = (s.song.voices.get(index as usize), s.voices.get(index as usize)) else { return 0 };
    let mut info = LmVoice {
        name: d.name.as_ptr(),
        kind: LM_TONAL,
        drum_kind: 0,
        gain: v.gain,
        pan: v.pan,
        frame_count: 0,
        hit_count: 0,
        drum_from_hz: 0.0,
        drum_to_hz: 0.0,
        drum_dash_ms: 0.0,
        drum_length_ms: 0.0,
    };
    match (&v.kind, &d.trace) {
        (VoiceKind::Tonal(_), VoiceTrace::Tonal(f)) => info.frame_count = f.len() as u64,
        (VoiceKind::Drum(drum), VoiceTrace::Drum(h)) => {
            info.kind = LM_DRUM;
            info.hit_count = h.len() as u64;
            // The built-in drums' numbers are the renderer's (render.rs, drum_sample).
            let (kind, from, to, dash, len) = match (drum.kind, &drum.dash) {
                (DrumKind::Dash, Some(x)) => (3, x.from_hz, x.to_hz, x.dash_ms, x.length_ms),
                (DrumKind::Kick, _) => (0, 160.0, 48.0, 35.0, 600.0),
                (DrumKind::Snare, _) => (1, 185.0, 185.0, 0.0, 300.0),
                (DrumKind::Hat, _) => (2, 8000.0, 8000.0, 0.0, 120.0),
                (DrumKind::Dash, None) => return 0,
            };
            info.drum_kind = kind;
            info.drum_from_hz = from;
            info.drum_to_hz = to;
            info.drum_dash_ms = dash;
            info.drum_length_ms = len;
        }
        _ => return 0,
    }
    *out = info;
    1
}

/// A tonal voice's frames (`frame_count` of them); null for a drum, a null song or a bad index.
///
/// # Safety
/// `song` null or live.
#[no_mangle]
pub unsafe extern "C" fn lm_frames(song: *const LmSong, index: u32) -> *const LmFrame {
    match song.as_ref().and_then(|s| s.voices.get(index as usize)).map(|d| &d.trace) {
        Some(VoiceTrace::Tonal(f)) => f.as_ptr(),
        _ => std::ptr::null(),
    }
}

/// A drum voice's hits, copied into `out` (up to `cap`). Returns the number written; 0 for a
/// tonal voice, a null song or a bad index.
///
/// # Safety
/// `song` null or live; `out` null or `cap` writable `LmHit`s.
#[no_mangle]
pub unsafe extern "C" fn lm_hits(song: *const LmSong, index: u32, out: *mut LmHit, cap: u64) -> u64 {
    let Some(s) = song.as_ref() else { return 0 };
    let Some(VoiceTrace::Drum(hits)) = s.voices.get(index as usize).map(|d| &d.trace) else { return 0 };
    if out.is_null() {
        return 0;
    }
    let rate = s.song.rate as f64;
    let n = hits.len().min(cap as usize);
    for (k, h) in hits.iter().take(n).enumerate() {
        *out.add(k) = hit(h, rate, s.song.bpm);
    }
    n as u64
}

fn hit(h: &DrumHit, rate: f64, bpm: f64) -> LmHit {
    // The beat is the renderer's own `t` for that sample: i * bpm / (60 * rate).
    LmHit { sample: h.sample, seconds: h.sample as f64 / rate, beat: h.sample as f64 * bpm / (60.0 * rate), amp: h.amp }
}

/// The state of tonal voice `index` at `seconds` into the song: the recorded frame at or before
/// that time (frames are `trace_step` samples apart). Returns 0, leaving `out` untouched, for a
/// drum, a bad index, or a time outside [0, duration).
///
/// # Safety
/// `song` null or live; `out` null or writable.
#[no_mangle]
pub unsafe extern "C" fn lm_sample(song: *const LmSong, index: u32, seconds: f64, out: *mut LmFrame) -> i32 {
    let (Some(s), Some(out)) = (song.as_ref(), out.as_mut()) else { return 0 };
    let Some(VoiceTrace::Tonal(frames)) = s.voices.get(index as usize).map(|d| &d.trace) else { return 0 };
    if !(seconds >= 0.0) {
        return 0; // also rejects NaN
    }
    let sample = (seconds * s.song.rate as f64).floor();
    if sample >= s.frames as f64 {
        return 0;
    }
    match frames.get(sample as usize / s.step) {
        Some(f) => {
            *out = *f;
            1
        }
        None => 0,
    }
}

// ------------------------------------------------------------------------------------------------ stream mode

use crate::compose::stream::Seed;
use crate::stream::Stream;
use std::collections::HashMap;
use std::sync::Mutex;

/// An endless song (stream mode). Opaque to C. One thread may pull audio while another reads stages and windows.
pub struct LmStream {
    stream: Stream,
    /// C copies of each stage's strings, made when first asked for and kept until [`lm_stream_free`].
    strings: Mutex<HashMap<u64, Box<StageStrings>>>,
    roster: Vec<CString>,
}

struct StageStrings {
    equation: CString,
    voices: CString,
    changed: CString,
    summary: CString,
    text: CString,
}

/// The stream's settings and progress.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct LmStreamInfo {
    pub bpm: f64,
    pub reference_hz: f64,
    /// Beats per stage (32: eight bars).
    pub stage_beats: f64,
    pub seed: u64,
    pub rate: u32,
    pub trace_step: u32,
    /// Voices the stream can ever have (the roster): every window has them all, in this order.
    pub roster_count: u32,
    pub stages_composed: u64,
    pub stages_rendered: u64,
    /// Frames handed out by `lm_stream_pull` so far.
    pub frames_read: u64,
}

/// One stage. The strings belong to the stream and live until `lm_stream_free`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LmStageInfo {
    /// From 0 (the overlay shows index + 1).
    pub index: u64,
    /// The absolute frame of its first sample, and its length in frames.
    pub start_frame: u64,
    pub frames: u64,
    pub start_beat: f64,
    pub beats: f64,
    /// 1 when its audio has been rendered.
    pub rendered: u32,
    /// 1 when it is a turn (a bigger change).
    pub turn: u32,
    /// The critic's score of the stage from its functions, 0..1.
    pub score: f64,
    /// Display lines, separated by '\n': a kind ('D' definition, 'T' time name), '1' if it changed at this stage
    /// else '0', a tab, then the markup (src/compose/display.rs).
    pub equation: *const c_char,
    /// The voices present, separated by spaces, in roster order.
    pub voices: *const c_char,
    /// Names (definitions and voices) changed since the stage before, separated by spaces; empty for stage 1.
    pub changed: *const c_char,
    /// What changed, in words.
    pub summary: *const c_char,
    /// The stage as a song file.
    pub text: *const c_char,
}

/// A seeded stream (the world's default **Stream**): since 2026-11-03 stage 1 is assembled from the move book. The
/// name is kept so the world binary and its header need no change.
pub const LM_STREAM_GRAMMAR: u32 = 1;
/// With `LM_STREAM_GRAMMAR`: the stream v2 seed instead, the best of six grammar songs from the seed.
pub const LM_STREAM_GRAMMAR_SONG: u32 = 2;
/// With `LM_STREAM_GRAMMAR`: the lens species alone (the move book as first written), never another species. Without
/// it a seeded stream picks its species from the seed (since 2026-11-03).
pub const LM_STREAM_LENS: u32 = 4;
/// With `LM_STREAM_GRAMMAR`: a species of strange tunings instead (the Explore option, 2026-11-03: 19-tone loops,
/// Bohlen-Pierce, quarter-tone maqam, just ratios), never one of the default species, which carry written harmony.
pub const LM_STREAM_EXPLORE: u32 = 8;

fn cstring(s: &str) -> CString {
    CString::new(s.replace('\0', " ")).unwrap_or_default()
}

/// Whether the stream's song has ended (a stream from the move book composes its own ending, see docs/composer.md).
/// Returns 1 and writes its length in frames (the ring-out included) to `frames` (when not null) once it has, else 0.
/// `lm_stream_pull` then returns fewer frames than asked at that frame, and 0 after it.
///
/// # Safety
/// `stream` null or live; `frames` null or writable.
#[no_mangle]
pub unsafe extern "C" fn lm_stream_ended(stream: *const LmStream, frames: *mut u64) -> i32 {
    let Some(s) = stream.as_ref() else { return 0 };
    match s.stream.ended_frames() {
        Some(n) => {
            if let Some(f) = frames.as_mut() {
                *f = n;
            }
            1
        }
        None => 0,
    }
}

/// Start a stream. `seed_song` is a song's text to start from, or null for the default (the kept squares song);
/// with `LM_STREAM_GRAMMAR` in `flags` the song is ignored and the stream is a song species chosen by `seed` and
/// arranged by it (with `LM_STREAM_LENS` as well: the lens species only, the move book as first written; with
/// `LM_STREAM_EXPLORE`: a species of strange tunings, the Explore option; with `LM_STREAM_GRAMMAR_SONG`: stage 1 is
/// the best of six grammar songs from `seed`, as in stream v2).
/// `seed` drives the evolution. Stage 1 is rendered before this returns; a worker thread then keeps `lookahead`
/// stages (at least 1) rendered ahead of the stage being pulled. `trace_step` as for `lm_load_*`. Null on error, with
/// the message in `err`.
///
/// # Safety
/// `seed_song` null or nul-terminated UTF-8; `err` null or `err_cap` writable bytes.
#[no_mangle]
pub unsafe extern "C" fn lm_stream_start(
    seed_song: *const c_char,
    seed: u64,
    flags: u32,
    lookahead: u32,
    trace_step: u32,
    err: *mut c_char,
    err_cap: usize,
) -> *mut LmStream {
    let grammar = flags & LM_STREAM_GRAMMAR != 0;
    let text = if seed_song.is_null() || grammar { Ok(None) } else { read_str(seed_song, "the seed song").map(Some) };
    let result = catch_unwind(AssertUnwindSafe(|| {
        let seed = match text? {
            _ if grammar && flags & LM_STREAM_GRAMMAR_SONG != 0 => Seed::Grammar(seed),
            _ if grammar && flags & LM_STREAM_LENS != 0 => Seed::Book(seed),
            _ if grammar && flags & LM_STREAM_EXPLORE != 0 => Seed::Explore(seed),
            _ if grammar => Seed::Species(seed),
            Some(text) => Seed::Song { text, seed },
            None => Seed::Song { text: crate::compose::stream::DEFAULT_SEED_SONG.to_string(), seed },
        };
        let stream = Stream::start(seed, lookahead as u64, trace_step as usize)?;
        let roster = stream.roster.iter().map(|n| cstring(n)).collect();
        Ok::<LmStream, String>(LmStream { stream, strings: Mutex::new(HashMap::new()), roster })
    }));
    match result {
        Ok(Ok(s)) => Box::into_raw(Box::new(s)),
        Ok(Err(message)) => {
            write_error(err, err_cap, &message);
            std::ptr::null_mut()
        }
        Err(_) => {
            write_error(err, err_cap, "internal error: the engine panicked while starting the stream");
            std::ptr::null_mut()
        }
    }
}

/// Stop the stream's worker and free it. Null is ignored. No other call on it may be running.
///
/// # Safety
/// `stream` from `lm_stream_start`, not freed.
#[no_mangle]
pub unsafe extern "C" fn lm_stream_free(stream: *mut LmStream) {
    if !stream.is_null() {
        let _ = catch_unwind(AssertUnwindSafe(|| drop(Box::from_raw(stream))));
    }
}

/// # Safety
/// `stream` null or live; `out` null or writable.
#[no_mangle]
pub unsafe extern "C" fn lm_stream_info(stream: *const LmStream, out: *mut LmStreamInfo) -> i32 {
    let (Some(s), Some(out)) = (stream.as_ref(), out.as_mut()) else { return 0 };
    let st = &s.stream;
    *out = LmStreamInfo {
        bpm: st.bpm,
        reference_hz: st.reference_hz,
        stage_beats: crate::compose::stream::STAGE_BEATS,
        seed: st.seed,
        rate: st.rate,
        trace_step: st.step as u32,
        roster_count: s.roster.len() as u32,
        stages_composed: st.composed(),
        stages_rendered: st.rendered(),
        frames_read: st.frames_read(),
    };
    1
}

/// The name of roster voice `index` (owned by the stream); null when out of range.
///
/// # Safety
/// `stream` null or live.
#[no_mangle]
pub unsafe extern "C" fn lm_stream_roster(stream: *const LmStream, index: u32) -> *const c_char {
    stream.as_ref().and_then(|s| s.roster.get(index as usize)).map_or(std::ptr::null(), |c| c.as_ptr())
}

/// Write the next `frames` stereo frames (interleaved f32, `2 * frames` values) into `out`, each stage at its gain.
/// Blocks while the stage it needs is still rendering (with a lookahead of 1 or more it should not). Returns the
/// frames written: fewer than asked only if the stream failed (then `lm_stream_error` says why).
///
/// # Safety
/// `stream` null or live; `out` null or `2 * frames` writable floats.
#[no_mangle]
pub unsafe extern "C" fn lm_stream_pull(stream: *const LmStream, out: *mut f32, frames: u64) -> u64 {
    let Some(s) = stream.as_ref() else { return 0 };
    if out.is_null() || frames == 0 {
        return 0;
    }
    let buf = std::slice::from_raw_parts_mut(out, (frames * 2) as usize);
    match catch_unwind(AssertUnwindSafe(|| s.stream.pull(buf))) {
        Ok(Ok(n)) => n as u64,
        _ => 0,
    }
}

/// The stage containing absolute frame `frame`.
///
/// # Safety
/// `stream` null or live.
#[no_mangle]
pub unsafe extern "C" fn lm_stream_stage_at(stream: *const LmStream, frame: u64) -> u64 {
    stream.as_ref().map_or(0, |s| s.stream.stage_of(frame))
}

/// Describe stage `index`. Returns 0 (out untouched) when it has not been composed yet. The stage after the one
/// being pulled is always composed (the upcoming equation).
///
/// # Safety
/// `stream` null or live; `out` null or writable.
#[no_mangle]
pub unsafe extern "C" fn lm_stream_stage(stream: *const LmStream, index: u64, out: *mut LmStageInfo) -> i32 {
    let (Some(s), Some(out)) = (stream.as_ref(), out.as_mut()) else { return 0 };
    let r = catch_unwind(AssertUnwindSafe(|| {
        let rec = s.stream.record(index)?;
        let mut map = s.strings.lock().ok()?;
        let strings = map.entry(index).or_insert_with(|| {
            Box::new(StageStrings {
                equation: cstring(&rec.display.join("\n")),
                voices: cstring(&rec.voices.join(" ")),
                changed: cstring(&rec.stage.changed.join(" ")),
                summary: cstring(&rec.stage.summary),
                text: cstring(&rec.stage.text),
            })
        });
        let start = s.stream.stage_start(index);
        Some(LmStageInfo {
            index,
            start_frame: start,
            frames: s.stream.stage_start(index + 1) - start,
            start_beat: index as f64 * crate::compose::stream::STAGE_BEATS,
            beats: crate::compose::stream::STAGE_BEATS,
            rendered: (index < s.stream.rendered()) as u32,
            turn: rec.stage.turn as u32,
            score: rec.stage.score,
            equation: strings.equation.as_ptr(),
            voices: strings.voices.as_ptr(),
            changed: strings.changed.as_ptr(),
            summary: strings.summary.as_ptr(),
            text: strings.text.as_ptr(),
        })
    }));
    match r {
        Ok(Some(info)) => {
            *out = info;
            1
        }
        _ => 0,
    }
}

/// The voice record of stages `first..=last` (rendered and still kept: the stage being pulled, the two before it and
/// those rendered ahead) as an ordinary song handle, read with `lm_info`, `lm_voice`, `lm_frames`, `lm_hits` and
/// `lm_sample`: every roster voice, in roster order (an absent voice has closed frames and no hits). Its time 0 is
/// absolute frame `*start_frame` (a multiple of the trace step, at or just before the first stage's start). Free it
/// with `lm_free`. Null on error, with the message in `err`.
///
/// # Safety
/// `stream` null or live; `start_frame` null or writable; `err` null or `err_cap` writable bytes.
#[no_mangle]
pub unsafe extern "C" fn lm_stream_window(
    stream: *const LmStream,
    first: u64,
    last: u64,
    start_frame: *mut u64,
    err: *mut c_char,
    err_cap: usize,
) -> *mut LmSong {
    let Some(s) = stream.as_ref() else {
        write_error(err, err_cap, "the stream is null");
        return std::ptr::null_mut();
    };
    let r = catch_unwind(AssertUnwindSafe(|| -> Result<(LmSong, u64), String> {
        let st = &s.stream;
        let (w, traces, audio) = st.window(first, last)?;
        let frames = audio.len() / 2;
        let mut song = st.roster_song.clone();
        let mut voices = Vec::new();
        let mut data = Vec::new();
        for (name, trace) in st.roster.iter().zip(traces) {
            let v = song.voices.iter().find(|v| &v.name == name).cloned().ok_or_else(|| format!("no roster voice '{name}'"))?;
            voices.push(v);
            data.push(VoiceData { name: cstring(name), trace });
        }
        song.voices = voices;
        song.beats = frames as f64 * st.bpm / (60.0 * st.rate as f64);
        song.effects.looped = false;
        Ok((LmSong { song, audio, frames, gain: 1.0, step: st.step, voices: data }, w))
    }));
    match r {
        Ok(Ok((song, w))) => {
            if let Some(p) = start_frame.as_mut() {
                *p = w;
            }
            Box::into_raw(Box::new(song))
        }
        Ok(Err(message)) => {
            write_error(err, err_cap, &message);
            std::ptr::null_mut()
        }
        Err(_) => {
            write_error(err, err_cap, "internal error: the engine panicked while building the stream's window");
            std::ptr::null_mut()
        }
    }
}

/// Save stages `last + 1 - count ..= last` as one ordinary song file at `path`. Returns 1, or 0 with the message in
/// `err`.
///
/// # Safety
/// `stream` null or live; `path` nul-terminated UTF-8; `err` null or `err_cap` writable bytes.
#[no_mangle]
pub unsafe extern "C" fn lm_stream_save(stream: *const LmStream, last: u64, count: u32, path: *const c_char, err: *mut c_char, err_cap: usize) -> i32 {
    let Some(s) = stream.as_ref() else {
        write_error(err, err_cap, "the stream is null");
        return 0;
    };
    let p = match read_str(path, "the path") {
        Ok(p) => p,
        Err(m) => {
            write_error(err, err_cap, &m);
            return 0;
        }
    };
    match catch_unwind(AssertUnwindSafe(|| s.stream.save(last, count, std::path::Path::new(&p)))) {
        Ok(Ok(_)) => 1,
        Ok(Err(m)) => {
            write_error(err, err_cap, &m);
            0
        }
        Err(_) => {
            write_error(err, err_cap, "internal error: the engine panicked while saving");
            0
        }
    }
}

/// Why the stream stopped, if it failed: writes the message into `buf` and returns 1; 0 when it has not failed.
///
/// # Safety
/// `stream` null or live; `buf` null or `cap` writable bytes.
#[no_mangle]
pub unsafe extern "C" fn lm_stream_error(stream: *const LmStream, buf: *mut c_char, cap: usize) -> i32 {
    match stream.as_ref().and_then(|s| s.stream.error()) {
        Some(e) => {
            write_error(buf, cap, &e);
            1
        }
        None => 0,
    }
}
