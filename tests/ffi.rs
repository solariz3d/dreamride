//! The C ABI the world loads songs through (src/ffi.rs), and the trace under it (render_traced).
//!
//! The claim under test: what the world draws is what the renderer used. So the trace is checked
//! against an independent evaluation of each voice's own expressions and of the documented
//! envelope, the traced mix against the untraced one bit for bit, and the C functions against the
//! trace, including every way a C caller can hand them something wrong.

use luminesce::expr::At;
use luminesce::ffi::*;
use luminesce::render::{render, render_traced, TonalFrame, VoiceTrace};
use luminesce::song::{Song, VoiceKind};
use std::ffi::{CStr, CString};
use std::ptr::{null, null_mut};

/// Two tonal voices (one reading note time in its pitch, an amp expression, a full envelope; one
/// with a long release and a send) and two drums (a built-in kick with velocities, a dash).
const SONG: &str = "bpm 120
beats 8
rate 8000
reverb_size 0.6
voice lead {
  wave = sine
  pitch = floor(t) + 0.5 sin(6 note_age) - 0.3 (note_left < 0.05)
  gate = 0.6 - frac(t)
  amp = 0.5 + 0.25 sin(t)
  attack = 20
  decay = 50
  sustain = 0.6
  release = 80
  gain = 0.4
}
voice pad {
  wave = triangle
  pitch = -12 + 0.1 t
  gate = (t < 3) + (t >= 5) - 0.5
  release = 400
  pan = -0.3
  reverb = 0.3
}
drum k {
  kind = kick
  trigger = 0.25 - frac(t)
  amp = 1 - 0.1 floor(t)
}
drum d {
  kind = dash
  trigger = 0.1 - frac(2t)
  from = 900
  to = 100
  dash = 10
  length = 120
}
";

fn parse(src: &str) -> Song {
    Song::parse(src).unwrap_or_else(|e| panic!("{e}"))
}

fn load(src: &str, step: u32) -> *mut LmSong {
    let c = CString::new(src).unwrap();
    let mut err = [0 as std::ffi::c_char; 256];
    let s = unsafe { lm_load_source(c.as_ptr(), step, err.as_mut_ptr(), err.len()) };
    assert!(!s.is_null(), "load failed: {}", unsafe { CStr::from_ptr(err.as_ptr()) }.to_string_lossy());
    s
}

fn info(s: *const LmSong) -> LmInfo {
    let mut i = LmInfo::default();
    assert_eq!(unsafe { lm_info(s, &mut i) }, 1);
    i
}

fn voice(s: *const LmSong, index: u32) -> LmVoice {
    let mut v = std::mem::MaybeUninit::<LmVoice>::uninit();
    assert_eq!(unsafe { lm_voice(s, index, v.as_mut_ptr()) }, 1, "voice {index}");
    unsafe { v.assume_init() }
}

fn frames(s: *const LmSong, index: u32) -> &'static [TonalFrame] {
    let v = voice(s, index);
    let p = unsafe { lm_frames(s, index) };
    assert!(!p.is_null());
    // The test never frees the songs whose frames it holds past the call.
    unsafe { std::slice::from_raw_parts(p, v.frame_count as usize) }
}

/// Bitwise equality that treats NaN as equal to NaN (the trace uses NaN for "not evaluated").
fn same(a: f64, b: f64) -> bool {
    a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan())
}

#[test]
fn the_traced_mix_is_bit_identical_to_the_plain_render() {
    let song = parse(SONG);
    let plain = render(&song).unwrap();
    for step in [1, 7, 48] {
        let (traced, _) = render_traced(&song, step).unwrap();
        assert_eq!(traced.gain.to_bits(), plain.gain.to_bits());
        assert!(plain.left.iter().zip(&traced.left).all(|(a, b)| a.to_bits() == b.to_bits()), "left, step {step}");
        assert!(plain.right.iter().zip(&traced.right).all(|(a, b)| a.to_bits() == b.to_bits()), "right, step {step}");
    }
}

#[test]
fn a_trace_step_of_zero_is_an_error() {
    let e = render_traced(&parse(SONG), 0).unwrap_err().to_string();
    assert!(e.contains("at least 1"), "{e}");
}

