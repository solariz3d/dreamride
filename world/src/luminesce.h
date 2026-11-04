// luminesce.h: the C ABI of the Luminesce engine (src/ffi.rs), as luminesce.dll.
//
// One call renders a song and records what the renderer used, so the world draws the song from the
// same numbers it plays. Every function accepts null pointers (returning 0 or null) and none of
// them throws. Pointers a function returns belong to the song and live until lm_free.
//
// Link with luminesce.dll.lib (target/release after `cargo build --release`).
#pragma once
#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct LmSong LmSong;

typedef struct LmInfo {
  double   bpm;
  double   beats;
  double   duration;      // seconds: beats * 60 / bpm
  double   reference_hz;  // the frequency of pitch 0
  double   mix_gain;      // the normalisation gain applied to the mix (1 when none)
  uint32_t rate;
  uint64_t frames;        // stereo frames in the audio buffer
  uint32_t voice_count;
  uint32_t trace_step;    // samples between tonal frames: frame k is sample k * trace_step
  uint32_t looped;        // 1 if the song is a loop
} LmInfo;

enum { LM_TONAL = 0, LM_DRUM = 1 };
enum { LM_KICK = 0, LM_SNARE = 1, LM_HAT = 2, LM_DASH = 3 };

typedef struct LmVoice {
  const char* name;       // nul-terminated UTF-8, owned by the song
  uint32_t kind;          // LM_TONAL or LM_DRUM
  uint32_t drum_kind;     // LM_KICK .. LM_DASH for drums, 0 for tonal voices
  double   gain;
  double   pan;
  uint64_t frame_count;   // tonal voices: frames in lm_frames
  uint64_t hit_count;     // drums: hits for lm_hits
  // Drums: one hit's pitch sweep f(tau) = to + (from - to) e^(-tau / dash). The built-in kick's own
  // sweep; the built-in snare and hat do not sweep (from == to).
  double   drum_from_hz;
  double   drum_to_hz;
  double   drum_dash_ms;
  double   drum_length_ms;
} LmVoice;

// A tonal voice's state at one sample, exactly as the renderer used it.
typedef struct LmFrame {
  double gate;   // the gate expression (sounds while > 0)
  double pitch;  // semitones above the reference; NaN where not evaluated (closed and released)
  double amp;    // the amp expression (1 without one); NaN where the pitch is NaN
  double level;  // envelope x amp x gain: what the oscillator was multiplied by (0 when silent)
} LmFrame;

typedef struct LmHit {
  uint64_t sample;
  double   seconds;
  double   beat;
  double   amp;    // the amp expression at the onset, before the voice's gain
} LmHit;

// Load and render. trace_step 0 means one frame per millisecond. On error returns null and writes a
// nul-terminated message (truncated to err_cap bytes) into err, which may be null.
LmSong* lm_load_file(const char* path_utf8, uint32_t trace_step, char* err, size_t err_cap);
LmSong* lm_load_source(const char* source_utf8, uint32_t trace_step, char* err, size_t err_cap);
void    lm_free(LmSong* song);

int32_t        lm_info(const LmSong* song, LmInfo* out);
const float*   lm_audio(const LmSong* song);  // interleaved stereo f32, 2 * frames values
int32_t        lm_voice(const LmSong* song, uint32_t index, LmVoice* out);
const LmFrame* lm_frames(const LmSong* song, uint32_t index);  // tonal voices only
uint64_t       lm_hits(const LmSong* song, uint32_t index, LmHit* out, uint64_t cap);  // drums only
// The frame at or before `seconds`; 0 (out untouched) outside [0, duration), for drums, bad index.
int32_t        lm_sample(const LmSong* song, uint32_t index, double seconds, LmFrame* out);

// ---- stream mode: an endless song, eight-bar equations each evolved from the last (src/stream.rs) ----
//
// lm_stream_start composes and renders stage 1 before it returns; a worker thread then keeps `lookahead` stages
// rendered ahead of the one being pulled. One thread may pull audio while another reads stages and windows.