/// The gate, pitch and amp each frame records are the voice's own expressions evaluated at that
/// sample (with note time found from the gates, as the README defines it), and the level is the
/// documented envelope times amp times gain. Silent samples carry NaN pitch and amp, level 0.
#[test]
fn tonal_frames_equal_an_independent_evaluation_of_the_voice() {
    let song = parse(SONG);
    let rate = song.rate as f64;
    let n = song.sample_count();
    for step in [1usize, 5] {
        let (_, trace) = render_traced(&song, step).unwrap();
        for (vi, v) in song.voices.iter().enumerate() {
            let VoiceKind::Tonal(tv) = &v.kind else { continue };
            let VoiceTrace::Tonal(fr) = &trace.voices[vi] else { panic!("voice {vi} should be tonal") };
            assert_eq!(fr.len(), (n + step - 1) / step, "{}: one frame per step", v.name);
            let beat = |i: usize| i as f64 * song.bpm / (60.0 * rate);
            let gates: Vec<f64> = (0..n).map(|i| tv.gate.eval(&song.program, beat(i))).collect();
            let (attack, decay, release) = (
                1.0 / (tv.attack_ms * 0.001 * rate),
                (1.0 - tv.sustain) / (tv.decay_ms * 0.001 * rate),
                1.0 / (tv.release_ms * 0.001 * rate),
            );
            let (mut env, mut attacking, mut prev) = (0.0f64, false, 0.0f64);
            let (mut start, mut end) = (0usize, 0usize);
            let (mut sounding, mut silent) = (0, 0);
            for i in 0..n {
                let g = gates[i];
                let onset = g > 0.0 && prev <= 0.0;
                prev = g;
                if onset {
                    start = i;
                    end = (i..n).find(|&j| gates[j] <= 0.0).unwrap_or(n);
                }
                let mut quiet = false;
                if g > 0.0 {
                    attacking |= onset;
                    if attacking {
                        env = (env + attack).min(1.0);
                        attacking = env < 1.0;
                    } else {
                        env = (env - decay).max(tv.sustain);
                    }
                } else {
                    attacking = false;
                    env = (env - release).max(0.0);
                    quiet = env == 0.0;
                }
                if i % step != 0 {
                    continue;
                }
                let f = fr[i / step];
                assert!(same(f.gate, g), "{} gate at sample {i}", v.name);
                if quiet {
                    silent += 1;
                    assert!(f.pitch.is_nan() && f.amp.is_nan() && f.level == 0.0, "{} silent at {i}: {f:?}", v.name);
                    continue;
                }
                sounding += 1;
                let at = At { t: beat(i), age: (i - start) as f64 / rate, left: (end as f64 - i as f64) / rate };
                let pitch = tv.pitch.eval_at(&song.program, &at);
                let amp = tv.amp.as_ref().map_or(1.0, |a| a.eval_at(&song.program, &at));
                assert!(same(f.pitch, pitch), "{} pitch at {i}: {} vs {pitch}", v.name, f.pitch);
                assert!(same(f.amp, amp), "{} amp at {i}: {} vs {amp}", v.name, f.amp);
                assert!(same(f.level, env * amp * v.gain), "{} level at {i}: {} vs {}", v.name, f.level, env * amp * v.gain);
            }
            // Both cases must actually occur, or the test proves nothing about one of them.
            assert!(sounding > 0 && silent > 0, "{}: sounding {sounding}, silent {silent}", v.name);
        }
    }
}

/// A single voice with no effects: where the trace says silent the mix is exactly zero, and where
/// it says sounding the mix never exceeds level x the centre pan gain (an oscillator is within +-1).
#[test]
fn the_traced_level_bounds_what_the_voice_put_in_the_mix() {
    let src = "bpm 120\nbeats 4\nrate 8000\nvoice s {\n wave = sine\n pitch = 0\n gate = 0.5 - frac(t)\n amp = 0.3 + 0.2 frac(t / 3)\n release = 60\n gain = 0.8\n}\n";
    let song = parse(src);
    let (mix, trace) = render_traced(&song, 1).unwrap();
    let VoiceTrace::Tonal(fr) = &trace.voices[0] else { panic!() };
    let centre = std::f64::consts::FRAC_1_SQRT_2;
    let (mut zeros, mut peaks) = (0, 0.0f64);
    for (i, f) in fr.iter().enumerate() {
        if f.level == 0.0 {
            assert_eq!(mix.left[i], 0.0, "sample {i} should be silent");
            zeros += 1;
        } else {
            let bound = f.level * centre * mix.gain;
            assert!(mix.left[i].abs() <= bound * (1.0 + 1e-12), "sample {i}: {} > {bound}", mix.left[i]);
            peaks = peaks.max(mix.left[i].abs() / bound);
        }
    }
    assert!(zeros > 1000, "the gaps between notes should be silent ({zeros})");
    assert!(peaks > 0.99, "the bound should be reached, not loose ({peaks})");
}

/// Hits are the rising edges of the trigger, evaluated independently, with the amp expression's
/// value at that sample; the C view adds the time in seconds and beats.
#[test]
fn drum_hits_are_the_rising_edges_of_the_trigger() {
    let song = parse(SONG);
    let s = load(SONG, 0);
    let rate = song.rate as f64;
    for vi in [2u32, 3] {
        let v = &song.voices[vi as usize];
        let VoiceKind::Drum(d) = &v.kind else { panic!() };
        let beat = |i: usize| i as f64 * song.bpm / (60.0 * rate);
        let mut expect = Vec::new();
        let mut prev = 0.0;
        for i in 0..song.sample_count() {
            let g = d.trigger.eval(&song.program, beat(i));
            if g > 0.0 && prev <= 0.0 {
                expect.push((i as u64, d.amp.as_ref().map_or(1.0, |a| a.eval(&song.program, beat(i)))));
            }
            prev = g;
        }
        let lv = voice(s, vi);
        assert_eq!(lv.kind, LM_DRUM);
        assert_eq!(lv.hit_count as usize, expect.len(), "{}", v.name);
        let mut hits = vec![LmHit { sample: 0, seconds: 0.0, beat: 0.0, amp: 0.0 }; expect.len() + 3];
        let got = unsafe { lm_hits(s, vi, hits.as_mut_ptr(), hits.len() as u64) } as usize;
        assert_eq!(got, expect.len());
        for (h, (sample, amp)) in hits.iter().zip(&expect) {
            assert_eq!(h.sample, *sample);
            assert!(same(h.amp, *amp), "{} amp {} vs {amp}", v.name, h.amp);
            assert!(same(h.seconds, *sample as f64 / rate));
            assert!(same(h.beat, beat(*sample as usize)));
        }
        // A short buffer gets the first hits only.
        let got = unsafe { lm_hits(s, vi, hits.as_mut_ptr(), 2) };
        assert_eq!(got, 2);
    }
    // The kick's velocities fall per beat, so the trace really carries the amp, not a constant.
    let mut hits = vec![LmHit { sample: 0, seconds: 0.0, beat: 0.0, amp: 0.0 }; 8];
    unsafe { lm_hits(s, 2, hits.as_mut_ptr(), 8) };
    assert!(hits[0].amp > hits[7].amp + 0.5);
    let dash = voice(s, 3);
    assert_eq!((dash.drum_kind, dash.drum_from_hz, dash.drum_to_hz, dash.drum_dash_ms), (3, 900.0, 100.0, 10.0));
    assert_eq!(voice(s, 2).drum_kind, 0);
    unsafe { lm_free(s) };
}

#[test]
fn lm_sample_returns_the_frame_at_or_before_the_time() {
    let s = load(SONG, 10);
    let i = info(s);
    assert_eq!(i.trace_step, 10);
    let fr = frames(s, 0);
    let mut f = TonalFrame { gate: 9.0, pitch: 9.0, amp: 9.0, level: 9.0 };
    for k in [0usize, 1, 9, 10, 11, 12345, 31999] {
        let secs = k as f64 / i.rate as f64;
        assert_eq!(unsafe { lm_sample(s, 0, secs, &mut f) }, 1, "sample {k}");
        let want = fr[k / 10];
        assert!(same(f.gate, want.gate) && same(f.pitch, want.pitch) && same(f.level, want.level), "sample {k}");
    }
    // Halfway between two samples is still the earlier one.
    unsafe { lm_sample(s, 0, 10.5 / i.rate as f64, &mut f) };
    assert!(same(f.gate, fr[1].gate));
    // Outside the song, NaN, a drum, a bad index: 0, and the frame is left alone.
    let sentinel = TonalFrame { gate: 7.0, pitch: 7.0, amp: 7.0, level: 7.0 };
    for (index, secs) in [(0u32, -0.001), (0, i.duration), (0, 1e9), (0, f64::NAN), (2, 1.0), (99, 1.0)] {
        let mut g = sentinel;
        assert_eq!(unsafe { lm_sample(s, index, secs, &mut g) }, 0, "voice {index} at {secs}");
        assert_eq!(g, sentinel);
    }
    unsafe { lm_free(s) };
}