typedef struct LmStream LmStream;

enum { LM_STREAM_GRAMMAR = 1,       // flags: a seeded stream, from the move book (since 2026-11-03)
       LM_STREAM_GRAMMAR_SONG = 2,    // with LM_STREAM_GRAMMAR: stage 1 from the grammar instead (stream v2)
       LM_STREAM_LENS = 4,            // with LM_STREAM_GRAMMAR: the lens species only (no species choice; 2026-11-03)
       LM_STREAM_EXPLORE = 8 };       // with LM_STREAM_GRAMMAR: a species of strange tunings (the Explore option)

typedef struct LmStreamInfo {
  double   bpm;
  double   reference_hz;
  double   stage_beats;      // 32: eight bars
  uint64_t seed;
  uint32_t rate;
  uint32_t trace_step;
  uint32_t roster_count;     // every voice the stream can have; every window has them all, in this order
  uint64_t stages_composed;
  uint64_t stages_rendered;
  uint64_t frames_read;      // frames handed out by lm_stream_pull so far
} LmStreamInfo;

typedef struct LmStageInfo {
  uint64_t index;            // from 0
  uint64_t start_frame;      // absolute frame of its first sample
  uint64_t frames;
  double   start_beat;
  double   beats;
  uint32_t rendered;         // 1 once its audio is rendered
  uint32_t turn;             // 1 for a bigger change
  double   score;            // the critic's score from its functions, 0..1
  // Strings owned by the stream until lm_stream_free:
  const char* equation;      // lines split by '\n': kind ('D' definition, 'T' time name), '1'/'0' changed, '\t', markup
  const char* voices;        // the voices present, space-separated, in roster order
  const char* changed;       // names changed since the stage before, space-separated
  const char* summary;       // what changed, in words
  const char* text;          // the stage as a song file
} LmStageInfo;

// seed_song: a song's text, or null for the default seed song. Null on error (message in err).
LmStream* lm_stream_start(const char* seed_song_utf8, uint64_t seed, uint32_t flags, uint32_t lookahead,
                          uint32_t trace_step, char* err, size_t err_cap);
void      lm_stream_free(LmStream* stream);
// 1 once the song has ended (a move-book stream composes its own ending), with its length in frames (ring-out
// included) in *frames; pull then returns fewer frames than asked at that frame, and 0 after it. 0 while it plays.
int32_t   lm_stream_ended(const LmStream* stream, uint64_t* frames);
int32_t   lm_stream_info(const LmStream* stream, LmStreamInfo* out);
const char* lm_stream_roster(const LmStream* stream, uint32_t index);
// Writes the next `frames` interleaved stereo frames; blocks while their stage renders. Returns frames written.
uint64_t  lm_stream_pull(const LmStream* stream, float* out, uint64_t frames);
uint64_t  lm_stream_stage_at(const LmStream* stream, uint64_t frame);
int32_t   lm_stream_stage(const LmStream* stream, uint64_t index, LmStageInfo* out);  // 0 until composed
// The voice record of stages first..last as a song handle (lm_info, lm_voice, lm_frames, lm_hits, lm_sample work on
// it; free with lm_free). Its time 0 is absolute frame *start_frame. Null on error (message in err).
LmSong*   lm_stream_window(const LmStream* stream, uint64_t first, uint64_t last, uint64_t* start_frame,
                           char* err, size_t err_cap);
// Saves stages last+1-count .. last as one song file. 1 on success, 0 with the message in err.
int32_t   lm_stream_save(const LmStream* stream, uint64_t last, uint32_t count, const char* path_utf8,
                         char* err, size_t err_cap);
int32_t   lm_stream_error(const LmStream* stream, char* buf, size_t cap);  // 1 (message in buf) if the stream failed

#ifdef __cplusplus
}
#endif