#[test]
fn the_audio_buffer_is_the_mix_interleaved_as_f32() {
    let mix = render(&parse(SONG)).unwrap();
    let s = load(SONG, 0);
    let i = info(s);
    assert_eq!(i.frames as usize, mix.frames());
    let a = unsafe { std::slice::from_raw_parts(lm_audio(s), 2 * i.frames as usize) };
    for k in 0..mix.frames() {
        assert_eq!(a[2 * k].to_bits(), (mix.left[k] as f32).to_bits(), "left {k}");
        assert_eq!(a[2 * k + 1].to_bits(), (mix.right[k] as f32).to_bits(), "right {k}");
    }
    assert!(same(i.mix_gain, mix.gain));
    unsafe { lm_free(s) };
}

#[test]
fn the_info_and_voices_describe_the_song() {
    let s = load(SONG, 0);
    let i = info(s);
    assert_eq!((i.bpm, i.beats, i.rate, i.voice_count, i.looped), (120.0, 8.0, 8000, 4, 0));
    assert_eq!(i.duration, 4.0);
    assert_eq!(i.trace_step, 8, "the default step is one millisecond");
    let names: Vec<String> = (0..4).map(|k| unsafe { CStr::from_ptr(voice(s, k).name) }.to_str().unwrap().to_owned()).collect();
    assert_eq!(names, ["lead", "pad", "k", "d"]);
    let pad = voice(s, 1);
    assert_eq!((pad.kind, pad.pan, pad.hit_count), (LM_TONAL, -0.3, 0));
    assert_eq!(pad.frame_count, 4000);
    unsafe { lm_free(s) };
}

#[test]
fn load_errors_come_back_as_messages() {
    let mut err = [0 as std::ffi::c_char; 256];
    let msg = |e: &[std::ffi::c_char]| unsafe { CStr::from_ptr(e.as_ptr()) }.to_str().unwrap().to_owned();
    // A syntax error, with its line.
    let bad = CString::new("bpm 120\nbeats 1\nvoice v {\n wave = sine\n pitch = 1 +\n gate = 1\n}\n").unwrap();
    assert!(unsafe { lm_load_source(bad.as_ptr(), 0, err.as_mut_ptr(), err.len()) }.is_null());
    assert!(msg(&err).starts_with("line 5"), "{}", msg(&err));
    // A render error (a NaN pitch).
    let nan = CString::new("bpm 120\nbeats 1\nvoice v {\n wave = sine\n pitch = sqrt(-1)\n gate = 1\n}\n").unwrap();
    assert!(unsafe { lm_load_source(nan.as_ptr(), 0, err.as_mut_ptr(), err.len()) }.is_null());
    assert!(msg(&err).contains("pitch is NaN"), "{}", msg(&err));
    // A missing file, and a null path.
    let path = CString::new("no/such/song.txt").unwrap();
    assert!(unsafe { lm_load_file(path.as_ptr(), 0, err.as_mut_ptr(), err.len()) }.is_null());
    assert!(msg(&err).contains("cannot read no/such/song.txt"), "{}", msg(&err));
    assert!(unsafe { lm_load_file(null(), 0, err.as_mut_ptr(), err.len()) }.is_null());
    assert!(msg(&err).contains("null"), "{}", msg(&err));
    // Not UTF-8.
    let latin = CString::new(vec![b'b', 0xE9, b'm']).unwrap();
    assert!(unsafe { lm_load_source(latin.as_ptr(), 0, err.as_mut_ptr(), err.len()) }.is_null());
    assert!(msg(&err).contains("UTF-8"), "{}", msg(&err));
    // A tiny buffer gets a truncated, terminated message; a null buffer is allowed.
    let mut tiny = [1 as std::ffi::c_char; 6];
    assert!(unsafe { lm_load_source(bad.as_ptr(), 0, tiny.as_mut_ptr(), tiny.len()) }.is_null());
    assert_eq!(msg(&tiny), "line ");
    assert!(unsafe { lm_load_source(bad.as_ptr(), 0, null_mut(), 0) }.is_null());
    let mut none = [1 as std::ffi::c_char; 1];
    assert!(unsafe { lm_load_source(bad.as_ptr(), 0, none.as_mut_ptr(), 1) }.is_null());
    assert_eq!(none[0], 0);
}

#[test]
fn null_songs_and_bad_indices_return_nothing() {
    let mut i = LmInfo::default();
    let mut v = std::mem::MaybeUninit::<LmVoice>::uninit();
    let mut f = TonalFrame { gate: 0.0, pitch: 0.0, amp: 0.0, level: 0.0 };
    let mut h = [LmHit { sample: 0, seconds: 0.0, beat: 0.0, amp: 0.0 }];
    unsafe {
        assert_eq!(lm_info(null(), &mut i), 0);
        assert!(lm_audio(null()).is_null());
        assert_eq!(lm_voice(null(), 0, v.as_mut_ptr()), 0);
        assert!(lm_frames(null(), 0).is_null());
        assert_eq!(lm_hits(null(), 0, h.as_mut_ptr(), 1), 0);
        assert_eq!(lm_sample(null(), 0, 0.0, &mut f), 0);
        lm_free(null_mut());
    }
    let s = load(SONG, 0);
    unsafe {
        assert_eq!(lm_info(s, null_mut()), 0);
        assert_eq!(lm_voice(s, 4, v.as_mut_ptr()), 0);
        assert_eq!(lm_voice(s, 0, null_mut()), 0);
        assert!(lm_frames(s, 2).is_null(), "a drum has no frames");
        assert!(lm_frames(s, 4).is_null());
        assert_eq!(lm_hits(s, 0, h.as_mut_ptr(), 1), 0, "a tonal voice has no hits");
        assert_eq!(lm_hits(s, 2, null_mut(), 5), 0);
        lm_free(s);
    }
}

/// The song the world opens first, through the file entry: its settings, its 22 voices in order,
/// and its lead's frames against its own gate.
#[test]
fn citypop_v4_loads_through_the_file_entry() {
    let path = CString::new(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/citypop_v4.txt")).unwrap();
    let mut err = [0 as std::ffi::c_char; 256];
    let s = unsafe { lm_load_file(path.as_ptr(), 0, err.as_mut_ptr(), err.len()) };
    assert!(!s.is_null(), "{}", unsafe { CStr::from_ptr(err.as_ptr()) }.to_string_lossy());
    let i = info(s);
    assert_eq!((i.bpm, i.beats, i.rate, i.looped, i.trace_step), (94.0, 64.0, 48000, 1, 48));
    let song = parse(&std::fs::read_to_string(path.to_str().unwrap()).unwrap());
    assert_eq!(i.voice_count as usize, song.voices.len());
    for (k, v) in song.voices.iter().enumerate() {
        assert_eq!(unsafe { CStr::from_ptr(voice(s, k as u32).name) }.to_str().unwrap(), v.name);
    }
    let lead = song.voices.iter().position(|v| v.name == "lead").unwrap();
    let VoiceKind::Tonal(tv) = &song.voices[lead].kind else { panic!() };
    let fr = frames(s, lead as u32);
    let mut sounding = 0;
    for (k, f) in fr.iter().enumerate().step_by(97) {
        let t = (k * 48) as f64 * 94.0 / (60.0 * 48000.0);
        assert!(same(f.gate, tv.gate.eval(&song.program, t)), "lead gate at frame {k}");
        if f.level > 0.0 {
            sounding += 1;
            assert!(f.pitch > -20.0 && f.pitch <= 24.0, "lead pitch {} at frame {k}", f.pitch);
        }
    }
    assert!(sounding > 50, "the lead plays ({sounding})");
    unsafe { lm_free(s) };
}
