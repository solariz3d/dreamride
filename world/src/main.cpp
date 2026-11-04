// Dreamride's world (Luminesce, W2): the original theme (the void), native D3D12 + DXR.
//
// The song is rendered by the engine (luminesce.dll), which also records what it used for every
// voice; the host plays the audio through XAudio2 and draws each voice from that record, so the
// picture and the sound are the same numbers. The playback position is the master clock.
//
// The wall: time runs along x, pitch goes up y. Each tonal voice is a glowing tube along its pitch
// curve while its gate is open; each drum hit is a downward dash, its own pitch sweep, at its true
// height where it fits the wall. The camera travels with the present, off to the side, looking back
// at the past, and breathes with the song's energy; nothing ahead of the present is drawn. A dark
// near-mirror floor reflects the tubes (DXR inline ray queries) and takes their light (an analytic
// line-light integral, recomputed each frame); kicks ring it. Note onsets flare, drum hits flash
// light into the fog, dust drifts in the graph's light. HDR, bloom, ACES, 8x MSAA. The look's
// numbers live in Theme (VoidTheme is the original theme).
//
// The window has a menu overlay drawn with Dear ImGui (src/imgui, MIT): a song library (every
// *.txt in examples/ and --songs folders, rendered off the main thread), transport with seeking,
// view and theme controls, and a keys card. Its window-free logic is in ui_logic.h, with tests.
//
// Stream mode (--stream, or the library's Stream entry): an endless song, eight-bar equations each evolved
// from the last by the engine (src/stream.rs), played as it is written. The world draws a window over the
// stage playing and the one before it, swapped at each seam, and typesets the playing stage's equation
// over the void (math_layout.h; E shows or hides it, S saves the last stages as a song).
//
// Headless harness (never opens a window): --capture <seconds> <out.png> (repeatable) renders
// offscreen and writes PNGs (--capture-menu draws the overlay too); --bench [frames] reports frame
// times (--bench-ui with the overlay). See "The world" in the repository README.

#define WIN32_LEAN_AND_MEAN
#define _CRT_SECURE_NO_WARNINGS
#define NOMINMAX
#include <windows.h>
#include <wrl/client.h>
#include <d3d12.h>
#include <dxgi1_6.h>
#include <dxcapi.h>
#include <xaudio2.h>
#include <timeapi.h>

#include <cstdio>
#include <cstdint>
#include <cstring>
#include <cmath>
#include <vector>
#include <string>
#include <algorithm>
#include <chrono>
#include <thread>
#include <atomic>
#include <memory>

#include "luminesce.h"
#include "ui_logic.h"
#include "folder_watch.h"
#include "math_layout.h"
#include "history.h"
#include "water.h"
#include "imgui/imgui.h"
#include "imgui/imgui_internal.h"
#include "imgui/imgui_impl_dx12.h"
#include "imgui/imgui_impl_win32.h"
extern IMGUI_IMPL_API LRESULT ImGui_ImplWin32_WndProcHandler(HWND, UINT, WPARAM, LPARAM);

using Microsoft::WRL::ComPtr;

// ================================================================ errors

[[noreturn]] static void Die(const char* what, HRESULT hr = E_FAIL) {
  fprintf(stderr, "FATAL: %s (hr=0x%08X)\n", what, (unsigned)hr);
  fflush(stderr);
  ExitProcess(1);
}
#define CHECK(x, what) do { HRESULT hr__ = (x); if (FAILED(hr__)) Die(what, hr__); } while (0)

// ================================================================ small maths

struct V3 { float x, y, z; };
static V3 operator+(V3 a, V3 b) { return { a.x + b.x, a.y + b.y, a.z + b.z }; }
static V3 operator-(V3 a, V3 b) { return { a.x - b.x, a.y - b.y, a.z - b.z }; }
static V3 operator*(V3 a, float s) { return { a.x * s, a.y * s, a.z * s }; }
static float Dot(V3 a, V3 b) { return a.x * b.x + a.y * b.y + a.z * b.z; }
static V3 Cross(V3 a, V3 b) { return { a.y * b.z - a.z * b.y, a.z * b.x - a.x * b.z, a.x * b.y - a.y * b.x }; }
static V3 Norm(V3 a) { float l = sqrtf(Dot(a, a)); return l > 0 ? a * (1.0f / l) : a; }

// Row-major 4x4, column vectors: clip = M * p. HLSL declares it row_major and uses mul(M, p).
struct M4 { float m[4][4]; };
static M4 Mul(const M4& a, const M4& b) {
  M4 r{};
  for (int i = 0; i < 4; ++i)
    for (int j = 0; j < 4; ++j)
      for (int k = 0; k < 4; ++k) r.m[i][j] += a.m[i][k] * b.m[k][j];
  return r;
}
static M4 LookAt(V3 eye, V3 target, V3 up) {
  V3 f = Norm(target - eye), s = Norm(Cross(f, up)), u = Cross(s, f);
  M4 r{};
  r.m[0][0] = s.x; r.m[0][1] = s.y; r.m[0][2] = s.z; r.m[0][3] = -Dot(s, eye);
  r.m[1][0] = u.x; r.m[1][1] = u.y; r.m[1][2] = u.z; r.m[1][3] = -Dot(u, eye);
  r.m[2][0] = -f.x; r.m[2][1] = -f.y; r.m[2][2] = -f.z; r.m[2][3] = Dot(f, eye);
  r.m[3][3] = 1;
  return r;
}
// Reverse-Z infinite perspective: depth = near / -z_view (1 at the near plane, 0 at infinity).
static M4 Perspective(float fovy, float aspect, float zn) {
  float f = 1.0f / tanf(fovy * 0.5f);
  M4 r{};
  r.m[0][0] = f / aspect; r.m[1][1] = f; r.m[2][3] = zn; r.m[3][2] = -1;
  return r;
}
static M4 Inverse(const M4& in) {
  // Gauss-Jordan on a copy; the matrices here are well conditioned.
  double a[4][8];
  for (int i = 0; i < 4; ++i)
    for (int j = 0; j < 8; ++j) a[i][j] = j < 4 ? in.m[i][j] : (j - 4 == i ? 1.0 : 0.0);
  for (int c = 0; c < 4; ++c) {
    int p = c;
    for (int r = c + 1; r < 4; ++r) if (fabs(a[r][c]) > fabs(a[p][c])) p = r;
    for (int j = 0; j < 8; ++j) std::swap(a[c][j], a[p][j]);
    double d = a[c][c];
    if (fabs(d) < 1e-20) Die("singular matrix");
    for (int j = 0; j < 8; ++j) a[c][j] /= d;
    for (int r = 0; r < 4; ++r) if (r != c) {
      double k = a[r][c];
      for (int j = 0; j < 8; ++j) a[r][j] -= k * a[c][j];
    }
  }
  M4 r{};
  for (int i = 0; i < 4; ++i) for (int j = 0; j < 4; ++j) r.m[i][j] = (float)a[i][j + 4];
  return r;
}

// ================================================================ settings (the geometry)

static const float kBeatLen     = 1.5f;   // metres of wall per beat
static const float kWallHeight  = 5.6f;   // the song's pitch range fills this (within limits)
static const float kFloorGap    = 0.35f;   // the lowest pitch sits this far above the floor
static const float kFovY        = 0.90f;  // radians

// ================================================================ the theme (the look)
//
// Everything a theme changes lives here: the palette by voice role, the void's colour, the floor's
// finish, how the picture answers the music, and how the camera breathes. kVoid is the original
// theme; a later theme is another Theme value, with no change to the renderer.

struct Role { const char* key; float r, g, b, intensity, radius, z; };

struct Theme {
  // Voice roles, matched in order against the voice's name (first match wins). Unmatched voices
  // get an even spread of hues.
  std::vector<Role> drums, tonal;
  // The void.
  float fog[3];
  float fogStart, fogBeats120;   // fog starts this far from the eye (m); trail length at 120 bpm (beats)
  float exposure, bloom, vignette;
  // The floor: a near-mirror. Roughness is the GGX alpha that sizes the gloss blur; F0 the
  // reflectance seen straight down (Schlick's Fresnel brings it to 1 at grazing); reflection rays
  // per pixel, at fixed sub-pixel offsets (stable: the same offsets every frame).
  float floorRough, floorF0, floorAlbedo, floorGrid, floorLight;
  uint32_t reflRays;
  // The music's response.
  float recencyBoost, recencyBeats;  // the newest stretch of every line glows brighter
  float flare, flareBeats;           // a note's onset: its whole new segment and its head flare
  float thickMin, thickRange;        // tube radius x (min + range x brightness): loudness is thickness
  float pitchHue;                    // hue turned by pitch class on the circle of fifths (radians)
  bool  dashTrueSize;                // a drum's dash at its true pitch height when it fits the wall
  float dashFlash, dashFlashSec;     // light a drum hit throws into the air and onto the floor
  float rippleAmp, rippleSpeed, rippleWidth, rippleDecay, rippleK;  // a kick's ring on the floor
  // The bass on the water (water.h): a note low on the wall shakes the floor into standing waves whose wavelength is
  // its pitch's (Faraday, the water's dispersion, times bassWaveScale). bassWaveAmp: the wave height (m) under a
  // voice at full loudness at the surface; bassWaveDecay: the waves' e-fold (s), in time and so, at their speed, in
  // distance; bassWaveReach: the coupling's e-fold with the voice's height above the water (m). Amp 0 turns it off.
  float bassWaveAmp, bassWaveScale, bassWaveDecay, bassWaveReach;
  float scatter;                     // in-scattering of the present and of hits in the fog
  float presentGlow;                 // light the sounding heads throw into the air
  // Dust in the air, lit by the graph's light: the past drifting through it.
  uint32_t dustCount; float dustBright, dustSizePx;
  // The camera: quiet and dense poses, mixed by the song's energy; a sway on the beat.
  V3 eyeQuiet, eyeDense; float targetYQuiet, targetYDense;
  float energyBeats;  // smoothing of the energy curve (two cascaded lags, beats each)
  float sway;         // metres, a two-beat pendulum
  // Live scales (the menu's Theme sliders; 1 is the theme as designed): the fog's density (the
  // trail's e-fold distance divides by it), the lines' glow (every voice's light), and how far the
  // camera breathes from its middle pose (0 holds it still).
  float fogDensity, glow, breathe;
};

static Theme VoidTheme() {
  Theme t;
  // A night-drive palette: violet bass, teal pads, amber keys, coral lead.
  t.drums = { { "kick",  1.00f, 0.24f, 0.12f, 7.5f, 0.034f, -0.55f },
              { "snare", 0.80f, 0.78f, 1.00f, 3.0f, 0.012f, -0.50f },
              { "hat",   0.70f, 0.95f, 1.00f, 3.5f, 0.009f, -0.45f } };
  t.tonal = { { "bass",    0.62f, 0.22f, 1.00f, 3.2f, 0.045f, -0.30f },
              { "padhigh", 0.55f, 0.85f, 1.00f, 2.6f, 0.016f, -0.05f },
              { "padup",   0.20f, 0.70f, 1.00f, 2.2f, 0.016f, -0.12f },
              { "padroot", 0.25f, 0.45f, 1.00f, 2.4f, 0.022f, -0.22f },
              { "pad",     0.10f, 0.95f, 0.85f, 2.4f, 0.022f, -0.18f },
              { "key",     1.00f, 0.72f, 0.28f, 3.5f, 0.018f, 0.08f },
              { "lead",    1.00f, 0.28f, 0.48f, 4.5f, 0.034f, 0.22f },
              { "harm",    1.00f, 0.50f, 0.70f, 3.5f, 0.026f, 0.16f } };
  t.fog[0] = 0.0035f; t.fog[1] = 0.0045f; t.fog[2] = 0.0085f;
  t.fogStart = 4.0f; t.fogBeats120 = 16.0f;
  t.exposure = 1.25f; t.bloom = 0.12f; t.vignette = 0.35f;
  t.floorRough = 0.0035f; t.floorF0 = 0.30f; t.floorAlbedo = 0.30f; t.floorGrid = 0.0f; t.floorLight = 3.0f;  // floorGrid 0 (was 0.45), 2026-11-03: the floor reads as just water, no grid
  t.reflRays = 4;
  t.recencyBoost = 2.2f; t.recencyBeats = 0.75f;
  t.flare = 2.0f; t.flareBeats = 0.30f;
  t.thickMin = 0.70f; t.thickRange = 0.55f;
  t.pitchHue = 0.30f;
  t.dashTrueSize = true; t.dashFlash = 3.0f; t.dashFlashSec = 0.16f;
  t.rippleAmp = 0.05f; t.rippleSpeed = 6.0f; t.rippleWidth = 0.9f; t.rippleDecay = 0.55f; t.rippleK = 5.0f;
  t.bassWaveAmp = 0.009f; t.bassWaveScale = 90.0f; t.bassWaveDecay = 1.2f; t.bassWaveReach = 1.0f;  // 2026-11-03
  t.scatter = 0.0012f; t.presentGlow = 0.04f;
  t.dustCount = 3000; t.dustBright = 1.8f; t.dustSizePx = 1.6f;
  t.eyeQuiet = { 2.6f, 1.05f, 7.3f }; t.eyeDense = { 3.9f, 1.95f, 10.0f };
  t.targetYQuiet = 0.55f; t.targetYDense = 0.66f;
  t.energyBeats = 8.0f; t.sway = 0.05f;  // energyBeats was 1.5 (2026-11-03): breathing, when on, follows the song slowly
  t.fogDensity = 1.0f; t.glow = 1.0f; t.breathe = 0.0f;  // breathe was 1 (2026-11-03): the camera holds a steady rail by default
  return t;
}
static Theme g_theme = VoidTheme();

// LM_THEME="name=value name=value ..." overrides the theme's numbers for a run (for tuning a look
// without rebuilding). An unknown name or a bad value is an error, not a silent default.
static bool ApplyThemeOverrides(const char* spec) {
  Theme& t = g_theme;
  struct F { const char* name; float* f; uint32_t* u; };
  const F fields[] = {
    { "fogStart", &t.fogStart }, { "fogBeats120", &t.fogBeats120 }, { "exposure", &t.exposure }, { "bloom", &t.bloom },
    { "vignette", &t.vignette }, { "floorRough", &t.floorRough }, { "floorF0", &t.floorF0 }, { "floorAlbedo", &t.floorAlbedo },
    { "floorGrid", &t.floorGrid }, { "floorLight", &t.floorLight }, { "reflRays", nullptr, &t.reflRays },
    { "recencyBoost", &t.recencyBoost }, { "recencyBeats", &t.recencyBeats }, { "flare", &t.flare }, { "flareBeats", &t.flareBeats },
    { "thickMin", &t.thickMin }, { "thickRange", &t.thickRange }, { "pitchHue", &t.pitchHue }, { "dashFlash", &t.dashFlash },
    { "dashFlashSec", &t.dashFlashSec }, { "rippleAmp", &t.rippleAmp }, { "rippleSpeed", &t.rippleSpeed },
    { "rippleWidth", &t.rippleWidth }, { "rippleDecay", &t.rippleDecay }, { "rippleK", &t.rippleK }, { "scatter", &t.scatter },
    { "bassWaveAmp", &t.bassWaveAmp }, { "bassWaveScale", &t.bassWaveScale }, { "bassWaveDecay", &t.bassWaveDecay },
    { "bassWaveReach", &t.bassWaveReach },
    { "presentGlow", &t.presentGlow }, { "dustCount", nullptr, &t.dustCount }, { "dustBright", &t.dustBright },
    { "dustSizePx", &t.dustSizePx }, { "energyBeats", &t.energyBeats }, { "sway", &t.sway },
    { "fogDensity", &t.fogDensity }, { "glow", &t.glow }, { "breathe", &t.breathe },
  };
  std::string s = spec;
  size_t p = 0;
  while (p < s.size()) {
    while (p < s.size() && (s[p] == ' ' || s[p] == ',' || s[p] == ';')) ++p;
    if (p >= s.size()) break;
    size_t e = s.find_first_of(" ,;", p);
    std::string item = s.substr(p, e == std::string::npos ? std::string::npos : e - p);
    p = e == std::string::npos ? s.size() : e;
    size_t eq = item.find('=');
    if (eq == std::string::npos) { fprintf(stderr, "LM_THEME: '%s' is not name=value\n", item.c_str()); return false; }
    std::string name = item.substr(0, eq), val = item.substr(eq + 1);
    char* end = nullptr;
    double x = strtod(val.c_str(), &end);
    if (val.empty() || *end || !std::isfinite(x)) { fprintf(stderr, "LM_THEME: bad value in '%s'\n", item.c_str()); return false; }
    bool found = false;
    for (const F& f : fields) {
      if (name != f.name) continue;
      found = true;
      if (f.f) *f.f = (float)x;
      else if (x >= 0 && x <= 1e6 && x == floor(x)) *f.u = (uint32_t)x;
      else { fprintf(stderr, "LM_THEME: %s takes a whole number\n", f.name); return false; }
    }
    if (!found) { fprintf(stderr, "LM_THEME: no theme value named '%s'\n", name.c_str()); return false; }
  }
  return true;
}
static const UINT  kFrames      = 2;      // frames in flight
static const DXGI_FORMAT kHdrFmt = DXGI_FORMAT_R16G16B16A16_FLOAT;
static const DXGI_FORMAT kLdrFmt = DXGI_FORMAT_R8G8B8A8_UNORM;
static const UINT  kIrrW = 512, kIrrH = 192;
static const UINT  kMaxSegs = 8192, kMaxHeads = 64;
static const int   kBloomLevels = 6;

// ================================================================ the song as geometry

// noteX: where the vertex's note began (song x, metres), for the onset flare; pitch in semitones
// for the pitch-class tint (kNoPitch for drums).
struct TubeVtx { float cx, cy, cz; float dx, dy, dz; float radius; uint32_t voice; float level; float noteX; float pitch; float pad; };
static_assert(sizeof(TubeVtx) == 48, "TubeVtx must match the HLSL TubeVtxS");
static const float kNoPitch = -1000.0f;
struct Seg { float ax, ay, az, p0; float bx, by, bz, p1; float r, g, b, p2; };
struct HeadGpu { float x, y, z, radius; float r, g, b, intensity; };

struct VoiceLook {
  std::string name;
  bool drum = false;
  float r = 1, g = 1, b = 1, intensity = 1;
  float radius = 0.025f;
  float z = 0;
  // Tonal: frames from the engine; brightness normalised by the voice's loudest level.
  const LmFrame* frames = nullptr;
  uint64_t frameCount = 0;
  double maxLevel = 0;
  // Drums.
  std::vector<LmHit> hits;
  double maxAmp = 0;
  LmVoice info{};
};

struct Song {
  LmSong* lm = nullptr;
  LmInfo info{};
  std::vector<VoiceLook> voices;
  double pMin = 0, pMax = 12;  // pitch range shown
  float semi = 0.1f;           // metres per semitone
  float songLen = 0;           // metres
  std::vector<TubeVtx> verts;
  std::vector<uint32_t> indices;
  std::vector<float> rtPos;    // xyz per vertex at the true radius (the BLAS)
  std::vector<float> energy;   // the song's energy in [0, 1], every kEnergyStep beats, smoothed
  // Stream mode (the song is a window over the stream's latest stages): its time 0 in stream seconds, and the wall's
  // pitch range and the energy's scale carried from window to window, each only ever growing, so neither jumps at a
  // seam.
  double streamStart = 0;
  bool streamRange = false;
  double fixMin = 1e9, fixMax = -1e9;
  float energyLo = 1e9f, energyHi = -1e9f;
  // A stream window's energy lags, unnormalised (EnergySeedAt reads them), and the state its own lags start from
  // (the window before's at this window's start; NaN: from its first sample).
  std::vector<double> energyLag1, energyLag2;
  double energySeed[2] = { NAN, NAN };
} g_song;
static bool g_streaming = false;  // the world plays a stream (StreamMode, below)
static const double kEnergyStep = 1.0 / 16.0;
// A stream's wall is framed once for the whole song (2026-11-03: at a seam where more voices come in, the
// camera was seen growing to fit). Its first window used to set the range and each later one widened it, so the wall
// shrank from 0.14 to about 0.065 m a semitone over a song, a step at every seam where a voice came in higher or
// lower. This frame holds every stream measured: book seeds 4821, 1, 2, 3, 77 and 9001, to their ends, reached
// -49..43 semitones with the 4-semitone margin, and G (seeds 1 and 5, 30 stages) up to 53 (world --camera-seam-test).
// A note beyond it still widens the frame (and says so) rather than drawing under the floor; an endless G stream can
// drift that far.
static const double kStreamPitchLo = -49.0, kStreamPitchHi = 54.0;
// A stream's energy scale (raw energy at the top of the camera's range; levels as they sound): seed 4821's first seven
// stages peaked at 0.17 (world --camera-seam-test prints the peak).
static const float kStreamEnergyFull = 0.2f;

static float PitchY(const Song& s, double p) { return kFloorGap + (float)((p - s.pMin) * s.semi); }
// A drum's dash, held within the wall: from the floor gap to a little above the highest voice.
static float DrumY(const Song& s, double p) { return PitchY(s, std::clamp(p, s.pMin, s.pMax + 3.0)); }
static const float kDashLen = 0.9f;  // the tallest a squeezed drum's dash is drawn, metres
static double HzToPitch(const Song& s, double hz) { return 12.0 * log2(hz / s.info.reference_hz); }

static void HsvToRgb(float h, float s, float v, float& r, float& g, float& b) {
  float i = floorf(h * 6), f = h * 6 - i, p = v * (1 - s), q = v * (1 - f * s), t = v * (1 - (1 - f) * s);
  switch (((int)i) % 6) { case 0: r = v; g = t; b = p; break; case 1: r = q; g = v; b = p; break;
    case 2: r = p; g = v; b = t; break; case 3: r = p; g = q; b = v; break;
    case 4: r = t; g = p; b = v; break; default: r = v; g = p; b = q; }
}

// Colour, thickness and depth by the voice's role, read from its name against the theme's roles;
// unknown names get an even spread of hues.
static void StyleVoice(VoiceLook& v, int index, int count, const Theme& th) {
  auto set = [&](float r, float g, float b, float k, float rad, float z) { v.r = r; v.g = g; v.b = b; v.intensity = k; v.radius = rad; v.z = z; };
  for (const Role& ro : v.drum ? th.drums : th.tonal) {
    if (v.name.find(ro.key) == std::string::npos) continue;
    set(ro.r, ro.g, ro.b, ro.intensity, ro.radius, ro.z);
    if (!v.drum) v.z += 0.012f * (index % 7);
    return;
  }
  if (v.drum) { float r, g, b; HsvToRgb(fmodf(index * 0.618f, 1.0f), 0.5f, 1, r, g, b); set(r, g, b, 4.0f, 0.012f, -0.5f); return; }
  { float r, g, b; HsvToRgb(fmodf(0.1f + index * 0.618f, 1.0f), 0.7f, 1, r, g, b); set(r, g, b, 3.0f, 0.022f, 0.3f * (index / (float)std::max(1, count - 1)) - 0.15f); }
  // Voices of the same role sit a hair apart in depth, so equal pitches never share a surface.
  v.z += 0.012f * (index % 7);
}

static double FrameBeat(const LmInfo& i, uint64_t k) {
  return (double)(k * i.trace_step) * i.bpm / (60.0 * i.rate);
}

static float Brightness(const VoiceLook& v, double level) {
  if (v.maxLevel <= 0) return 0;
  return (float)sqrt(std::max(level, 0.0) / v.maxLevel);
}

struct PathPt { float x, y, b; };

// Ramer-Douglas-Peucker on (x, y, brightness): keep the points that bend the curve or change its
// light by more than the tolerances.
static void Simplify(const std::vector<PathPt>& p, size_t i0, size_t i1, std::vector<char>& keep) {
  if (i1 <= i0 + 1) return;
  const float tolPos = 0.0025f, tolB = 0.03f;
  PathPt a = p[i0], b = p[i1];
  float dx = b.x - a.x, dy = b.y - a.y, len2 = dx * dx + dy * dy;
  float worst = 0; size_t wi = i0;
  for (size_t i = i0 + 1; i < i1; ++i) {
    float t = len2 > 0 ? std::clamp(((p[i].x - a.x) * dx + (p[i].y - a.y) * dy) / len2, 0.0f, 1.0f) : 0;
    float ex = a.x + dx * t - p[i].x, ey = a.y + dy * t - p[i].y;
    float eb = (a.b + (b.b - a.b) * t - p[i].b) * (tolPos / tolB);
    float e = sqrtf(ex * ex + ey * ey + eb * eb);
    if (e > worst) { worst = e; wi = i; }
  }
  // Long straight runs are split anyway, so a held note's light can follow its envelope.
  if (worst > tolPos || p[i1].x - p[i0].x > 2.0f * kBeatLen) {
    if (wi == i0) wi = (i0 + i1) / 2;
    keep[wi] = 1;
    Simplify(p, i0, wi, keep);
    Simplify(p, wi, i1, keep);
  }
}

// A tube along a polyline in the wall plane: rings of `sides` vertices, oriented by the tangent.
// Its radius follows its brightness (loudness is thickness); noteX is where its note began.
static void EmitTube(Song& s, const Theme& th, const std::vector<PathPt>& pts, const VoiceLook& v, uint32_t voice, int sides, float noteX) {
  if (pts.size() < 2) return;
  auto& V = s.verts; auto& I = s.indices; auto& P = s.rtPos;
  uint32_t base = (uint32_t)V.size();
  for (size_t i = 0; i < pts.size(); ++i) {
    const PathPt& a = pts[i > 0 ? i - 1 : i];
    const PathPt& b = pts[i + 1 < pts.size() ? i + 1 : i];
    V3 t = Norm(V3{ b.x - a.x, b.y - a.y, 0 });
    if (Dot(t, t) == 0) t = { 1, 0, 0 };
    V3 n = { -t.y, t.x, 0 }, w = { 0, 0, 1 };
    float r = v.radius * (th.thickMin + th.thickRange * std::clamp(pts[i].b, 0.0f, 1.0f));
    float pitch = v.drum ? kNoPitch : (float)((pts[i].y - kFloorGap) / s.semi + s.pMin);
    for (int s = 0; s < sides; ++s) {
      float th = 2.0f * 3.14159265f * s / sides;
      V3 d = n * cosf(th) + w * sinf(th);
      V.push_back({ pts[i].x, pts[i].y, v.z, d.x, d.y, d.z, r, voice, pts[i].b, noteX, pitch, 0 });
      P.push_back(pts[i].x + d.x * r); P.push_back(pts[i].y + d.y * r); P.push_back(v.z + d.z * r);
    }
  }
  for (size_t i = 0; i + 1 < pts.size(); ++i)
    for (int s = 0; s < sides; ++s) {
      uint32_t a = base + (uint32_t)(i * sides + s), b = base + (uint32_t)(i * sides + (s + 1) % sides);
      uint32_t c = a + sides, d = b + sides;
      I.insert(I.end(), { a, c, b, b, c, d });
    }
}

// The pitch sweep of a drum hit, tau seconds in, and how bright it is then.
static double DrumPitch(const Song& s, const VoiceLook& v, double tau) {
  const LmVoice& d = v.info;
  if (d.drum_from_hz == d.drum_to_hz || d.drum_dash_ms <= 0) {
    // No sweep: a short fixed fall of three semitones, so it still reads as a downward dash.
    double len = std::max(d.drum_length_ms, 1.0) * 0.001 * 0.25;
    return HzToPitch(s, d.drum_from_hz) - 3.0 * std::min(tau / len, 1.0);
  }
  double hz = d.drum_to_hz + (d.drum_from_hz - d.drum_to_hz) * exp(-tau / (d.drum_dash_ms * 0.001));
  return HzToPitch(s, hz);
}
static double DrumDuration(const VoiceLook& v) {
  const LmVoice& d = v.info;
  double len = std::max(d.drum_length_ms, 1.0) * 0.001;
  if (d.drum_from_hz == d.drum_to_hz || d.drum_dash_ms <= 0) return len * 0.25;
  // Until the sweep is within a semitone and a half of where it settles.
  double k = d.drum_dash_ms * 0.001;
  double gap = fabs(d.drum_from_hz - d.drum_to_hz), settle = d.drum_to_hz * 0.0905;
  return std::min(len, k * log(std::max(gap / settle, 1.01)));
}

// Where a drum's dash is drawn: falling from yTop to yEnd. At its true size when the theme asks and
// the sweep lies within the wall, as the reference draws a kick: a pitch line dashing down about
// three octaves. Otherwise (a hat at 7 kHz) squeezed to at most kDashLen, clamped into the wall.
struct DashSpan { float yTop, yEnd, squeeze; bool trueSize; };
static DashSpan DashOf(const Song& s, const Theme& th, const VoiceLook& v) {
  double pTop = DrumPitch(s, v, 0), pEnd = DrumPitch(s, v, DrumDuration(v));
  DashSpan d{};
  d.yEnd = DrumY(s, pEnd); d.yTop = DrumY(s, pTop);
  d.trueSize = th.dashTrueSize && pTop <= s.pMax + 3.0 && pEnd >= s.pMin - 1.0;
  d.squeeze = (!d.trueSize && d.yTop - d.yEnd > kDashLen) ? kDashLen / (d.yTop - d.yEnd) : 1.0f;
  return d;
}
static float DashTopDrawn(const DashSpan& d) { return d.yEnd + (d.yTop - d.yEnd) * d.squeeze; }

static void BuildGeometry(Song& s, const Theme& th) {
  // The pitch range shown: every sounding tonal frame and every drum's sweep.
  double lo = 1e9, hi = -1e9;
  for (auto& v : s.voices) {
    if (!v.drum) {
      for (uint64_t k = 0; k < v.frameCount; ++k) {
        const LmFrame& f = v.frames[k];
        if (f.gate > 0 && std::isfinite(f.pitch)) { lo = std::min(lo, f.pitch); hi = std::max(hi, f.pitch); }
        v.maxLevel = std::max(v.maxLevel, f.level);
      }
    } else {
      // Drums do not set the range: a hat's sweep at 7 kHz would stretch the wall and squeeze
      // every tonal voice. Their dashes are clamped into it instead (DrumY).
      for (auto& h : v.hits) v.maxAmp = std::max(v.maxAmp, h.amp);
    }
  }
  if (lo > hi) { lo = -12; hi = 12; }
  if (s.streamRange) {
    // A stream's wall: the stream's one frame (kStreamPitchLo..Hi), set by its first window and carried to every
    // later one; widened only by a note beyond it, with room to spare.
    if (s.fixMin > s.fixMax) { s.fixMin = kStreamPitchLo; s.fixMax = kStreamPitchHi; }
    if (lo - 4 < s.fixMin || hi + 4 > s.fixMax)
      printf("  the stream's notes (%.1f..%.1f) reach past its frame (%.1f..%.1f): the frame widens\n", lo, hi, s.fixMin, s.fixMax);
    s.pMin = s.fixMin = std::min(s.fixMin, lo - 4);
    s.pMax = s.fixMax = std::max(s.fixMax, hi + 4);
  } else {
    s.pMin = lo - 1; s.pMax = hi + 1;
  }
  s.semi = std::clamp(kWallHeight / (float)(s.pMax - s.pMin), 0.05f, 0.14f);
  s.songLen = (float)(s.info.beats * kBeatLen);

  for (uint32_t vi = 0; vi < s.voices.size(); ++vi) {
    VoiceLook& v = s.voices[vi];
    if (!v.drum) {
      std::vector<PathPt> run;
      auto flush = [&]() {
        if (run.size() >= 2) {
          std::vector<char> keep(run.size(), 0);
          keep.front() = keep.back() = 1;
          Simplify(run, 0, run.size() - 1, keep);
          std::vector<PathPt> kept;
          for (size_t i = 0; i < run.size(); ++i) if (keep[i]) kept.push_back(run[i]);
          EmitTube(s, th, kept, v, vi, 8, run.front().x);
        }
        run.clear();
      };
      for (uint64_t k = 0; k < v.frameCount; ++k) {
        const LmFrame& f = v.frames[k];
        // Gate off: no tube. (A release still sounds, and lights the head, but draws no line.)
        if (f.gate > 0 && std::isfinite(f.pitch)) {
          // A jump of more than three quarters of a semitone within one frame is a new note, not a
          // glide: the tube breaks there, so chord changes do not draw vertical connectors. Glides
          // and scoops, which take tens of frames, stay continuous.
          if (k > 0 && !run.empty() && std::isfinite(v.frames[k - 1].pitch) && fabs(f.pitch - v.frames[k - 1].pitch) > 0.75) flush();
          run.push_back({ (float)(FrameBeat(s.info, k) * kBeatLen), PitchY(s, f.pitch), Brightness(v, f.level) });
        } else {
          flush();
        }
      }
      flush();
    } else {
      double dur = DrumDuration(v);
      // A drum's sweep can cover three octaves in a tenth of a second: drawn at its true height
      // (DashOf) it is a bright streak falling through the wall, the way the reference draws it.
      DashSpan span = DashOf(s, th, v);
      float yEnd = span.yEnd, squeeze = span.squeeze;
      printf("  %s: dash %.1f -> %.1f semitones, %s (%.2f m)\n", v.name.c_str(), DrumPitch(s, v, 0), DrumPitch(s, v, dur),
             span.trueSize ? "true size" : "squeezed", DashTopDrawn(span) - yEnd);
      for (auto& h : v.hits) {
        std::vector<PathPt> pts;
        const int n = 18;
        for (int i = 0; i < n; ++i) {
          double u = (double)i / (n - 1), tau = dur * u * u;  // denser where the sweep is fastest
          double beat = h.beat + tau * s.info.bpm / 60.0;
          float fade = (float)(1.0 - 0.85 * u);
          float b = (float)(v.maxAmp > 0 ? h.amp / v.maxAmp : 1.0) * fade;
          pts.push_back({ (float)(beat * kBeatLen), yEnd + (DrumY(s, DrumPitch(s, v, tau)) - yEnd) * squeeze, b });
        }
        EmitTube(s, th, pts, v, vi, 6, (float)(h.beat * kBeatLen));
      }
    }
  }
  printf("geometry: %zu vertices, %zu triangles, pitch %.1f..%.1f semitones at %.3f m each\n",
         s.verts.size(), s.indices.size() / 3, s.pMin, s.pMax, s.semi);
}

// The song's energy, for the camera: how much is sounding (each tonal voice's brightness, weighted
// by its role's intensity) plus the density of drum hits, every kEnergyStep beats; then two cascaded
// lags of th.energyBeats each (so the camera's velocity, not only its position, is continuous),
// run round a looped song until it settles; then normalised to [0, 1]. A function of song time
// alone, so every frame and every capture at the same moment agree.
// A stream window measures levels as they sound (not against each voice's loudest in the window, which differs from
// window to window), and starts its lags from the window before's state at its first moment (s.energySeed), so two
// windows agree on the stage they share and the camera does not step where one replaces the other.
static void BuildEnergy(Song& s, const Theme& th) {
  const LmInfo& i = s.info;
  const bool stream = s.streamRange;
  size_t n = (size_t)std::max(1.0, ceil(i.beats / kEnergyStep));
  std::vector<double> raw(n, 0.0);
  double wsum = 0;
  for (auto& v : s.voices) if (!v.drum) wsum += v.intensity;
  const size_t spread = (size_t)std::max(1.0, 1.0 / kEnergyStep);  // a hit counts for one beat
  for (auto& v : s.voices) {
    if (!v.drum) {
      for (size_t k = 0; k < n; ++k) {
        uint64_t f = (uint64_t)(k * kEnergyStep * 60.0 / i.bpm * i.rate) / std::max(i.trace_step, 1u);
        if (f >= v.frameCount) continue;
        const LmFrame& fr = v.frames[f];
        if (fr.gate > 0 && std::isfinite(fr.level))
          raw[k] += v.intensity * (stream ? (float)sqrt(std::max(fr.level, 0.0)) : Brightness(v, fr.level)) / std::max(wsum, 1e-9);
      }
    } else {
      for (auto& h : v.hits) {
        double w = 0.08 * (stream ? std::max(h.amp, 0.0) : v.maxAmp > 0 ? h.amp / v.maxAmp : 1.0) / spread;
        for (size_t j = 0; j < spread; ++j) {
          size_t k = (size_t)(h.beat / kEnergyStep) + j;
          if (i.looped) k %= n;
          if (k < n) raw[k] += w;
        }
      }
    }
  }
  double a = 1.0 - exp(-kEnergyStep / std::max<double>(th.energyBeats, 1e-3));
  double l1 = raw[0], l2 = raw[0];
  if (stream && std::isfinite(s.energySeed[0]) && std::isfinite(s.energySeed[1])) { l1 = s.energySeed[0]; l2 = s.energySeed[1]; }
  s.energy.assign(n, 0.0f);
  if (stream) s.energyLag1.assign(n, 0.0);
  for (int pass = 0; pass < (i.looped ? 4 : 1); ++pass)
    for (size_t k = 0; k < n; ++k) {
      l1 += (raw[k] - l1) * a; l2 += (l1 - l2) * a; s.energy[k] = (float)l2;
      if (stream) s.energyLag1[k] = l1;
    }
  if (stream) s.energyLag2.assign(s.energy.begin(), s.energy.end());
  auto [lo, hi] = std::minmax_element(s.energy.begin(), s.energy.end());
  float e0 = *lo, e1 = *hi;
  if (s.streamRange) {
    // A stream keeps one fixed scale for the whole song, so the energy never rescales at a seam (it used to widen
    // with each window, and the camera stepped up to 2.5 m in one frame where a louder stage came in).
    e0 = s.energyLo = 0.0f;
    e1 = s.energyHi = kStreamEnergyFull;
  }
  for (float& e : s.energy) e = e1 - e0 > 1e-6f ? std::clamp((e - e0) / (e1 - e0), 0.0f, 1.0f) : 0.5f;
}

// A stream window's energy lags just before stream time `secs` (the next window's first moment), for that window's
// energySeed. False when `secs` is not inside `s`.
static bool EnergySeedAt(const Song& s, double secs, double out[2]) {
  if (!s.streamRange || s.energyLag1.empty() || !(s.info.bpm > 0)) return false;
  long long j = llround((secs - s.streamStart) * s.info.bpm / 60.0 / kEnergyStep) - 1;
  if (j < 0 || j >= (long long)s.energyLag1.size()) return false;
  out[0] = s.energyLag1[(size_t)j]; out[1] = s.energyLag2[(size_t)j];
  return true;
}

// The energy at a song beat, interpolated (wrapping on a looped song).
static float EnergyAt(double beat) {
  const auto& E = g_song.energy;
  if (E.empty()) return 0.5f;
  double k = beat / kEnergyStep;
  if (g_song.info.looped) { double n = (double)E.size(); k = fmod(fmod(k, n) + n, n); }
  else k = std::clamp(k, 0.0, (double)E.size() - 1);
  size_t k0 = (size_t)k, k1 = (k0 + 1) % E.size();
  float f = (float)(k - k0);
  return E[k0] + (E[k1] - E[k0]) * f;
}

static bool VoicesAndGeometry(Song& s, const Theme& th, std::string& error);

// Render a song and build what the world draws from it, into `s` (a fresh Song). Touches no
// global, so the window runs it on a loader thread while the current song keeps playing; `th` is
// the theme it is built with (a copy, so the menu's sliders can move meanwhile). On failure `s`
// owns nothing, `error` says why, and it is printed too.
// source: the song's text instead of a file (path then only names it in messages): the history's replays.
static bool LoadSongInto(Song& s, const char* path, const Theme& th, std::string& error, const std::string* source = nullptr) {
  char err[1024] = {};
  auto t0 = std::chrono::steady_clock::now();
  s.lm = source ? lm_load_source(source->c_str(), 0, err, sizeof(err)) : lm_load_file(path, 0, err, sizeof(err));
  if (!s.lm) { fprintf(stderr, "cannot load %s: %s\n", path, err); error = err; return false; }
  lm_info(s.lm, &s.info);
  const LmInfo& i = s.info;
  auto fail = [&](const std::string& why) {
    fprintf(stderr, "%s: %s\n", path, why.c_str());
    error = why; lm_free(s.lm); s.lm = nullptr; s.voices.clear();
    return false;
  };
  if (!(i.bpm > 0) || !(i.duration > 0) || !(i.beats > 0) || i.rate == 0 || i.frames == 0)
    return fail("the song has no length or no tempo");
  double secs = std::chrono::duration<double>(std::chrono::steady_clock::now() - t0).count();
  printf("song: %s\n  %.1f bpm, %.0f beats, %.2f s, %u voices, rendered in %.2f s%s\n", path, i.bpm, i.beats,
         i.duration, i.voice_count, secs, i.looped ? ", loops" : "");
  if (!VoicesAndGeometry(s, th, error)) return fail(error);
  return true;
}

// The voices of a loaded song (s.lm), styled, and its geometry and energy. False (with `error`) if the world cannot
// draw it.
static bool VoicesAndGeometry(Song& s, const Theme& th, std::string& error) {
  const LmInfo& i = s.info;
  if (i.voice_count > 32) { error = "the world draws at most 32 voices (this song has " + std::to_string(i.voice_count) + ")"; return false; }
  s.voices.clear();
  for (uint32_t k = 0; k < i.voice_count; ++k) {
    VoiceLook v;
    if (!lm_voice(s.lm, k, &v.info)) Die("lm_voice");
    v.name = v.info.name;
    v.drum = v.info.kind == LM_DRUM;
    if (v.drum) {
      v.hits.resize(v.info.hit_count);
      if (v.info.hit_count) lm_hits(s.lm, k, v.hits.data(), v.info.hit_count);
    } else {
      v.frames = lm_frames(s.lm, k);
      v.frameCount = v.info.frame_count;
    }
    StyleVoice(v, (int)k, (int)i.voice_count, th);
    s.voices.push_back(std::move(v));
  }
  BuildGeometry(s, th);
  BuildEnergy(s, th);
  return true;
}

// The void with no song in it: what the window shows before the first song has rendered. A tempo
// and a length, so the clock, the fog and the camera have numbers to work with; no voices.
static void MakeEmptySong(Song& s, const Theme& th) {
  s = Song{};
  s.info.bpm = 90; s.info.beats = 16; s.info.duration = s.info.beats * 60.0 / s.info.bpm;
  s.info.reference_hz = 440; s.info.rate = 48000; s.info.trace_step = 48; s.info.looped = 1;
  s.info.frames = (uint64_t)(s.info.duration * s.info.rate);
  BuildGeometry(s, th);
  BuildEnergy(s, th);
}

// ================================================================ D3D12 core

static ComPtr<IDXGIFactory6> g_factory;
static ComPtr<ID3D12Device5> g_device;
static ComPtr<ID3D12CommandQueue> g_queue;
static ComPtr<ID3D12CommandAllocator> g_alloc[kFrames];
static ComPtr<ID3D12GraphicsCommandList4> g_cmd;
static ComPtr<ID3D12Fence> g_fence;
static HANDLE g_fenceEvent = nullptr;
static UINT64 g_fenceValue[kFrames] = {};
static UINT64 g_fenceNext = 1;
static UINT g_msaa = 8;

static void CreateDevice() {
#ifdef LM_DEBUG_LAYER
  ComPtr<ID3D12Debug> dbg;
  if (SUCCEEDED(D3D12GetDebugInterface(IID_PPV_ARGS(&dbg)))) dbg->EnableDebugLayer();
#endif
  CHECK(CreateDXGIFactory2(0, IID_PPV_ARGS(&g_factory)), "CreateDXGIFactory2");
  ComPtr<IDXGIAdapter1> chosen;
  for (UINT i = 0;; ++i) {
    ComPtr<IDXGIAdapter1> a;
    if (g_factory->EnumAdapterByGpuPreference(i, DXGI_GPU_PREFERENCE_HIGH_PERFORMANCE, IID_PPV_ARGS(&a)) == DXGI_ERROR_NOT_FOUND) break;
    DXGI_ADAPTER_DESC1 d{}; a->GetDesc1(&d);
    if (d.Flags & DXGI_ADAPTER_FLAG_SOFTWARE) continue;
    if (SUCCEEDED(D3D12CreateDevice(a.Get(), D3D_FEATURE_LEVEL_12_1, __uuidof(ID3D12Device), nullptr))) {
      chosen = a;
      char name[256] = {};
      WideCharToMultiByte(CP_UTF8, 0, d.Description, -1, name, sizeof(name), nullptr, nullptr);
      printf("adapter: %s\n", name);
      break;
    }
  }
  if (!chosen) Die("no D3D12 adapter with feature level 12.1");
  CHECK(D3D12CreateDevice(chosen.Get(), D3D_FEATURE_LEVEL_12_1, IID_PPV_ARGS(&g_device)), "D3D12CreateDevice");
  D3D12_FEATURE_DATA_D3D12_OPTIONS5 o5{};
  g_device->CheckFeatureSupport(D3D12_FEATURE_D3D12_OPTIONS5, &o5, sizeof(o5));
  if (o5.RaytracingTier < D3D12_RAYTRACING_TIER_1_1) Die("this GPU lacks DXR tier 1.1 (inline ray queries)");
  D3D12_FEATURE_DATA_MULTISAMPLE_QUALITY_LEVELS ms{ kHdrFmt, 8 };
  if (FAILED(g_device->CheckFeatureSupport(D3D12_FEATURE_MULTISAMPLE_QUALITY_LEVELS, &ms, sizeof(ms))) || ms.NumQualityLevels == 0) g_msaa = 4;

  D3D12_COMMAND_QUEUE_DESC qd{}; qd.Type = D3D12_COMMAND_LIST_TYPE_DIRECT;
  CHECK(g_device->CreateCommandQueue(&qd, IID_PPV_ARGS(&g_queue)), "CreateCommandQueue");
  for (UINT i = 0; i < kFrames; ++i)
    CHECK(g_device->CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT, IID_PPV_ARGS(&g_alloc[i])), "CreateCommandAllocator");
  CHECK(g_device->CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, g_alloc[0].Get(), nullptr, IID_PPV_ARGS(&g_cmd)), "CreateCommandList");
  CHECK(g_cmd->Close(), "Close");
  CHECK(g_device->CreateFence(0, D3D12_FENCE_FLAG_NONE, IID_PPV_ARGS(&g_fence)), "CreateFence");
  g_fenceEvent = CreateEventA(nullptr, FALSE, FALSE, nullptr);
}

static void WaitFence(UINT64 v) {
  if (g_fence->GetCompletedValue() >= v) return;
  CHECK(g_fence->SetEventOnCompletion(v, g_fenceEvent), "SetEventOnCompletion");
  WaitForSingleObject(g_fenceEvent, INFINITE);
}
static UINT64 Signal() {
  UINT64 v = g_fenceNext++;
  CHECK(g_queue->Signal(g_fence.Get(), v), "Signal");
  return v;
}
static void WaitIdle() { WaitFence(Signal()); }

static ComPtr<ID3D12Resource> MakeBuffer(UINT64 size, D3D12_HEAP_TYPE heap, D3D12_RESOURCE_STATES state,
                                         D3D12_RESOURCE_FLAGS flags = D3D12_RESOURCE_FLAG_NONE) {
  D3D12_HEAP_PROPERTIES hp{}; hp.Type = heap;
  D3D12_RESOURCE_DESC rd{};
  rd.Dimension = D3D12_RESOURCE_DIMENSION_BUFFER; rd.Width = std::max<UINT64>(size, 256); rd.Height = 1;
  rd.DepthOrArraySize = 1; rd.MipLevels = 1; rd.SampleDesc = { 1, 0 }; rd.Layout = D3D12_TEXTURE_LAYOUT_ROW_MAJOR; rd.Flags = flags;
  ComPtr<ID3D12Resource> r;
  CHECK(g_device->CreateCommittedResource(&hp, D3D12_HEAP_FLAG_NONE, &rd, state, nullptr, IID_PPV_ARGS(&r)), "CreateCommittedResource buffer");
  return r;
}

struct Tex {
  ComPtr<ID3D12Resource> r;
  D3D12_RESOURCE_STATES st = D3D12_RESOURCE_STATE_COMMON;
  UINT w = 0, h = 0;
};

static Tex MakeTex(UINT w, UINT h, DXGI_FORMAT fmt, D3D12_RESOURCE_FLAGS flags, D3D12_RESOURCE_STATES st,
                   UINT samples = 1, const D3D12_CLEAR_VALUE* clear = nullptr) {
  D3D12_HEAP_PROPERTIES hp{}; hp.Type = D3D12_HEAP_TYPE_DEFAULT;
  D3D12_RESOURCE_DESC rd{};
  rd.Dimension = D3D12_RESOURCE_DIMENSION_TEXTURE2D; rd.Width = w; rd.Height = h; rd.DepthOrArraySize = 1;
  rd.MipLevels = 1; rd.Format = fmt; rd.SampleDesc = { samples, 0 }; rd.Flags = flags;
  Tex t; t.w = w; t.h = h; t.st = st;
  CHECK(g_device->CreateCommittedResource(&hp, D3D12_HEAP_FLAG_NONE, &rd, st, clear, IID_PPV_ARGS(&t.r)), "CreateCommittedResource texture");
  return t;
}

static std::vector<D3D12_RESOURCE_BARRIER> g_barriers;
static void To(Tex& t, D3D12_RESOURCE_STATES s) {
  if (t.st == s) return;
  D3D12_RESOURCE_BARRIER b{};
  b.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION;
  b.Transition.pResource = t.r.Get(); b.Transition.StateBefore = t.st; b.Transition.StateAfter = s;
  b.Transition.Subresource = D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES;
  g_barriers.push_back(b);
  t.st = s;
}
static void UavBarrier(ID3D12Resource* r) {
  D3D12_RESOURCE_BARRIER b{}; b.Type = D3D12_RESOURCE_BARRIER_TYPE_UAV; b.UAV.pResource = r;
  g_barriers.push_back(b);
}
static void Flush() {
  if (!g_barriers.empty()) { g_cmd->ResourceBarrier((UINT)g_barriers.size(), g_barriers.data()); g_barriers.clear(); }
}

// Upload once into a default-heap buffer (blocking; used at load).
static ComPtr<ID3D12Resource> UploadStatic(const void* data, UINT64 size, D3D12_RESOURCE_STATES finalState) {
  auto dst = MakeBuffer(size, D3D12_HEAP_TYPE_DEFAULT, D3D12_RESOURCE_STATE_COPY_DEST);
  auto up = MakeBuffer(size, D3D12_HEAP_TYPE_UPLOAD, D3D12_RESOURCE_STATE_GENERIC_READ);
  void* p = nullptr; CHECK(up->Map(0, nullptr, &p), "Map upload"); memcpy(p, data, size); up->Unmap(0, nullptr);
  CHECK(g_alloc[0]->Reset(), "alloc reset"); CHECK(g_cmd->Reset(g_alloc[0].Get(), nullptr), "cmd reset");
  g_cmd->CopyBufferRegion(dst.Get(), 0, up.Get(), 0, size);
  D3D12_RESOURCE_BARRIER b{}; b.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION; b.Transition.pResource = dst.Get();
  b.Transition.StateBefore = D3D12_RESOURCE_STATE_COPY_DEST; b.Transition.StateAfter = finalState;
  b.Transition.Subresource = D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES;
  g_cmd->ResourceBarrier(1, &b);
  CHECK(g_cmd->Close(), "close"); ID3D12CommandList* l[] = { g_cmd.Get() }; g_queue->ExecuteCommandLists(1, l);
  WaitIdle();
  return dst;
}

// ================================================================ descriptors

static ComPtr<ID3D12DescriptorHeap> g_heap, g_rtvHeap, g_dsvHeap;
static UINT g_heapStride = 0, g_rtvStride = 0;
static UINT g_heapUsed = 0;
static const UINT kHeapSize = 256;

// A binding set: 4 SRVs (t4..t7) and 4 UAVs (u0..u3), contiguous, for one pass.
struct BindSet { UINT srv = 0, uav = 0; };

static D3D12_CPU_DESCRIPTOR_HANDLE Cpu(UINT i) { auto h = g_heap->GetCPUDescriptorHandleForHeapStart(); h.ptr += (SIZE_T)i * g_heapStride; return h; }
static D3D12_GPU_DESCRIPTOR_HANDLE Gpu(UINT i) { auto h = g_heap->GetGPUDescriptorHandleForHeapStart(); h.ptr += (UINT64)i * g_heapStride; return h; }

static BindSet MakeSet(std::initializer_list<const Tex*> srvs, std::initializer_list<const Tex*> uavs) {
  if (g_heapUsed + 8 > kHeapSize) Die("descriptor heap full");
  BindSet b{ g_heapUsed, g_heapUsed + 4 };
  g_heapUsed += 8;
  UINT k = 0;
  for (; k < 4; ++k) {
    const Tex* t = k < srvs.size() ? srvs.begin()[k] : nullptr;
    D3D12_SHADER_RESOURCE_VIEW_DESC d{};
    d.Shader4ComponentMapping = D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING;
    d.Format = kHdrFmt;
    if (t && t->r->GetDesc().SampleDesc.Count > 1) d.ViewDimension = D3D12_SRV_DIMENSION_TEXTURE2DMS;
    else { d.ViewDimension = D3D12_SRV_DIMENSION_TEXTURE2D; d.Texture2D.MipLevels = 1; }
    g_device->CreateShaderResourceView(t ? t->r.Get() : nullptr, &d, Cpu(b.srv + k));
  }
  for (k = 0; k < 4; ++k) {
    const Tex* t = k < uavs.size() ? uavs.begin()[k] : nullptr;
    D3D12_UNORDERED_ACCESS_VIEW_DESC d{}; d.Format = kHdrFmt; d.ViewDimension = D3D12_UAV_DIMENSION_TEXTURE2D;
    g_device->CreateUnorderedAccessView(t ? t->r.Get() : nullptr, nullptr, &d, Cpu(b.uav + k));
  }
  return b;
}

// ================================================================ shaders and pipelines

static DxcCreateInstanceProc g_dxcCreate = nullptr;
static std::string g_shaderSrc;
static std::wstring g_exeDir;

static std::wstring ExeDir() {
  wchar_t p[MAX_PATH]; GetModuleFileNameW(nullptr, p, MAX_PATH);
  std::wstring s = p; return s.substr(0, s.find_last_of(L"\\/") + 1);
}

static void LoadShaderSource() {
  HMODULE m = LoadLibraryW((g_exeDir + L"dxcompiler.dll").c_str());
  if (!m) m = LoadLibraryW(L"dxcompiler.dll");
  if (!m) Die("cannot load dxcompiler.dll (build.bat copies it next to the exe)");
  g_dxcCreate = (DxcCreateInstanceProc)GetProcAddress(m, "DxcCreateInstance");
  if (!g_dxcCreate) Die("DxcCreateInstance not found");
  std::wstring path = g_exeDir + L"world.hlsl";
  FILE* f = _wfopen(path.c_str(), L"rb");
  if (!f) Die("cannot open world.hlsl next to the exe");
  fseek(f, 0, SEEK_END); long n = ftell(f); fseek(f, 0, SEEK_SET);
  g_shaderSrc.resize(n);
  if (fread(g_shaderSrc.data(), 1, n, f) != (size_t)n) Die("cannot read world.hlsl");
  fclose(f);
}

static ComPtr<IDxcBlob> Compile(const wchar_t* entry, const wchar_t* target) {
  ComPtr<IDxcCompiler3> comp;
  CHECK(g_dxcCreate(CLSID_DxcCompiler, IID_PPV_ARGS(&comp)), "DxcCompiler");
  DxcBuffer src{ g_shaderSrc.data(), g_shaderSrc.size(), DXC_CP_UTF8 };
  LPCWSTR args[] = { L"-E", entry, L"-T", target, L"-HV", L"2021", L"-O3", L"-Qstrip_reflect" };
  ComPtr<IDxcResult> res;
  CHECK(comp->Compile(&src, args, _countof(args), nullptr, IID_PPV_ARGS(&res)), "dxc Compile");
  HRESULT status = S_OK; res->GetStatus(&status);
  ComPtr<IDxcBlobUtf8> errs; res->GetOutput(DXC_OUT_ERRORS, IID_PPV_ARGS(&errs), nullptr);
  if (FAILED(status)) {
    fprintf(stderr, "shader %ls failed:\n%s\n", entry, errs && errs->GetStringLength() ? errs->GetStringPointer() : "");
    Die("shader compile");
  }
  ComPtr<IDxcBlob> obj; CHECK(res->GetOutput(DXC_OUT_OBJECT, IID_PPV_ARGS(&obj), nullptr), "dxc output");
  return obj;
}

static ComPtr<ID3D12RootSignature> g_rs;
enum RootParam { RP_CONST = 0, RP_FRAME, RP_TLAS, RP_VERTS, RP_INDICES, RP_HEADS, RP_SRV, RP_UAV };

static void CreateRootSignature() {
  D3D12_DESCRIPTOR_RANGE srv{ D3D12_DESCRIPTOR_RANGE_TYPE_SRV, 4, 4, 0, 0 };
  D3D12_DESCRIPTOR_RANGE uav{ D3D12_DESCRIPTOR_RANGE_TYPE_UAV, 4, 0, 0, 0 };
  D3D12_ROOT_PARAMETER p[8]{};
  p[RP_CONST].ParameterType = D3D12_ROOT_PARAMETER_TYPE_32BIT_CONSTANTS; p[RP_CONST].Constants = { 0, 0, 16 };
  p[RP_FRAME].ParameterType = D3D12_ROOT_PARAMETER_TYPE_CBV; p[RP_FRAME].Descriptor = { 1, 0 };
  p[RP_TLAS].ParameterType = D3D12_ROOT_PARAMETER_TYPE_SRV; p[RP_TLAS].Descriptor = { 0, 0 };
  p[RP_VERTS].ParameterType = D3D12_ROOT_PARAMETER_TYPE_SRV; p[RP_VERTS].Descriptor = { 1, 0 };
  p[RP_INDICES].ParameterType = D3D12_ROOT_PARAMETER_TYPE_SRV; p[RP_INDICES].Descriptor = { 2, 0 };
  p[RP_HEADS].ParameterType = D3D12_ROOT_PARAMETER_TYPE_SRV; p[RP_HEADS].Descriptor = { 3, 0 };
  p[RP_SRV].ParameterType = D3D12_ROOT_PARAMETER_TYPE_DESCRIPTOR_TABLE; p[RP_SRV].DescriptorTable = { 1, &srv };
  p[RP_UAV].ParameterType = D3D12_ROOT_PARAMETER_TYPE_DESCRIPTOR_TABLE; p[RP_UAV].DescriptorTable = { 1, &uav };
  for (auto& q : p) q.ShaderVisibility = D3D12_SHADER_VISIBILITY_ALL;
  D3D12_STATIC_SAMPLER_DESC s{};
  s.Filter = D3D12_FILTER_MIN_MAG_MIP_LINEAR;
  s.AddressU = s.AddressV = s.AddressW = D3D12_TEXTURE_ADDRESS_MODE_CLAMP;
  s.MaxLOD = D3D12_FLOAT32_MAX; s.ShaderVisibility = D3D12_SHADER_VISIBILITY_ALL;
  D3D12_ROOT_SIGNATURE_DESC d{ 8, p, 1, &s, D3D12_ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT };
  ComPtr<ID3DBlob> blob, err;
  if (FAILED(D3D12SerializeRootSignature(&d, D3D_ROOT_SIGNATURE_VERSION_1, &blob, &err))) {
    fprintf(stderr, "%s\n", err ? (const char*)err->GetBufferPointer() : ""); Die("root signature");
  }
  CHECK(g_device->CreateRootSignature(0, blob->GetBufferPointer(), blob->GetBufferSize(), IID_PPV_ARGS(&g_rs)), "CreateRootSignature");
}

static ComPtr<ID3D12PipelineState> g_psoSky, g_psoFloor, g_psoTube, g_psoHead, g_psoTonemap, g_psoDust;
static ComPtr<ID3D12PipelineState> g_psoLight, g_psoResolve, g_psoDown, g_psoUp, g_psoReflect, g_psoReflBlur;

static ComPtr<ID3D12PipelineState> ComputePSO(const wchar_t* entry) {
  auto cs = Compile(entry, L"cs_6_5");
  D3D12_COMPUTE_PIPELINE_STATE_DESC d{}; d.pRootSignature = g_rs.Get();
  d.CS = { cs->GetBufferPointer(), cs->GetBufferSize() };
  ComPtr<ID3D12PipelineState> p; CHECK(g_device->CreateComputePipelineState(&d, IID_PPV_ARGS(&p)), "compute PSO");
  return p;
}

static ComPtr<ID3D12PipelineState> GraphicsPSO(const wchar_t* vs, const wchar_t* ps, const D3D12_INPUT_ELEMENT_DESC* il, UINT ilCount,
                                               DXGI_FORMAT rtv, UINT samples, bool depth, bool additive = false) {
  auto v = Compile(vs, L"vs_6_5"), p = Compile(ps, L"ps_6_5");
  D3D12_GRAPHICS_PIPELINE_STATE_DESC d{};
  d.pRootSignature = g_rs.Get();
  d.VS = { v->GetBufferPointer(), v->GetBufferSize() };
  d.PS = { p->GetBufferPointer(), p->GetBufferSize() };
  d.InputLayout = { il, ilCount };
  d.PrimitiveTopologyType = D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE;
  d.RasterizerState.FillMode = D3D12_FILL_MODE_SOLID;
  d.RasterizerState.CullMode = D3D12_CULL_MODE_NONE;
  d.RasterizerState.DepthClipEnable = TRUE;
  d.RasterizerState.MultisampleEnable = samples > 1;
  d.BlendState.RenderTarget[0].RenderTargetWriteMask = D3D12_COLOR_WRITE_ENABLE_ALL;
  d.SampleMask = UINT_MAX;
  d.NumRenderTargets = 1; d.RTVFormats[0] = rtv;
  d.SampleDesc = { samples, 0 };
  if (depth) {
    d.DSVFormat = DXGI_FORMAT_D32_FLOAT;
    d.DepthStencilState.DepthEnable = TRUE;
    d.DepthStencilState.DepthWriteMask = D3D12_DEPTH_WRITE_MASK_ALL;
    d.DepthStencilState.DepthFunc = D3D12_COMPARISON_FUNC_GREATER;  // reverse Z
    if (additive) {
      // Light added into the air (the dust): tested against the scene, never hiding anything.
      d.DepthStencilState.DepthWriteMask = D3D12_DEPTH_WRITE_MASK_ZERO;
      auto& b = d.BlendState.RenderTarget[0];
      b.BlendEnable = TRUE;
      b.SrcBlend = b.DestBlend = D3D12_BLEND_ONE; b.BlendOp = D3D12_BLEND_OP_ADD;
      b.SrcBlendAlpha = b.DestBlendAlpha = D3D12_BLEND_ONE; b.BlendOpAlpha = D3D12_BLEND_OP_ADD;
    }
  } else if (samples > 1) {
    d.DSVFormat = DXGI_FORMAT_D32_FLOAT;  // the sky shares the pass's depth target, untouched
  }
  ComPtr<ID3D12PipelineState> pso; CHECK(g_device->CreateGraphicsPipelineState(&d, IID_PPV_ARGS(&pso)), "graphics PSO");
  return pso;
}

static void CreatePipelines() {
  CreateRootSignature();
  D3D12_INPUT_ELEMENT_DESC tube[] = {
    { "CENTER", 0, DXGI_FORMAT_R32G32B32_FLOAT, 0, 0, D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA, 0 },
    { "DIR", 0, DXGI_FORMAT_R32G32B32_FLOAT, 0, 12, D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA, 0 },
    { "RADIUS", 0, DXGI_FORMAT_R32_FLOAT, 0, 24, D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA, 0 },
    { "VOICE", 0, DXGI_FORMAT_R32_UINT, 0, 28, D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA, 0 },
    { "LEVEL", 0, DXGI_FORMAT_R32_FLOAT, 0, 32, D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA, 0 },
    { "NOTEX", 0, DXGI_FORMAT_R32_FLOAT, 0, 36, D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA, 0 },
    { "PITCH", 0, DXGI_FORMAT_R32_FLOAT, 0, 40, D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA, 0 },
  };
  D3D12_INPUT_ELEMENT_DESC head[] = {
    { "POSITION", 0, DXGI_FORMAT_R32G32B32_FLOAT, 0, 0, D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA, 0 },
  };
  g_psoSky = GraphicsPSO(L"FullscreenVS", L"SkyPS", nullptr, 0, kHdrFmt, g_msaa, false);
  g_psoFloor = GraphicsPSO(L"FloorVS", L"FloorPS", nullptr, 0, kHdrFmt, g_msaa, true);
  g_psoTube = GraphicsPSO(L"TubeVS", L"TubePS", tube, _countof(tube), kHdrFmt, g_msaa, true);
  g_psoHead = GraphicsPSO(L"HeadVS", L"HeadPS", head, _countof(head), kHdrFmt, g_msaa, true);
  g_psoDust = GraphicsPSO(L"DustVS", L"DustPS", nullptr, 0, kHdrFmt, g_msaa, true, true);
  g_psoTonemap = GraphicsPSO(L"FullscreenVS", L"TonemapPS", nullptr, 0, kLdrFmt, 1, false);
  g_psoLight = ComputePSO(L"FloorLightCS");
  g_psoResolve = ComputePSO(L"ResolveCS");
  g_psoDown = ComputePSO(L"BloomDownCS");
  g_psoUp = ComputePSO(L"BloomUpCS");
  g_psoReflect = ComputePSO(L"ReflectCS");
  g_psoReflBlur = ComputePSO(L"ReflectBlurCS");
}

// ================================================================ scene resources

static ComPtr<ID3D12Resource> g_vb, g_ib, g_rtPosBuf, g_blas, g_sphereVB, g_sphereIB;
static UINT g_sphereIndexCount = 0;
static D3D12_VERTEX_BUFFER_VIEW g_vbv{}, g_sphereVbv{};
static D3D12_INDEX_BUFFER_VIEW g_ibv{}, g_sphereIbv{};

struct PerFrame {
  ComPtr<ID3D12Resource> cb, segs, heads, instances, tlas, tlasScratch;
  uint8_t *cbPtr = nullptr, *segPtr = nullptr, *headPtr = nullptr, *instPtr = nullptr;
  ComPtr<ID3D12QueryHeap> queries;
  ComPtr<ID3D12Resource> queryReadback;
} g_pf[kFrames];
static UINT64 g_tlasSize = 0, g_tlasScratchSize = 0;
static const UINT kQueries = 16;
static UINT64 g_tsFreq = 1;

static void BuildSphere() {
  // An icosphere, subdivided twice: 320 triangles, smooth at the head's size on screen.
  std::vector<V3> v; std::vector<uint32_t> idx;
  const float t = (1.0f + sqrtf(5.0f)) / 2.0f;
  V3 base[12] = { {-1,t,0},{1,t,0},{-1,-t,0},{1,-t,0},{0,-1,t},{0,1,t},{0,-1,-t},{0,1,-t},{t,0,-1},{t,0,1},{-t,0,-1},{-t,0,1} };
  for (auto& b : base) v.push_back(Norm(b));
  uint32_t f[60] = { 0,11,5, 0,5,1, 0,1,7, 0,7,10, 0,10,11, 1,5,9, 5,11,4, 11,10,2, 10,7,6, 7,1,8,
                     3,9,4, 3,4,2, 3,2,6, 3,6,8, 3,8,9, 4,9,5, 2,4,11, 6,2,10, 8,6,7, 9,8,1 };
  idx.assign(f, f + 60);
  for (int s = 0; s < 2; ++s) {
    std::vector<uint32_t> out;
    for (size_t i = 0; i < idx.size(); i += 3) {
      uint32_t a = idx[i], b = idx[i + 1], c = idx[i + 2];
      auto mid = [&](uint32_t x, uint32_t y) { v.push_back(Norm((v[x] + v[y]) * 0.5f)); return (uint32_t)v.size() - 1; };
      uint32_t ab = mid(a, b), bc = mid(b, c), ca = mid(c, a);
      out.insert(out.end(), { a, ab, ca, b, bc, ab, c, ca, bc, ab, bc, ca });
    }
    idx = out;
  }
  g_sphereVB = UploadStatic(v.data(), v.size() * sizeof(V3), D3D12_RESOURCE_STATE_VERTEX_AND_CONSTANT_BUFFER);
  g_sphereIB = UploadStatic(idx.data(), idx.size() * 4, D3D12_RESOURCE_STATE_INDEX_BUFFER);
  g_sphereVbv = { g_sphereVB->GetGPUVirtualAddress(), (UINT)(v.size() * sizeof(V3)), sizeof(V3) };
  g_sphereIbv = { g_sphereIB->GetGPUVirtualAddress(), (UINT)(idx.size() * 4), DXGI_FORMAT_R32_UINT };
  g_sphereIndexCount = (UINT)idx.size();
}

// The song's geometry on the GPU: its tubes and dashes, and the BLAS over them. Run again for each
// new song (the window waits for the GPU to be idle first: the old buffers are released here).
static void CreateSongResources() {
  Song& s = g_song;
  if (s.indices.empty()) { s.verts.push_back({}); s.verts.push_back({}); s.verts.push_back({}); s.indices = { 0, 1, 2 };
                           s.rtPos.assign(9, -1e6f); }
  const D3D12_RESOURCE_STATES vbState = D3D12_RESOURCE_STATE_VERTEX_AND_CONSTANT_BUFFER | D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE | D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE;
  const D3D12_RESOURCE_STATES ibState = D3D12_RESOURCE_STATE_INDEX_BUFFER | D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE | D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE;
  g_vb = UploadStatic(s.verts.data(), s.verts.size() * sizeof(TubeVtx), vbState);
  g_ib = UploadStatic(s.indices.data(), s.indices.size() * 4, ibState);
  g_rtPosBuf = UploadStatic(s.rtPos.data(), s.rtPos.size() * 4, D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE);
  g_vbv = { g_vb->GetGPUVirtualAddress(), (UINT)(s.verts.size() * sizeof(TubeVtx)), sizeof(TubeVtx) };
  g_ibv = { g_ib->GetGPUVirtualAddress(), (UINT)(s.indices.size() * 4), DXGI_FORMAT_R32_UINT };

  // The BLAS: every tube and dash of the song, built once. Non-opaque, so ray queries can reject
  // the parts ahead of the present.
  D3D12_RAYTRACING_GEOMETRY_DESC geo{};
  geo.Type = D3D12_RAYTRACING_GEOMETRY_TYPE_TRIANGLES;
  geo.Flags = D3D12_RAYTRACING_GEOMETRY_FLAG_NONE;
  geo.Triangles.VertexBuffer = { g_rtPosBuf->GetGPUVirtualAddress(), 12 };
  geo.Triangles.VertexFormat = DXGI_FORMAT_R32G32B32_FLOAT;
  geo.Triangles.VertexCount = (UINT)(s.rtPos.size() / 3);
  geo.Triangles.IndexBuffer = g_ib->GetGPUVirtualAddress();
  geo.Triangles.IndexFormat = DXGI_FORMAT_R32_UINT;
  geo.Triangles.IndexCount = (UINT)s.indices.size();
  D3D12_BUILD_RAYTRACING_ACCELERATION_STRUCTURE_INPUTS in{};
  in.Type = D3D12_RAYTRACING_ACCELERATION_STRUCTURE_TYPE_BOTTOM_LEVEL;
  in.Flags = D3D12_RAYTRACING_ACCELERATION_STRUCTURE_BUILD_FLAG_PREFER_FAST_TRACE;
  in.NumDescs = 1; in.DescsLayout = D3D12_ELEMENTS_LAYOUT_ARRAY; in.pGeometryDescs = &geo;
  D3D12_RAYTRACING_ACCELERATION_STRUCTURE_PREBUILD_INFO pre{};
  g_device->GetRaytracingAccelerationStructurePrebuildInfo(&in, &pre);
  g_blas = MakeBuffer(pre.ResultDataMaxSizeInBytes, D3D12_HEAP_TYPE_DEFAULT, D3D12_RESOURCE_STATE_RAYTRACING_ACCELERATION_STRUCTURE, D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS);
  auto scratch = MakeBuffer(pre.ScratchDataSizeInBytes, D3D12_HEAP_TYPE_DEFAULT, D3D12_RESOURCE_STATE_UNORDERED_ACCESS, D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS);
  D3D12_BUILD_RAYTRACING_ACCELERATION_STRUCTURE_DESC bd{};
  bd.Inputs = in; bd.DestAccelerationStructureData = g_blas->GetGPUVirtualAddress(); bd.ScratchAccelerationStructureData = scratch->GetGPUVirtualAddress();
  CHECK(g_alloc[0]->Reset(), "alloc reset"); CHECK(g_cmd->Reset(g_alloc[0].Get(), nullptr), "cmd reset");
  g_cmd->BuildRaytracingAccelerationStructure(&bd, 0, nullptr);
  CHECK(g_cmd->Close(), "close"); ID3D12CommandList* l[] = { g_cmd.Get() }; g_queue->ExecuteCommandLists(1, l);
  WaitIdle();
  printf("BLAS: %.1f MB\n", pre.ResultDataMaxSizeInBytes / 1048576.0);
}

// Everything else the scene needs, made once: the head sphere, the TLAS and the per-frame buffers.
static void CreateSceneResources() {
  BuildSphere();
  CreateSongResources();

  // The TLAS holds two copies of the song (this loop and the one before), moved every frame.
  D3D12_BUILD_RAYTRACING_ACCELERATION_STRUCTURE_INPUTS tin{};
  tin.Type = D3D12_RAYTRACING_ACCELERATION_STRUCTURE_TYPE_TOP_LEVEL;
  tin.Flags = D3D12_RAYTRACING_ACCELERATION_STRUCTURE_BUILD_FLAG_PREFER_FAST_BUILD;
  tin.NumDescs = 2; tin.DescsLayout = D3D12_ELEMENTS_LAYOUT_ARRAY;
  D3D12_RAYTRACING_ACCELERATION_STRUCTURE_PREBUILD_INFO tpre{};
  g_device->GetRaytracingAccelerationStructurePrebuildInfo(&tin, &tpre);
  g_tlasSize = tpre.ResultDataMaxSizeInBytes; g_tlasScratchSize = tpre.ScratchDataSizeInBytes;

  CHECK(g_queue->GetTimestampFrequency(&g_tsFreq), "timestamp frequency");
  for (auto& f : g_pf) {
    f.cb = MakeBuffer(4096, D3D12_HEAP_TYPE_UPLOAD, D3D12_RESOURCE_STATE_GENERIC_READ);
    f.segs = MakeBuffer(kMaxSegs * sizeof(Seg), D3D12_HEAP_TYPE_UPLOAD, D3D12_RESOURCE_STATE_GENERIC_READ);
    f.heads = MakeBuffer(kMaxHeads * sizeof(HeadGpu), D3D12_HEAP_TYPE_UPLOAD, D3D12_RESOURCE_STATE_GENERIC_READ);
    f.instances = MakeBuffer(2 * sizeof(D3D12_RAYTRACING_INSTANCE_DESC), D3D12_HEAP_TYPE_UPLOAD, D3D12_RESOURCE_STATE_GENERIC_READ);
    f.tlas = MakeBuffer(g_tlasSize, D3D12_HEAP_TYPE_DEFAULT, D3D12_RESOURCE_STATE_RAYTRACING_ACCELERATION_STRUCTURE, D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS);
    f.tlasScratch = MakeBuffer(g_tlasScratchSize, D3D12_HEAP_TYPE_DEFAULT, D3D12_RESOURCE_STATE_UNORDERED_ACCESS, D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS);
    CHECK(f.cb->Map(0, nullptr, (void**)&f.cbPtr), "map cb");
    CHECK(f.segs->Map(0, nullptr, (void**)&f.segPtr), "map segs");
    CHECK(f.heads->Map(0, nullptr, (void**)&f.headPtr), "map heads");
    CHECK(f.instances->Map(0, nullptr, (void**)&f.instPtr), "map instances");
    D3D12_QUERY_HEAP_DESC qd{ D3D12_QUERY_HEAP_TYPE_TIMESTAMP, kQueries, 0 };
    CHECK(g_device->CreateQueryHeap(&qd, IID_PPV_ARGS(&f.queries)), "query heap");
    f.queryReadback = MakeBuffer(kQueries * 8, D3D12_HEAP_TYPE_READBACK, D3D12_RESOURCE_STATE_COPY_DEST);
  }
}

// ================================================================ size-dependent targets

struct Targets {
  UINT w = 0, h = 0;
  Tex msaa, depth, hdr, irr, ldr;  // ldr: the offscreen output (capture/bench)
  Tex refl, reflTmp;               // the floor's reflection, full resolution
  Tex down[kBloomLevels], up[kBloomLevels];
  BindSet light, reflect, blurV, blurH, main, resolve, downSet[kBloomLevels], upSet[kBloomLevels], tonemap;
} g_t;

static void CreateTargets(UINT w, UINT h, bool offscreen) {
  g_t = Targets{};
  g_heapUsed = 0;
  g_t.w = w; g_t.h = h;
  D3D12_CLEAR_VALUE cc{}; cc.Format = kHdrFmt;
  D3D12_CLEAR_VALUE dc{}; dc.Format = DXGI_FORMAT_D32_FLOAT; dc.DepthStencil.Depth = 0.0f;
  g_t.msaa = MakeTex(w, h, kHdrFmt, D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET, D3D12_RESOURCE_STATE_RENDER_TARGET, g_msaa, &cc);
  g_t.depth = MakeTex(w, h, DXGI_FORMAT_D32_FLOAT, D3D12_RESOURCE_FLAG_ALLOW_DEPTH_STENCIL | D3D12_RESOURCE_FLAG_DENY_SHADER_RESOURCE,
                      D3D12_RESOURCE_STATE_DEPTH_WRITE, g_msaa, &dc);
  g_t.hdr = MakeTex(w, h, kHdrFmt, D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS, D3D12_RESOURCE_STATE_UNORDERED_ACCESS);
  g_t.irr = MakeTex(kIrrW, kIrrH, kHdrFmt, D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS, D3D12_RESOURCE_STATE_UNORDERED_ACCESS);
  // Full resolution: at half resolution a two-centimetre tube's mirror image aliases into dots.
  g_t.refl = MakeTex(w, h, kHdrFmt, D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS, D3D12_RESOURCE_STATE_UNORDERED_ACCESS);
  g_t.reflTmp = MakeTex(w, h, kHdrFmt, D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS, D3D12_RESOURCE_STATE_UNORDERED_ACCESS);
  if (offscreen) {
    D3D12_CLEAR_VALUE lc{}; lc.Format = kLdrFmt;
    g_t.ldr = MakeTex(w, h, kLdrFmt, D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET, D3D12_RESOURCE_STATE_RENDER_TARGET, 1, &lc);
  }
  for (int k = 0; k < kBloomLevels; ++k) {
    UINT bw = std::max(1u, w >> (k + 1)), bh = std::max(1u, h >> (k + 1));
    g_t.down[k] = MakeTex(bw, bh, kHdrFmt, D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS, D3D12_RESOURCE_STATE_UNORDERED_ACCESS);
    g_t.up[k] = MakeTex(bw, bh, kHdrFmt, D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS, D3D12_RESOURCE_STATE_UNORDERED_ACCESS);
  }
  g_t.light = MakeSet({}, { &g_t.irr });
  g_t.reflect = MakeSet({}, { &g_t.refl });
  g_t.blurV = MakeSet({ &g_t.refl }, { &g_t.reflTmp });
  g_t.blurH = MakeSet({ &g_t.reflTmp }, { &g_t.refl });
  g_t.main = MakeSet({ &g_t.irr, &g_t.refl }, {});
  g_t.resolve = MakeSet({ &g_t.msaa }, { &g_t.hdr });
  for (int k = 0; k < kBloomLevels; ++k) g_t.downSet[k] = MakeSet({ k == 0 ? &g_t.hdr : &g_t.down[k - 1] }, { &g_t.down[k] });
  for (int k = kBloomLevels - 1; k >= 0; --k)
    g_t.upSet[k] = MakeSet({ k == kBloomLevels - 1 ? &g_t.down[k] : &g_t.up[k + 1], &g_t.down[k] }, { &g_t.up[k] });
  g_t.tonemap = MakeSet({ &g_t.hdr, &g_t.up[0] }, {});

  D3D12_DESCRIPTOR_HEAP_DESC rd{ D3D12_DESCRIPTOR_HEAP_TYPE_RTV, 1 + 3, D3D12_DESCRIPTOR_HEAP_FLAG_NONE, 0 };
  CHECK(g_device->CreateDescriptorHeap(&rd, IID_PPV_ARGS(&g_rtvHeap)), "rtv heap");
  D3D12_DESCRIPTOR_HEAP_DESC dd{ D3D12_DESCRIPTOR_HEAP_TYPE_DSV, 1, D3D12_DESCRIPTOR_HEAP_FLAG_NONE, 0 };
  CHECK(g_device->CreateDescriptorHeap(&dd, IID_PPV_ARGS(&g_dsvHeap)), "dsv heap");
  g_device->CreateRenderTargetView(g_t.msaa.r.Get(), nullptr, g_rtvHeap->GetCPUDescriptorHandleForHeapStart());
  g_device->CreateDepthStencilView(g_t.depth.r.Get(), nullptr, g_dsvHeap->GetCPUDescriptorHandleForHeapStart());
}

static D3D12_CPU_DESCRIPTOR_HANDLE Rtv(UINT i) { auto h = g_rtvHeap->GetCPUDescriptorHandleForHeapStart(); h.ptr += (SIZE_T)i * g_rtvStride; return h; }

// ================================================================ the frame

struct FrameCB {
  M4 viewProj, invViewProj;
  float camPos[4], world[4], fog[4], fogP[4], irrRegion[4], floorP[4], screen[4];
  uint32_t counts[4];
  float post[4];
  float voice[32][4];
  float debug[4];
  float expr[4];         // x flare, y flare beats, z pitch hue (rad), w scatter strength
  float rippleP[4];      // x speed (m/s), y width (m), z decay (s), w wavenumber (rad/m)
  float dustP[4];        // x count, y brightness, z the present's song x (m), w size (px)
  float ripple[4][4];    // x, z (relative to the present), age (s), amplitude (0: none)
  float lightPos[4][4];  // xyz, w the light's own radius (m)
  float lightCol[4][4];  // rgb x intensity (0: none)
  float bass[6][4];      // the bass on the water, per note: x, z (relative to the present), k (rad/m), phase (rad)
  float bassB[6][4];     // r0 (inner edge, m: 0 while it still drives), r1 (the front, m), height (m; 0: none), e-fold (m)
};
static_assert(sizeof(FrameCB) <= 4096, "frame constants");

struct View {
  double total = 0;    // seconds since playback began (continuous across loops)
  double songSec = 0;  // position in the song, [0, duration)
  int loop = 0;        // how many times the song has wrapped (previous loop drawn when > 0)
};

static const float kIrrX0 = -48.0f, kIrrX1 = 8.0f, kIrrZ0 = -8.0f, kIrrZ1 = 13.0f;
static const float kDustPeriod = 72.0f;  // must match DUST_PERIOD in world.hlsl

// Light segments for the floor: the recent past of every tonal voice, coarsely, and every drum
// hit for a moment after it lands. Positions are relative to the present.
static UINT BuildSegments(const View& v, Seg* out, double fogBeats) {
  Song& s = g_song;
  const LmInfo& i = s.info;
  double now = v.songSec * i.bpm / 60.0;
  double window = std::min(fogBeats * 2.2, 40.0);
  const double step = 0.125;
  UINT n = 0;
  auto frameAt = [&](const VoiceLook& vl, double beat, LmFrame& f) -> bool {
    // A beat before the song's start belongs to the previous loop, if there was one.
    if (beat < 0) { if (v.loop == 0 || !i.looped) return false; beat += i.beats; }
    uint64_t k = (uint64_t)(beat * 60.0 / i.bpm * i.rate) / i.trace_step;
    if (k >= vl.frameCount) return false;
    f = vl.frames[k];
    return f.gate > 0 && std::isfinite(f.pitch);
  };
  // The last fineSpan beats in eighth-beat segments; the older, fogged past in coarser ones (the
  // floor light's cost is segments x texels, and a dense song has thousands). Both grids are fixed
  // in song beats, so segments do not shimmer as the present moves.
  const double coarse = 0.5, fineSpan = 4.0;
  const double bFine = floor((now - fineSpan) / coarse) * coarse;
  for (auto& vl : s.voices) {
    if (vl.drum) continue;
    double b0 = floor((now - window) / coarse) * coarse;
    LmFrame fa{}, fb{};
    bool ha = frameAt(vl, b0, fa);
    for (double prev = b0; prev < now - 1e-9 && n < kMaxSegs;) {
      double b = prev + (prev < bFine - 1e-9 ? coarse : step);
      double bb = std::min(b, now);
      bool hb = frameAt(vl, bb, fb);
      // A note change inside a segment would draw a diagonal of light between two notes: skip it.
      if (ha && hb && fabs(fa.pitch - fb.pitch) <= 0.75) {
        float bright = 0.5f * (Brightness(vl, fa.level) + Brightness(vl, fb.level));
        float age = (float)(now - bb);
        float rec = 1.0f + 2.0f * expf(-age / 0.6f);
        float k = vl.intensity * g_theme.glow * bright * rec;
        out[n++] = { (float)((prev - now) * kBeatLen), PitchY(s, fa.pitch), vl.z, 0,
                     (float)((bb - now) * kBeatLen), PitchY(s, fb.pitch), vl.z, 0, vl.r * k, vl.g * k, vl.b * k, 0 };
      }
      fa = fb; ha = hb;
      prev = b;
    }
  }
  // Drum flashes: the dash's light lingers for about a third of a beat.
  for (auto& vl : s.voices) {
    if (!vl.drum || vl.hits.empty()) continue;
    DashSpan span = DashOf(s, g_theme, vl);
    double bottom = span.yEnd, top = DashTopDrawn(span);
    for (int pass = 0; pass < 2; ++pass) {
      if (pass == 1 && (v.loop == 0 || !i.looped)) break;
      double shift = pass == 0 ? 0 : -i.beats;
      for (auto& h : vl.hits) {
        double hb = h.beat + shift;
        double age = now - hb;
        if (age < 0 || age > 1.5 || n >= kMaxSegs) continue;
        float k = vl.intensity * g_theme.glow * 3.0f * (float)(vl.maxAmp > 0 ? h.amp / vl.maxAmp : 1) * expf((float)(-age / 0.3));
        float x = (float)((hb - now) * kBeatLen);
        out[n++] = { x, (float)top, vl.z, 0, x + 0.05f, (float)bottom, vl.z, 0, vl.r * k, vl.g * k, vl.b * k, 0 };
      }
    }
  }
  return n;
}

// Beats since the note sounding at frame k began: back along the open gate until it closes or the
// pitch jumps (the same rule that breaks the tubes). Capped, since only the onset's flare needs it.
static double NoteAgeBeats(const VoiceLook& vl, uint64_t k) {
  const LmInfo& i = g_song.info;
  const uint64_t cap = (uint64_t)(4.0 * 60.0 / i.bpm * i.rate / std::max(i.trace_step, 1u));
  uint64_t j = k;
  while (j > 0 && k - j < cap) {
    const LmFrame& a = vl.frames[j - 1];
    const LmFrame& b = vl.frames[j];
    if (!(a.gate > 0) || !std::isfinite(a.pitch) || fabs(b.pitch - a.pitch) > 0.75) break;
    --j;
  }
  return (double)((k - j) * i.trace_step) * i.bpm / (60.0 * i.rate);
}

static float Flare(double ageBeats) {
  return ageBeats >= 0 ? g_theme.flare * expf((float)(-ageBeats / g_theme.flareBeats)) : 0.0f;
}

// The draw-heads: each tonal voice that is sounding now, at the present. A head flares at its
// note's onset and follows the voice's live level.
static UINT BuildHeads(const View& v, HeadGpu* out) {
  Song& s = g_song;
  const LmInfo& i = s.info;
  UINT n = 0;
  uint64_t fk = (uint64_t)(v.songSec * i.rate) / std::max(i.trace_step, 1u);
  for (uint32_t k = 0; k < s.voices.size() && n < kMaxHeads; ++k) {
    VoiceLook& vl = s.voices[k];
    if (vl.drum) continue;
    LmFrame f{};
    if (!lm_sample(s.lm, k, v.songSec, &f)) continue;
    if (!(f.level > 0) || !std::isfinite(f.pitch)) continue;
    float b = Brightness(vl, f.level);
    float flare = (f.gate > 0 && fk < vl.frameCount) ? Flare(NoteAgeBeats(vl, fk)) : 0.0f;
    // Open gate: the head blooms; in the release it fades with the level.
    float k2 = vl.intensity * g_theme.glow * b * (f.gate > 0 ? 3.0f : 1.2f) * (1.0f + flare);
    float rad = vl.radius * (g_theme.thickMin + g_theme.thickRange * b) * 2.6f * (1.0f + 0.25f * flare);
    out[n++] = { 0.0f, PitchY(s, f.pitch), vl.z, rad, vl.r, vl.g, vl.b, k2 * 1.5f };
  }
  return n;
}

// The lights in the air: the present (the sounding heads together) and the most recent drum hits,
// each a brief flash at its dash. They scatter in the fog, and the kicks ring the floor.
static void BuildLightsAndRipples(const View& v, const HeadGpu* heads, UINT headCount, FrameCB& cb) {
  Song& s = g_song;
  const LmInfo& i = s.info;
  const Theme& th = g_theme;
  double now = v.songSec * i.bpm / 60.0;
  // The present: every sounding head's light, gathered where they stand.
  float sum[3] = {}, wy = 0, wz = 0, w = 0;
  for (UINT k = 0; k < headCount; ++k) {
    const HeadGpu& h = heads[k];
    sum[0] += h.r * h.intensity; sum[1] += h.g * h.intensity; sum[2] += h.b * h.intensity;
    wy += h.y * h.intensity; wz += h.z * h.intensity; w += h.intensity;
  }
  if (w > 0) {
    cb.lightPos[0][0] = -0.2f; cb.lightPos[0][1] = wy / w; cb.lightPos[0][2] = wz / w + 0.2f; cb.lightPos[0][3] = 0.6f;
    for (int c = 0; c < 3; ++c) cb.lightCol[0][c] = sum[c] * th.presentGlow;
  }
  // Recent drum hits, strongest first: up to three flashes, up to four kick ripples.
  struct Flash { float strength, x, y, z, r, g, b; };
  struct Ring { double age; float x, z, amp; };
  std::vector<Flash> flashes; std::vector<Ring> rings;
  const double secPerBeat = 60.0 / i.bpm;
  for (auto& vl : s.voices) {
    if (!vl.drum || vl.hits.empty()) continue;
    DashSpan span = DashOf(s, g_theme, vl);
    bool kick = vl.info.drum_kind == LM_KICK || vl.name.find("kick") != std::string::npos;
    for (int pass = 0; pass < 2; ++pass) {
      if (pass == 1 && (v.loop == 0 || !i.looped)) break;
      double shift = pass == 0 ? 0 : -i.beats;
      for (auto& h : vl.hits) {
        double age = (now - (h.beat + shift)) * secPerBeat;  // seconds
        if (age < 0 || age > 5.0 * th.rippleDecay) continue;
        float amp = (float)(vl.maxAmp > 0 ? h.amp / vl.maxAmp : 1.0);
        float x = (float)((h.beat + shift - now) * kBeatLen);
        if (age < 4.0 * th.dashFlashSec) {
          float k = vl.intensity * th.glow * th.dashFlash * amp * expf((float)(-age / th.dashFlashSec));
          flashes.push_back({ k, x + 0.1f, 0.5f * (span.yEnd + DashTopDrawn(span)), vl.z + 0.15f, vl.r * k, vl.g * k, vl.b * k });
        }
        if (kick) rings.push_back({ age, x, vl.z + 0.3f, th.rippleAmp * amp });
      }
    }
  }
  std::sort(flashes.begin(), flashes.end(), [](const Flash& a, const Flash& b) { return a.strength > b.strength; });
  for (size_t k = 0; k < flashes.size() && k < 3; ++k) {
    const Flash& f = flashes[k];
    cb.lightPos[1 + k][0] = f.x; cb.lightPos[1 + k][1] = f.y; cb.lightPos[1 + k][2] = f.z; cb.lightPos[1 + k][3] = 0.7f;
    cb.lightCol[1 + k][0] = f.r; cb.lightCol[1 + k][1] = f.g; cb.lightCol[1 + k][2] = f.b;
  }
  std::sort(rings.begin(), rings.end(), [](const Ring& a, const Ring& b) { return a.age < b.age; });
  for (size_t k = 0; k < rings.size() && k < 4; ++k) {
    cb.ripple[k][0] = rings[k].x; cb.ripple[k][1] = rings[k].z; cb.ripple[k][2] = (float)rings[k].age; cb.ripple[k][3] = rings[k].amp;
  }
}

// The bass on the water (water.h). Every tonal voice shakes the floor, as hard as its live loudness times its
// coupling (exp(-height / reach): a bass near the water rings it, a lead high on the wall barely touches it). Each
// note is one source, centred on the water under where its draw-head stood when it began (so a note starting sends
// a fresh ring out, and the rings recede with the past like the kick's). Its waves have the wavelength its pitch
// raises now (a glide tightens or loosens them as it goes). Its crests move out at their drawn phase speed (wavelength
// x the slowed Faraday frequency) and its front, where the energy has reached, at the group velocity, both integrated
// from the onset over the trace so a glide stays continuous. The waves lose an e-fold every bassWaveDecay seconds
// (so, at the group velocity, every speed x decay metres) and spread as 1 / sqrt(r).
// When a note stops sounding its drive stops: an inner edge follows the front out. The strongest six notes are drawn.
static void BuildBassWaves(const View& v, FrameCB& cb) {
  const Song& s = g_song;
  const LmInfo& i = s.info;
  const Theme& th = g_theme;
  if (!(th.bassWaveAmp > 0) || !(th.bassWaveScale > 0) || !(th.bassWaveDecay > 0) || !(th.bassWaveReach > 0)) return;
  if (!(i.rate > 0) || !(i.bpm > 0) || !(i.reference_hz > 0)) return;
  const uint32_t step = std::max(i.trace_step, 1u);
  const double dt = (double)step / i.rate;                       // seconds per trace frame
  const uint64_t fk = (uint64_t)(v.songSec * i.rate) / step;     // the frame sounding now
  const uint64_t alive = (uint64_t)(4.0 * th.bassWaveDecay / dt);  // a stopped note's waves, drawn for 4 e-folds
  const uint64_t maxNote = (uint64_t)(30.0 / dt);                // a drone's onset is looked for this far back
  const uint64_t stride = std::max<uint64_t>(1, (uint64_t)(0.005 / dt));  // the integration's step: 5 ms
  struct Src { float x, z, k, phase, r0, r1, amp, L, rank; };
  std::vector<Src> srcs;
  for (const VoiceLook& vl : s.voices) {
    if (vl.drum || !vl.frames || vl.frameCount == 0 || fk >= vl.frameCount || vl.maxLevel <= 0) continue;
    auto active = [&](uint64_t j) { const LmFrame& f = vl.frames[j]; return std::isfinite(f.pitch) && f.level > 0; };
    // A note starts at frame j when j sounds and j - 1 did not, or the pitch jumps (the tubes' rule), or the gate
    // opens again (a retrigger at the same pitch).
    auto starts = [&](uint64_t j) {
      if (j == 0) return true;
      const LmFrame& a = vl.frames[j - 1];
      const LmFrame& b = vl.frames[j];
      return !(std::isfinite(a.pitch) && a.level > 0) || fabs(b.pitch - a.pitch) > 0.75 || (b.gate > 0 && !(a.gate > 0));
    };
    // Back from now: the latest two notes whose waves are still alive.
    uint64_t j = fk;
    for (int found = 0; found < 2;) {
      while (j > 0 && !active(j) && fk - j <= alive) --j;  // the quiet since the last note ended
      if (!active(j) || fk - j > alive) break;
      const uint64_t end = j;                              // the note's last sounding frame
      // A voice high on the wall (or this note too quiet: under 2% of the full height) does not visibly move the water.
      if (th.bassWaveAmp * Brightness(vl, vl.frames[end].level) * water::Coupling(PitchY(s, vl.frames[end].pitch), th.bassWaveReach) <= 0.02 * th.bassWaveAmp) break;
      while (j > 0 && !starts(j) && end - j < maxNote) --j;
      const uint64_t onset = j;
      ++found;
      // Integrate the note's phase and front from its onset, on a stride aligned to the onset (deterministic).
      double phase = 0, r1 = 0, k = 0, vp = 0, nu = 0;
      for (uint64_t q = onset; q <= end; q += stride) {
        const LmFrame& f = vl.frames[q];
        double hz = i.reference_hz * exp2(f.pitch / 12.0);
        double lam = water::FaradayWavelength(hz, th.bassWaveScale);
        nu = water::VisualHz(hz);
        if (!(lam > 0)) continue;
        double span = (double)std::min<uint64_t>(stride, end + 1 - q) * dt;
        vp = lam * nu;
        phase += 2.0 * 3.14159265358979 * nu * span;
        r1 += vp * water::GroupOverPhase(hz) * span;  // the front: energy moves at the group velocity
      }
      const LmFrame& last = vl.frames[end];
      double hzLast = i.reference_hz * exp2(last.pitch / 12.0);
      double lamLast = water::FaradayWavelength(hzLast, th.bassWaveScale);
      if (lamLast > 0) {
        k = 2.0 * 3.14159265358979 / lamLast;
        nu = water::VisualHz(hzLast);
        vp = lamLast * nu;
        // After its last frame the note no longer drives: its waves move on, an inner edge behind them.
        double since = (double)(fk - end) * dt;
        phase += 2.0 * 3.14159265358979 * nu * since;
        double cg = vp * water::GroupOverPhase(hzLast);
        r1 += cg * since;
        double r0 = end == fk ? 0.0 : cg * since;
        double amp = th.bassWaveAmp * Brightness(vl, last.level) * water::Coupling(PitchY(s, last.pitch), th.bassWaveReach);
        double L = std::max(cg * th.bassWaveDecay, 1e-3);
        double rank = amp * k * exp(-r0 / L);  // the steepest slope it still draws
        srcs.push_back({ (float)((FrameBeat(i, onset) - FrameBeat(i, fk)) * kBeatLen), vl.z, (float)k,
                           (float)fmod(phase, 2.0 * 3.14159265358979), (float)r0, (float)r1, (float)amp, (float)L, (float)rank });
      }
      if (onset == 0) break;
      j = onset - 1;
    }
  }
  std::sort(srcs.begin(), srcs.end(), [](const Src& a, const Src& b) { return a.rank > b.rank; });
  for (size_t n = 0; n < srcs.size() && n < 6; ++n) {
    const Src& q = srcs[n];
    cb.bass[n][0] = q.x; cb.bass[n][1] = q.z; cb.bass[n][2] = q.k; cb.bass[n][3] = q.phase;
    cb.bassB[n][0] = q.r0; cb.bassB[n][1] = q.r1; cb.bassB[n][2] = q.amp; cb.bassB[n][3] = q.L;
  }
}

struct Camera { V3 eye, target; };

// The camera rides with the present: ahead of it and off to the side, looking back along the
// wall at the past. It never faces the wall straight on. It breathes with the song: closer and
// lower in quiet parts, back and up in dense ones (the smoothed energy curve, so it never lurches),
// with a slow drift and a small two-beat pendulum that grows with the energy. The theme's breathe
// scales how far it moves from the middle pose (0 holds it there, with no pendulum: the default since 2026-11-03,
// when the camera reacting at a stream's seams looked unnatural; Theme, Camera breathing brings it back).
//
// Pull-back (g_camWide 1; the menu eases it from 0 and back): lifted up and back, looking along the
// wall at the trail the song has left, with the fog thinned by kWideFog so the trail shows whole.
static float g_camWide = 0;
static const float kWideFog = 2.5f;  // pulled back, the fog's distance is (1 + this) times longer
// At a stream's end, the length of trail (m) the pull-back frames: the whole song's (0: the usual pull-back). The fog
// then reaches past the song's start.
static float g_camSpan = 0;
static Camera CameraAt(const View& v) {
  const Theme& th = g_theme;
  float wallMid = kFloorGap + (float)((g_song.pMax - g_song.pMin) * g_song.semi) * 0.5f;
  float t = (float)v.total;
  double beat = v.songSec * g_song.info.bpm / 60.0;
  float e = EnergyAt(beat);
  e = e * e * (3.0f - 2.0f * e);
  e = std::clamp(0.5f + (e - 0.5f) * th.breathe, 0.0f, 1.0f);
  double beats = v.total * g_song.info.bpm / 60.0;  // continuous across loops: the pendulum never jumps
  float pend = th.sway * th.breathe * (0.4f + 0.6f * e) * sinf((float)(fmod(beats, 2.0) * 3.14159265));
  float drift = 0.25f * sinf(t * 0.21f) + 0.12f * sinf(t * 0.13f + 1.0f);
  Camera c;
  V3 q = th.eyeQuiet, d = th.eyeDense;
  c.eye = { q.x + (d.x - q.x) * e, q.y + (d.y - q.y) * e + 0.10f * sinf(t * 0.17f) + 0.3f * pend,
            q.z + (d.z - q.z) * e + drift + pend };
  c.target = { -7.0f, wallMid * (th.targetYQuiet + (th.targetYDense - th.targetYQuiet) * e), -1.6f };
  if (g_camWide > 0) {
    float k = g_camWide * g_camWide * (3.0f - 2.0f * g_camWide);
    Camera wide;
    wide.eye = { 5.5f, wallMid + 2.4f + 0.3f * sinf(t * 0.11f), 17.0f + drift };
    wide.target = { -17.0f, wallMid * 0.80f, -1.0f };
    if (g_camSpan > 0) {  // a song's end: up and back far enough that its whole trail, start to end, is in view
      float L = std::max(g_camSpan, 40.0f);
      wide.eye = { 0.12f * L, wallMid + 0.34f * L + 0.3f * sinf(t * 0.11f), 0.78f * L + drift };
      wide.target = { -0.47f * L, wallMid * 0.5f, 0.0f };
    }
    c.eye = c.eye + (wide.eye - c.eye) * k;
    c.target = c.target + (wide.target - c.target) * k;
  }
  return c;
}

static double g_fogBeats = 16;

// The menu overlay's draw data (ImGui), recorded onto the output after the tonemap; defined with the UI.
static void RecordUi();

// ui: draw the overlay built this frame (ImGui::Render already called) over the picture.
static void RecordFrame(UINT fi, const View& v, ID3D12Resource* outTarget, D3D12_CPU_DESCRIPTOR_HANDLE outRtv, bool timestamps, bool ui = false) {
  PerFrame& pf = g_pf[fi];
  Song& s = g_song;
  const LmInfo& info = s.info;
  const UINT w = g_t.w, h = g_t.h;

  // ---- constants
  Camera cam = CameraAt(v);
  M4 view = LookAt(cam.eye, cam.target, { 0, 1, 0 });
  M4 proj = Perspective(kFovY, (float)w / h, 0.05f);
  FrameCB cb{};
  cb.viewProj = Mul(proj, view);
  cb.invViewProj = Inverse(cb.viewProj);
  float pixelAngle = 2.0f * tanf(kFovY * 0.5f) / h;
  cb.camPos[0] = cam.eye.x; cb.camPos[1] = cam.eye.y; cb.camPos[2] = cam.eye.z; cb.camPos[3] = pixelAngle;
  double nowBeat = v.songSec * info.bpm / 60.0;
  double barLen = 4.0 * kBeatLen;
  double barPhase = fmod(nowBeat * kBeatLen / barLen, 1.0);
  cb.world[0] = kBeatLen; cb.world[1] = (float)nowBeat; cb.world[2] = (float)barPhase; cb.world[3] = (float)barLen;
  // Fog follows tempo: about sixteen beats of trail at 120 bpm, more at a slow tempo.
  const Theme& th = g_theme;
  float fogDist = (float)(g_fogBeats * kBeatLen * 0.55) * (1.0f + kWideFog * g_camWide) / std::max(th.fogDensity, 0.05f);
  if (g_camSpan > 0) {
    float k = g_camWide * g_camWide * (3.0f - 2.0f * g_camWide);
    fogDist = std::max(fogDist, fogDist + (2.0f * std::max(g_camSpan, 40.0f) - fogDist) * k);
  }
  cb.fog[0] = th.fog[0]; cb.fog[1] = th.fog[1]; cb.fog[2] = th.fog[2]; cb.fog[3] = fogDist;
  cb.fogP[0] = th.fogStart; cb.fogP[1] = th.recencyBeats; cb.fogP[2] = th.recencyBoost; cb.fogP[3] = th.exposure;
  cb.irrRegion[0] = kIrrX0; cb.irrRegion[1] = kIrrZ0; cb.irrRegion[2] = kIrrX1 - kIrrX0; cb.irrRegion[3] = kIrrZ1 - kIrrZ0;
  cb.floorP[0] = th.floorRough; cb.floorP[1] = th.floorF0; cb.floorP[2] = th.floorAlbedo; cb.floorP[3] = th.floorGrid;
  cb.screen[0] = (float)w; cb.screen[1] = (float)h; cb.screen[2] = 1.0f / w; cb.screen[3] = 1.0f / h;
  cb.post[0] = th.bloom; cb.post[1] = th.vignette; cb.post[2] = th.floorLight; cb.post[3] = (float)v.songSec;
  for (size_t k = 0; k < s.voices.size() && k < 32; ++k) {
    auto& vl = s.voices[k];
    cb.voice[k][0] = vl.r; cb.voice[k][1] = vl.g; cb.voice[k][2] = vl.b; cb.voice[k][3] = vl.intensity * th.glow;
  }
  cb.expr[0] = th.flare; cb.expr[1] = th.flareBeats; cb.expr[2] = th.pitchHue; cb.expr[3] = th.scatter;
  cb.rippleP[0] = th.rippleSpeed; cb.rippleP[1] = th.rippleWidth; cb.rippleP[2] = th.rippleDecay; cb.rippleP[3] = th.rippleK;
  // The dust is still in the world, so it recedes with the past: its field (kDustPeriod metres,
  // repeating) is placed by the distance travelled since playback began, continuous across loops.
  cb.dustP[0] = (float)th.dustCount; cb.dustP[1] = th.dustBright; cb.dustP[3] = th.dustSizePx;
  cb.dustP[2] = (float)fmod(v.total * info.bpm / 60.0 * kBeatLen, (double)kDustPeriod);
  UINT segCount = BuildSegments(v, (Seg*)pf.segPtr, g_fogBeats);
  HeadGpu heads[kMaxHeads];
  UINT headCount = BuildHeads(v, heads);
  memcpy(pf.headPtr, heads, headCount * sizeof(HeadGpu));
  BuildLightsAndRipples(v, heads, headCount, cb);
  BuildBassWaves(v, cb);
  cb.counts[0] = segCount; cb.counts[1] = headCount; cb.counts[2] = std::clamp(th.reflRays, 1u, 4u); cb.counts[3] = g_msaa;
  if (const char* d = getenv("LM_DEBUG_VIEW")) cb.debug[0] = (float)atoi(d);
  memcpy(pf.cbPtr, &cb, sizeof(cb));

  // ---- the two copies of the song: this loop and, once it has wrapped, the one before
  float xNow = (float)(nowBeat * kBeatLen);
  float offsets[2] = { -xNow, -xNow - s.songLen };
  int copies = (v.loop > 0 && info.looped) ? 2 : 1;
  auto* inst = (D3D12_RAYTRACING_INSTANCE_DESC*)pf.instPtr;
  for (int k = 0; k < 2; ++k) {
    D3D12_RAYTRACING_INSTANCE_DESC d{};
    d.Transform[0][0] = d.Transform[1][1] = d.Transform[2][2] = 1;
    d.Transform[0][3] = k < copies ? offsets[k] : 1e6f;  // an unused copy is parked far away
    d.InstanceMask = k < copies ? 0xFF : 0;
    d.Flags = D3D12_RAYTRACING_INSTANCE_FLAG_TRIANGLE_CULL_DISABLE;
    d.AccelerationStructure = g_blas->GetGPUVirtualAddress();
    inst[k] = d;
  }

  CHECK(g_alloc[fi]->Reset(), "alloc reset");
  CHECK(g_cmd->Reset(g_alloc[fi].Get(), nullptr), "cmd reset");
  if (timestamps) g_cmd->EndQuery(pf.queries.Get(), D3D12_QUERY_TYPE_TIMESTAMP, 0);
  ID3D12DescriptorHeap* heaps[] = { g_heap.Get() };
  g_cmd->SetDescriptorHeaps(1, heaps);

  // ---- TLAS
  D3D12_BUILD_RAYTRACING_ACCELERATION_STRUCTURE_DESC tb{};
  tb.Inputs.Type = D3D12_RAYTRACING_ACCELERATION_STRUCTURE_TYPE_TOP_LEVEL;
  tb.Inputs.Flags = D3D12_RAYTRACING_ACCELERATION_STRUCTURE_BUILD_FLAG_PREFER_FAST_BUILD;
  tb.Inputs.NumDescs = 2; tb.Inputs.DescsLayout = D3D12_ELEMENTS_LAYOUT_ARRAY;
  tb.Inputs.InstanceDescs = pf.instances->GetGPUVirtualAddress();
  tb.DestAccelerationStructureData = pf.tlas->GetGPUVirtualAddress();
  tb.ScratchAccelerationStructureData = pf.tlasScratch->GetGPUVirtualAddress();
  g_cmd->BuildRaytracingAccelerationStructure(&tb, 0, nullptr);
  UavBarrier(pf.tlas.Get());

  // ---- the floor's light
  To(g_t.irr, D3D12_RESOURCE_STATE_UNORDERED_ACCESS);
  Flush();
  g_cmd->SetComputeRootSignature(g_rs.Get());
  g_cmd->SetComputeRootConstantBufferView(RP_FRAME, pf.cb->GetGPUVirtualAddress());
  g_cmd->SetComputeRootShaderResourceView(RP_TLAS, pf.tlas->GetGPUVirtualAddress());
  g_cmd->SetComputeRootShaderResourceView(RP_VERTS, g_vb->GetGPUVirtualAddress());
  g_cmd->SetComputeRootShaderResourceView(RP_INDICES, g_ib->GetGPUVirtualAddress());
  g_cmd->SetComputeRootShaderResourceView(RP_HEADS, pf.segs->GetGPUVirtualAddress());
  g_cmd->SetComputeRootDescriptorTable(RP_SRV, Gpu(g_t.light.srv));
  g_cmd->SetComputeRootDescriptorTable(RP_UAV, Gpu(g_t.light.uav));
  {
    float c[4] = { (float)kIrrW, (float)kIrrH, 0, 0 };
    g_cmd->SetComputeRoot32BitConstants(RP_CONST, 4, c, 0);
  }
  g_cmd->SetPipelineState(g_psoLight.Get());
  g_cmd->Dispatch((kIrrW + 7) / 8, (kIrrH + 7) / 8, 1);
  if (timestamps) g_cmd->EndQuery(pf.queries.Get(), D3D12_QUERY_TYPE_TIMESTAMP, 1);
  // ---- the floor's reflection: a mirror ray per pixel, then the gloss as a blur
  {
    Tex& r = g_t.refl; Tex& tmp = g_t.reflTmp;
    To(r, D3D12_RESOURCE_STATE_UNORDERED_ACCESS);
    To(tmp, D3D12_RESOURCE_STATE_UNORDERED_ACCESS);
    Flush();
    g_cmd->SetComputeRootShaderResourceView(RP_HEADS, pf.heads->GetGPUVirtualAddress());
    float c[8] = { (float)r.w, (float)r.h, 0, 0, 0, 0, 0, 0 };
    g_cmd->SetComputeRoot32BitConstants(RP_CONST, 8, c, 0);
    g_cmd->SetPipelineState(g_psoReflect.Get());
    g_cmd->SetComputeRootDescriptorTable(RP_SRV, Gpu(g_t.reflect.srv));
    g_cmd->SetComputeRootDescriptorTable(RP_UAV, Gpu(g_t.reflect.uav));
    g_cmd->Dispatch((r.w + 7) / 8, (r.h + 7) / 8, 1);
    To(r, D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE);
    Flush();
    g_cmd->SetPipelineState(g_psoReflBlur.Get());
    float v[8] = { (float)r.w, (float)r.h, 0, 0, 0, 1, 1.0f, 0 };  // along the screen's vertical, full width
    g_cmd->SetComputeRoot32BitConstants(RP_CONST, 8, v, 0);
    g_cmd->SetComputeRootDescriptorTable(RP_SRV, Gpu(g_t.blurV.srv));
    g_cmd->SetComputeRootDescriptorTable(RP_UAV, Gpu(g_t.blurV.uav));
    g_cmd->Dispatch((r.w + 7) / 8, (r.h + 7) / 8, 1);
    To(tmp, D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE);
    To(r, D3D12_RESOURCE_STATE_UNORDERED_ACCESS);
    Flush();
    float hz[8] = { (float)r.w, (float)r.h, 0, 0, 1, 0, 0.08f, 0 };  // across it, much narrower
    g_cmd->SetComputeRoot32BitConstants(RP_CONST, 8, hz, 0);
    g_cmd->SetComputeRootDescriptorTable(RP_SRV, Gpu(g_t.blurH.srv));
    g_cmd->SetComputeRootDescriptorTable(RP_UAV, Gpu(g_t.blurH.uav));
    g_cmd->Dispatch((r.w + 7) / 8, (r.h + 7) / 8, 1);
  }
  if (timestamps) g_cmd->EndQuery(pf.queries.Get(), D3D12_QUERY_TYPE_TIMESTAMP, 2);

  // ---- the main pass: sky, floor (raytraced reflections), tubes, heads, 8x MSAA HDR
  To(g_t.irr, D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE);
  To(g_t.refl, D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE);
  To(g_t.msaa, D3D12_RESOURCE_STATE_RENDER_TARGET);
  Flush();
  D3D12_CPU_DESCRIPTOR_HANDLE rtv = g_rtvHeap->GetCPUDescriptorHandleForHeapStart();
  D3D12_CPU_DESCRIPTOR_HANDLE dsv = g_dsvHeap->GetCPUDescriptorHandleForHeapStart();
  float clear[4] = { cb.fog[0], cb.fog[1], cb.fog[2], 1 };
  g_cmd->ClearRenderTargetView(rtv, clear, 0, nullptr);
  g_cmd->ClearDepthStencilView(dsv, D3D12_CLEAR_FLAG_DEPTH, 0.0f, 0, 0, nullptr);
  g_cmd->OMSetRenderTargets(1, &rtv, FALSE, &dsv);
  D3D12_VIEWPORT vp{ 0, 0, (float)w, (float)h, 0, 1 };
  D3D12_RECT sc{ 0, 0, (LONG)w, (LONG)h };
  g_cmd->RSSetViewports(1, &vp); g_cmd->RSSetScissorRects(1, &sc);
  g_cmd->SetGraphicsRootSignature(g_rs.Get());
  g_cmd->SetGraphicsRootConstantBufferView(RP_FRAME, pf.cb->GetGPUVirtualAddress());
  g_cmd->SetGraphicsRootShaderResourceView(RP_TLAS, pf.tlas->GetGPUVirtualAddress());
  g_cmd->SetGraphicsRootShaderResourceView(RP_VERTS, g_vb->GetGPUVirtualAddress());
  g_cmd->SetGraphicsRootShaderResourceView(RP_INDICES, g_ib->GetGPUVirtualAddress());
  g_cmd->SetGraphicsRootShaderResourceView(RP_HEADS, pf.heads->GetGPUVirtualAddress());
  g_cmd->SetGraphicsRootDescriptorTable(RP_SRV, Gpu(g_t.main.srv));
  g_cmd->SetGraphicsRootDescriptorTable(RP_UAV, Gpu(g_t.main.uav));
  g_cmd->IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);

  g_cmd->SetPipelineState(g_psoSky.Get());
  g_cmd->DrawInstanced(3, 1, 0, 0);
  g_cmd->SetPipelineState(g_psoFloor.Get());
  g_cmd->DrawInstanced(6, 1, 0, 0);
  g_cmd->SetPipelineState(g_psoTube.Get());
  g_cmd->IASetVertexBuffers(0, 1, &g_vbv);
  g_cmd->IASetIndexBuffer(&g_ibv);
  for (int k = 0; k < copies; ++k) {
    float c[4] = { offsets[k], 0, 0, 0 };
    g_cmd->SetGraphicsRoot32BitConstants(RP_CONST, 4, c, 0);
    g_cmd->DrawIndexedInstanced((UINT)s.indices.size(), 1, 0, 0, 0);
  }
  if (headCount) {
    g_cmd->SetPipelineState(g_psoHead.Get());
    g_cmd->IASetVertexBuffers(0, 1, &g_sphereVbv);
    g_cmd->IASetIndexBuffer(&g_sphereIbv);
    g_cmd->DrawIndexedInstanced(g_sphereIndexCount, headCount, 0, 0, 0);
  }
  if (g_theme.dustCount) {
    g_cmd->SetPipelineState(g_psoDust.Get());
    g_cmd->DrawInstanced(6, g_theme.dustCount, 0, 0);
  }
  if (timestamps) g_cmd->EndQuery(pf.queries.Get(), D3D12_QUERY_TYPE_TIMESTAMP, 3);

  // ---- resolve
  To(g_t.msaa, D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE);
  To(g_t.hdr, D3D12_RESOURCE_STATE_UNORDERED_ACCESS);
  Flush();
  g_cmd->SetPipelineState(g_psoResolve.Get());
  g_cmd->SetComputeRootDescriptorTable(RP_SRV, Gpu(g_t.resolve.srv));
  g_cmd->SetComputeRootDescriptorTable(RP_UAV, Gpu(g_t.resolve.uav));
  g_cmd->Dispatch((w + 7) / 8, (h + 7) / 8, 1);
  if (timestamps) g_cmd->EndQuery(pf.queries.Get(), D3D12_QUERY_TYPE_TIMESTAMP, 4);

  // ---- bloom: down the chain, then back up
  To(g_t.hdr, D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE | D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE);
  g_cmd->SetPipelineState(g_psoDown.Get());
  for (int k = 0; k < kBloomLevels; ++k) {
    Tex& src = k == 0 ? g_t.hdr : g_t.down[k - 1];
    To(g_t.down[k], D3D12_RESOURCE_STATE_UNORDERED_ACCESS);
    if (k > 0) To(g_t.down[k - 1], D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE);
    Flush();
    float c[8] = { (float)g_t.down[k].w, (float)g_t.down[k].h, 1.0f / src.w, 1.0f / src.h, k == 0 ? 1.0f : 0.0f, 0, 0, 0 };
    g_cmd->SetComputeRoot32BitConstants(RP_CONST, 8, c, 0);
    g_cmd->SetComputeRootDescriptorTable(RP_SRV, Gpu(g_t.downSet[k].srv));
    g_cmd->SetComputeRootDescriptorTable(RP_UAV, Gpu(g_t.downSet[k].uav));
    g_cmd->Dispatch((g_t.down[k].w + 7) / 8, (g_t.down[k].h + 7) / 8, 1);
  }
  To(g_t.down[kBloomLevels - 1], D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE);
  g_cmd->SetPipelineState(g_psoUp.Get());
  for (int k = kBloomLevels - 1; k >= 0; --k) {
    Tex& coarse = k == kBloomLevels - 1 ? g_t.down[k] : g_t.up[k + 1];
    if (k < kBloomLevels - 1) To(g_t.up[k + 1], D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE);
    To(g_t.up[k], D3D12_RESOURCE_STATE_UNORDERED_ACCESS);
    Flush();
    float c[8] = { (float)g_t.up[k].w, (float)g_t.up[k].h, 1.0f / coarse.w, 1.0f / coarse.h, 1.0f,
                   k == kBloomLevels - 1 ? 0.0f : 1.0f, 0, 0 };  // the deepest level has nothing coarser
    g_cmd->SetComputeRoot32BitConstants(RP_CONST, 8, c, 0);
    g_cmd->SetComputeRootDescriptorTable(RP_SRV, Gpu(g_t.upSet[k].srv));
    g_cmd->SetComputeRootDescriptorTable(RP_UAV, Gpu(g_t.upSet[k].uav));
    g_cmd->Dispatch((g_t.up[k].w + 7) / 8, (g_t.up[k].h + 7) / 8, 1);
  }
  if (timestamps) g_cmd->EndQuery(pf.queries.Get(), D3D12_QUERY_TYPE_TIMESTAMP, 5);

  // ---- tonemap into the output
  To(g_t.up[0], D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE);
  Flush();
  g_cmd->OMSetRenderTargets(1, &outRtv, FALSE, nullptr);
  g_cmd->SetGraphicsRootDescriptorTable(RP_SRV, Gpu(g_t.tonemap.srv));
  g_cmd->SetPipelineState(g_psoTonemap.Get());
  g_cmd->DrawInstanced(3, 1, 0, 0);
  if (ui) RecordUi();  // the overlay, onto the same output (timed with the tonemap)
  (void)outTarget;
  if (timestamps) {
    g_cmd->EndQuery(pf.queries.Get(), D3D12_QUERY_TYPE_TIMESTAMP, 6);
    g_cmd->ResolveQueryData(pf.queries.Get(), D3D12_QUERY_TYPE_TIMESTAMP, 0, 7, pf.queryReadback.Get(), 0);
  }
}

// ================================================================ offscreen output: PNG

static uint32_t g_crcTable[256];
static void CrcInit() {
  for (uint32_t n = 0; n < 256; ++n) { uint32_t c = n; for (int k = 0; k < 8; ++k) c = c & 1 ? 0xEDB88320u ^ (c >> 1) : c >> 1; g_crcTable[n] = c; }
}
static uint32_t Crc(const uint8_t* p, size_t n, uint32_t c = 0xFFFFFFFFu) { for (size_t i = 0; i < n; ++i) c = g_crcTable[(c ^ p[i]) & 0xFF] ^ (c >> 8); return c; }

// A PNG with stored (uncompressed) deflate blocks: no compression library, exact pixels.
static bool WritePng(const char* path, const uint8_t* rgba, UINT w, UINT h, UINT pitch) {
  CrcInit();
  std::vector<uint8_t> raw; raw.reserve((size_t)(w * 3 + 1) * h);
  for (UINT y = 0; y < h; ++y) {
    raw.push_back(0);
    const uint8_t* row = rgba + (size_t)y * pitch;
    for (UINT x = 0; x < w; ++x) { raw.push_back(row[x * 4]); raw.push_back(row[x * 4 + 1]); raw.push_back(row[x * 4 + 2]); }
  }
  std::vector<uint8_t> z = { 0x78, 0x01 };
  uint32_t a = 1, b = 0;
  for (uint8_t c : raw) { a = (a + c) % 65521; b = (b + a) % 65521; }
  for (size_t off = 0; off < raw.size();) {
    size_t n = std::min<size_t>(65535, raw.size() - off);
    bool last = off + n == raw.size();
    z.push_back(last ? 1 : 0);
    z.push_back((uint8_t)(n & 0xFF)); z.push_back((uint8_t)(n >> 8));
    z.push_back((uint8_t)(~n & 0xFF)); z.push_back((uint8_t)((~n >> 8) & 0xFF));
    z.insert(z.end(), raw.begin() + off, raw.begin() + off + n);
    off += n;
  }
  uint32_t adler = (b << 16) | a;
  for (int s = 24; s >= 0; s -= 8) z.push_back((uint8_t)(adler >> s));
  FILE* f = fopen(path, "wb");
  if (!f) return false;
  auto be32 = [&](uint32_t v) { uint8_t q[4] = { (uint8_t)(v >> 24), (uint8_t)(v >> 16), (uint8_t)(v >> 8), (uint8_t)v }; fwrite(q, 1, 4, f); };
  auto chunk = [&](const char* type, const std::vector<uint8_t>& data) {
    be32((uint32_t)data.size());
    std::vector<uint8_t> td(type, type + 4); td.insert(td.end(), data.begin(), data.end());
    fwrite(td.data(), 1, td.size(), f);
    be32(Crc(td.data(), td.size()) ^ 0xFFFFFFFFu);
  };
  const uint8_t sig[8] = { 137, 80, 78, 71, 13, 10, 26, 10 };
  fwrite(sig, 1, 8, f);
  std::vector<uint8_t> ihdr = { (uint8_t)(w >> 24), (uint8_t)(w >> 16), (uint8_t)(w >> 8), (uint8_t)w,
                                (uint8_t)(h >> 24), (uint8_t)(h >> 16), (uint8_t)(h >> 8), (uint8_t)h, 8, 2, 0, 0, 0 };
  chunk("IHDR", ihdr); chunk("IDAT", z); chunk("IEND", {});
  bool ok = ferror(f) == 0;
  fclose(f);
  return ok;
}

// Fog follows tempo: about sixteen beats of trail at 120 bpm, more at a slow tempo (per song).
static void UpdateFogBeats() { g_fogBeats = g_theme.fogBeats120 * std::clamp(120.0 / g_song.info.bpm, 0.6, 2.0); }

static void InitRenderer(UINT w, UINT h, bool offscreen) {
  D3D12_DESCRIPTOR_HEAP_DESC hd{ D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV, kHeapSize, D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE, 0 };
  CHECK(g_device->CreateDescriptorHeap(&hd, IID_PPV_ARGS(&g_heap)), "descriptor heap");
  g_heapStride = g_device->GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV);
  g_rtvStride = g_device->GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_RTV);
  CreatePipelines();
  CreateSceneResources();
  CreateTargets(w, h, offscreen);
  UpdateFogBeats();
  printf("render: %ux%u, %ux MSAA, fog %.1f beats\n", w, h, g_msaa, g_fogBeats);
}

static D3D12_CPU_DESCRIPTOR_HANDLE OffscreenRtv() {
  D3D12_CPU_DESCRIPTOR_HANDLE h = Rtv(1);
  g_device->CreateRenderTargetView(g_t.ldr.r.Get(), nullptr, h);
  return h;
}

static View ViewAtSeconds(double secs) {
  View v; double d = g_song.info.duration;
  v.total = secs;
  if (g_streaming) {  // the window's own time: the stream never loops and never ends
    v.songSec = std::clamp(secs - g_song.streamStart, 0.0, std::max(0.0, d - 1e-6));
    return v;
  }
  if (g_song.info.looped && secs >= d) { v.loop = (int)floor(secs / d); v.songSec = fmod(secs, d); }
  else v.songSec = std::clamp(secs, 0.0, std::max(0.0, d - 1e-6));
  return v;
}

// The overlay for a headless frame, built at song time `secs` (defined with the UI).
static void BuildHeadlessUi(double secs, int frames = 3);

// ui: draw the menu overlay too, in the state --capture-menu set up (ApplyCapturePanel).
static bool Capture(double secs, const char* path, bool ui = false) {
  D3D12_CPU_DESCRIPTOR_HANDLE rtv = OffscreenRtv();
  View v = ViewAtSeconds(secs);
  if (ui) BuildHeadlessUi(secs);
  To(g_t.ldr, D3D12_RESOURCE_STATE_RENDER_TARGET);
  RecordFrame(0, v, g_t.ldr.r.Get(), rtv, false, ui);
  To(g_t.ldr, D3D12_RESOURCE_STATE_COPY_SOURCE);
  Flush();
  D3D12_PLACED_SUBRESOURCE_FOOTPRINT fp{}; UINT64 total = 0;
  auto desc = g_t.ldr.r->GetDesc();
  g_device->GetCopyableFootprints(&desc, 0, 1, 0, &fp, nullptr, nullptr, &total);
  auto rb = MakeBuffer(total, D3D12_HEAP_TYPE_READBACK, D3D12_RESOURCE_STATE_COPY_DEST);
  D3D12_TEXTURE_COPY_LOCATION dst{ rb.Get(), D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT }; dst.PlacedFootprint = fp;
  D3D12_TEXTURE_COPY_LOCATION src{ g_t.ldr.r.Get(), D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX }; src.SubresourceIndex = 0;
  g_cmd->CopyTextureRegion(&dst, 0, 0, 0, &src, nullptr);
  CHECK(g_cmd->Close(), "close");
  ID3D12CommandList* l[] = { g_cmd.Get() }; g_queue->ExecuteCommandLists(1, l);
  WaitIdle();
  if (FAILED(g_device->GetDeviceRemovedReason())) Die("device removed during capture", g_device->GetDeviceRemovedReason());
  uint8_t* p = nullptr; CHECK(rb->Map(0, nullptr, (void**)&p), "map readback");
  bool ok = WritePng(path, p + fp.Offset, g_t.w, g_t.h, fp.Footprint.RowPitch);
  rb->Unmap(0, nullptr);
  printf("capture: t = %.3f s (beat %.2f) -> %s%s\n", secs, v.songSec * g_song.info.bpm / 60.0, path, ok ? "" : "  FAILED TO WRITE");
  return ok;
}

// ui: build and draw the menu overlay every frame (--bench-ui), to measure what it costs.
static void Bench(int frames, double startSec, bool ui = false) {
  D3D12_CPU_DESCRIPTOR_HANDLE rtv = OffscreenRtv();
  std::vector<double> gpu, cpu;
  UINT64 fenceOf[kFrames] = {};
  auto t0 = std::chrono::steady_clock::now();
  auto prev = t0;
  for (int i = 0; i < frames + (int)kFrames; ++i) {
    UINT fi = i % kFrames;
    WaitFence(fenceOf[fi]);
    if (i >= (int)kFrames) {
      // Read this slot's timestamps from the frame that used it last.
      uint64_t* ts = nullptr; D3D12_RANGE r{ 0, 7 * 8 };
      CHECK(g_pf[fi].queryReadback->Map(0, &r, (void**)&ts), "map ts");
      gpu.push_back((ts[6] - ts[0]) * 1000.0 / g_tsFreq);
      g_pf[fi].queryReadback->Unmap(0, nullptr);
      auto now = std::chrono::steady_clock::now();
      cpu.push_back(std::chrono::duration<double, std::milli>(now - prev).count());
      prev = now;
    }
    if (i >= frames) { continue; }
    View v = ViewAtSeconds(startSec + i / 240.0);
    if (ui) BuildHeadlessUi(v.total, 1);
    To(g_t.ldr, D3D12_RESOURCE_STATE_RENDER_TARGET);
    RecordFrame(fi, v, g_t.ldr.r.Get(), rtv, true, ui);
    Flush();
    CHECK(g_cmd->Close(), "close");
    ID3D12CommandList* l[] = { g_cmd.Get() }; g_queue->ExecuteCommandLists(1, l);
    fenceOf[fi] = Signal();
  }
  WaitIdle();
  if (FAILED(g_device->GetDeviceRemovedReason())) Die("device removed during bench", g_device->GetDeviceRemovedReason());
  // Per-pass split from the last frame.
  uint64_t* ts = nullptr; D3D12_RANGE r{ 0, 7 * 8 };
  CHECK(g_pf[(frames - 1) % kFrames].queryReadback->Map(0, &r, (void**)&ts), "map ts");
  double passes[6];
  for (int k = 0; k < 6; ++k) passes[k] = (ts[k + 1] - ts[k]) * 1000.0 / g_tsFreq;
  g_pf[(frames - 1) % kFrames].queryReadback->Unmap(0, nullptr);
  auto stats = [](std::vector<double> x, const char* what) {
    std::sort(x.begin(), x.end());
    double sum = 0; for (double q : x) sum += q;
    printf("  %s: avg %.3f ms, p50 %.3f, p99 %.3f, max %.3f (%zu frames)\n", what, sum / x.size(), x[x.size() / 2],
           x[std::min(x.size() - 1, (size_t)(x.size() * 0.99))], x.back(), x.size());
  };
  printf("bench: %ux%u, %ux MSAA, %d frames from t = %.1f s%s\n", g_t.w, g_t.h, g_msaa, frames, startSec,
         ui ? ", with the menu overlay" : "");
  stats(gpu, "GPU frame");
  stats(cpu, "CPU frame (throughput, 2 in flight, no vsync)");
  printf("  last frame by pass: TLAS and floor light %.3f, reflection RT %.3f, main (floor, tubes, heads, dust) %.3f, resolve %.3f, bloom %.3f, tonemap%s %.3f ms\n",
         passes[0], passes[1], passes[2], passes[3], passes[4], ui ? " and overlay" : "", passes[5]);
}


// ================================================================ audio and the clock

static IXAudio2* g_xa = nullptr;
static IXAudio2MasteringVoice* g_master = nullptr;
static IXAudio2SourceVoice* g_src = nullptr;

// The audio engine and its output device, once per run. False (and printed) when there is none;
// the window then plays silently, its clock running on its own.
static bool InitAudioEngine() {
  HRESULT hr = CoInitializeEx(nullptr, COINIT_MULTITHREADED);
  if (FAILED(hr) && hr != RPC_E_CHANGED_MODE) { fprintf(stderr, "audio: CoInitializeEx failed\n"); return false; }
  if (FAILED(XAudio2Create(&g_xa, 0, XAUDIO2_DEFAULT_PROCESSOR))) { fprintf(stderr, "audio: XAudio2Create failed\n"); g_xa = nullptr; return false; }
  if (FAILED(g_xa->CreateMasteringVoice(&g_master))) {
    fprintf(stderr, "audio: no output device\n");
    g_master = nullptr; g_xa->Release(); g_xa = nullptr;
    return false;
  }
  return true;
}
static void DestroyVoice() { if (g_src) { g_src->Stop(); g_src->DestroyVoice(); g_src = nullptr; } }

// A source voice playing the song from `total` seconds (on a looped song, from total mod its
// length): the rest of the song, then, when `repeat`, the whole song looped forever; otherwise the
// stream ends there. A new voice counts SamplesPlayed from 0, so the position is always the seek
// point plus what this voice has played: seeking is a new voice.
static bool SubmitVoiceAt(double total, bool repeat, float volume, bool start) {
  DestroyVoice();
  if (!g_xa || !g_song.lm) return false;
  const LmInfo& i = g_song.info;
  WAVEFORMATEX fmt{}; fmt.wFormatTag = WAVE_FORMAT_IEEE_FLOAT; fmt.nChannels = 2; fmt.nSamplesPerSec = i.rate;
  fmt.wBitsPerSample = 32; fmt.nBlockAlign = 8; fmt.nAvgBytesPerSec = i.rate * 8;
  if (FAILED(g_xa->CreateSourceVoice(&g_src, &fmt))) { fprintf(stderr, "audio: CreateSourceVoice failed\n"); g_src = nullptr; return false; }
  double songSec = i.looped ? fmod(std::max(total, 0.0), i.duration) : std::clamp(total, 0.0, i.duration);
  uint64_t from = std::min<uint64_t>((uint64_t)(songSec * i.rate), i.frames - 1);
  XAUDIO2_BUFFER rest{};
  rest.AudioBytes = (UINT32)(i.frames * 8);
  rest.pAudioData = (const BYTE*)lm_audio(g_song.lm);
  rest.PlayBegin = (UINT32)from;
  rest.PlayLength = (UINT32)(i.frames - from);
  if (!repeat) rest.Flags = XAUDIO2_END_OF_STREAM;
  bool ok = SUCCEEDED(g_src->SubmitSourceBuffer(&rest));
  if (ok && repeat) {
    XAUDIO2_BUFFER whole{};
    whole.AudioBytes = rest.AudioBytes; whole.pAudioData = rest.pAudioData; whole.LoopCount = XAUDIO2_LOOP_INFINITE;
    ok = SUCCEEDED(g_src->SubmitSourceBuffer(&whole));
  }
  if (!ok) { fprintf(stderr, "audio: SubmitSourceBuffer failed\n"); DestroyVoice(); return false; }
  g_src->SetVolume(volume);
  if (start) g_src->Start(0);
  return true;
}

static bool StartAudio(float volume) {
  if (!InitAudioEngine()) return false;
  return SubmitVoiceAt(0, g_song.info.looped != 0, volume, true);
}
static void StopAudio() {
  DestroyVoice();
  if (g_master) { g_master->DestroyVoice(); g_master = nullptr; }
  if (g_xa) { g_xa->Release(); g_xa = nullptr; }
}
// Seconds of audio the current voice has consumed since it started (counts across loops).
// The rate the stream's voice plays at (0 when no stream plays): in stream mode the drawn song is only a picture of
// the stream (a window, or the whole song's trail rendered at another rate), so the clock is the stream's own.
static uint32_t g_streamVoiceRate = 0;
static double AudioSeconds() {
  if (!g_src) return 0;
  XAUDIO2_VOICE_STATE st{}; g_src->GetState(&st, 0);
  return (double)st.SamplesPlayed / (g_streamVoiceRate ? g_streamVoiceRate : g_song.info.rate);
}

// The visual clock: steady between the audio engine's coarse position updates (10-20 ms steps), and
// pulled gently toward the audio position so it never drifts (gain 1/s: a correction of a few
// milliseconds changes the speed by well under one percent, which the eye does not see as judder).
// The audio is the master; the frame jumps only if the two disagree by more than 60 ms (a stall).
struct Clock {
  double vis = 0;
  std::chrono::steady_clock::time_point last;
  bool started = false;
  double Tick(double audio) {
    auto now = std::chrono::steady_clock::now();
    if (!started) { started = true; last = now; vis = audio; return vis; }
    double dt = std::chrono::duration<double>(now - last).count();
    last = now;
    vis += dt;
    double err = audio - vis;
    if (fabs(err) > 0.06) vis = audio;
    else vis += err * std::min(1.0, dt * 1.0);
    return vis;
  }
};

// Silent test of the clock against the audio engine (volume 0): how far the smoothed clock strays
// from the audio position, and how coarse the audio position is.
static int AudioClockTest(double seconds) {
  if (!StartAudio(0.0f)) return 1;
  Clock c;
  double maxErr = 0, lastAudio = -1, maxStep = 0; int steps = 0;
  auto t0 = std::chrono::steady_clock::now();
  double prevVis = -1, minDelta = 1e9, maxSpeedErr = 0;
  auto prevWall = t0;
  while (std::chrono::duration<double>(std::chrono::steady_clock::now() - t0).count() < seconds) {
    double a = AudioSeconds();
    double v = c.Tick(a);
    auto wall = std::chrono::steady_clock::now();
    double dWall = std::chrono::duration<double>(wall - prevWall).count();
    prevWall = wall;
    if (a != lastAudio) { if (lastAudio >= 0) { maxStep = std::max(maxStep, a - lastAudio); ++steps; } lastAudio = a; }
    if (std::chrono::duration<double>(wall - t0).count() > 1.0) {  // past the start-up snap
      maxErr = std::max(maxErr, fabs(v - a));
      if (prevVis >= 0) {
        minDelta = std::min(minDelta, v - prevVis);
        if (dWall > 0) maxSpeedErr = std::max(maxSpeedErr, fabs((v - prevVis) / dWall - 1.0));
      }
    }
    prevVis = v;
    Sleep(3);  // roughly a 300 Hz frame
  }
  StopAudio();
  printf("audio clock (silent, %.1f s): the audio position advanced in %d steps, the largest %.1f ms; the visual clock stayed "
         "within %.2f ms of it, its speed within %.2f%% of real time, never backwards: %s\n",
         seconds, steps, maxStep * 1000, maxErr * 1000, maxSpeedErr * 100, minDelta >= 0 ? "yes" : "NO");
  return minDelta >= 0 ? 0 : 1;
}

// ================================================================ the player (transport)
//
// The position is "total" seconds: continuous across a looped song's wraps (ViewAtSeconds turns it
// into the song position and the loop count), so the previous loop is drawn behind the present as
// it sounded. A seek stays within the current loop, so the trail behind the new position is the
// song's own record up to there (the whole song is rendered before it plays).

struct Player {
  bool playing = false;
  bool loop = true;     // at the end, repeat the song (off: go on to the library's next song)
  float volume = 1.0f;
  double base = 0;      // the position where the current voice (or silent run) began
  double silent = 0;    // with no audio device: seconds played since base
  bool ended = false;   // held at the end (loop off), waiting for the next song or a play
  Clock clock;
  double now = 0;       // the position drawn this frame
} g_play;

static bool HaveSong() { return g_song.lm != nullptr; }
static bool Repeats() { return g_song.info.looped && g_play.loop; }
static double PlayerAudio() { return g_play.base + (g_src ? AudioSeconds() : g_play.silent); }
static double SongSecondsNow() { return ViewAtSeconds(g_play.now).songSec; }

static void PlayerSeekTotal(double total) {
  if (g_streaming) return;  // a stream has no seeking: the future is not written yet
  g_play.base = std::max(total, 0.0);
  g_play.silent = 0;
  g_play.ended = false;
  g_play.clock = Clock{};
  if (HaveSong()) SubmitVoiceAt(g_play.base, Repeats(), g_play.volume, g_play.playing);
  g_play.now = g_play.base;
}
// To a position in the song (seconds from its start), clamped into it, within the current loop.
static void PlayerSeekSong(double songSec) {
  if (!HaveSong() || g_streaming) return;
  double d = g_song.info.duration;
  double loops = g_song.info.looped ? floor(std::max(g_play.now, 0.0) / d) : 0.0;
  PlayerSeekTotal(loops * d + ui::ClampSeek(songSec, d));
}
static void PlayerSetPlaying(bool p) {
  if (!HaveSong()) { g_play.playing = false; return; }
  if (p && g_play.ended) { g_play.playing = true; PlayerSeekTotal(0); return; }  // play at the end: from the top
  g_play.playing = p;
  if (g_src) { if (p) g_src->Start(0); else g_src->Stop(); }
}
static void PlayerSetVolume(float v) {
  g_play.volume = std::clamp(std::isfinite(v) ? v : 1.0f, 0.0f, 1.0f);
  if (g_src) g_src->SetVolume(g_play.volume);
}
// Loop on/off. A looped song's voice is resubmitted where it is (with or without the endless loop
// behind it); for a song that is not a loop, the end decides (OnSongEnd).
static void PlayerToggleLoop() {
  if (g_streaming) return;  // endless already
  g_play.loop = !g_play.loop;
  if (HaveSong() && g_song.info.looped && !g_play.ended) PlayerSeekTotal(g_play.now);
}

static void RequestSong(int index);
static int NextAfterEnd();

// The song reached its end and does not repeat: a song that is not a loop, with loop on, starts
// again; with loop off it holds at the end and the library's next song is rendered and played.
static bool ReplayOnEnd();  // a replay from the history takes its next stretch here (below)
static void OnSongEnd() {
  if (ReplayOnEnd()) return;
  if (g_play.loop) { PlayerSeekTotal(0); return; }
  // Hold at the end. XAudio2 resets SamplesPlayed when a stream ends, so the voice can no longer
  // say where it is: the position becomes the song's end itself, and the voice goes.
  double d = g_song.info.duration;
  double end = g_song.info.looped ? (floor(g_play.base / d) + 1) * d : d;
  DestroyVoice();
  g_play.base = end - 1e-6;  // just inside the loop that ended, so its trail stays drawn
  g_play.silent = 0;
  g_play.playing = false;
  g_play.ended = true;
  int next = NextAfterEnd();
  if (next >= 0) RequestSong(next);
}

// Advance the player by one frame (dt seconds of wall time) and return the position to draw.
static double PlayerFrameTime(double dt) {
  if (!HaveSong()) { g_play.now += dt * 0.25; return g_play.now; }  // the empty void drifts slowly
  if (g_play.playing && !g_src) g_play.silent += dt;
  if (g_play.playing && !Repeats() && !g_streaming) {
    bool ended;
    if (g_src) { XAUDIO2_VOICE_STATE st{}; g_src->GetState(&st, XAUDIO2_VOICE_NOSAMPLESPLAYED); ended = st.BuffersQueued == 0; }
    else {
      double d = g_song.info.duration;
      double end = g_song.info.looped ? (floor(g_play.base / d) + 1) * d : d;
      ended = PlayerAudio() >= end;
    }
    if (ended) OnSongEnd();
  }
  double audio = PlayerAudio();
  g_play.now = g_play.playing ? g_play.clock.Tick(audio) : audio;
  return g_play.now;
}

// ================================================================ the library and the loader

struct Library {
  std::vector<std::string> dirs;      // examples\ and every --songs folder
  std::vector<ui::SongEntry> songs;
  std::vector<std::string> errors;    // folders or files that could not be read
  int current = -1;                   // the song loaded now (-1: none, or one given outside the library)
  std::string currentTitle, currentDescription;
  FolderWatch watch;                  // the folders, so a song saved while Dreamride runs shows up
  ui::RescanClock rescan;
} g_lib;

// What a stream starts from (the CLI sets it; the library's Stream entry and R use it).
struct StreamConfig {
  uint64_t seed = 1;
  uint32_t flags = 0;          // LM_STREAM_GRAMMAR: stage 1 from the grammar
  std::string seedText;        // a seed song's text (empty: the engine's default, the kept squares song)
  bool fresh = true;           // a new grammar seed from the clock at each start (no --seed, no --stream-song)
  bool explore = false;        // the Explore entry: a fresh seed takes LM_STREAM_EXPLORE (a species of strange tunings)
};

// One song renders at a time, on its own thread, while the current one keeps playing. A song asked
// for meanwhile waits its turn (the latest ask wins); when the render finishes the main thread
// swaps it in between frames.
struct Loader {
  std::thread thread;
  std::atomic<bool> done{ false };
  bool busy = false;
  std::string path, title;
  int index = -1;
  std::chrono::steady_clock::time_point start;
  std::unique_ptr<Song> song;  // the result: written by the thread before it sets `done`
  LmStream* stream = nullptr;  // with it, for the library's Stream entry: the stream its window came from
  StreamConfig streamUsed;     // and what started it (the seed drawn), for the history
  std::string error;
  bool queued = false;
  std::string qPath, qTitle;
  int qIndex = -1;
  int failedIndex = -2;        // the last song that would not load (-2: none; -1: one outside the library)
  std::string failedTitle, failedError;
} g_loader;

static std::string StemOf(const std::string& path) { return std::filesystem::u8path(path).stem().u8string(); }

static StreamConfig g_streamCfg;
static uint64_t g_streamReplay = 0;  // R in stream mode: the next "Stream" start replays this seed instead of a fresh one
static bool g_streamNextFresh = false;  // the next "Stream" start is a new song even after --seed (the auto-next song)

// The config a library entry starts: "Stream from G" is always G with seed 1; "Stream" is the command line's, or a
// replay of the seed playing (R); "Explore (strange tunings)" a fresh seed among the strange tunings, or its replay.
static StreamConfig StreamConfigFor(const std::string& path) {
  if (path == ui::kStreamGPath) { StreamConfig g; g.fresh = false; return g; }
  const bool explore = path == ui::kStreamExplorePath;
  StreamConfig c = explore ? StreamConfig{} : g_streamCfg;
  c.explore = explore;
  const uint32_t grammar = LM_STREAM_GRAMMAR | (explore ? (uint32_t)LM_STREAM_EXPLORE : 0u);
  if (explore) c.flags = grammar;
  if (g_streamReplay) { c.seed = g_streamReplay; c.flags = grammar; c.seedText.clear(); c.fresh = false; g_streamReplay = 0; }
  if (g_streamNextFresh) { c.fresh = true; g_streamNextFresh = false; }
  return c;
}
static LmStream* StartStreamNow(const StreamConfig& c, std::string& error, StreamConfig* used = nullptr);
static bool LoadStreamWindowInto(Song& s, LmStream* st, uint64_t first, uint64_t last, const Theme& th, std::string& error);
static void SwapInStream(LmStream* st, std::unique_ptr<Song> s, int index, const StreamConfig& cfg, uint64_t startStage = 0,
                         const std::shared_ptr<hist::Record>& attach = nullptr, const std::string& title = "");
static void StopStream();
static void ReplayCancel();

static void StartLoad(const std::string& path, const std::string& title, int index) {
  Loader& L = g_loader;
  ReplayCancel();  // anything asked for from the library ends a replay from the history
  if (L.busy) { L.queued = true; L.qPath = path; L.qTitle = title; L.qIndex = index; return; }
  L.busy = true; L.done = false;
  L.path = path; L.title = title; L.index = index;
  L.start = std::chrono::steady_clock::now();
  L.song.reset(); L.error.clear(); L.stream = nullptr;
  Theme th = g_theme;
  StreamConfig cfg = ui::IsStreamPath(path) ? StreamConfigFor(path) : g_streamCfg;
  L.thread = std::thread([path, th, cfg]() {
    auto s = std::make_unique<Song>();
    std::string err;
    bool ok;
    if (ui::IsStreamPath(path)) {  // a Stream entry: compose and render stage 1, then its window
      LmStream* st = StartStreamNow(cfg, err, &g_loader.streamUsed);
      ok = st && LoadStreamWindowInto(*s, st, 0, 0, th, err);
      if (!ok && st) lm_stream_free(st);
      g_loader.stream = ok ? st : nullptr;
    } else if (ui::IsSeedPath(path)) {  // a seed file: the stream it names, at its generator version, from stage 1
      char e[1024] = {};
      LmStream* st = lm_stream_start_seed_file(path.c_str(), 1, 0, e, sizeof(e));
      if (!st) err = e;
      if (st) {
        LmStreamInfo i{}; lm_stream_info(st, &i);
        StreamConfig used; used.seed = i.seed; used.flags = LM_STREAM_GRAMMAR; used.fresh = false;
        g_loader.streamUsed = used;
        printf("stream: seed file %s: seed %llu at generator version %u\n", path.c_str(), (unsigned long long)i.seed, lm_stream_version(st));
      }
      ok = st && LoadStreamWindowInto(*s, st, 0, 0, th, err);
      if (!ok && st) lm_stream_free(st);
      g_loader.stream = ok ? st : nullptr;
    } else {
      ok = LoadSongInto(*s, path.c_str(), th, err);
    }
    g_loader.song = ok ? std::move(s) : nullptr;
    g_loader.error = ok ? std::string() : (err.empty() ? std::string("unknown error") : err);
    g_loader.done.store(true, std::memory_order_release);
  });
}

static void RequestSong(int index) {
  if (index < 0 || index >= (int)g_lib.songs.size()) return;
  if (g_loader.busy) {
    if (g_loader.index == index) { g_loader.queued = false; return; }  // already rendering: drop any later ask
    if (g_loader.queued && g_loader.qIndex == index) return;
  } else if (index == g_lib.current && HaveSong() && !g_lib.songs[index].isNew) {
    PlayerSetPlaying(true);  // the song already loaded: play it, no second render (unless its file changed)
    return;
  }
  StartLoad(g_lib.songs[index].path, g_lib.songs[index].title, index);
}

// The library index the next/previous keys step from: the song on its way in, else the current one.
static int LibraryIndexNow() {
  if (g_loader.busy) return g_loader.queued ? g_loader.qIndex : g_loader.index;
  return g_lib.current;
}
static int NextAfterEnd() {
  int n = (int)g_lib.songs.size();
  if (g_lib.current < 0 || n < 2) return -1;
  return ui::StepIndex(g_lib.current, n, +1);
}

static void SetWindowTitleFor(const std::string& title);

// Between frames, on the main thread: the GPU and the audio let go of the old song, then the new
// one's geometry goes up and it plays from the start.
static void SwapInSong(std::unique_ptr<Song> s, int index, const std::string& title, const std::string& path) {
  StopStream();
  WaitIdle();
  DestroyVoice();
  if (g_song.lm) lm_free(g_song.lm);
  g_song = std::move(*s);
  CreateSongResources();
  UpdateFogBeats();
  g_lib.current = index >= 0 ? index : ui::FindSong(g_lib.songs, path);
  g_lib.currentTitle = title;
  g_lib.currentDescription = g_lib.current >= 0 ? g_lib.songs[g_lib.current].description : std::string();
  if (g_lib.current >= 0) g_lib.songs[g_lib.current].isNew = false;  // played: no longer new
  g_play.playing = true;
  PlayerSeekTotal(0);
  SetWindowTitleFor(title);
}

static void PollLoader() {
  Loader& L = g_loader;
  if (!L.busy || !L.done.load(std::memory_order_acquire)) return;
  L.thread.join();
  L.busy = false; L.done = false;
  std::unique_ptr<Song> s = std::move(L.song);
  if (L.queued) {
    L.queued = false;
    if (L.qPath != L.path) {  // a later ask wins: this render is dropped
      if (s && s->lm) lm_free(s->lm);
      if (L.stream) { lm_stream_free(L.stream); L.stream = nullptr; }
      StartLoad(L.qPath, L.qTitle, L.qIndex);
      return;
    }
  }
  if (s && L.stream) { L.failedIndex = -2; LmStream* st = L.stream; L.stream = nullptr; SwapInStream(st, std::move(s), L.index, L.streamUsed); }
  else if (s) { L.failedIndex = -2; SwapInSong(std::move(s), L.index, L.title, L.path); }
  else { L.failedIndex = L.index; L.failedTitle = L.title; L.failedError = L.error; }
}

// The library refreshing itself: a song saved, added, renamed or deleted in a song folder while
// Dreamride runs shows up in the Library (marked new) without a restart. The folders are watched with
// change notifications, polled without waiting each frame. A moment after the last change, the
// folders are scanned on a thread (it reads the *.txt heads: about 1-10 ms for ten songs, which is
// a frame), and the main thread merges the result between frames. Every index into the list
// follows its song.
struct LibraryScan {
  std::thread thread;
  std::atomic<bool> done{ false };
  bool busy = false;
  std::vector<ui::SongEntry> songs;  // written by the thread before it sets `done`
  std::vector<std::string> errors;
} g_scan;

static void ApplyRescan(std::vector<ui::SongEntry> fresh, const std::vector<std::string>& errors) {
  ui::Rescan r = ui::MergeRescan(g_lib.songs, std::move(fresh));
  if (!r.changed) return;
  auto remap = [&](int i) { return ui::RemapIndex(g_lib.songs, i, r.songs); };
  g_lib.current = remap(g_lib.current);
  if (g_loader.index >= 0) g_loader.index = remap(g_loader.index);
  if (g_loader.qIndex >= 0) g_loader.qIndex = remap(g_loader.qIndex);
  if (g_loader.failedIndex >= 0) {
    int f = remap(g_loader.failedIndex);
    g_loader.failedIndex = (f < 0 || r.songs[f].isNew) ? -2 : f;  // gone, or rewritten: the old error no longer applies
  }
  g_lib.songs = std::move(r.songs);
  if (g_lib.current >= 0) g_lib.currentDescription = g_lib.songs[g_lib.current].description;
  for (auto& e : errors) fprintf(stderr, "library: %s\n", e.c_str());
}

static void PollLibrary(double now) {
  if (g_lib.watch.Changed()) g_lib.rescan.Signal(now);
  if (g_scan.busy && g_scan.done.load(std::memory_order_acquire)) {
    g_scan.thread.join();
    g_scan.busy = false;
    ApplyRescan(std::move(g_scan.songs), g_scan.errors);
  }
  // A change during a scan stays due, and is scanned when this one is in.
  if (!g_scan.busy && g_lib.rescan.Due(now)) {
    g_scan.busy = true;
    g_scan.done = false;
    g_scan.songs.clear();
    g_scan.errors.clear();
    g_scan.thread = std::thread([dirs = g_lib.dirs]() {
      g_scan.songs = ui::WithStream(ui::ScanSongs(dirs, &g_scan.errors));
      g_scan.done.store(true, std::memory_order_release);
    });
  }
}

static void ShutdownLibraryScan() {
  if (g_scan.thread.joinable()) g_scan.thread.join();
  g_scan.busy = false;
}

static void ShutdownLoader() {
  Loader& L = g_loader;
  if (L.thread.joinable()) L.thread.join();
  if (L.song && L.song->lm) lm_free(L.song->lm);
  L.song.reset();
  if (L.stream) { lm_stream_free(L.stream); L.stream = nullptr; }
  L.busy = false;
}

// ================================================================ the stream history (world/src/history.h)
//
// Every stream that starts is recorded as it is composed, so nothing is lost: one plain-text file per stream in
// %APPDATA%\Dreamride\history (or --history-dir), with the seed, where it came from, the composer's version and every
// stage's song text. Each frame the main thread only copies a newly composed stage's strings (a few kB, once per
// stage) and posts them; the worker thread does every file operation. A write that fails is shown on screen until one
// succeeds, and the next write puts the whole record down, so a failure loses nothing that is still in memory.

struct HistoryState {
  std::string dir;                    // the history folder (UTF-8)
  bool record = true;                 // record streams (headless tests and captures do not, unless --history-dir)
  std::unique_ptr<hist::Worker> worker;
  std::string version;                // this engine's composer version (set by the worker at start)
  std::string writeError;             // shown on screen until a write succeeds
  std::string toast; bool toastBad = false; double toastUntil = 0;
  // The list (the History tab).
  std::vector<hist::Summary> list;
  std::vector<std::string> listErrors;
  size_t deleted = 0;
  bool scanBusy = false, scanAgain = false;
  // The recording now (a stream playing).
  std::shared_ptr<hist::Session> live;
  LmStream* liveStream = nullptr;
  std::string livePath;               // once the worker has written it
  uint64_t nextStage = 0, heardStages = 0, stagesRecorded = 0;
  double heardSeconds = 0;
  // The tab.
  std::string open;                   // the entry shown open (its path)
  int from = 1, to = 1;               // its range, stages from 1
  std::string confirmDelete;          // the entry asking "Delete?"
  bool confirmEmpty = false;
  std::string undoFrom, undoTo;       // the last delete: where it went, where it was
} g_hist;

static std::string g_histVersionW;    // the worker's copy of the composer's version (only its jobs read it)

static double HistNow() {
  return std::chrono::duration<double>(std::chrono::steady_clock::now().time_since_epoch()).count();
}
static void HistToast(const std::string& text, bool bad) {
  g_hist.toast = text; g_hist.toastBad = bad; g_hist.toastUntil = HistNow() + (bad ? 8.0 : 4.0);
  fprintf(bad ? stderr : stdout, "history: %s\n", text.c_str());
}

// The history folder: %APPDATA%\Dreamride\history. Empty with `err` when there is no APPDATA.
static std::string DefaultHistoryDir(std::string& err) {
  const wchar_t* app = _wgetenv(L"APPDATA");
  if (!app || !*app) { err = "APPDATA is not set, so there is nowhere to keep the history"; return {}; }
  return (std::filesystem::path(app) / L"Dreamride" / L"history").u8string();
}

static void HistScan();

// Starts the worker: it first reads the composer's version (a hash of luminesce.dll beside the exe), then lists.
static void HistInit(const std::string& dir, bool record) {
  g_hist.dir = dir;
  g_hist.record = record && !dir.empty();
  g_hist.worker = std::make_unique<hist::Worker>();
  std::filesystem::path dll = std::filesystem::path(g_exeDir) / L"luminesce.dll";
  g_hist.worker->Post([dll] {
    hist::Worker::Done d;
    std::string err;
    std::string v = hist::ComposerVersionOf(dll, err);
    g_histVersionW = v.empty() ? std::string("unknown") : v;
    if (v.empty()) d.error = err + " (recordings will say the composer's version is unknown)";
    d.onMain = [v] { g_hist.version = v; };
    return d;
  });
  if (!dir.empty()) HistScan();
}

// Rescan the folder on the worker (the tab shows the old list meanwhile).
static void HistScan() {
  if (!g_hist.worker || g_hist.dir.empty()) return;
  if (g_hist.scanBusy) { g_hist.scanAgain = true; return; }
  g_hist.scanBusy = true;
  std::string dir = g_hist.dir;
  g_hist.worker->Post([dir] {
    auto scan = std::make_shared<hist::Scan>(hist::ScanDir(dir));
    hist::Worker::Done d;
    d.onMain = [scan] {
      g_hist.list = std::move(scan->entries);
      g_hist.listErrors = std::move(scan->errors);
      g_hist.deleted = scan->deleted;
      g_hist.scanBusy = false;
      if (g_hist.scanAgain) { g_hist.scanAgain = false; HistScan(); }
    };
    return d;
  });
}

// Each frame (and when quitting): what the worker has finished.
static void PollHistory() {
  if (!g_hist.worker) return;
  for (auto& d : g_hist.worker->Take()) {
    if (d.onMain) d.onMain();
    if (!d.error.empty()) {
      if (d.sessionWrite) g_hist.writeError = d.error;
      HistToast(d.error, true);
    } else if (d.sessionWrite) {
      g_hist.writeError.clear();
    }
  }
}

// Posts a change to the recording: `apply` changes the worker's record, `chunk` is what it appends to the file.
static void HistPost(const std::shared_ptr<hist::Session>& s, std::function<void(hist::Record&)> apply, std::string chunk, bool rescan = false) {
  if (!s || !g_hist.worker) return;
  g_hist.worker->Post([s, apply, chunk, rescan] {
    if (apply) apply(s->rec);
    if (s->rec.composer.empty()) s->rec.composer = g_histVersionW;
    hist::Worker::Done d;
    d.sessionWrite = true;
    bool first = !s->written;  // a whole-record write: a new file in the list (or one recovered)
    d.error = hist::Sync(*s, chunk);
    if (d.error.empty()) {
      std::string path = s->rec.path;
      d.onMain = [path, first, rescan] {
        if (g_hist.live && g_hist.livePath != path) g_hist.livePath = path;
        if (first || rescan) HistScan();
      };
    } else {
      d.error = "History is not being saved: " + d.error;
    }
    return d;
  });
}

static std::string OriginOf(const StreamConfig& c, uint64_t seed) {
  if (c.flags & LM_STREAM_GRAMMAR) return "grammar seed " + std::to_string(seed);
  if (c.seedText.empty()) return "G, the kept squares song";
  std::string d = ui::DescriptionOf(c.seedText);
  return d.empty() ? std::string("a given song") : "a given song (" + d + ")";
}

// A stream has started playing: record it, at once (its first stage is composed before it starts). `attach` continues
// a recording already on disk (a replay that has run past its recorded stages) instead of starting a new one.
static void HistBegin(LmStream* st, const StreamConfig& cfg, const std::shared_ptr<hist::Record>& attach = nullptr) {
  g_hist.live.reset();
  g_hist.liveStream = nullptr;
  if (!g_hist.record || !g_hist.worker || !st) return;
  auto s = std::make_shared<hist::Session>();
  s->dir = g_hist.dir;
  if (attach) {
    s->rec = *attach;
    s->written = true;  // it is on disk; appends go on from here
    g_hist.livePath = attach->path;
    g_hist.nextStage = attach->stages.size();
    g_hist.heardStages = attach->heardStages;
    g_hist.heardSeconds = attach->heardSeconds;
  } else {
    LmStreamInfo i{}; lm_stream_info(st, &i);
    hist::Record& r = s->rec;
    hist::Now(r.started, r.startedUnix);
    r.seed = i.seed;
    r.flags = cfg.flags;
    r.origin = OriginOf(cfg, i.seed);
    r.composer = g_hist.version;
    r.bpm = i.bpm;
    r.stageBeats = i.stage_beats;
    for (uint32_t k = 0; k < i.roster_count; ++k) { const char* n = lm_stream_roster(st, k); r.roster.push_back(n ? n : "?"); }
    r.seedSong = (cfg.flags & LM_STREAM_GRAMMAR) ? std::string() : cfg.seedText;
    g_hist.livePath.clear();
    g_hist.nextStage = 0;
    g_hist.heardStages = 0;
    g_hist.heardSeconds = 0;
  }
  g_hist.stagesRecorded = g_hist.nextStage;
  g_hist.live = s;
  g_hist.liveStream = st;
  if (!attach) HistPost(s, nullptr, "");  // the header goes down now, before any stage
}

// Each frame while a stream plays: record every stage composed since, and how far it has been heard.
static void HistPoll(double now, uint64_t endFrame = UINT64_MAX) {
  if (!g_hist.live || !g_hist.liveStream) return;
  LmStream* st = g_hist.liveStream;
  LmStreamInfo i{}; lm_stream_info(st, &i);
  while (g_hist.nextStage < i.stages_composed) {
    LmStageInfo si{};
    if (!lm_stream_stage(st, g_hist.nextStage, &si)) break;
    hist::Stage s;
    s.index = si.index;
    s.turn = si.turn != 0;
    s.summary = si.summary ? si.summary : "";
    s.text = hist::NormalizeText(si.text ? si.text : "");
    std::string chunk = hist::StageText(s);
    HistPost(g_hist.live, [s](hist::Record& r) { r.stages.push_back(s); }, chunk);
    ++g_hist.nextStage;
    g_hist.stagesRecorded = g_hist.nextStage;
  }
  uint64_t frame = std::min<uint64_t>((uint64_t)(std::max(now, 0.0) * i.rate), endFrame > 0 ? endFrame - 1 : 0);  // not past an ending
  uint64_t k = lm_stream_stage_at(st, frame);
  if (k + 1 > g_hist.heardStages) {  // once per stage reached
    g_hist.heardStages = k + 1;
    g_hist.heardSeconds = now;
    uint64_t n = g_hist.heardStages; double secs = now;
    HistPost(g_hist.live, [n, secs](hist::Record& r) { r.heardStages = n; r.heardSeconds = secs; }, hist::HeardText(n, secs));
  }
}

// The stream has ended on its own: its entry becomes a finished song, with its length.
static void HistComplete(double seconds, uint64_t stages) {
  if (!g_hist.live) return;
  uint64_t n = stages;
  HistPost(g_hist.live, [n, seconds](hist::Record& r) { r.complete = true; r.completeStages = n; r.completeSeconds = seconds; },
           hist::CompleteText(n, seconds), true);
}

// The stream stops (another song, a replay, quitting): the last word on how far it was heard.
static void HistEnd(double now) {
  if (!g_hist.live) return;
  if (now > g_hist.heardSeconds) {
    uint64_t n = g_hist.heardStages; double secs = now;
    HistPost(g_hist.live, [n, secs](hist::Record& r) { r.heardStages = n; r.heardSeconds = secs; }, hist::HeardText(n, secs), true);
  } else {
    HistScan();
  }
  g_hist.live.reset();
  g_hist.liveStream = nullptr;
}

// ================================================================ stream mode
//
// The engine composes and renders the stream ahead of the playhead (src/stream.rs). Here: a feeder thread pulls its
// audio into XAudio2 in tenth-of-a-second buffers (so the voice plays one long stream, and its SamplesPlayed is the
// stream's own position); the song the world draws is a window over the stage playing and the one before it (its
// trail), built on a thread while the stage before plays and swapped in at the seam; and the playing stage's
// equation is typeset over the void.

struct StreamMode {
  LmStream* s = nullptr;
  LmStreamInfo info{};
  uint64_t windowStage = 0;          // the stage the drawn window ends at (the present's stage)
  // The next window, (k, k + 1), built while stage k plays.
  std::thread prep;
  std::atomic<bool> prepDone{ false };
  bool prepBusy = false;
  uint64_t prepStage = 0;
  std::unique_ptr<Song> prepSong;
  std::string prepError;
  // Audio.
  std::thread feeder;
  std::atomic<bool> feederStop{ false };
  std::atomic<double> silentPos{ 0.0 };  // with no audio device: the position, so the feeder pulls only as fast as it plays
  std::vector<std::vector<float>> blocks;
  // The overlay.
  mathui::EqMode eqMode = mathui::EqMode::Compact;  // E cycles compact, full, hidden
  bool fromG = false;                              // started from G ("Stream from G"), not a grammar seed
  float eqAlpha = 0;
  std::string toast; bool toastBad = false; double toastUntil = 0;
  StreamConfig cfg;                                // what started it (its seed drawn), for the history
  // The ending (a stream that ends on its own): the last notes ring out, the camera lifts back over the trail and
  // holds, the history marks a finished song, and after a breath the next song starts (View: auto-next).
  enum class End { None, RingOut, Hold, Breath, Waiting } end = End::None;
  std::atomic<uint64_t> endFrame{ UINT64_MAX };    // the song's length in frames, its ring-out included (the engine's)
  std::atomic<uint64_t> stopFrame{ UINT64_MAX };   // the feeder pulls nothing past this (the end)
  double endSec = 0, phaseAt = 0;
  double ringSec = 0;                              // where the last stage (the ring-out) starts: the pull-back begins
  bool wideBefore = false;
  // The whole song's trail for the pull-back (every stage merged, as a replay merges them), built on a thread once the
  // end is known (it takes about 20 s), and swapped in as the pull-back begins, or when it is in.
  struct WholeJob {
    std::atomic<bool> done{ false };
    std::unique_ptr<Song> song;
    std::string err;
    ~WholeJob() { if (song && song->lm) lm_free(song->lm); }
  };
  std::shared_ptr<WholeJob> whole;   // the job (its thread holds it too: a stopped stream never waits for it)
  bool wholeShown = false;
} g_stream;
// The whole-trail builds' threads: joined once done (each frame), and all of them on the way out.
static struct WholeThreads {
  std::vector<std::pair<std::thread, std::shared_ptr<StreamMode::WholeJob>>> v;
  void Join(bool all) {
    for (size_t i = 0; i < v.size();) {
      if (all || v[i].second->done.load(std::memory_order_acquire)) { v[i].first.join(); v.erase(v.begin() + i); }
      else ++i;
    }
  }
  ~WholeThreads() { Join(true); }
} g_wholeThreads;
static float g_camSpanTarget = 0;  // where g_camSpan eases to (the whole song's trail, once it is in)

// How long each part of the ending lasts, seconds.
static const double kEndHold = 4.0, kEndBreath = 3.0;
static bool g_autoNext = true;          // View: when a stream ends, start a new one after the breath
static float g_camEase = 1.2f;          // seconds the camera takes between follow and pull-back (slower at an ending)

// Whether the stream's song has ended (a move-book stream composes its own ending: a coda, a final ritard, then a stage
// that rings out into the reverb), and if so its length in frames, the ring-out included. The engine knows it once the
// last stage is rendered, a stage or so before it is heard.
static bool StreamEnded(LmStream* st, uint64_t* endFrame) {
  return st && lm_stream_ended(st, endFrame) == 1 && *endFrame > 0;
}

static const uint32_t kStreamBlock = 4800;  // frames per buffer handed to XAudio2 (0.1 s at 48 kHz)
static const uint32_t kStreamQueued = 4;    // buffers kept queued ahead of the device
static const uint32_t kStreamSlots = 8;     // buffers in the ring (a slot is reused only after it has played)

static LmStream* StartStreamNow(const StreamConfig& asked, std::string& error, StreamConfig* used) {
  char err[1024] = {};
  auto t0 = std::chrono::steady_clock::now();
  StreamConfig c = asked;
  if (c.fresh) {  // a new song: a grammar seed from the clock (and the process, so two launches in a tick differ)
    uint64_t entropy = (uint64_t)std::chrono::system_clock::now().time_since_epoch().count() ^ ((uint64_t)GetCurrentProcessId() << 40);
    c.seed = ui::FreshStreamSeed(entropy);
    c.flags = LM_STREAM_GRAMMAR | (c.explore ? (uint32_t)LM_STREAM_EXPLORE : 0u);
    c.seedText.clear();
  }
  if (used) { *used = c; used->fresh = false; }
  LmStream* st = lm_stream_start(c.seedText.empty() ? nullptr : c.seedText.c_str(), c.seed, c.flags, 1, 0, err, sizeof(err));
  if (!st) { error = err; fprintf(stderr, "cannot start the stream: %s\n", err); return nullptr; }
  LmStreamInfo i{}; lm_stream_info(st, &i);
  printf("stream: seed %llu%s, %.1f bpm, %u voices in its roster, stage 1 rendered in %.2f s\n", (unsigned long long)i.seed,
         (c.flags & LM_STREAM_GRAMMAR) ? " (grammar)" : (c.seedText.empty() ? " (the kept squares song)" : " (a given song)"), i.bpm,
         i.roster_count, std::chrono::duration<double>(std::chrono::steady_clock::now() - t0).count());
  if (c.flags & LM_STREAM_GRAMMAR) printf("stream: play this one again with --stream --seed %llu\n", (unsigned long long)i.seed);
  return st;
}

// The window over stages first..last, built into `s` with the range and energy scale carried from `s`'s fields (the
// caller copies them from the window it replaces). Touches no global.
static bool LoadStreamWindowInto(Song& s, LmStream* st, uint64_t first, uint64_t last, const Theme& th, std::string& error) {
  char err[1024] = {};
  uint64_t start = 0;
  s.lm = lm_stream_window(st, first, last, &start, err, sizeof(err));
  if (!s.lm) { error = err; fprintf(stderr, "stream window %llu..%llu: %s\n", (unsigned long long)first + 1, (unsigned long long)last + 1, err); return false; }
  lm_info(s.lm, &s.info);
  s.streamStart = (double)start / s.info.rate;
  s.streamRange = true;
  if (!VoicesAndGeometry(s, th, error)) { lm_free(s.lm); s.lm = nullptr; return false; }
  return true;
}

static void StreamFeeder() {
  LmStream* st = g_stream.s;
  const uint32_t rate = g_stream.info.rate;
  uint32_t slot = 0;
  std::vector<float> scratch(2 * kStreamBlock);
  while (!g_stream.feederStop.load()) {
    // An ended stream: pull up to its end (the engine's ring-out has decayed there), then nothing.
    uint64_t want = kStreamBlock;
    {
      LmStreamInfo i{}; lm_stream_info(st, &i);
      uint64_t stop = g_stream.stopFrame.load();
      if (i.frames_read >= stop) { Sleep(5); continue; }
      want = std::min<uint64_t>(want, stop - i.frames_read);
    }
    if (g_src) {
      XAUDIO2_VOICE_STATE vs{}; g_src->GetState(&vs, XAUDIO2_VOICE_NOSAMPLESPLAYED);
      if (vs.BuffersQueued < kStreamQueued) {
        std::vector<float>& b = g_stream.blocks[slot];
        uint64_t got = lm_stream_pull(st, b.data(), want);
        uint64_t endF = 0;
        if (got == 0 && StreamEnded(st, &endF)) { Sleep(5); continue; }  // at the end before the main thread saw it
        if (got == 0) break;  // the stream failed: lm_stream_error says why (shown by PollStream)
        XAUDIO2_BUFFER xb{}; xb.AudioBytes = (UINT32)(got * 8); xb.pAudioData = (const BYTE*)b.data();
        if (FAILED(g_src->SubmitSourceBuffer(&xb))) { fprintf(stderr, "audio: SubmitSourceBuffer failed (stream)\n"); break; }
        slot = (slot + 1) % kStreamSlots;
        continue;
      }
    } else {
      // No device: pull only up to a second ahead of the silent clock, so the stream composes at the pace it plays.
      LmStreamInfo i{}; lm_stream_info(st, &i);
      if ((double)i.frames_read < (g_stream.silentPos.load() + 1.0) * rate) {
        uint64_t endF = 0;
        if (lm_stream_pull(st, scratch.data(), want) == 0) {
          if (!StreamEnded(st, &endF)) break;
          Sleep(5);
        }
        continue;
      }
    }
    Sleep(5);
  }
}

static void StartStreamAudio() {
  DestroyVoice();
  g_stream.blocks.assign(kStreamSlots, std::vector<float>(2 * kStreamBlock, 0.0f));
  if (g_xa) {
    WAVEFORMATEX fmt{}; fmt.wFormatTag = WAVE_FORMAT_IEEE_FLOAT; fmt.nChannels = 2; fmt.nSamplesPerSec = g_stream.info.rate;
    fmt.wBitsPerSample = 32; fmt.nBlockAlign = 8; fmt.nAvgBytesPerSec = g_stream.info.rate * 8;
    if (FAILED(g_xa->CreateSourceVoice(&g_src, &fmt))) { fprintf(stderr, "audio: CreateSourceVoice failed (stream)\n"); g_src = nullptr; }
    else { g_src->SetVolume(g_play.volume); if (g_play.playing) g_src->Start(0); }
  }
  g_stream.feederStop = false;
  g_stream.silentPos = g_play.base;
  g_stream.endFrame = UINT64_MAX;
  g_stream.stopFrame = UINT64_MAX;
  g_streamVoiceRate = g_stream.info.rate;
  g_stream.feeder = std::thread(StreamFeeder);
}

static void SetCameraWide(bool wide);  // with the overlay (it holds the View tab's choice)
static bool CameraWide();

// Leave stream mode: the feeder and the window builder stop, the voice goes, the stream is freed. The drawn window
// (g_song) stays until the next song replaces it.
static void StopStream() {
  if (!g_stream.s) return;
  HistEnd(g_stream.end != StreamMode::End::None ? std::min(g_play.now, g_stream.endSec) : g_play.now);  // not the ring-out
  if (g_stream.end != StreamMode::End::None) {
    SetCameraWide(g_stream.wideBefore); g_camEase = 1.2f;
    g_camSpanTarget = 0;
    if (g_stream.wideBefore) g_camSpan = 0;  // the usual pull-back again; otherwise the span goes once the camera is down
  }
  g_stream.end = StreamMode::End::None;
  g_stream.feederStop = true;
  if (g_stream.feeder.joinable()) g_stream.feeder.join();
  if (g_stream.prep.joinable()) g_stream.prep.join();
  g_stream.whole.reset();  // a build still running finishes on its own and frees what it made
  g_stream.wholeShown = false;
  g_stream.prepBusy = false;
  if (g_stream.prepSong && g_stream.prepSong->lm) lm_free(g_stream.prepSong->lm);
  g_stream.prepSong.reset();
  DestroyVoice();
  lm_stream_free(g_stream.s);
  g_stream.s = nullptr;
  g_streaming = false;
  g_streamVoiceRate = 0;
}

// A stream starts playing. It is recorded in the history at once. `startStage` > 0 (with its window ending there) plays
// on from that stage: a replay from the history continuing past its recorded stages, whose recording `attach` goes on.
static void SwapInStream(LmStream* st, std::unique_ptr<Song> s, int index, const StreamConfig& cfg, uint64_t startStage,
                         const std::shared_ptr<hist::Record>& attach, const std::string& title) {
  StopStream();
  WaitIdle();
  DestroyVoice();
  if (g_song.lm) lm_free(g_song.lm);
  g_song = std::move(*s);
  CreateSongResources();
  UpdateFogBeats();
  g_stream.s = st;
  g_stream.cfg = cfg;
  lm_stream_info(st, &g_stream.info);
  g_stream.windowStage = startStage;
  g_streaming = true;
  g_lib.current = index >= 0 ? index : (startStage ? -1 : 0);
  // a Stream entry, or a seed file (its title and description are the entry's own)
  bool known = g_lib.current >= 0 && g_lib.current < (int)g_lib.songs.size() &&
               (ui::IsStream(g_lib.songs[g_lib.current]) || ui::IsSeedPath(g_lib.songs[g_lib.current].path));
  g_stream.fromG = known ? g_lib.songs[g_lib.current].path == ui::kStreamGPath : (!(cfg.flags & LM_STREAM_GRAMMAR) && cfg.seedText.empty());
  g_lib.currentTitle = !title.empty() ? title : known ? g_lib.songs[g_lib.current].title : std::string("Stream");
  g_lib.currentDescription = known ? g_lib.songs[g_lib.current].description : std::string();
  double startSec = 0;
  if (startStage) { LmStageInfo si{}; if (lm_stream_stage(st, startStage, &si)) startSec = (double)si.start_frame / g_stream.info.rate; }
  g_play.playing = true;
  g_play.base = startSec; g_play.silent = 0; g_play.ended = false; g_play.clock = Clock{}; g_play.now = startSec;
  StartStreamAudio();
  HistBegin(st, cfg, attach);
  SetWindowTitleFor(g_lib.currentTitle);
}

// R in stream mode: the same stream again from its first stage (the same seed, not a new song).
static void RestartStream() {
  const bool explore = (g_stream.cfg.flags & LM_STREAM_EXPLORE) != 0;
  const char* path = g_stream.fromG ? ui::kStreamGPath : explore ? ui::kStreamExplorePath : ui::kStreamPath;
  int at = -1;
  for (int i = 0; i < (int)g_lib.songs.size(); ++i) if (g_lib.songs[i].path == path) { at = i; break; }
  if (!g_stream.fromG && (g_streamCfg.fresh || explore)) g_streamReplay = g_stream.info.seed;  // a fresh seed: keep it
  StartLoad(path, g_stream.fromG ? "Stream from G" : explore ? "Explore (strange tunings)" : "Stream", at);
}

// Swap the drawn window for one ending at stage k (between frames, on the main thread).
static double g_lastSwapMs = 0;  // what the last swap cost the main thread (the stream test reports it)
static void SwapInWindow(std::unique_ptr<Song> s, uint64_t k) {
  auto t0 = std::chrono::steady_clock::now();
  WaitIdle();
  if (g_song.lm) lm_free(g_song.lm);
  g_song = std::move(*s);
  CreateSongResources();
  g_stream.windowStage = k;
  g_lastSwapMs = std::chrono::duration<double, std::milli>(std::chrono::steady_clock::now() - t0).count();
}

// A window ending at stage k, with the current window's range and energy scale carried.
static double StageStartSeconds(uint64_t k);
static std::unique_ptr<Song> StreamWindowFor(uint64_t k, std::string& err) {
  auto s = std::make_unique<Song>();
  s->fixMin = g_song.fixMin; s->fixMax = g_song.fixMax; s->energyLo = g_song.energyLo; s->energyHi = g_song.energyHi;
  EnergySeedAt(g_song, StageStartSeconds(k > 0 ? k - 1 : 0), s->energySeed);  // when the drawn window holds that moment
  if (!LoadStreamWindowInto(*s, g_stream.s, k > 0 ? k - 1 : 0, k, g_theme, err)) return nullptr;
  return s;
}

static double StageStartSeconds(uint64_t k) {
  LmStageInfo si{};
  if (!g_stream.s || !lm_stream_stage(g_stream.s, k, &si)) return 0;
  return (double)si.start_frame / g_stream.info.rate;
}

static uint64_t StageAtSeconds(double secs) {
  return g_stream.s ? lm_stream_stage_at(g_stream.s, (uint64_t)(std::max(secs, 0.0) * g_stream.info.rate)) : 0;
}

static void ToastStream(const std::string& text, bool bad, double now) {
  g_stream.toast = text; g_stream.toastBad = bad; g_stream.toastUntil = now + 4.0;
}

// The ending, each frame in stream mode. When the engine says the song has ended (lm_stream_ended: a move-book stream
// composes its own coda and a last stage that rings out into the reverb), the feeder plays to its last frame; as the
// ring-out begins the camera lifts slowly back over the whole song's trail (built on a thread from every stage, as a
// replay merges them) and holds it; once the ring-out has sounded the history marks the entry a finished song with its
// length; then, with auto-next on, a breath and a new song (a fresh seed). The ending card offers "Replay this song".
static void StartNextStream();
// The stream's stages 0..last as a record (its stage texts, copied: read on the main thread, so a build on a thread
// never touches a stream that may be freed under it).
static hist::Record StreamRecordUpTo(LmStream* st, uint64_t last, std::string& err) {
  hist::Record r;
  LmStreamInfo i{};
  if (!st || !lm_stream_info(st, &i)) { err = "no stream"; return r; }
  r.seed = i.seed; r.bpm = i.bpm; r.stageBeats = i.stage_beats; r.origin = "the stream";
  for (uint32_t k = 0; k < i.roster_count; ++k) { const char* n = lm_stream_roster(st, k); if (n) r.roster.push_back(n); }
  for (uint64_t k = 0; k <= last; ++k) {
    LmStageInfo si{};
    if (!lm_stream_stage(st, k, &si)) { err = "stage " + std::to_string(k + 1) + " is not composed"; r.stages.clear(); return r; }
    hist::Stage s;
    s.index = si.index; s.turn = si.turn != 0; s.summary = si.summary ? si.summary : ""; s.text = hist::NormalizeText(si.text ? si.text : "");
    r.stages.push_back(s);
  }
  return r;
}

// The whole song, every stage of `r`, as one song for the world to draw (its time 0 is the stream's): the stages merged
// as a replay merges them (hist::MergeStages), so the trail is the song's own functions. Rendered at 24 kHz (it is
// drawn, never played: its frames are the same milliseconds, and it renders in about half the time). The wall's range
// and the energy's scale are carried from `carry`. Touches no global; null with `err` on failure.
static std::unique_ptr<Song> BuildWholeSong(const hist::Record& r, const Song& carry, const Theme& th, std::string& err) {
  if (r.stages.empty()) { if (err.empty()) err = "no stages"; return nullptr; }
  std::string text = hist::MergeStages(r, 0, r.stages.size() - 1, "The whole song", err);
  if (text.empty()) return nullptr;
  size_t beats = text.find("\nbeats ");
  if (beats == std::string::npos || text.find("\nrate ") != std::string::npos) { err = "the merged song has no beats line, or a rate"; return nullptr; }
  text.insert(beats + 1, "rate 24000\n");
  auto s = std::make_unique<Song>();
  s->streamRange = true;
  s->fixMin = carry.fixMin; s->fixMax = carry.fixMax; s->energyLo = carry.energyLo; s->energyHi = carry.energyHi;
  if (!LoadSongInto(*s, "the whole song", th, err, &text)) return nullptr;
  s->streamStart = 0;
  return s;
}

// The pull-back over the whole song: its trail swapped in for the window (the present stays where it is: both are on
// the stream's clock), and the camera eases back until all of it, start to end, is in view.
static void ShowWholeSong(std::unique_ptr<Song> s, uint64_t lastStage) {
  SwapInWindow(std::move(s), lastStage);
  g_camSpanTarget = (float)(g_song.info.beats * kBeatLen);
  g_stream.wholeShown = true;
}

// How long the hold waits, past kEndHold, for the whole trail to come in before the breath begins anyway.
static const double kWholeWait = 25.0;

static void PollStreamEnding(double now) {
  using E = StreamMode::End;
  if (g_stream.end == E::None) {
    uint64_t endFrame = 0;
    if (!StreamEnded(g_stream.s, &endFrame)) return;
    g_stream.endFrame = endFrame;
    g_stream.stopFrame = endFrame;  // the engine's own ring-out ends there
    g_stream.endSec = (double)endFrame / g_stream.info.rate;
    uint64_t last = lm_stream_stage_at(g_stream.s, endFrame - 1);
    LmStageInfo si{};
    g_stream.ringSec = lm_stream_stage(g_stream.s, last, &si) ? (double)si.start_frame / g_stream.info.rate : g_stream.endSec;
    g_stream.end = E::RingOut;
    g_stream.phaseAt = g_stream.endSec;
    printf("stream: the song ends at %.1f s (%llu stages, the ring-out from %.1f s)\n", g_stream.endSec, (unsigned long long)last + 1, g_stream.ringSec);
    // Its whole trail, built on a thread while the last stages play.
    auto job = std::make_shared<StreamMode::WholeJob>();
    g_stream.whole = job; g_stream.wholeShown = false;
    std::string err;
    hist::Record r = StreamRecordUpTo(g_stream.s, last, err);
    Song carry; carry.fixMin = g_song.fixMin; carry.fixMax = g_song.fixMax; carry.energyLo = g_song.energyLo; carry.energyHi = g_song.energyHi;
    Theme th = g_theme;
    g_wholeThreads.v.emplace_back(std::thread([job, r, carry, th, err]() mutable {
      job->song = BuildWholeSong(r, carry, th, err);
      if (!job->song) job->err = err;
      job->done.store(true, std::memory_order_release);
    }), job);
    return;
  }
  // The whole trail, once built: shown as the pull-back begins, or at once if that has begun (the camera eases on back).
  auto job = g_stream.whole;
  bool wholeIn = job && job->done.load(std::memory_order_acquire);
  bool pulling = g_camEase >= 5.0f;
  if (wholeIn && !g_stream.wholeShown && (pulling || now >= g_stream.ringSec)) {
    if (job->song) {
      ShowWholeSong(std::move(job->song), lm_stream_stage_at(g_stream.s, g_stream.endFrame.load() - 1));
      if (g_stream.end == E::Hold) g_stream.phaseAt = HistNow();  // it is held a while once it is in view
    }
    else fprintf(stderr, "stream: the whole song's trail: %s (the pull-back shows the last stages)\n", job->err.c_str());
    g_stream.whole.reset();
  }
  // The ring-out runs on the stream's clock (the audio); after it the audio has stopped, so the hold and the breath run
  // on the wall clock.
  double wall = HistNow();
  switch (g_stream.end) {
    case E::RingOut:
      // The last stage rings out: lift back, slowly, over the trail (the whole song's once it is in).
      if (now >= g_stream.ringSec && !pulling) {
        g_stream.wideBefore = CameraWide();
        g_camEase = 5.0f;
        g_camSpan = g_camSpanTarget = std::max(g_camSpanTarget, (float)(g_song.info.beats * kBeatLen));
        SetCameraWide(true);
      }
      if (now >= g_stream.endSec - 0.05) {
        HistComplete(g_stream.endSec, lm_stream_stage_at(g_stream.s, g_stream.endFrame.load() - 1) + 1);
        g_stream.end = E::Hold; g_stream.phaseAt = wall;
      }
      break;
    case E::Hold:  // held until the whole trail has been seen (or has failed, or is very late)
      if (wall - g_stream.phaseAt >= kEndHold && (!g_stream.whole || wall - g_stream.phaseAt >= kEndHold + kWholeWait)) {
        g_stream.end = g_autoNext ? E::Breath : E::Waiting; g_stream.phaseAt = wall;
      }
      break;
    case E::Breath:  // auto-next turned off during the breath waits instead
      if (!g_autoNext) g_stream.end = E::Waiting;
      else if (wall - g_stream.phaseAt >= kEndBreath) { g_stream.end = E::Waiting; g_stream.phaseAt = 1e300; StartNextStream(); }  // asked once
      break;
    case E::Waiting:  // until the card's buttons, or auto-next turned on again
      if (g_autoNext && !g_loader.busy && wall - g_stream.phaseAt >= kEndBreath) { g_stream.phaseAt = wall; g_stream.end = E::Breath; }
      break;
    default: break;
  }
}

// Each frame in stream mode: build the next window ahead, and swap it in at the seam.
static void PollStream(double now) {
  g_wholeThreads.Join(false);
  if (!g_streaming || !g_stream.s) return;
  g_stream.silentPos = now;
  uint64_t k = StageAtSeconds(now);
  if (g_stream.prepBusy && g_stream.prepDone.load(std::memory_order_acquire)) {
    g_stream.prep.join();
    g_stream.prepBusy = false;
  }
  // Stage k + 1 rendered: its window (k, k + 1) is built on a thread while stage k plays.
  if (!g_stream.prepBusy && !g_stream.prepSong && g_stream.windowStage == k) {
    LmStageInfo si{};
    if (lm_stream_stage(g_stream.s, k + 1, &si) && si.rendered) {
      g_stream.prepBusy = true; g_stream.prepDone = false; g_stream.prepStage = k + 1;
      auto s = std::make_unique<Song>();
      s->fixMin = g_song.fixMin; s->fixMax = g_song.fixMax; s->energyLo = g_song.energyLo; s->energyHi = g_song.energyHi;
      EnergySeedAt(g_song, StageStartSeconds(k), s->energySeed);  // the drawn window (k - 1, k) holds stage k's start
      g_stream.prepSong = std::move(s);
      Song* target = g_stream.prepSong.get();
      LmStream* st = g_stream.s;
      Theme th = g_theme;
      g_stream.prep = std::thread([target, st, k, th]() {
        std::string err;
        if (!LoadStreamWindowInto(*target, st, k, k + 1, th, err)) g_stream.prepError = err;
        g_stream.prepDone.store(true, std::memory_order_release);
      });
    }
  }
  if (k > g_stream.windowStage) {
    if (g_stream.prepBusy) { g_stream.prep.join(); g_stream.prepBusy = false; }  // its target is touched below
    if (g_stream.prepSong && g_stream.prepSong->lm && g_stream.prepStage == k) {
      SwapInWindow(std::move(g_stream.prepSong), k);
    } else {
      // Not built ahead (a stall, a pause at the seam): build it now.
      if (g_stream.prepSong) { if (g_stream.prepSong->lm) lm_free(g_stream.prepSong->lm); }
      std::string err;
      auto s = StreamWindowFor(k, err);
      if (s) SwapInWindow(std::move(s), k);
      else { ToastStream("The stream's window: " + err, true, now); g_stream.windowStage = k; }  // once, not every frame
    }
    g_stream.prepSong.reset();
  }
  char err[512];
  if (lm_stream_error(g_stream.s, err, sizeof(err)) && g_stream.toastUntil < now) ToastStream(std::string("The stream stopped: ") + err, true, now + 1e9);
  HistPoll(now, g_stream.endFrame.load());
  PollStreamEnding(now);
}

// S: the last four stages heard (the playing one included) as one song in examples\, where the library finds it.
static void SaveStreamStretch(double now) {
  if (!g_streaming || !g_stream.s || g_lib.dirs.empty()) return;
  uint64_t cur = StageAtSeconds(now);
  uint32_t count = (uint32_t)std::min<uint64_t>(4, cur + 1);
  uint64_t first = cur + 1 - count;
  namespace fs = std::filesystem;
  fs::path dir = fs::u8path(g_lib.dirs[0]);
  std::string name = ui::StreamSaveName(g_stream.info.seed, first + 1, cur + 1,
                                        [&](const std::string& n) { std::error_code ec; return fs::exists(dir / fs::u8path(n), ec); });
  std::string path = (dir / fs::u8path(name)).u8string();
  char err[1024] = {};
  if (lm_stream_save(g_stream.s, cur, count, path.c_str(), err, sizeof(err))) {
    // The whole seed too: one small file that replays the stream from its start at its generator version (a stream
    // from a song has none; that is not an error, the stretch is saved).
    char seedName[96];
    snprintf(seedName, sizeof(seedName), "stream_s%llu_v%u.seed", (unsigned long long)g_stream.info.seed, lm_stream_version(g_stream.s));
    char serr[512] = {};
    const bool seedSaved = lm_stream_save_seed(g_stream.s, (dir / fs::u8path(seedName)).u8string().c_str(), serr, sizeof(serr)) != 0;
    if (!seedSaved) printf("stream: no seed file: %s\n", serr);
    char msg[256];
    snprintf(msg, sizeof(msg), u8"Saved stages %llu–%llu as %s%s%s", (unsigned long long)first + 1, (unsigned long long)cur + 1, name.c_str(),
             seedSaved ? ", and the whole seed as " : "", seedSaved ? seedName : "");
    printf("stream: %s\n", msg);
    ToastStream(msg, false, now);
  } else {
    fprintf(stderr, "stream: cannot save: %s\n", err);
    ToastStream(std::string("Could not save: ") + err, true, now);
  }
}

// The auto-next song (and the ending card's "Next song"): Stream, from a fresh seed.
static void StartNextStream() {
  int at = -1;
  for (int i = 0; i < (int)g_lib.songs.size(); ++i) if (g_lib.songs[i].path == ui::kStreamPath) { at = i; break; }
  g_streamNextFresh = true;
  StartLoad(ui::kStreamPath, "Stream", at);
}

// ================================================================ replaying from the history
//
// A recorded stream plays again from its recorded stages, whatever the composer writes now: the stages are merged into
// ordinary songs (hist::MergeStages, the engine's own way of saving a stretch) a few at a time, each rendered on a
// thread while the one before plays, and each stretch starts one stage early and is played from that stage's end, so
// the voices and the reverb arrive at the seam already sounding. After the last recorded stage, if this engine's
// composer is the one that recorded it, the stream goes on live from its seed: a stream is started on a thread and
// played (silently, as fast as it renders) up to that stage, every recorded stage is checked against what it composes
// now, and the world switches to it at the seam, recording on into the same entry. If anything differs it says so and
// stops at the end of the recording instead.

static const size_t kReplayFirstChunk = 2, kReplayChunk = 4;  // stages per stretch (the first is short: it plays sooner)

struct ReplayChunk {
  std::atomic<bool> done{ false };
  std::shared_ptr<hist::Record> rec;  // loaded by the first stretch, then shared
  std::unique_ptr<Song> song;
  std::string err;
  size_t first = 0, last = 0;         // positions in rec->stages (last clamped to the recording)
  double startAt = 0;                 // where in the song `first` begins (after its lead-in stage)
  ~ReplayChunk() { if (song && song->lm) lm_free(song->lm); }
};

struct Continuation {
  std::atomic<bool> done{ false }, cancel{ false };
  std::atomic<uint64_t> progress{ 0 };  // the stage it has reached
  StreamConfig cfg;
  uint64_t stage = 0;                   // the first stage after the recording
  LmStream* st = nullptr;
  std::unique_ptr<Song> song;           // its window, ending at `stage`
  std::string err;
  ~Continuation() { if (song && song->lm) lm_free(song->lm); if (st) lm_stream_free(st); }
};

struct Replay {
  bool active = false, started = false, waiting = false;
  std::string path, title, recorded;
  uint64_t seed = 0;
  std::shared_ptr<hist::Record> rec;
  size_t from = 0, to = SIZE_MAX, next = 0, playFirst = 0, playLast = 0;
  std::shared_ptr<ReplayChunk> chunk;
  std::shared_ptr<Continuation> cont;
  std::string note;                     // whether it will go on past the recording, and why not
  std::vector<std::pair<std::thread, std::shared_ptr<std::atomic<bool>>>> threads;
} g_replay;

// The last stage a replay may play: the recording's last, or the last before the song ended.
static size_t ReplayLastAllowed(const hist::Record& r) {
  size_t last = r.stages.empty() ? 0 : r.stages.size() - 1;
  if (r.complete && r.completeStages > 0) last = std::min(last, (size_t)r.completeStages - 1);
  return last;
}

static void ReplayLaunchChunk(size_t first, size_t last) {
  auto job = std::make_shared<ReplayChunk>();
  job->rec = g_replay.rec;
  job->first = first; job->last = last;
  auto fin = std::make_shared<std::atomic<bool>>(false);
  std::string path = g_replay.path;
  Theme th = g_theme;
  g_replay.chunk = job;
  g_replay.threads.emplace_back(std::thread([job, path, th, fin]() {
    std::string e;
    if (!job->rec) {
      auto r = std::make_shared<hist::Record>();
      if (hist::LoadRecord(path, *r, e)) job->rec = r;
      else job->err = e;
    }
    if (job->rec) {
      const hist::Record& r = *job->rec;
      if (r.stages.empty()) job->err = "this recording has no stages";
      else if (!(r.bpm > 0)) job->err = "this recording has no tempo";
      else if (job->first > ReplayLastAllowed(r)) job->err = "no such stage in this recording";
      else {
        job->last = std::min(job->last, ReplayLastAllowed(r));
        size_t pre = job->first > 0 ? job->first - 1 : job->first;  // a stage of lead-in, played over unheard
        std::string text = hist::MergeStages(r, pre, job->last, "Replayed", e);
        auto s = std::make_unique<Song>();
        if (text.empty()) job->err = e;
        else if (!LoadSongInto(*s, "the replay", th, e, &text)) job->err = e;
        else { job->song = std::move(s); job->startAt = (job->first - pre) * r.stageBeats * 60.0 / r.bpm; }
      }
    }
    job->done.store(true, std::memory_order_release);
    fin->store(true);
  }), fin);
}

// The stream from the record's seed, played silently up to its first unrecorded stage, every recorded stage checked
// against what the composer writes now, and its window built there. On a thread; `c.cancel` stops it. (Also run by
// --history-test.)
static void PrepareContinuation(Continuation& c, const hist::Record& r, const Theme& th) {
  std::string err;
  c.st = StartStreamNow(c.cfg, err);
  if (!c.st) { c.err = "cannot start its stream: " + err; return; }
  LmStreamInfo i{}; lm_stream_info(c.st, &i);
  const uint64_t N = c.stage;
  std::vector<float> scratch(2 * 48000);
  uint64_t target = UINT64_MAX;
  for (;;) {
    if (c.cancel) return;
    LmStageInfo si{};
    if (target == UINT64_MAX && lm_stream_stage(c.st, N, &si)) target = si.start_frame;
    lm_stream_info(c.st, &i);
    if (i.frames_read >= target) break;
    uint64_t n = std::min<uint64_t>(48000, target == UINT64_MAX ? 48000 : target - i.frames_read);
    if (lm_stream_pull(c.st, scratch.data(), n) == 0) {
      uint64_t endF = 0;
      if (StreamEnded(c.st, &endF)) { c.err = "the song ends with its recorded stages, so there is nothing to go on to"; return; }
      char e[512] = {}; lm_stream_error(c.st, e, sizeof(e));
      c.err = std::string("its stream stopped: ") + e;
      return;
    }
    c.progress = lm_stream_stage_at(c.st, i.frames_read + n);
  }
  if (i.frames_read != target) { c.err = "its stream ran past the stage to continue from"; return; }
  for (uint64_t k = 0; k < N && k < r.stages.size(); ++k) {
    LmStageInfo si{};
    if (!lm_stream_stage(c.st, k, &si) || hist::NormalizeText(si.text ? si.text : "") != r.stages[k].text) {
      c.err = "the composer now writes stage " + std::to_string(k + 1) + " of this seed differently, so it cannot go on from the recording";
      return;
    }
  }
  for (;;) {
    if (c.cancel) return;
    LmStageInfo si{};
    if (lm_stream_stage(c.st, N, &si) && si.rendered) break;
    uint64_t endF = 0;
    if (!lm_stream_stage(c.st, N, &si) && StreamEnded(c.st, &endF)) { c.err = "the song ends with its recorded stages, so there is nothing to go on to"; return; }
    char e[512] = {};
    if (lm_stream_error(c.st, e, sizeof(e))) { c.err = std::string("its stream stopped: ") + e; return; }
    Sleep(10);
  }
  auto s = std::make_unique<Song>();
  if (!LoadStreamWindowInto(*s, c.st, N > 0 ? N - 1 : 0, N, th, err)) { c.err = err; return; }
  c.song = std::move(s);
}

// The record is in (the first stretch read it): may it go on past its recorded stages?
static void ReplayDecideContinuation() {
  const hist::Record& r = *g_replay.rec;
  g_replay.to = std::min(g_replay.to, ReplayLastAllowed(r));
  if (r.complete) { g_replay.note = "A finished song: it plays to its end."; return; }
  if (g_replay.to + 1 < r.stages.size()) { g_replay.note = "Plays the stages chosen."; return; }
  if (g_hist.version.empty() || r.composer != g_hist.version) {
    g_replay.note = "Recorded with another composer: the recording plays exactly, then it stops (it cannot go on from its seed).";
    return;
  }
  g_replay.note = "After the recording it goes on from its seed, as new stages.";
  auto c = std::make_shared<Continuation>();
  c->cfg.seed = r.seed; c->cfg.flags = r.flags; c->cfg.seedText = r.seedSong; c->cfg.fresh = false;
  c->stage = r.stages.size();
  auto rec = g_replay.rec;
  auto fin = std::make_shared<std::atomic<bool>>(false);
  Theme th = g_theme;
  g_replay.cont = c;
  g_replay.threads.emplace_back(std::thread([c, rec, th, fin]() {
    PrepareContinuation(*c, *rec, th);
    c->done.store(true, std::memory_order_release);
    fin->store(true);
  }), fin);
}

// Replay entry `path`, stages from..to (positions; to SIZE_MAX: to the end of the recording).
static void ReplayStart(const std::string& path, size_t from, size_t to, uint64_t seed, const std::string& started) {
  ReplayCancel();
  g_replay.active = true; g_replay.started = false; g_replay.waiting = false;
  g_replay.path = path; g_replay.seed = seed; g_replay.recorded = started;
  g_replay.rec.reset();
  g_replay.from = from; g_replay.to = to; g_replay.next = from;
  g_replay.note.clear();
  g_replay.title = u8"Replay  ·  seed " + std::to_string(seed);
  ReplayLaunchChunk(from, std::min(to, from + kReplayFirstChunk - 1));
}

// Ends a replay (another song asked for, or a new replay). Its threads finish on their own and are joined later.
static void ReplayCancel() {
  if (g_replay.cont) { g_replay.cont->cancel = true; g_replay.cont.reset(); }
  g_replay.chunk.reset();
  g_replay.active = false;
  g_replay.waiting = false;
}

static void HoldAtEnd() {
  DestroyVoice();
  g_play.base = g_song.info.duration - 1e-6;
  g_play.silent = 0;
  g_play.playing = false;
  g_play.ended = true;
}

static void ReplayDescribe() {
  char b[256];
  snprintf(b, sizeof(b), u8"Stages %zu–%zu  ·  recorded %s", g_replay.playFirst + 1, g_replay.playLast + 1, g_replay.recorded.c_str());
  g_lib.currentDescription = b;
}

static void ReplaySwapIn() {
  std::shared_ptr<ReplayChunk> c = std::move(g_replay.chunk);
  std::unique_ptr<Song> s = std::move(c->song);
  bool wasActive = g_replay.active;
  SwapInSong(std::move(s), -1, g_replay.title, "<replay>");
  g_replay.active = wasActive;
  PlayerSeekTotal(c->startAt);
  g_replay.started = true;
  g_replay.waiting = false;
  g_replay.playFirst = c->first; g_replay.playLast = c->last;
  g_replay.next = c->last + 1;
  ReplayDescribe();
  if (g_replay.next <= g_replay.to) ReplayLaunchChunk(g_replay.next, std::min(g_replay.to, g_replay.next + kReplayChunk - 1));
}

static void ReplayContinue() {
  std::shared_ptr<Continuation> c = std::move(g_replay.cont);
  LmStream* st = c->st; c->st = nullptr;
  std::unique_ptr<Song> s = std::move(c->song);
  auto rec = g_replay.rec;
  g_replay.active = false;
  printf("history: the replay goes on from seed %llu at stage %llu\n", (unsigned long long)rec->seed, (unsigned long long)c->stage + 1);
  SwapInStream(st, std::move(s), -1, c->cfg, c->stage, rec, u8"Stream  ·  seed " + std::to_string(rec->seed));
}

// At the end of a stretch (OnSongEnd): the next stretch, the stream going on, or the end. True when a replay is playing.
static bool ReplayOnEnd() {
  if (!g_replay.active || !g_replay.started) return false;
  if (g_replay.chunk && g_replay.chunk->done && g_replay.chunk->err.empty() && g_replay.chunk->song) { ReplaySwapIn(); return true; }
  if (g_replay.next <= g_replay.to || g_replay.cont) {  // still rendering: hold, and PollReplay goes on when it is in
    HoldAtEnd();
    g_replay.waiting = true;
    if (g_replay.cont && g_replay.cont->done && g_replay.cont->err.empty()) ReplayContinue();
    return true;
  }
  HoldAtEnd();
  g_replay.active = false;
  g_lib.currentDescription = u8"The replay has played every stage chosen.  " + g_replay.note;
  return true;
}

// Each frame: join finished threads, take a stretch that is in, and go on.
static void PollReplay() {
  for (size_t i = 0; i < g_replay.threads.size();) {
    if (g_replay.threads[i].second->load()) { g_replay.threads[i].first.join(); g_replay.threads.erase(g_replay.threads.begin() + i); }
    else ++i;
  }
  if (!g_replay.active) return;
  auto c = g_replay.chunk;
  if (c && c->done.load(std::memory_order_acquire)) {
    if (!g_replay.rec && c->rec) { g_replay.rec = c->rec; ReplayDecideContinuation(); }
    if (!c->err.empty()) {
      HistToast("Cannot replay: " + c->err, true);
      ReplayCancel();
      return;
    }
    if (!g_replay.started || g_replay.waiting) ReplaySwapIn();
    return;
  }
  auto k = g_replay.cont;
  if (g_replay.waiting && g_replay.next > g_replay.to && k && k->done.load(std::memory_order_acquire)) {
    if (k->err.empty()) ReplayContinue();
    else {
      HistToast(k->err, true);
      g_replay.cont.reset();
      g_replay.active = false;
      g_lib.currentDescription = "The replay has played every recorded stage. It stops here: " + k->err;
    }
  }
}

static void ShutdownReplay() {
  ReplayCancel();
  for (auto& t : g_replay.threads) t.first.join();
  g_replay.threads.clear();
}

// Headless (captures, benches): play the stream up to `secs` (pulling and discarding its audio, so it composes and
// renders that far) and draw the window there. Times must not go back more than a stage (the engine keeps two).
static bool AdvanceHeadlessStream(double secs) {
  uint64_t want = (uint64_t)(std::max(secs, 0.0) * g_stream.info.rate);
  std::vector<float> scratch(2 * 48000);
  for (;;) {
    LmStreamInfo i{}; lm_stream_info(g_stream.s, &i);
    if (i.frames_read >= want) break;
    uint64_t n = std::min<uint64_t>(48000, want - i.frames_read);
    if (lm_stream_pull(g_stream.s, scratch.data(), n) == 0) {
      char err[512] = {}; lm_stream_error(g_stream.s, err, sizeof(err));
      fprintf(stderr, "stream: %s\n", err);
      return false;
    }
  }
  uint64_t k = StageAtSeconds(secs);
  if (k != g_stream.windowStage) {
    std::string err;
    auto s = StreamWindowFor(k, err);
    if (!s) return false;
    SwapInWindow(std::move(s), k);
  }
  return true;
}

// Silent test of stream mode as the window plays it (volume 0, no window opens): the Stream entry starts on the
// loader thread, the feeder keeps XAudio2 fed, each next window is built ahead and swapped in at its seam. Checks that
// the audio never runs dry, that its position keeps pace with the wall clock, that the drawn window follows the
// stage playing, and how long each swap holds the main thread. If the song ends within SECONDS (give it room: a song
// runs 4-6 minutes), it is followed to the next song: the audio stops at the engine's end, the pull-back shows the
// whole song's trail, the history (with --history-dir) marks it finished, and auto-next starts a new song.
static int StreamTest(double seconds) {
  bool device = InitAudioEngine();
  if (!device) fprintf(stderr, "stream test: no audio device; the silent clock and the feeder's pacing are tested instead\n");
  g_play.volume = 0;
  StartLoad(ui::kStreamPath, "Stream", 0);
  auto t0 = std::chrono::steady_clock::now();
  auto last = t0;
  double started = -1, maxSwap = 0, sumSwap = 0;
  int swaps = 0, starved = 0, behind = 0;
  uint64_t lastWindow = 0;
  // The ending, when the song ends: what was seen of it.
  bool ended = false, pulledBack = false, held = false, nextStarted = false;
  double endSec = 0, endPlayed = 0, endWall = 0, spanShown = 0, songLen = 0, endBpm = 0;
  uint64_t endSeed = 0, endStage = 0;
  int endSwaps = 0, endBehind = 0;
  for (;;) {
    auto now = std::chrono::steady_clock::now();
    double wall = std::chrono::duration<double>(now - t0).count();
    double dt = std::chrono::duration<double>(now - last).count();
    last = now;
    PollLoader();
    if (!g_streaming) {
      if (!g_loader.busy) { fprintf(stderr, "stream test: the stream did not start: %s\n", g_loader.failedError.c_str()); return 1; }
      if (wall > 30 + (ended ? endWall : 0)) { fprintf(stderr, "stream test: the stream took over 30 s to start\n"); return 1; }
      Sleep(5);
      continue;
    }
    if (started < 0) { started = wall; printf("stream test: the stream started %.2f s after it was asked for\n", wall); }
    if (ended && g_stream.info.seed != endSeed) {  // auto-next: a new song from a fresh seed
      nextStarted = true;
      printf("stream test: the next song started (seed %llu) %.1f s after the end\n", (unsigned long long)g_stream.info.seed, wall - started - endPlayed);
      break;
    }
    double t = PlayerFrameTime(dt);
    PollStream(t);
    if (!ended && g_stream.end != StreamMode::End::None) {
      ended = true; endSec = g_stream.endSec; endSeed = g_stream.info.seed;
      printf("stream test: the engine says the song ends at %.1f s\n", endSec);
    }
    bool over = ended && t >= endSec - 0.05;  // the audio has stopped: nothing is queued, and the clock stands
    if (g_stream.wholeShown && !pulledBack && CameraWide()) {
      pulledBack = true; spanShown = g_camSpanTarget; songLen = g_song.info.duration;
      printf("stream test: the pull-back began at %.1f s over %.0f m of trail (%.1f s of song)\n", t, spanShown, songLen);
    }
    if (g_stream.end == StreamMode::End::Hold) held = true;
    if (g_stream.windowStage != lastWindow && !g_stream.wholeShown) {
      ++swaps; maxSwap = std::max(maxSwap, g_lastSwapMs); sumSwap += g_lastSwapMs; lastWindow = g_stream.windowStage;
      printf("  stage %llu at %.2f s: window swapped in %.1f ms\n", (unsigned long long)lastWindow + 1, t, g_lastSwapMs);
    }
    if (g_src && wall - started > 0.5 && !over) {
      XAUDIO2_VOICE_STATE vs{}; g_src->GetState(&vs, XAUDIO2_VOICE_NOSAMPLESPLAYED);
      if (vs.BuffersQueued == 0) ++starved;
    }
    if (!g_stream.wholeShown && StageAtSeconds(t) != g_stream.windowStage && t - StageStartSeconds(StageAtSeconds(t)) > 0.1) ++behind;
    if (over && endPlayed == 0) {
      endPlayed = t; endWall = wall - started; endStage = StageAtSeconds(endPlayed - 1e-3); endBpm = g_stream.info.bpm;
      endSwaps = swaps; endBehind = behind;
    }
    if (!ended && wall - started >= seconds) break;
    if (ended && wall - started >= seconds + 60) break;  // the ending did not finish: reported below
    Sleep(4);
  }
  int fails = 0;
  auto report = [&](const char* what, bool ok) { printf("  %-66s %s\n", what, ok ? "ok" : "FAIL"); if (!ok) ++fails; };
  if (!ended) {
    double played = PlayerAudio(), wall = std::chrono::duration<double>(std::chrono::steady_clock::now() - t0).count() - started;
    uint64_t stage = StageAtSeconds(played);
    printf("stream test (silent): %.1f s played in %.1f s, stage %llu, %d windows swapped in (%.1f ms on average, at most %.1f)\n",
           played, wall, (unsigned long long)stage + 1, swaps, swaps ? sumSwap / swaps : 0.0, maxSwap);
    report("the audio never ran dry (no moment with nothing queued)", starved == 0);
    report("the audio position kept pace with the wall clock (within 0.25 s)", fabs(played - wall) < 0.25);
    report("the drawn window followed the stage playing", behind == 0 && g_stream.windowStage == stage);
    report("a window was swapped in at every seam passed", swaps == (int)stage);
  } else {
    printf("stream test (silent): the song played to its end, %.1f s in %.1f s, %llu stages, %d windows swapped in (%.1f ms on average, at most %.1f)\n",
           endPlayed, endWall, (unsigned long long)endStage + 1, endSwaps, endSwaps ? sumSwap / std::max(swaps, 1) : 0.0, maxSwap);
    report("the audio never ran dry before the end", starved == 0);
    report("the audio stopped at the engine's end (within 0.1 s)", endPlayed > 0 && fabs(endPlayed - endSec) < 0.1);
    report("the audio position kept pace with the wall clock (within 0.25 s)", endPlayed > 0 && fabs(endPlayed - endWall) < 0.25);
    report("the drawn window followed the stage playing", endBehind == 0);
    report("a window was swapped in at every seam before the pull-back", endSwaps >= (int)endStage - 1 && endSwaps <= (int)endStage);
    report("the pull-back framed the whole song's trail", pulledBack && songLen > 0 && fabs(songLen - endSec) < 32 * 60.0 / endBpm &&
           fabs(spanShown - songLen * endBpm / 60.0 * kBeatLen) < 1.0);
    report("the ending card held after the ring-out", held);
    if (g_hist.record && g_hist.worker) {
      g_hist.worker->Flush(); PollHistory();
      bool complete = false;
      for (auto& e : hist::ScanDir(g_hist.dir).entries) if (e.seed == endSeed && e.complete) complete = true;
      report("the history marked the song finished", complete);
    }
    report("auto-next started a new song after the breath", nextStarted);
  }
  StopStream();
  g_wholeThreads.Join(true);
  StopAudio();
  printf("stream test: %s\n", fails ? "FAILED" : "all ok");
  return fails ? 1 : 0;
}

// The camera across a stream's seams (no window, no GPU, no audio): a seeded stream's windows are built in order, as
// PollStream swaps them in, and the camera is evaluated at 60 frames a second from 3 s before each seam to 3 s after
// it (the old window before the seam, the new one after). Reported per seam: how far the camera's eye and target move
// in one frame, at the seam and after it, against the same 3 s before it; and the wall's scale (metres per semitone)
// and floor pitch on each side. A camera that never visibly reacts to a seam moves no more across it than it does on
// its own, and the wall keeps one scale for the whole song. (Reported 2026-11-03: at the seam from one eight-bar
// stage to the next, where more voices come in, the camera moved unnaturally.)
static int CameraSeamTest(uint64_t seed, int seams, uint32_t flags = LM_STREAM_GRAMMAR) {
  StreamConfig c; c.fresh = false; c.seed = seed; c.flags = flags;
  std::string err;
  LmStream* st = StartStreamNow(c, err, nullptr);
  if (!st) { fprintf(stderr, "camera seam test: %s\n", err.c_str()); return 1; }
  LmStreamInfo info{}; lm_stream_info(st, &info);
  std::vector<float> buf(2 * kStreamBlock);
  // Pull audio (as fast as it renders) until stage k is rendered; false once the song has ended before it.
  auto renderedUpTo = [&](uint64_t k) {
    for (;;) {
      LmStageInfo si{};
      if (lm_stream_stage(st, k, &si) && si.rendered) return true;
      uint64_t endF = 0;
      if (lm_stream_pull(st, buf.data(), kStreamBlock) == 0) return false;
      if (StreamEnded(st, &endF) && lm_stream_stage_at(st, endF - 1) < k) return false;
    }
  };
  Song saved = std::move(g_song); g_song = Song{};
  float wideWas = g_camWide; g_camWide = 0;
  auto camAt = [&](Song& s, double secs) {
    std::swap(g_song, s);
    View v; v.total = secs; v.songSec = std::clamp(secs - g_song.streamStart, 0.0, std::max(0.0, g_song.info.duration - 1e-6));
    Camera cam = CameraAt(v);
    std::swap(g_song, s);
    return cam;
  };
  auto window = [&](uint64_t first, uint64_t last, const Song* carry) {
    auto s = std::make_unique<Song>();
    if (carry) {
      s->fixMin = carry->fixMin; s->fixMax = carry->fixMax; s->energyLo = carry->energyLo; s->energyHi = carry->energyHi;
      LmStageInfo si{};
      if (lm_stream_stage(st, first, &si)) EnergySeedAt(*carry, (double)si.start_frame / info.rate, s->energySeed);  // as PollStream does
    }
    std::string e;
    if (!LoadStreamWindowInto(*s, st, first, last, g_theme, e)) { fprintf(stderr, "camera seam test: window %llu..%llu: %s\n", (unsigned long long)first + 1, (unsigned long long)last + 1, e.c_str()); return std::unique_ptr<Song>(); }
    return s;
  };
  std::unique_ptr<Song> cur = window(0, 0, nullptr);
  int fails = 0, measured = 0;
  double worstJump = 0, worstSurge = -1e9, worstMoveRatio = 0, energyPeak = 0;
  float worstScale = 0;
  const double fps = 60.0, dtF = 1.0 / fps;
  printf("camera seam test: seed %llu, %.1f bpm, %.1f s a stage; mm at 60 fps\n", (unsigned long long)info.seed,
         info.bpm, info.stage_beats * 60.0 / info.bpm);
  printf("  seam  | move/frame: 3 s before, at seam | jump at seam: eye, target | change of move: before, after |"
         " wall m/semitone   pitch frame\n");
  for (int k = 1; cur && k <= seams; ++k) {
    if (!renderedUpTo((uint64_t)k)) { printf("  (the song ended before seam %d)\n", k); break; }
    std::unique_ptr<Song> next = window((uint64_t)k - 1, (uint64_t)k, cur.get());
    if (!next) { ++fails; break; }
    double S = 0;
    { LmStageInfo si{}; lm_stream_stage(st, (uint64_t)k, &si); S = (double)si.start_frame / info.rate; }
    // Frames on a 60 fps grid with the seam between two of them, as the window swaps at the first frame past it.
    double t0 = S - 3.0 + 0.5 * dtF;
    // Per frame: the move (mm), and the change of the move from the frame before (mm a frame, a frame: a surge or a
    // lurch shows here, a smooth drift does not). At the seam: the jump, the old window's camera against the new one's
    // at the same moment.
    auto dist = [](V3 a, V3 b) { V3 d = a - b; return sqrt((double)Dot(d, d)) * 1000.0; };
    double moveB = 0, moveAt = 0, accB = 0, accA = 0, jumpEye = 0, jumpTg = 0;
    Camera prev = camAt(*cur, t0);
    V3 lastMoveE{ 0, 0, 0 }, lastMoveT{ 0, 0, 0 };
    for (int f = 1; f < (int)(6.0 * fps); ++f) {
      double t = t0 + f * dtF;
      Camera cm = t < S ? camAt(*cur, t) : camAt(*next, t);
      V3 mE = cm.eye - prev.eye, mT = cm.target - prev.target;
      double move = std::max(dist(cm.eye, prev.eye), dist(cm.target, prev.target));
      double acc = f > 1 ? std::max(dist(mE, lastMoveE), dist(mT, lastMoveT)) : 0.0;
      bool crossing = t >= S && t - dtF < S;
      if (crossing) {
        Camera old = camAt(*cur, t);
        jumpEye = dist(cm.eye, old.eye); jumpTg = dist(cm.target, old.target); moveAt = move;
      }
      if (t < S) { moveB = std::max(moveB, move); accB = std::max(accB, acc); }
      else accA = std::max(accA, acc);
      lastMoveE = mE; lastMoveT = mT; prev = cm;
    }
    printf("  %2d->%-2d|  %7.2f  %8.2f | %8.2f  %8.2f | %7.3f  %7.3f | %.4f -> %.4f   %.1f..%.1f -> %.1f..%.1f\n", k, k + 1, moveB,
           moveAt, jumpEye, jumpTg, accB, accA, cur->semi, next->semi, cur->pMin, cur->pMax, next->pMin, next->pMax);
    worstJump = std::max({ worstJump, jumpEye, jumpTg });
    worstSurge = std::max(worstSurge, accA - std::max(1.5 * accB, accB + 0.02));
    worstMoveRatio = std::max(worstMoveRatio, moveAt / std::max(moveB, 0.05));
    worstScale = std::max({ worstScale, fabsf(next->semi - cur->semi), (float)fabs(next->pMin - cur->pMin), (float)fabs(next->pMax - cur->pMax) });
    for (double e : next->energyLag2) energyPeak = std::max(energyPeak, e);
    ++measured;
    if (cur->lm) lm_free(cur->lm);
    cur = std::move(next);
  }
  if (cur && cur->lm) lm_free(cur->lm);
  lm_stream_free(st);
  g_song = std::move(saved); g_camWide = wideWas;
  auto report = [&](const char* what, bool ok) { printf("  %-80s %s\n", what, ok ? "ok" : "FAIL"); if (!ok) ++fails; };
  printf("camera seam test: %d seams; the largest jump at a seam %.3f mm; the seam frame's move at most %.2fx the largest in the"
         " 3 s before; the raw energy peaked at %.3f (the stream's full scale is %.2f)\n", measured, worstJump, worstMoveRatio,
         energyPeak, kStreamEnergyFull);
  report("seams were measured (at least 3)", measured >= 3);
  report("the camera does not jump at a seam (eye and target within 1 mm of the old window's)", worstJump <= 1.0);
  report("the seam frame moves no more than the frames before it (within 1.25x)", worstMoveRatio <= 1.25);
  report("the camera does not surge after a seam (its move changes no faster than before)", worstSurge <= 0);
  report("the wall keeps one scale and range for the whole song", worstScale == 0);
  printf("camera seam test: %s\n", fails ? "FAILED" : "all ok");
  return fails ? 1 : 0;
}

// The history against the real engine (no window, no audio): a stream is recorded as the window records it, read back
// from disk and compared with what the engine composed; one recorded stage replayed must sound exactly as the stage
// itself; two replayed must play each stage's functions; the stream must go on from the recording's seed (and refuse
// to when a recorded stage differs); and a write that fails must be reported. `keepDir` keeps the recording there.
static int HistoryTest(const std::string& keepDir) {
  namespace fs = std::filesystem;
  int fails = 0;
  auto report = [&](const char* what, bool ok) { printf("  %-74s %s\n", what, ok ? "ok" : "FAIL"); if (!ok) ++fails; };
  fs::path dir = keepDir.empty() ? fs::temp_directory_path() / ("dreamride_history_test_" + std::to_string(GetCurrentProcessId())) : fs::u8path(keepDir);
  std::error_code ec;
  if (keepDir.empty()) fs::remove_all(dir, ec);
  HistInit(dir.u8string(), true);
  g_hist.worker->Flush(); PollHistory();
  printf("history test: recording into %s\n", dir.u8string().c_str());
  report("the composer's version is read from luminesce.dll", !g_hist.version.empty());

  StreamConfig cfg; cfg.seed = g_streamCfg.seed != 1 ? g_streamCfg.seed : 4821; cfg.flags = LM_STREAM_GRAMMAR; cfg.fresh = false;  // --seed N
  std::string err; StreamConfig used;
  LmStream* st = StartStreamNow(cfg, err, &used);
  if (!st) { fprintf(stderr, "history test: cannot start a stream: %s\n", err.c_str()); return 1; }
  HistBegin(st, used);
  LmStreamInfo i{}; lm_stream_info(st, &i);
  const uint64_t kStages = 3;
  std::vector<float> scratch(2 * 4800);
  double now = 0;
  for (;;) {
    lm_stream_info(st, &i);
    now = (double)i.frames_read / i.rate;
    HistPoll(now);
    if (lm_stream_stage_at(st, i.frames_read) >= kStages) break;
    if (lm_stream_pull(st, scratch.data(), 4800) == 0) { fprintf(stderr, "history test: the stream stopped\n"); return 1; }
  }
  std::string path = g_hist.livePath;
  HistEnd(now);
  g_hist.worker->Flush(); PollHistory();
  report("every write succeeded", g_hist.writeError.empty());

  hist::Record rec;
  std::string livePath;
  for (auto& e : hist::ScanDir(dir.u8string()).entries) if (e.seed == cfg.seed) { livePath = e.path; break; }  // the newest
  bool loaded = !livePath.empty() && hist::LoadRecord(livePath, rec, err);
  report("the recording is on disk, named by its date and seed", loaded && fs::u8path(livePath).filename().u8string().find("_" + std::to_string(cfg.seed)) != std::string::npos);
  if (!loaded) { lm_stream_free(st); return 1; }
  lm_stream_info(st, &i);
  bool same = rec.stages.size() >= kStages && rec.stages.size() <= i.stages_composed;
  for (size_t k = 0; k < rec.stages.size(); ++k) {
    LmStageInfo si{};
    same = same && lm_stream_stage(st, k, &si) && rec.stages[k].text == hist::NormalizeText(si.text) && rec.stages[k].summary == si.summary;
  }
  report("every stage read back is the text the engine composed, byte for byte", same);
  report("its seed, origin, roster and how far it was heard", rec.seed == cfg.seed && rec.flags == LM_STREAM_GRAMMAR && rec.roster.size() == i.roster_count &&
         rec.heardStages == kStages + 1 && rec.composer == g_hist.version);

  // One stage replayed sounds exactly as the stage played alone.
  char e1[512] = {}, e2[512] = {};
  std::string one = hist::MergeStages(rec, 0, 0, "Replayed", err);
  LmSong* a = lm_load_source(one.c_str(), 0, e1, sizeof(e1));
  LmSong* b = lm_load_source(rec.stages[0].text.c_str(), 0, e2, sizeof(e2));
  bool equal = a && b;
  if (equal) {
    LmInfo ia{}, ib{}; lm_info(a, &ia); lm_info(b, &ib);
    equal = ia.frames == ib.frames && memcmp(lm_audio(a), lm_audio(b), (size_t)ia.frames * 2 * sizeof(float)) == 0;
  } else fprintf(stderr, "  %s %s\n", e1, e2);
  report("one recorded stage replayed is the stage's own audio, sample for sample", equal);
  if (a) lm_free(a);
  if (b) lm_free(b);

  // Two stages replayed: the second plays its own functions (each voice's pitch where both sound).
  std::string two = hist::MergeStages(rec, 0, 1, "Replayed", err);
  LmSong* m = lm_load_source(two.c_str(), 0, e1, sizeof(e1));
  LmSong* s1 = lm_load_source(rec.stages[1].text.c_str(), 0, e2, sizeof(e2));
  size_t compared = 0, differ = 0;
  if (m && s1) {
    LmInfo im{}, is{}; lm_info(m, &im); lm_info(s1, &is);
    double stageSec = rec.stageBeats * 60.0 / rec.bpm;
    for (uint32_t v = 0; v < is.voice_count; ++v) {
      LmVoice vs{}; lm_voice(s1, v, &vs);
      if (vs.kind != LM_TONAL) continue;
      for (uint32_t w = 0; w < im.voice_count; ++w) {
        LmVoice vm{}; lm_voice(m, w, &vm);
        if (strcmp(vm.name, vs.name) != 0) continue;
        const LmFrame* fs1 = lm_frames(s1, v);
        // The merged song's frames sit on its own millisecond grid, which the stage's start (32 beats, not a whole
        // number of milliseconds) does not fall on: lm_sample there returns the frame up to a millisecond before.
        // So the merged pitch is the stage's function at an instant between the stage's frames k - 1 and k, and is
        // checked to lie between them (a continuous glide moves within that millisecond; a wrong function does not).
        for (uint64_t k = 7; k < vs.frame_count; k += 7) {
          double t = (double)k * is.trace_step / is.rate;
          LmFrame fm{};
          if (!lm_sample(m, w, t + stageSec, &fm)) continue;
          const LmFrame& a = fs1[k - 1];
          const LmFrame& b = fs1[k];
          if (!(a.gate > 0 && b.gate > 0 && fm.gate > 0) || !std::isfinite(a.pitch) || !std::isfinite(b.pitch) || !std::isfinite(fm.pitch)) continue;
          ++compared;
          if (fm.pitch < std::min(a.pitch, b.pitch) - 1e-6 || fm.pitch > std::max(a.pitch, b.pitch) + 1e-6) ++differ;
        }
      }
    }
  } else fprintf(stderr, "  %s %s\n", e1, e2);
  printf("  two stages replayed: %zu pitch samples compared, %zu outside the stage's own frames around them\n", compared, differ);
  report("two stages replayed play the second stage's own functions", compared > 100 && differ * 1000 <= compared);
  if (m) lm_free(m);
  if (s1) lm_free(s1);

  // Every stretch of the recording merges into a song the engine plays (the move book's own names, a T(u), its
  // instrument swaps): each stage with the next, and all of them.
  {
    bool all = true;
    std::string why;
    for (size_t a = 0; a < rec.stages.size(); ++a) {
      size_t b = a + 1 < rec.stages.size() ? a + 1 : a;
      for (size_t last : { b, rec.stages.size() - 1 }) {
        if (a > 0 && last == rec.stages.size() - 1 && last != b) continue;  // the whole recording once, from stage 1
        std::string t = hist::MergeStages(rec, a, last, "Replayed", err);
        LmSong* x = t.empty() ? nullptr : lm_load_source(t.c_str(), 0, e1, sizeof(e1));
        if (!x) { all = false; why = "stages " + std::to_string(a + 1) + "-" + std::to_string(last + 1) + ": " + (t.empty() ? err : std::string(e1)); }
        else lm_free(x);
      }
    }
    if (!all) fprintf(stderr, "  %s\n", why.c_str());
    report("every stretch of the recording merges into a song the engine plays", all);
  }

  // Going on from the recording: the seed composes the same stages, so it continues; a changed stage stops it.
  {
    Continuation c; c.cfg = used; c.stage = rec.stages.size();
    PrepareContinuation(c, rec, g_theme);
    if (!c.err.empty()) fprintf(stderr, "  %s\n", c.err.c_str());
    report("the stream goes on from its seed after the recorded stages", c.err.empty() && c.song && c.song->lm);
    hist::Record changed = rec;
    changed.stages[1].text += "# edited\n";
    Continuation d; d.cfg = used; d.stage = 2;
    PrepareContinuation(d, changed, g_theme);
    report("it refuses to go on when the composer writes a recorded stage differently", d.err.find("differently") != std::string::npos);
  }

  // A history folder that cannot be made: the failure is reported, for the screen.
  {
    fs::path blocker = dir / "blocked";
    hist::WriteText(blocker, "a file where the folder should be", false);
    std::string saved = g_hist.dir;
    g_hist.dir = blocker.u8string();
    g_hist.writeError.clear();
    HistBegin(st, used);
    g_hist.worker->Flush(); PollHistory();
    report("a recording that cannot be written is reported on screen", g_hist.writeError.find("not being saved") != std::string::npos);
    g_hist.live.reset(); g_hist.liveStream = nullptr;
    g_hist.dir = saved;
    g_hist.writeError.clear();
  }
  lm_stream_free(st);
  if (keepDir.empty()) fs::remove_all(dir, ec);
  else fs::remove(dir / "blocked", ec);
  printf("history test: %s\n", fails ? "FAILED" : "all ok");
  return fails ? 1 : 0;
}

// Silent test of a replay as the window plays it (volume 0, no window): history entry `path` replays from its first
// stage; every stretch must come in at its seam in order, and after the recording, if its composer is this one, the
// stream must go on live from the next stage, recorded into the same entry.
static int ReplayTest(const std::string& path) {
  bool device = InitAudioEngine();
  if (!device) fprintf(stderr, "replay test: no audio device; the silent clock is used\n");
  g_play.volume = 0;
  g_play.loop = true;
  hist::Record rec; std::string err;
  if (!hist::LoadRecord(path, rec, err)) { fprintf(stderr, "replay test: %s\n", err.c_str()); return 1; }
  g_hist.worker->Flush(); PollHistory();
  bool sameComposer = rec.composer == g_hist.version && !rec.complete;
  size_t last = ReplayLastAllowed(rec);
  double stageSec = rec.stageBeats * 60.0 / rec.bpm, budget = (last + 1) * stageSec + 25;
  printf("replay test: %s, %zu stages to replay (%.0f s)%s\n", path.c_str(), last + 1, (last + 1) * stageSec,
         sameComposer ? ", then it should go on live" : "");
  ReplayStart(path, 0, SIZE_MAX, rec.seed, rec.started);
  auto t0 = std::chrono::steady_clock::now(), prev = t0;
  std::vector<size_t> firsts;
  size_t lastFirst = SIZE_MAX;
  bool wentLive = false, inOrder = true;
  uint64_t liveStage = 0;
  for (;;) {
    auto now = std::chrono::steady_clock::now();
    double wall = std::chrono::duration<double>(now - t0).count(), dt = std::chrono::duration<double>(now - prev).count();
    prev = now;
    PollLoader(); PollHistory(); PollReplay();
    double t = PlayerFrameTime(dt);
    PollStream(t);
    if (g_replay.started && g_replay.playFirst != lastFirst && !g_streaming) {
      if (!firsts.empty() && g_replay.playFirst != firsts.back() + (firsts.size() == 1 ? kReplayFirstChunk : kReplayChunk)) inOrder = false;
      firsts.push_back(g_replay.playFirst); lastFirst = g_replay.playFirst;
      printf("  %.1f s: stages %zu-%zu playing from %.2f s of their song\n", wall, g_replay.playFirst + 1, g_replay.playLast + 1, g_play.now);
    }
    if (g_streaming && !wentLive) {
      wentLive = true; liveStage = StageAtSeconds(t);
      printf("  %.1f s: live from stage %llu (stream time %.2f s), recording into %s\n", wall, (unsigned long long)liveStage + 1, t, g_hist.livePath.c_str());
    }
    if (wentLive && t > StageStartSeconds(liveStage) + 3) break;
    if (!g_replay.active && !g_streaming && g_replay.started) break;  // played to its end
    if (wall > budget) { fprintf(stderr, "replay test: out of time\n"); break; }
    Sleep(4);
  }
  int fails = 0;
  auto report = [&](const char* what, bool ok) { printf("  %-70s %s\n", what, ok ? "ok" : "FAIL"); if (!ok) ++fails; };
  report("every stretch came in, in order, from stage 1", !firsts.empty() && firsts[0] == 0 && inOrder);
  report("the stretches covered every recorded stage", !firsts.empty() && (g_replay.playLast == last || wentLive));
  if (sameComposer) {
    report("then it went on live from the stage after the recording", wentLive && liveStage == rec.stages.size());
    report("recording on into the same entry", wentLive && g_hist.livePath == path);
  } else {
    report("then it stopped (another composer recorded it)", !wentLive);
  }
  StopStream();
  ShutdownReplay();
  g_wholeThreads.Join(true);
  g_hist.worker->Flush(); PollHistory();
  StopAudio();
  printf("replay test: %s\n", fails ? "FAILED" : "all ok");
  return fails ? 1 : 0;
}

// Silent test of seeking (volume 0, no window): after each seek the audio position must be the seek
// point plus the time since, within 50 ms; a looped song must play on across its wrap; a seek past
// the end is held inside the song; with loop off, the voice must end at the song's end.
static int SeekTest() {
  if (!InitAudioEngine()) { fprintf(stderr, "seek test: no audio device\n"); return 1; }
  const double d = g_song.info.duration;
  int fails = 0;
  g_play.volume = 0; g_play.playing = true; g_play.loop = true;
  auto report = [&](const char* what, bool ok, double got, double want) {
    printf("  %-44s %s (position %.3f s, expected %.3f s)\n", what, ok ? "ok  " : "FAIL", got, want);
    if (!ok) ++fails;
  };
  auto playFrom = [&](const char* what, double target, int ms) {
    PlayerSeekTotal(target);
    auto t0 = std::chrono::steady_clock::now();
    Sleep(ms);
    double el = std::chrono::duration<double>(std::chrono::steady_clock::now() - t0).count();
    double got = PlayerAudio(), want = target + el;
    report(what, fabs(got - want) < 0.05, got, want);
  };
  printf("seek test (silent): %.2f s song%s\n", d, g_song.info.looped ? ", a loop" : "");
  playFrom("from the start", 0, 300);
  playFrom("from the middle", d * 0.5, 300);
  if (g_song.info.looped) playFrom("across the loop's wrap (from 0.2 s before it)", d - 0.2, 500);
  // Seeks within the current loop, clamped to the song.
  g_play.now = g_song.info.looped ? 2.5 * d : 0.5 * d;
  double loopBase = g_song.info.looped ? 2 * d : 0;
  PlayerSeekSong(3.0);
  report("a song-time seek stays in the current loop", fabs(g_play.base - (loopBase + 3.0)) < 1e-9, g_play.base, loopBase + 3.0);
  g_play.now = loopBase + 3.0;
  PlayerSeekSong(d + 10);
  report("a seek past the end is held inside the song", fabs(g_play.base - (loopBase + d - ui::kSeekEndMargin)) < 1e-9, g_play.base,
         loopBase + d - ui::kSeekEndMargin);
  // Loop off: the voice ends with the song.
  // (No next song is asked for here: the library's current song is set aside for the check.)
  int current = g_lib.current;
  g_lib.current = -1;
  g_play.loop = false;
  PlayerSeekTotal(d - 0.3);
  Sleep(700);
  PlayerFrameTime(0.0);  // the frame that finds the end
  double endAt = d;  // from d - 0.3, the song ends at d (the first loop's end on a loop)
  report("loop off: it ends, and holds at the song's end", g_play.ended && fabs(PlayerAudio() - endAt) < 0.01, PlayerAudio(), endAt);
  g_lib.current = current;
  StopAudio();
  printf("seek test: %s\n", fails ? "FAILED" : "all ok");
  return fails ? 1 : 0;
}

// ================================================================ the window (globals)

static HWND g_hwnd = nullptr;
static ComPtr<IDXGISwapChain3> g_swap;
static ComPtr<ID3D12Resource> g_back[3];
static HANDLE g_frameWait = nullptr;
static bool g_running = true, g_resize = false, g_fullscreen = false;
static RECT g_windowRect{};
static const UINT kBackBuffers = 3;

static void ToggleFullscreen() {
  if (!g_hwnd) return;
  g_fullscreen = !g_fullscreen;
  if (g_fullscreen) {
    GetWindowRect(g_hwnd, &g_windowRect);
    MONITORINFO mi{ sizeof(mi) }; GetMonitorInfo(MonitorFromWindow(g_hwnd, MONITOR_DEFAULTTONEAREST), &mi);
    SetWindowLongPtr(g_hwnd, GWL_STYLE, WS_POPUP | WS_VISIBLE);
    SetWindowPos(g_hwnd, HWND_TOP, mi.rcMonitor.left, mi.rcMonitor.top, mi.rcMonitor.right - mi.rcMonitor.left,
                 mi.rcMonitor.bottom - mi.rcMonitor.top, SWP_FRAMECHANGED);
  } else {
    SetWindowLongPtr(g_hwnd, GWL_STYLE, WS_OVERLAPPEDWINDOW | WS_VISIBLE);
    SetWindowPos(g_hwnd, nullptr, g_windowRect.left, g_windowRect.top, g_windowRect.right - g_windowRect.left,
                 g_windowRect.bottom - g_windowRect.top, SWP_FRAMECHANGED | SWP_NOZORDER);
  }
}

static void SetWindowTitleFor(const std::string& title) {
  if (!g_hwnd) return;
  std::string t = title.empty() ? std::string("Dreamride") : std::string(u8"Dreamride · ") + title;
  wchar_t w[512];
  if (MultiByteToWideChar(CP_UTF8, 0, t.c_str(), -1, w, 512) > 0) SetWindowTextW(g_hwnd, w);
}

// ================================================================ the menu overlay (Dear ImGui)
//
// Dark, translucent, minimal. A transport bar at the bottom (title, seek bar, controls), a side menu
// (Library, View, Theme; Tab or M), a keys card (H) and a quit question (Esc). All of it fades away
// while a song plays and nothing is touched, and comes back on any input. Sizes are in units of
// pixels at 96 dpi (U), scaled by the monitor's DPI; text is Segoe UI, sized the same way.

struct UiState {
  bool ready = false;      // ImGui initialised
  bool platform = false;   // fed by the window (imgui_impl_win32); headless otherwise
  ui::Overlay ov;
  int tab = 0;             // the menu's tab: 0 library, 1 view, 2 theme
  float alpha = 1;         // the overlay's opacity now
  double idle = 0;         // seconds since the last input
  bool hovered = false;    // the mouse is over the overlay (it stays while pointed at)
  bool hud = true;         // a faint corner caption (song and time) while the overlay is away
  bool wide = false;       // the camera: follow (false) or pull back
  bool dragging = false;   // scrubbing the seek bar ...
  double dragPos = 0;      // ... to this song position (seconds)
  bool volDrag = false;
  float scale = 1;         // DPI scale (1 at 96 dpi)
  int lastMouseX = -100000, lastMouseY = -100000;
  ImFont* font = nullptr;      // Segoe UI (ImGui's own font if it is missing)
  ImFont* fontBold = nullptr;  // Segoe UI Semibold
  ImFont* math = nullptr;      // the live equation: Cambria (upright) ...
  ImFont* mathItalic = nullptr;  // ... and Cambria Italic, Segoe UI if they are missing
  bool capEqOnly = false;      // --capture-menu equation: the equation alone, the rest of the overlay away
  ComPtr<ID3D12DescriptorHeap> heap;
  std::vector<char> slotUsed;
  UINT slotStride = 0;
} g_ui;

static void SetCameraWide(bool wide) { g_ui.wide = wide; }
static bool CameraWide() { return g_ui.wide; }

// The menu's tabs, in the order shown, and the tab each shows (g_ui.tab).
enum { kTabLibrary = 0, kTabView = 1, kTabTheme = 2, kTabHistory = 3 };

static Theme g_themeStart;                  // the theme as launched (LM_THEME applied): Reset goes back here
static const double kIdleSeconds = 2.5;     // still this long while playing, and the overlay fades
static const ImVec4 kText(0.87f, 0.90f, 0.95f, 1), kDim(0.56f, 0.61f, 0.71f, 1);
static const ImVec4 kAccent(0.47f, 0.86f, 0.95f, 1), kWarn(1.0f, 0.56f, 0.50f, 1);

static float U(float x) { return x * g_ui.scale; }
static ImU32 Col(const ImVec4& c, float a = 1) { return ImGui::GetColorU32(ImVec4(c.x, c.y, c.z, c.w * a)); }
static ImU32 White(float a) { return ImGui::GetColorU32(ImVec4(1, 1, 1, a)); }

static void UiSrvAlloc(ImGui_ImplDX12_InitInfo*, D3D12_CPU_DESCRIPTOR_HANDLE* cpu, D3D12_GPU_DESCRIPTOR_HANDLE* gpu) {
  for (size_t i = 0; i < g_ui.slotUsed.size(); ++i) {
    if (g_ui.slotUsed[i]) continue;
    g_ui.slotUsed[i] = 1;
    *cpu = g_ui.heap->GetCPUDescriptorHandleForHeapStart(); cpu->ptr += i * g_ui.slotStride;
    *gpu = g_ui.heap->GetGPUDescriptorHandleForHeapStart(); gpu->ptr += i * g_ui.slotStride;
    return;
  }
  Die("the overlay's descriptor heap is full");
}
static void UiSrvFree(ImGui_ImplDX12_InitInfo*, D3D12_CPU_DESCRIPTOR_HANDLE cpu, D3D12_GPU_DESCRIPTOR_HANDLE) {
  size_t i = (cpu.ptr - g_ui.heap->GetCPUDescriptorHandleForHeapStart().ptr) / g_ui.slotStride;
  if (i < g_ui.slotUsed.size()) g_ui.slotUsed[i] = 0;
}

static void ApplyUiStyle(float scale) {
  ImGuiStyle& st = ImGui::GetStyle();
  st = ImGuiStyle();
  ImGui::StyleColorsDark(&st);
  st.WindowRounding = 14; st.ChildRounding = 10; st.FrameRounding = 7; st.PopupRounding = 10; st.GrabRounding = 7;
  st.ScrollbarRounding = 6; st.WindowBorderSize = 1; st.PopupBorderSize = 1; st.FrameBorderSize = 0; st.ChildBorderSize = 0;
  st.WindowPadding = ImVec2(18, 16); st.FramePadding = ImVec2(10, 5); st.ItemSpacing = ImVec2(10, 8);
  st.ItemInnerSpacing = ImVec2(8, 6); st.ScrollbarSize = 6; st.GrabMinSize = 12;
  ImVec4* c = st.Colors;
  c[ImGuiCol_Text] = kText;
  c[ImGuiCol_TextDisabled] = kDim;
  c[ImGuiCol_WindowBg] = ImVec4(0.030f, 0.036f, 0.060f, 0.80f);
  c[ImGuiCol_ChildBg] = ImVec4(0, 0, 0, 0);
  c[ImGuiCol_PopupBg] = ImVec4(0.040f, 0.047f, 0.075f, 0.95f);
  c[ImGuiCol_Border] = ImVec4(1, 1, 1, 0.07f);
  c[ImGuiCol_FrameBg] = ImVec4(1, 1, 1, 0.07f);
  c[ImGuiCol_FrameBgHovered] = ImVec4(1, 1, 1, 0.11f);
  c[ImGuiCol_FrameBgActive] = ImVec4(1, 1, 1, 0.15f);
  c[ImGuiCol_SliderGrab] = kAccent;
  c[ImGuiCol_SliderGrabActive] = ImVec4(0.75f, 0.96f, 1.0f, 1);
  c[ImGuiCol_CheckMark] = kAccent;
  c[ImGuiCol_Button] = ImVec4(1, 1, 1, 0.07f);
  c[ImGuiCol_ButtonHovered] = ImVec4(1, 1, 1, 0.13f);
  c[ImGuiCol_ButtonActive] = ImVec4(kAccent.x, kAccent.y, kAccent.z, 0.35f);
  c[ImGuiCol_Header] = ImVec4(kAccent.x, kAccent.y, kAccent.z, 0.14f);
  c[ImGuiCol_HeaderHovered] = ImVec4(1, 1, 1, 0.06f);
  c[ImGuiCol_HeaderActive] = ImVec4(kAccent.x, kAccent.y, kAccent.z, 0.22f);
  c[ImGuiCol_Separator] = ImVec4(1, 1, 1, 0.08f);
  c[ImGuiCol_ScrollbarBg] = ImVec4(0, 0, 0, 0);
  c[ImGuiCol_ScrollbarGrab] = ImVec4(1, 1, 1, 0.10f);
  c[ImGuiCol_ScrollbarGrabHovered] = ImVec4(1, 1, 1, 0.18f);
  c[ImGuiCol_ScrollbarGrabActive] = ImVec4(1, 1, 1, 0.26f);
  st.ScaleAllSizes(scale);
  st.FontScaleDpi = scale;
  st.FontSizeBase = 17.0f;
}

static void InitUi(HWND hwnd, float scale) {
  IMGUI_CHECKVERSION();
  ImGui::CreateContext();
  ImGuiIO& io = ImGui::GetIO();
  io.IniFilename = nullptr;  // nothing written next to the exe
  io.LogFilename = nullptr;
  // Keyboard navigation stays off: the world's shortcuts own the keys (Space must not press a button).
  D3D12_DESCRIPTOR_HEAP_DESC hd{ D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV, 16, D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE, 0 };
  CHECK(g_device->CreateDescriptorHeap(&hd, IID_PPV_ARGS(&g_ui.heap)), "overlay descriptor heap");
  g_ui.slotStride = g_device->GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV);
  g_ui.slotUsed.assign(16, 0);
  if (hwnd) {
    if (!ImGui_ImplWin32_Init(hwnd)) Die("ImGui win32 backend");
    g_ui.platform = true;
  }
  ImGui_ImplDX12_InitInfo info;
  info.Device = g_device.Get();
  info.CommandQueue = g_queue.Get();
  info.NumFramesInFlight = kFrames;
  info.RTVFormat = kLdrFmt;
  info.DSVFormat = DXGI_FORMAT_UNKNOWN;
  info.SrvDescriptorHeap = g_ui.heap.Get();
  info.SrvDescriptorAllocFn = UiSrvAlloc;
  info.SrvDescriptorFreeFn = UiSrvFree;
  if (!ImGui_ImplDX12_Init(&info)) Die("ImGui D3D12 backend");
  // Segoe UI ships with Windows 10 and 11; without it the overlay uses ImGui's own font.
  char win[MAX_PATH] = {};
  UINT n = GetWindowsDirectoryA(win, MAX_PATH);
  std::string fonts = (n > 0 && n < MAX_PATH) ? std::string(win) + "\\Fonts\\" : std::string();
  auto load = [&](const char* file) -> ImFont* {
    std::string p = fonts + file;
    if (fonts.empty() || GetFileAttributesA(p.c_str()) == INVALID_FILE_ATTRIBUTES) {
      fprintf(stderr, "overlay: %s not found, using ImGui's own font\n", p.c_str());
      return nullptr;
    }
    return io.Fonts->AddFontFromFileTTF(p.c_str(), 17.0f);
  };
  g_ui.font = load("segoeui.ttf");
  if (!g_ui.font) g_ui.font = io.Fonts->AddFontDefault();
  g_ui.fontBold = load("seguisb.ttf");
  if (!g_ui.fontBold) g_ui.fontBold = g_ui.font;
  // The equation's faces: Cambria (cambria.ttc's first face) and Cambria Italic, which ship with Windows.
  g_ui.math = load("cambria.ttc");
  if (!g_ui.math) g_ui.math = g_ui.font;
  g_ui.mathItalic = load("cambriai.ttf");
  if (!g_ui.mathItalic) { g_ui.mathItalic = load("segoeuii.ttf"); if (!g_ui.mathItalic) g_ui.mathItalic = g_ui.math; }
  io.FontDefault = g_ui.font;
  g_ui.scale = scale;
  ApplyUiStyle(scale);
  g_ui.ready = true;
}

static void ShutdownUi() {
  if (!g_ui.ready) return;
  WaitIdle();
  ImGui_ImplDX12_Shutdown();
  if (g_ui.platform) ImGui_ImplWin32_Shutdown();
  ImGui::DestroyContext();
  g_ui.ready = false;
}

static void RecordUi() {
  ImDrawData* dd = g_ui.ready ? ImGui::GetDrawData() : nullptr;
  if (!dd || !dd->Valid) return;
  ID3D12DescriptorHeap* heaps[] = { g_ui.heap.Get() };
  g_cmd->SetDescriptorHeaps(1, heaps);
  ImGui_ImplDX12_RenderDrawData(dd, g_cmd.Get());
}

// ---------------------------------------------------------------- drawing helpers

// A triangle filled with its corners in clockwise screen order (ImGui's anti-aliasing wants it).
static void Tri(ImDrawList* dl, ImVec2 a, ImVec2 b, ImVec2 c, ImU32 col) {
  float cross = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
  if (cross < 0) std::swap(b, c);
  dl->AddTriangleFilled(a, b, c, col);
}

enum class Icon { Play, Pause, Prev, Next, Restart, Loop, Menu, Volume, Close };

// Icons drawn as shapes, crisp at any DPI: centre c, radius r.
static void DrawIcon(ImDrawList* dl, Icon ic, ImVec2 c, float r, ImU32 col, float vol = 1) {
  const float t = std::max(1.2f, r * 0.17f);
  const float pi = 3.14159265f;
  switch (ic) {
    case Icon::Play: Tri(dl, ImVec2(c.x - r * 0.55f, c.y - r * 0.78f), ImVec2(c.x + r * 0.85f, c.y), ImVec2(c.x - r * 0.55f, c.y + r * 0.78f), col); break;
    case Icon::Pause: {
      float w = r * 0.30f, g = r * 0.22f;
      dl->AddRectFilled(ImVec2(c.x - g - w, c.y - r * 0.72f), ImVec2(c.x - g, c.y + r * 0.72f), col, w * 0.35f);
      dl->AddRectFilled(ImVec2(c.x + g, c.y - r * 0.72f), ImVec2(c.x + g + w, c.y + r * 0.72f), col, w * 0.35f);
    } break;
    case Icon::Prev: case Icon::Next: {
      float s = ic == Icon::Next ? 1.0f : -1.0f;
      Tri(dl, ImVec2(c.x - s * r * 0.62f, c.y - r * 0.62f), ImVec2(c.x + s * r * 0.38f, c.y), ImVec2(c.x - s * r * 0.62f, c.y + r * 0.62f), col);
      float bx = c.x + s * r * 0.55f;
      dl->AddRectFilled(ImVec2(bx - t * 0.6f, c.y - r * 0.62f), ImVec2(bx + t * 0.6f, c.y + r * 0.62f), col, t * 0.3f);
    } break;
    case Icon::Restart: {
      float a0 = -pi * 0.30f, a1 = pi * 1.25f, rr = r * 0.68f;
      dl->PathArcTo(c, rr, a0, a1, 24);
      dl->PathStroke(col, 0, t);
      ImVec2 e(c.x + cosf(a0) * rr, c.y + sinf(a0) * rr);
      Tri(dl, ImVec2(e.x - r * 0.38f, e.y - r * 0.12f), ImVec2(e.x + r * 0.18f, e.y - r * 0.40f), ImVec2(e.x + r * 0.10f, e.y + r * 0.22f), col);
    } break;
    case Icon::Loop: {
      float w = r * 0.80f, h = r * 0.48f;
      dl->AddRect(ImVec2(c.x - w, c.y - h), ImVec2(c.x + w, c.y + h), col, h * 0.9f, 0, t);
      Tri(dl, ImVec2(c.x + r * 0.05f, c.y - h - r * 0.26f), ImVec2(c.x + r * 0.42f, c.y - h), ImVec2(c.x + r * 0.05f, c.y - h + r * 0.26f), col);
      Tri(dl, ImVec2(c.x - r * 0.05f, c.y + h - r * 0.26f), ImVec2(c.x - r * 0.42f, c.y + h), ImVec2(c.x - r * 0.05f, c.y + h + r * 0.26f), col);
    } break;
    case Icon::Menu:
      for (int k = -1; k <= 1; ++k) dl->AddLine(ImVec2(c.x - r * 0.7f, c.y + k * r * 0.5f), ImVec2(c.x + r * 0.7f, c.y + k * r * 0.5f), col, t);
      break;
    case Icon::Volume: {
      float x0 = c.x - r * 0.85f;
      dl->AddRectFilled(ImVec2(x0, c.y - r * 0.28f), ImVec2(x0 + r * 0.38f, c.y + r * 0.28f), col, t * 0.3f);
      dl->AddQuadFilled(ImVec2(x0 + r * 0.30f, c.y - r * 0.28f), ImVec2(x0 + r * 0.80f, c.y - r * 0.68f),
                        ImVec2(x0 + r * 0.80f, c.y + r * 0.68f), ImVec2(x0 + r * 0.30f, c.y + r * 0.28f), col);
      ImVec2 o(x0 + r * 0.80f, c.y);
      if (vol <= 0.001f) {
        dl->AddLine(ImVec2(o.x + r * 0.30f, c.y - r * 0.30f), ImVec2(o.x + r * 0.80f, c.y + r * 0.30f), col, t);
        dl->AddLine(ImVec2(o.x + r * 0.30f, c.y + r * 0.30f), ImVec2(o.x + r * 0.80f, c.y - r * 0.30f), col, t);
      } else {
        for (int k = 0; k < (vol > 0.5f ? 2 : 1); ++k) {
          dl->PathArcTo(o, r * (0.42f + 0.38f * k), -pi * 0.28f, pi * 0.28f, 12);
          dl->PathStroke(col, 0, t);
        }
      }
    } break;
    case Icon::Close:
      dl->AddLine(ImVec2(c.x - r * 0.55f, c.y - r * 0.55f), ImVec2(c.x + r * 0.55f, c.y + r * 0.55f), col, t);
      dl->AddLine(ImVec2(c.x - r * 0.55f, c.y + r * 0.55f), ImVec2(c.x + r * 0.55f, c.y - r * 0.55f), col, t);
      break;
  }
}

static void Tooltip(const char* text) {
  ImGui::PushStyleVar(ImGuiStyleVar_WindowPadding, ImVec2(U(10), U(6)));
  ImGui::SetTooltip("%s", text);
  ImGui::PopStyleVar();
}

static bool IconButton(const char* id, Icon ic, float size, bool on = false, const char* tip = nullptr, float vol = 1) {
  ImVec2 p = ImGui::GetCursorScreenPos();
  bool clicked = ImGui::InvisibleButton(id, ImVec2(size, size));
  bool hov = ImGui::IsItemHovered(), held = ImGui::IsItemActive();
  ImDrawList* dl = ImGui::GetWindowDrawList();
  ImVec2 c(p.x + size * 0.5f, p.y + size * 0.5f);
  if (hov || held) dl->AddCircleFilled(c, size * 0.5f, White(held ? 0.15f : 0.08f), 32);
  DrawIcon(dl, ic, c, size * 0.30f, on ? Col(kAccent) : (hov ? White(1) : Col(kText)), vol);
  if (hov && tip) Tooltip(tip);
  return clicked;
}

// A button showing text centred in a circle (the help button's "?").
static bool GlyphButton(const char* id, const char* glyph, float size, const char* tip) {
  ImVec2 p = ImGui::GetCursorScreenPos();
  bool clicked = ImGui::InvisibleButton(id, ImVec2(size, size));
  bool hov = ImGui::IsItemHovered(), held = ImGui::IsItemActive();
  ImDrawList* dl = ImGui::GetWindowDrawList();
  ImVec2 c(p.x + size * 0.5f, p.y + size * 0.5f);
  if (hov || held) dl->AddCircleFilled(c, size * 0.5f, White(held ? 0.15f : 0.08f), 32);
  ImGui::PushFont(g_ui.fontBold, 17.0f);
  ImVec2 ts = ImGui::CalcTextSize(glyph);
  dl->AddText(ImVec2(c.x - ts.x * 0.5f, c.y - ts.y * 0.5f), hov ? White(1) : Col(kText), glyph);
  ImGui::PopFont();
  if (hov && tip) Tooltip(tip);
  return clicked;
}

// One line of text in the current font, cut with an ellipsis to `width` (the width available if 0).
static void EllipsisLine(const std::string& s, const ImVec4& col, float width = 0) {
  ImVec2 p = ImGui::GetCursorScreenPos();
  float w = width > 0 ? width : ImGui::GetContentRegionAvail().x, h = ImGui::GetTextLineHeight();
  ImGui::PushStyleColor(ImGuiCol_Text, col);
  ImGui::RenderTextEllipsis(ImGui::GetWindowDrawList(), p, ImVec2(p.x + w, p.y + h), p.x + w, s.c_str(), s.c_str() + s.size(), nullptr);
  ImGui::PopStyleColor();
  ImGui::Dummy(ImVec2(w, h));
}

// `text` wrapped into at most maxLines lines of `width` at `pos`, the last cut with an ellipsis, in
// the current font. True when it was cut.
static bool ClampedText(ImDrawList* dl, ImVec2 pos, float width, const std::string& text, int maxLines, const ImVec4& col) {
  ImFont* f = ImGui::GetFont();
  float size = ImGui::GetFontSize(), y = pos.y;
  const char* s = text.c_str();
  const char* end = s + text.size();
  ImGui::PushStyleColor(ImGuiCol_Text, col);
  bool cut = false;
  for (int line = 0; line < maxLines && s < end; ++line, y += size * 1.22f) {
    if (line == maxLines - 1) {
      cut = f->CalcTextSizeA(size, FLT_MAX, 0, s, end).x > width;
      ImGui::RenderTextEllipsis(dl, ImVec2(pos.x, y), ImVec2(pos.x + width, y + size * 1.2f), pos.x + width, s, end, nullptr);
      break;
    }
    const char* e = f->CalcWordWrapPosition(size, s, end, width);
    if (e <= s) e = s + 1;
    dl->AddText(f, size, ImVec2(pos.x, y), ImGui::GetColorU32(ImGuiCol_Text), s, e);
    s = e;
    while (s < end && *s == ' ') ++s;
  }
  ImGui::PopStyleColor();
  return cut;
}

// A thin horizontal track (the seek bar, the volume slider), filled to `frac`, with a knob.
static void DrawTrack(ImDrawList* dl, ImVec2 p, float w, float h, float frac, bool hot, float knob) {
  float y = p.y + h * 0.5f, th = hot ? U(4.5f) : U(3);
  frac = std::clamp(frac, 0.0f, 1.0f);
  dl->AddRectFilled(ImVec2(p.x, y - th * 0.5f), ImVec2(p.x + w, y + th * 0.5f), White(0.14f), th * 0.5f);
  dl->AddRectFilled(ImVec2(p.x, y - th * 0.5f), ImVec2(p.x + w * frac, y + th * 0.5f), Col(kAccent), th * 0.5f);
  if (knob > 0) dl->AddCircleFilled(ImVec2(p.x + w * frac, y), hot ? knob * 1.3f : knob, White(0.95f), 24);
}

static double DragTotal() {
  double d = g_song.info.duration;
  double loops = g_song.info.looped ? floor(std::max(g_play.now, 0.0) / d) : 0.0;
  return loops * d + g_ui.dragPos;
}

static void SeekBar(float width) {
  ImGuiIO& io = ImGui::GetIO();
  ImVec2 p = ImGui::GetCursorScreenPos();
  float h = U(22);
  ImGui::InvisibleButton("##seek", ImVec2(width, h));
  bool hov = ImGui::IsItemHovered(), act = ImGui::IsItemActive();
  double d = g_song.info.duration, cur = SongSecondsNow();
  ImDrawList* dl = ImGui::GetWindowDrawList();
  if (g_streaming) {
    // A stream cannot be seeked (what lies ahead is not written yet): the bar shows how far the playing stage has gone.
    uint64_t k = StageAtSeconds(g_play.now);
    double a = StageStartSeconds(k), b = StageStartSeconds(k + 1);
    DrawTrack(dl, p, width, h, b > a ? (float)((g_play.now - a) / (b - a)) : 0.0f, false, 0);
    if (hov) Tooltip(("Stage " + std::to_string(k + 1) + u8" of an endless stream").c_str());
    return;
  }
  if (!HaveSong()) {
    // Nothing to seek: while the first song renders, a slow light runs along the empty track.
    DrawTrack(dl, p, width, h, 0, false, 0);
    if (g_loader.busy) {
      float ph = (float)fmod(ImGui::GetTime() * 0.45, 1.0), y = p.y + h * 0.5f;
      float a = p.x + width * std::max(0.0f, ph - 0.18f), b = p.x + width * ph;
      dl->AddRectFilled(ImVec2(a, y - U(1.5f)), ImVec2(b, y + U(1.5f)), Col(kAccent, 0.8f), U(1.5f));
    }
    return;
  }
  auto at = [&](float x) { return ui::ClampSeek((double)std::clamp((x - p.x) / width, 0.0f, 1.0f) * d, d); };
  if (act) { g_ui.dragging = true; g_ui.dragPos = at(io.MousePos.x); }
  if (ImGui::IsItemDeactivated() && g_ui.dragging) { g_ui.dragging = false; PlayerSeekSong(g_ui.dragPos); }
  if (hov && io.MouseWheel != 0) PlayerSeekSong(cur + io.MouseWheel * ui::kWheelSeekStep);
  double shown = g_ui.dragging ? g_ui.dragPos : cur;
  DrawTrack(dl, p, width, h, (float)(shown / d), hov || act, U(5.5f));
  if (hov || act) Tooltip(ui::FormatTime(act ? g_ui.dragPos : at(io.MousePos.x)).c_str());
}

static void VolumeSlider(float width) {
  ImGuiIO& io = ImGui::GetIO();
  ImVec2 p = ImGui::GetCursorScreenPos();
  float h = U(22);
  ImGui::InvisibleButton("##volume", ImVec2(width, h));
  bool hov = ImGui::IsItemHovered(), act = ImGui::IsItemActive();
  g_ui.volDrag = act;
  if (act) PlayerSetVolume(std::clamp((io.MousePos.x - p.x) / width, 0.0f, 1.0f));
  if (hov && io.MouseWheel != 0) PlayerSetVolume(ui::StepVolume(g_play.volume, io.MouseWheel > 0 ? +1 : -1));
  DrawTrack(ImGui::GetWindowDrawList(), p, width, h, g_play.volume, hov || act, U(4.5f));
  if (hov || act) { char b[32]; snprintf(b, sizeof(b), "Volume %d%%", (int)std::lround(g_play.volume * 100)); Tooltip(b); }
}

static const ImGuiWindowFlags kPanelFlags = ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoMove |
  ImGuiWindowFlags_NoCollapse | ImGuiWindowFlags_NoSavedSettings | ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoScrollWithMouse |
  ImGuiWindowFlags_NoFocusOnAppearing | ImGuiWindowFlags_NoNav;

static std::string LoadingSeconds() {
  double s = std::chrono::duration<double>(std::chrono::steady_clock::now() - g_loader.start).count();
  char b[32]; snprintf(b, sizeof(b), "%.0f s", std::max(0.0, s));
  return b;
}

// ---------------------------------------------------------------- the transport bar

static void DrawTransport(float panelRight) {
  ImGuiIO& io = ImGui::GetIO();
  float W = io.DisplaySize.x, H = io.DisplaySize.y, m = U(20);
  float left = panelRight > 0 ? panelRight + m : m;  // beside the menu when it is open
  float avail = std::max(U(200), W - left - m);
  float w = std::min(U(860), avail);
  ImGui::SetNextWindowPos(ImVec2(left + avail * 0.5f, H - m), ImGuiCond_Always, ImVec2(0.5f, 1.0f));
  ImGui::SetNextWindowSize(ImVec2(w, 0));
  ImGui::Begin("##transport", nullptr, kPanelFlags);
  float cw = ImGui::GetContentRegionAvail().x;
  float x0 = ImGui::GetCursorPosX();

  // The title, and the time on the right.
  std::string title = HaveSong() ? g_lib.currentTitle : std::string("Dreamride");
  std::string time = HaveSong() ? ui::FormatTime(g_ui.dragging ? g_ui.dragPos : SongSecondsNow()) + "  /  " + ui::FormatTime(g_song.info.duration)
                                : std::string();
  if (g_streaming) time = "stage " + std::to_string(StageAtSeconds(g_play.now) + 1) + u8"  ·  " + ui::FormatTime(g_play.now);
  ImGui::PushFont(nullptr, 15.0f);
  float tw = time.empty() ? 0 : ImGui::CalcTextSize(time.c_str()).x;
  ImGui::PopFont();
  float rowY = ImGui::GetCursorPosY();
  ImGui::PushFont(g_ui.fontBold, 19.0f);
  EllipsisLine(title, kText, cw - tw - U(16));
  float titleH = ImGui::GetTextLineHeight();
  ImGui::PopFont();
  if (!time.empty()) {
    float after = ImGui::GetCursorPosY();
    ImGui::PushFont(nullptr, 15.0f);
    ImGui::SetCursorPos(ImVec2(x0 + cw - tw, rowY + (titleH - ImGui::GetTextLineHeight()) * 0.5f));
    ImGui::TextColored(kDim, "%s", time.c_str());
    ImGui::PopFont();
    ImGui::SetCursorPosY(after);
  }

  // What it is, what is rendering, or what went wrong.
  ImGui::PushFont(nullptr, 14.5f);
  if (g_loader.busy) {
    EllipsisLine(std::string(u8"Rendering ") + (g_loader.queued ? g_loader.qTitle : g_loader.title) + u8"…  " + LoadingSeconds(), kAccent);
  } else if (g_replay.active && (!g_replay.started || g_replay.waiting)) {
    std::string what = !g_replay.started ? u8"Replay: rendering the first stages…"
                     : g_replay.next <= g_replay.to ? u8"Replay: rendering the next stages…"
                     : (g_replay.cont ? u8"Replay: catching the composer up to stage " +
                                            std::to_string(g_replay.cont->progress.load() + 1) + u8" to go on from the recording…"
                                      : std::string(u8"Replay: going on…"));
    EllipsisLine(what, kAccent);
  } else if (g_loader.failedIndex != -2) {
    EllipsisLine("Could not load " + g_loader.failedTitle + ": " + g_loader.failedError, kWarn);
  } else if (HaveSong()) {
    EllipsisLine(g_lib.currentDescription.empty() ? std::string(" ") : g_lib.currentDescription, kDim);
  } else {
    EllipsisLine(u8"Choose a song from the library  ·  Tab", kDim);
  }
  ImGui::PopFont();

  SeekBar(cw);

  // The controls: previous, play/pause, next, restart on the left; loop, volume, menu, help right.
  const float big = U(46), sm = U(36);
  float row = ImGui::GetCursorPosY();
  auto smallRow = [&]() { ImGui::SetCursorPosY(row + (big - sm) * 0.5f); };
  smallRow();
  if (IconButton("##prev", Icon::Prev, sm, false, "Previous song  (P)")) RequestSong(ui::StepIndex(LibraryIndexNow(), (int)g_lib.songs.size(), -1));
  ImGui::SameLine();
  ImGui::SetCursorPosY(row);
  if (IconButton("##play", g_play.playing ? Icon::Pause : Icon::Play, big, false, g_play.playing ? "Pause  (Space)" : "Play  (Space)")) {
    if (HaveSong()) PlayerSetPlaying(!g_play.playing);
    else if (!g_loader.busy) RequestSong(g_lib.current >= 0 ? g_lib.current : 0);
  }
  ImGui::SameLine(); smallRow();
  if (IconButton("##next", Icon::Next, sm, false, "Next song  (N)")) RequestSong(ui::StepIndex(LibraryIndexNow(), (int)g_lib.songs.size(), +1));
  ImGui::SameLine(); smallRow();
  if (IconButton("##restart", Icon::Restart, sm, false, g_streaming ? "Start the stream again  (R)" : "Restart  (R)") && HaveSong()) {
    if (g_streaming) RestartStream(); else PlayerSeekTotal(0);
  }

  float volW = U(96), sp = ImGui::GetStyle().ItemSpacing.x;
  float rightW = sm + sp + sm + U(4) + volW + U(18) + sm + sp + sm;
  ImGui::SameLine(x0 + cw - rightW);
  smallRow();
  if (IconButton("##loop", Icon::Loop, sm, g_play.loop, g_play.loop ? "Loop: on  (L)" : "Loop: off, then the next song  (L)")) PlayerToggleLoop();
  ImGui::SameLine(); smallRow();
  static float beforeMute = 1.0f;
  if (IconButton("##mute", Icon::Volume, sm, false, g_play.volume > 0 ? "Mute" : "Unmute", g_play.volume)) {
    if (g_play.volume > 0) { beforeMute = g_play.volume; PlayerSetVolume(0); } else PlayerSetVolume(beforeMute > 0 ? beforeMute : 1.0f);
  }
  ImGui::SameLine(0, U(4));
  ImGui::SetCursorPosY(row + (big - U(22)) * 0.5f);
  VolumeSlider(volW);
  ImGui::SameLine(0, U(18)); smallRow();
  if (IconButton("##menu", Icon::Menu, sm, g_ui.ov.menu, "Menu  (Tab)")) g_ui.ov.menu = !g_ui.ov.menu;
  ImGui::SameLine(); smallRow();
  if (GlyphButton("##help", "?", sm, "Keys  (H)")) g_ui.ov.help = !g_ui.ov.help;
  ImGui::End();
}

// ---------------------------------------------------------------- the menu: library, view, theme

static void SectionLabel(const char* t) {
  ImGui::PushFont(g_ui.fontBold, 12.5f);
  ImGui::TextColored(kDim, "%s", t);
  ImGui::PopFont();
}

static void DrawLibrary() {
  if (g_lib.songs.empty()) {
    ImGui::PushTextWrapPos(0);
    ImGui::TextColored(kDim, "No songs found. Put .txt songs in examples\\ or start Dreamride with --songs <folder>.");
    ImGui::PopTextWrapPos();
  }
  ImDrawList* dl = ImGui::GetWindowDrawList();
  ImGui::PushFont(g_ui.fontBold, 16.0f);
  float titleH = ImGui::GetFontSize();
  ImGui::PopFont();
  ImGui::PushFont(nullptr, 14.0f);
  float lineH = ImGui::GetFontSize() * 1.22f;
  ImGui::PopFont();
  for (int i = 0; i < (int)g_lib.songs.size(); ++i) {
    const ui::SongEntry& e = g_lib.songs[i];
    bool cur = i == g_lib.current;
    bool rendering = g_loader.busy && (g_loader.queued ? g_loader.qIndex == i : g_loader.index == i);
    bool skipped = g_loader.busy && g_loader.queued && g_loader.index == i;  // to be dropped for a later ask
    bool failed = !g_loader.busy && g_loader.failedIndex == i;
    float w = ImGui::GetContentRegionAvail().x, pad = U(12);
    float h = pad + titleH + U(4) + lineH * 2 + pad * 0.6f;
    ImVec2 p = ImGui::GetCursorScreenPos();
    ImGui::PushID(i);
    bool click = ImGui::InvisibleButton("##row", ImVec2(w, h));
    bool hov = ImGui::IsItemHovered();
    ImGui::PopID();
    if (cur) dl->AddRectFilled(p, ImVec2(p.x + w, p.y + h), Col(kAccent, 0.09f), U(10));
    else if (hov || rendering) dl->AddRectFilled(p, ImVec2(p.x + w, p.y + h), White(0.05f), U(10));
    if (cur) dl->AddRectFilled(ImVec2(p.x + U(1), p.y + U(14)), ImVec2(p.x + U(4), p.y + h - U(14)), Col(kAccent), U(2));

    // Its state, on the right of the title.
    std::string badge; ImVec4 badgeCol = kDim;
    if (rendering) { int dots = (int)(ImGui::GetTime() * 2.5) % 4; badge = "rendering" + std::string(dots, '.') + std::string(3 - dots, ' '); badgeCol = kAccent; }
    else if (skipped) badge = "skipped";
    else if (failed) { badge = "failed"; badgeCol = kWarn; }
    else if (e.isNew) { badge = "new"; badgeCol = kAccent; }
    else if (cur) badge = g_play.playing ? "playing" : (g_play.ended ? "ended" : "paused");
    ImGui::PushFont(nullptr, 13.5f);
    float bw = badge.empty() ? 0 : ImGui::CalcTextSize("rendering...").x;
    if (!badge.empty()) dl->AddText(ImVec2(p.x + w - pad - ImGui::CalcTextSize(badge.c_str()).x, p.y + pad + U(2)), Col(badgeCol), badge.c_str());
    ImGui::PopFont();

    ImGui::PushFont(g_ui.fontBold, 16.0f);
    ImGui::PushStyleColor(ImGuiCol_Text, cur ? kAccent : (hov ? ImVec4(1, 1, 1, 1) : kText));
    float tx = p.x + pad + U(4), tmax = p.x + w - pad - bw - U(8);
    ImGui::RenderTextEllipsis(dl, ImVec2(tx, p.y + pad), ImVec2(tmax, p.y + pad + titleH), tmax, e.title.c_str(), e.title.c_str() + e.title.size(), nullptr);
    ImGui::PopStyleColor();
    ImGui::PopFont();

    ImGui::PushFont(nullptr, 14.0f);
    const std::string& desc = failed ? g_loader.failedError : e.description;
    bool cut = ClampedText(dl, ImVec2(tx, p.y + pad + titleH + U(4)), w - 2 * pad - U(4),
                           desc.empty() ? std::string(u8"—") : desc, 2, failed ? kWarn : kDim);
    ImGui::PopFont();
    if (hov && cut) {
      ImGui::PushStyleVar(ImGuiStyleVar_WindowPadding, ImVec2(U(12), U(8)));
      ImGui::BeginTooltip();
      ImGui::PushTextWrapPos(U(420));
      ImGui::TextUnformatted(desc.c_str());
      ImGui::PopTextWrapPos();
      ImGui::EndTooltip();
      ImGui::PopStyleVar();
    }
    if (click) RequestSong(i);
    ImGui::Dummy(ImVec2(0, U(2)));
  }
  ImGui::Dummy(ImVec2(0, U(6)));
  ImGui::Separator();
  ImGui::PushFont(nullptr, 13.0f);
  ImGui::PushTextWrapPos(0);
  for (auto& d : g_lib.dirs) ImGui::TextColored(kDim, "%s", d.c_str());
  for (auto& e : g_lib.errors) ImGui::TextColored(kWarn, "%s", e.c_str());
  ImGui::PopTextWrapPos();
  ImGui::PopFont();
}

// A choice drawn as a segmented control. Returns the index picked (or `cur`).
static int Segmented(const char* id, const char* const* items, int count, int cur) {
  ImGui::PushID(id);
  float w = ImGui::GetContentRegionAvail().x, h = U(34), each = w / count;
  ImVec2 p = ImGui::GetCursorScreenPos();
  ImDrawList* dl = ImGui::GetWindowDrawList();
  dl->AddRectFilled(p, ImVec2(p.x + w, p.y + h), White(0.05f), U(9));
  int picked = cur;
  for (int k = 0; k < count; ++k) {
    ImVec2 q(p.x + each * k, p.y);
    ImGui::SetCursorScreenPos(q);
    ImGui::PushID(k);
    if (ImGui::InvisibleButton("##seg", ImVec2(each, h))) picked = k;
    bool hov = ImGui::IsItemHovered();
    ImGui::PopID();
    if (k == cur) dl->AddRectFilled(ImVec2(q.x + U(3), q.y + U(3)), ImVec2(q.x + each - U(3), q.y + h - U(3)), Col(kAccent, 0.18f), U(7));
    ImVec2 ts = ImGui::CalcTextSize(items[k]);
    dl->AddText(ImVec2(q.x + (each - ts.x) * 0.5f, q.y + (h - ts.y) * 0.5f), k == cur ? Col(kAccent) : (hov ? White(1) : Col(kText)), items[k]);
  }
  ImGui::SetCursorScreenPos(ImVec2(p.x, p.y + h));
  ImGui::Dummy(ImVec2(w, 0));
  ImGui::PopID();
  return picked;
}

static void Note(const char* t) {
  ImGui::PushFont(nullptr, 13.5f);
  ImGui::PushTextWrapPos(0);
  ImGui::TextColored(kDim, "%s", t);
  ImGui::PopTextWrapPos();
  ImGui::PopFont();
}

static void DrawView() {
  SectionLabel("CAMERA");
  const char* cams[] = { "Follow", "Pull back" };
  g_ui.wide = Segmented("cam", cams, 2, g_ui.wide ? 1 : 0) == 1;
  Note("Follow rides with the present. Pull back lifts up and away to show the trail the song has left.");
  ImGui::Dummy(ImVec2(0, U(10)));
  SectionLabel("WINDOW");
  if (!g_ui.platform) ImGui::BeginDisabled();
  bool fs = g_fullscreen;
  if (ImGui::Checkbox("Fullscreen  (F11)", &fs)) ToggleFullscreen();
  if (!g_ui.platform) ImGui::EndDisabled();
  ImGui::Checkbox("HUD: song and time in the corner", &g_ui.hud);
  Note("The HUD stays, faintly, while the rest of this overlay fades away during a song.");
  ImGui::Dummy(ImVec2(0, U(10)));
  SectionLabel("WHEN A STREAM ENDS");
  ImGui::Checkbox("Start the next song", &g_autoNext);
  Note("A stream that ends rings out, the camera lifts back over the whole trail, and after a breath a new song "
       "begins. Off, it waits on the ending card.");
}

// A theme value: its name, a slider (no text over the knob), and the value in its own column.
static void ThemeSlider(const char* label, float* v, float lo, float hi, const char* fmt, ImGuiSliderFlags flags = 0) {
  float x0 = ImGui::GetCursorPosX(), cw = ImGui::GetContentRegionAvail().x;
  float labelW = U(140), valueW = U(58);
  ImGui::AlignTextToFramePadding();
  ImGui::TextUnformatted(label);
  ImGui::SameLine(x0 + labelW);
  ImGui::SetNextItemWidth(cw - labelW - valueW);
  ImGui::PushID(label);
  ImGui::SliderFloat("##s", v, lo, hi, "", flags | ImGuiSliderFlags_AlwaysClamp | ImGuiSliderFlags_NoRoundToFormat);
  ImGui::PopID();
  char b[32];
  snprintf(b, sizeof(b), fmt, *v);
  ImGui::SameLine(x0 + cw - ImGui::CalcTextSize(b).x);
  ImGui::TextColored(kDim, "%s", b);
}


static void DrawThemeTab() {
  Theme& t = g_theme;
  SectionLabel("THE LOOK");
  ImGui::PushStyleVar(ImGuiStyleVar_FramePadding, ImVec2(U(8), U(4)));
  ThemeSlider("Bloom", &t.bloom, 0.0f, 0.5f, "%.2f");
  ThemeSlider("Fog density", &t.fogDensity, 0.25f, 3.0f, "%.2fx");
  ThemeSlider("Floor roughness", &t.floorRough, 0.0005f, 0.05f, "%.4f", ImGuiSliderFlags_Logarithmic);
  ThemeSlider("Glow", &t.glow, 0.25f, 3.0f, "%.2fx");
  ThemeSlider("Camera breathing", &t.breathe, 0.0f, 2.0f, "%.2fx");
  ThemeSlider("Dust", &t.dustBright, 0.0f, 6.0f, "%.1f");
  ThemeSlider("Bass on the water", &t.bassWaveAmp, 0.0f, 0.06f, "%.3f m");
  ThemeSlider("Bass ripple size", &t.bassWaveScale, 20.0f, 300.0f, "%.0fx", ImGuiSliderFlags_Logarithmic);
  ThemeSlider("Bass ripple decay", &t.bassWaveDecay, 0.2f, 4.0f, "%.2f s", ImGuiSliderFlags_Logarithmic);
  ImGui::PopStyleVar();
  ImGui::Dummy(ImVec2(0, U(6)));
  if (ImGui::Button("Reset to defaults", ImVec2(-FLT_MIN, U(34)))) {
    const Theme& d = g_themeStart;
    t.bloom = d.bloom; t.fogDensity = d.fogDensity; t.floorRough = d.floorRough;
    t.glow = d.glow; t.breathe = d.breathe; t.dustBright = d.dustBright;
    t.bassWaveAmp = d.bassWaveAmp; t.bassWaveScale = d.bassWaveScale; t.bassWaveDecay = d.bassWaveDecay;
  }
  Note("Changes apply live, for this run. To keep a look, set it with LM_THEME (README, \"The world\").");
}

// ---------------------------------------------------------------- the History tab

// A five-pointed star, filled or outlined.
static void DrawStar(ImDrawList* dl, ImVec2 c, float r, ImU32 col, bool filled) {
  ImVec2 pts[10];
  for (int k = 0; k < 10; ++k) {
    float a = -1.5707963f + k * 0.62831853f, rr = (k & 1) ? r * 0.45f : r;
    pts[k] = ImVec2(c.x + cosf(a) * rr, c.y + sinf(a) * rr);
  }
  if (filled) dl->AddConcavePolyFilled(pts, 10, col);
  else dl->AddPolyline(pts, 10, col, ImDrawFlags_Closed, U(1.3f));
}

static bool IsLiveEntry(const std::string& path) { return g_hist.live && !path.empty() && path == g_hist.livePath; }

// The file work behind the tab's actions, all on the worker; each says how it went and rescans.
static void HistStar(const std::string& path, bool star) {
  int i = hist::Find(g_hist.list, path);
  if (i >= 0) { g_hist.list[i].starred = star; hist::Sort(g_hist.list); }  // at once, so the row moves under the click
  if (IsLiveEntry(path)) { HistPost(g_hist.live, [star](hist::Record& r) { r.starred = star; }, hist::StarText(star), true); return; }
  g_hist.worker->Post([path, star] {
    hist::Worker::Done d;
    d.error = hist::WriteText(std::filesystem::u8path(path), hist::StarText(star), true);
    d.onMain = [] { HistScan(); };
    return d;
  });
}

static void HistDelete(const std::string& path) {
  if (IsLiveEntry(path)) { HistToast("The stream playing is still being recorded: delete it once it has stopped", true); return; }
  g_hist.worker->Post([path] {
    hist::Worker::Done d;
    std::string moved;
    d.error = hist::MoveToDeleted(path, moved);
    if (d.error.empty()) d.onMain = [path, moved] {
      g_hist.undoFrom = moved; g_hist.undoTo = path;
      if (g_hist.open == path) g_hist.open.clear();
      HistToast("Moved to deleted: " + std::filesystem::u8path(path).filename().u8string(), false);
      HistScan();
    };
    else d.error = "Could not delete: " + d.error;
    return d;
  });
}

static void HistUndo() {
  if (g_hist.undoFrom.empty()) return;
  std::string from = g_hist.undoFrom, to = g_hist.undoTo;
  g_hist.undoFrom.clear(); g_hist.undoTo.clear();
  g_hist.worker->Post([from, to] {
    hist::Worker::Done d;
    std::string restored;
    d.error = hist::Restore(from, to, restored);
    if (d.error.empty()) d.onMain = [restored] { HistToast("Restored " + std::filesystem::u8path(restored).filename().u8string(), false); HistScan(); };
    else d.error = "Could not undo: " + d.error;
    return d;
  });
}

static void HistEmptyDeleted() {
  std::string dir = g_hist.dir;
  g_hist.undoFrom.clear(); g_hist.undoTo.clear();  // what it would undo is gone for good
  g_hist.worker->Post([dir] {
    hist::Worker::Done d;
    size_t n = 0;
    d.error = hist::EmptyDeleted(dir, n);
    if (!d.error.empty()) d.error = "Could not empty deleted: " + d.error;
    d.onMain = [n] { HistToast("Removed " + std::to_string(n) + (n == 1 ? " deleted stream for good" : " deleted streams for good"), false); HistScan(); };
    return d;
  });
}

// Stages first..last (positions) of an entry as a song in examples\, where the library finds it.
static void HistSaveSong(const std::string& path, size_t first, size_t last) {
  if (g_lib.dirs.empty()) { HistToast("There is no song folder to save into", true); return; }
  std::string dir = g_lib.dirs[0];
  g_hist.worker->Post([path, first, last, dir]() mutable {
    hist::Worker::Done d;
    hist::Record r;
    std::string e;
    if (!hist::LoadRecord(path, r, e)) { d.error = "Could not save the song: " + e; return d; }
    if (r.stages.empty()) { d.error = "Could not save the song: the recording has no stages"; return d; }
    if (last >= r.stages.size()) last = r.stages.size() - 1;
    std::string text = hist::MergeStages(r, first, last, "Saved", e);
    if (text.empty()) { d.error = "Could not save the song: " + e; return d; }
    char stem[96];
    snprintf(stem, sizeof(stem), "history_s%llu_%llu-%llu", (unsigned long long)r.seed, (unsigned long long)r.stages[first].index + 1,
             (unsigned long long)r.stages[last].index + 1);
    std::filesystem::path out = hist::UniquePath(std::filesystem::u8path(dir), stem);
    e = hist::WriteText(out, text, false);
    if (!e.empty()) { d.error = "Could not save the song: " + e; return d; }
    std::string name = out.filename().u8string();
    d.onMain = [name] { HistToast("Saved as " + name + " (in the Library)", false); };
    return d;
  });
}

static std::string EntryWhen(const hist::Summary& e) { return e.started.size() >= 16 ? e.started.substr(0, 16) : e.started; }

static void DrawHistory() {
  ImDrawList* dl = ImGui::GetWindowDrawList();
  if (!g_hist.writeError.empty()) {
    ImGui::PushFont(nullptr, 14.0f);
    ImGui::PushTextWrapPos(0);
    ImGui::TextColored(kWarn, "%s", g_hist.writeError.c_str());
    ImGui::PopTextWrapPos();
    ImGui::PopFont();
    ImGui::Dummy(ImVec2(0, U(4)));
  }
  if (!g_hist.undoFrom.empty()) {
    ImGui::PushFont(nullptr, 14.0f);
    ImGui::AlignTextToFramePadding();
    ImGui::TextColored(kDim, "Deleted %s", std::filesystem::u8path(g_hist.undoTo).filename().u8string().c_str());
    ImGui::SameLine();
    if (ImGui::SmallButton("Undo")) HistUndo();
    ImGui::PopFont();
    ImGui::Dummy(ImVec2(0, U(4)));
  }
  if (g_hist.dir.empty()) { Note("The history is off: there is no folder to keep it in."); return; }
  if (g_hist.list.empty()) {
    Note(g_hist.scanBusy ? u8"Reading the history…" : "No streams yet. Every stream that plays is kept here: its seed and every stage, "
                                                    "so it can be played again, saved as a song, starred or deleted.");
  }
  ImGui::PushFont(g_ui.fontBold, 15.5f);
  float titleH = ImGui::GetFontSize();
  ImGui::PopFont();
  ImGui::PushFont(nullptr, 13.5f);
  float lineH = ImGui::GetFontSize() * 1.22f;
  ImGui::PopFont();
  std::string clickedOpen;
  bool openClicked = false;
  for (int i = 0; i < (int)g_hist.list.size(); ++i) {
    const hist::Summary& e = g_hist.list[i];
    bool live = IsLiveEntry(e.path), open = g_hist.open == e.path;
    bool replaying = g_replay.active && g_replay.path == e.path;
    uint64_t stages = live ? std::max<uint64_t>(e.stages, g_hist.stagesRecorded) : e.stages;
    uint64_t heard = live ? g_hist.heardStages : e.heardStages;
    double secs = live ? g_hist.heardSeconds : e.heardSeconds;
    float w = ImGui::GetContentRegionAvail().x, pad = U(12);
    float h = pad + titleH + U(4) + lineH * 2 + pad * 0.6f;
    ImVec2 p = ImGui::GetCursorScreenPos();
    ImGui::PushID(e.path.c_str());
    if (!open && !ImGui::IsRectVisible(ImVec2(w, h))) {  // off screen: its space only
      ImGui::Dummy(ImVec2(w, h + U(2)));
      ImGui::PopID();
      continue;
    }
    // The star, on the left of the title: a button of its own.
    float sr = U(8);
    ImVec2 sc(p.x + pad + sr, p.y + pad + titleH * 0.5f + U(1));
    ImGui::SetCursorScreenPos(ImVec2(sc.x - sr - U(3), sc.y - sr - U(3)));
    bool starClick = ImGui::InvisibleButton("##star", ImVec2(2 * sr + U(6), 2 * sr + U(6)));
    bool starHov = ImGui::IsItemHovered();
    if (starHov) Tooltip(e.starred ? "Starred: kept at the top  (click to unstar)" : "Star: keep at the top");
    ImGui::SetCursorScreenPos(p);
    ImGui::SetNextItemAllowOverlap();
    bool click = ImGui::InvisibleButton("##row", ImVec2(w, h));
    bool hov = ImGui::IsItemHovered() && !starHov;
    if (open) dl->AddRectFilled(p, ImVec2(p.x + w, p.y + h), Col(kAccent, 0.09f), U(10));
    else if (hov) dl->AddRectFilled(p, ImVec2(p.x + w, p.y + h), White(0.05f), U(10));
    DrawStar(dl, sc, sr, e.starred ? Col(kAccent) : (starHov ? White(0.9f) : White(0.35f)), e.starred);
    // Its state, on the right.
    std::string badge; ImVec4 badgeCol = kDim;
    if (live) { badge = "recording"; badgeCol = kAccent; }
    else if (replaying) { badge = "replaying"; badgeCol = kAccent; }
    else if (e.complete) badge = "finished";
    else if (!g_hist.version.empty() && e.composer != g_hist.version) badge = "older composer";
    ImGui::PushFont(nullptr, 13.0f);
    float bw = badge.empty() ? 0 : ImGui::CalcTextSize(badge.c_str()).x;
    if (!badge.empty()) dl->AddText(ImVec2(p.x + w - pad - bw, p.y + pad + U(2)), Col(badgeCol), badge.c_str());
    ImGui::PopFont();
    float tx = sc.x + sr + U(9), tmax = p.x + w - pad - bw - U(8);
    std::string title = EntryWhen(e) + u8"  ·  seed " + std::to_string(e.seed);
    ImGui::PushFont(g_ui.fontBold, 15.5f);
    ImGui::PushStyleColor(ImGuiCol_Text, open ? kAccent : (hov ? ImVec4(1, 1, 1, 1) : kText));
    ImGui::RenderTextEllipsis(dl, ImVec2(tx, p.y + pad), ImVec2(tmax, p.y + pad + titleH), tmax, title.c_str(), title.c_str() + title.size(), nullptr);
    ImGui::PopStyleColor();
    ImGui::PopFont();
    ImGui::PushFont(nullptr, 13.5f);
    char facts[160];
    if (e.complete) snprintf(facts, sizeof(facts), u8"A finished song  ·  %llu stages  ·  %s", (unsigned long long)e.stages,
                             ui::FormatTime(e.completeSeconds).c_str());
    else snprintf(facts, sizeof(facts), u8"%llu %s heard  ·  %s", (unsigned long long)heard, heard == 1 ? "stage" : "stages", ui::FormatTime(secs).c_str());
    float ly = p.y + pad + titleH + U(4);
    dl->AddText(ImVec2(tx, ly), Col(kDim), facts);
    std::string note = e.note.empty() ? std::string(u8"—") : e.note;
    ImGui::PushStyleColor(ImGuiCol_Text, kDim);
    float nmax = p.x + w - pad;
    ImGui::RenderTextEllipsis(dl, ImVec2(tx, ly + lineH), ImVec2(nmax, ly + 2 * lineH), nmax, note.c_str(), note.c_str() + note.size(), nullptr);
    ImGui::PopStyleColor();
    ImGui::PopFont();
    if (hov && !e.note.empty()) Tooltip(e.note.c_str());
    if (starClick) HistStar(e.path, !e.starred);
    else if (click) { openClicked = true; clickedOpen = open ? std::string() : e.path; }
    ImGui::SetCursorScreenPos(ImVec2(p.x, p.y + h));

    if (open) {
      // The range, the actions, and the confirm in place of the actions when Delete was pressed.
      ImGui::Indent(U(12));
      ImGui::Dummy(ImVec2(0, U(4)));
      int n = (int)std::max<uint64_t>(stages, 1);
      g_hist.from = std::clamp(g_hist.from, 1, n);
      g_hist.to = std::clamp(g_hist.to, g_hist.from, n);
      ImGui::PushFont(nullptr, 14.0f);
      float cw = ImGui::GetContentRegionAvail().x - U(12);
      ImGui::AlignTextToFramePadding();
      ImGui::TextColored(kDim, "Stages");
      ImGui::SameLine(U(76));
      ImGui::SetNextItemWidth((cw - U(64)) * 0.5f - U(4));
      ImGui::SliderInt("##from", &g_hist.from, 1, n, "from %d", ImGuiSliderFlags_AlwaysClamp);
      ImGui::SameLine();
      ImGui::SetNextItemWidth((cw - U(64)) * 0.5f - U(4));
      ImGui::SliderInt("##to", &g_hist.to, 1, n, "to %d", ImGuiSliderFlags_AlwaysClamp);
      if (g_hist.to < g_hist.from) g_hist.to = g_hist.from;
      bool whole = g_hist.from == 1 && g_hist.to == n;
      ImGui::Dummy(ImVec2(0, U(2)));
      float bw4 = (cw - 3 * ImGui::GetStyle().ItemSpacing.x) / 4;
      if (g_hist.confirmDelete == e.path) {
        ImGui::TextColored(kWarn, "Move this stream to deleted?");
        if (ImGui::Button("Delete", ImVec2(bw4, U(32)))) { g_hist.confirmDelete.clear(); HistDelete(e.path); }
        ImGui::SameLine();
        if (ImGui::Button("Keep", ImVec2(bw4, U(32)))) g_hist.confirmDelete.clear();
        Note("It goes to the history's deleted folder, and Undo brings it back.");
      } else {
        if (ImGui::Button("Replay", ImVec2(bw4, U(32))))
          ReplayStart(e.path, (size_t)g_hist.from - 1, whole ? SIZE_MAX : (size_t)g_hist.to - 1, e.seed, EntryWhen(e));
        ImGui::SameLine();
        if (ImGui::Button("Save as song", ImVec2(bw4, U(32)))) HistSaveSong(e.path, (size_t)g_hist.from - 1, (size_t)g_hist.to - 1);
        ImGui::SameLine();
        if (ImGui::Button(e.starred ? "Unstar" : "Star", ImVec2(bw4, U(32)))) HistStar(e.path, !e.starred);
        ImGui::SameLine();
        if (live) ImGui::BeginDisabled();
        if (ImGui::Button("Delete", ImVec2(bw4, U(32)))) g_hist.confirmDelete = e.path;
        if (live) ImGui::EndDisabled();
        std::string about;
        if (e.complete) about = "A finished song: Replay plays it to its end.";
        else if (g_hist.version.empty() || e.composer != g_hist.version)
          about = "Recorded with another composer than this one: Replay plays the recorded stages exactly, and stops there.";
        else about = whole ? "Replay plays the recorded stages, then goes on from its seed with new ones."
                           : "Replay plays the stages chosen. With all of them, it goes on from its seed after.";
        if (live) about = "Recording now. " + about;
        Note(about.c_str());
        if (replaying && !g_replay.note.empty()) Note(g_replay.note.c_str());
      }
      ImGui::PopFont();
      ImGui::Unindent(U(12));
      ImGui::Dummy(ImVec2(0, U(6)));
    }
    ImGui::Dummy(ImVec2(0, U(2)));
    ImGui::PopID();
  }
  if (openClicked) {
    g_hist.open = clickedOpen;
    g_hist.confirmDelete.clear();
    int i = hist::Find(g_hist.list, clickedOpen);
    g_hist.from = 1;
    g_hist.to = i >= 0 ? (int)std::max<uint64_t>(g_hist.list[i].stages, 1) : 1;
  }
  ImGui::Dummy(ImVec2(0, U(6)));
  ImGui::Separator();
  ImGui::PushFont(nullptr, 13.0f);
  if (g_hist.deleted > 0) {
    if (g_hist.confirmEmpty) {
      ImGui::TextColored(kWarn, "Remove %zu deleted %s for good? This cannot be undone.", g_hist.deleted, g_hist.deleted == 1 ? "stream" : "streams");
      if (ImGui::SmallButton("Empty deleted")) { g_hist.confirmEmpty = false; HistEmptyDeleted(); }
      ImGui::SameLine();
      if (ImGui::SmallButton("Cancel")) g_hist.confirmEmpty = false;
    } else {
      char b[64]; snprintf(b, sizeof(b), "Empty deleted (%zu)", g_hist.deleted);
      if (ImGui::SmallButton(b)) g_hist.confirmEmpty = true;
    }
  }
  ImGui::PushTextWrapPos(0);
  ImGui::TextColored(kDim, "%s", g_hist.dir.c_str());
  for (auto& err : g_hist.listErrors) ImGui::TextColored(kWarn, "%s", err.c_str());
  ImGui::PopTextWrapPos();
  ImGui::PopFont();
}

// The menu panel, on the left. Returns its right edge.
static float DrawMenu() {
  ImGuiIO& io = ImGui::GetIO();
  float m = U(20), pw = std::min(U(400), io.DisplaySize.x - 2 * m);
  ImGui::SetNextWindowPos(ImVec2(m, m));
  ImGui::SetNextWindowSize(ImVec2(pw, io.DisplaySize.y - 2 * m));
  ImGui::Begin("##menu", nullptr, kPanelFlags);
  // The word mark, spaced out, and a close button.
  ImDrawList* dl = ImGui::GetWindowDrawList();
  ImVec2 p = ImGui::GetCursorScreenPos();
  ImGui::PushFont(g_ui.fontBold, 13.0f);
  float x = p.x;
  for (const char* c = "DREAMRIDE"; *c; ++c) {
    dl->AddText(ImVec2(x, p.y + U(7)), Col(kDim), c, c + 1);
    x += ImGui::CalcTextSize(c, c + 1).x + U(3.5f);
  }
  ImGui::PopFont();
  float cw = ImGui::GetContentRegionAvail().x;
  ImGui::SetCursorScreenPos(ImVec2(p.x + cw - U(28), p.y));
  if (IconButton("##close", Icon::Close, U(28), false, "Close  (Tab)")) g_ui.ov.menu = false;
  ImGui::Dummy(ImVec2(0, U(4)));
  const char* tabs[] = { "Library", "History", "View", "Theme" };
  static const int order[] = { kTabLibrary, kTabHistory, kTabView, kTabTheme };
  int shown = 0;
  for (int k = 0; k < 4; ++k) if (order[k] == g_ui.tab) shown = k;
  int picked = order[Segmented("tabs", tabs, 4, shown)];
  if (picked == kTabHistory && g_ui.tab != kTabHistory) HistScan();  // fresh when opened
  g_ui.tab = picked;
  ImGui::Dummy(ImVec2(0, U(6)));
  ImGui::BeginChild("##tab", ImVec2(0, 0), ImGuiChildFlags_None, ImGuiWindowFlags_NoNav);
  if (g_ui.tab == kTabLibrary) DrawLibrary();
  else if (g_ui.tab == kTabHistory) DrawHistory();
  else if (g_ui.tab == kTabView) DrawView();
  else DrawThemeTab();
  ImGui::EndChild();
  ImGui::End();
  return m + pw;
}

// ---------------------------------------------------------------- the keys card, the quit question

static void KeyCap(const char* k) {
  // An arrow is one small glyph in Segoe UI: drawn larger so it reads as a key.
  bool glyph = (unsigned char)k[0] >= 0x80;
  float h = ImGui::GetTextLineHeight() + U(8);  // every cap the same height
  if (glyph) ImGui::PushFont(nullptr, 21.0f);
  ImVec2 ts = ImGui::CalcTextSize(k);
  float w = std::max(ts.x + U(16), U(30));
  ImVec2 p = ImGui::GetCursorScreenPos();
  ImGui::Dummy(ImVec2(w, h));
  ImDrawList* dl = ImGui::GetWindowDrawList();
  dl->AddRectFilled(p, ImVec2(p.x + w, p.y + h), White(0.06f), U(6));
  dl->AddRect(p, ImVec2(p.x + w, p.y + h), White(0.20f), U(6), 0, U(1));
  dl->AddText(ImVec2(p.x + (w - ts.x) * 0.5f, p.y + (h - ts.y) * 0.5f), Col(kText), k);
  if (glyph) ImGui::PopFont();
}

static void DrawHelp() {
  ImGuiIO& io = ImGui::GetIO();
  ImGui::SetNextWindowPos(ImVec2(io.DisplaySize.x * 0.5f, io.DisplaySize.y * 0.45f), ImGuiCond_Always, ImVec2(0.5f, 0.5f));
  ImGui::PushStyleVar(ImGuiStyleVar_WindowPadding, ImVec2(U(28), U(24)));
  ImGui::SetNextWindowBgAlpha(0.94f);  // a card read over the bright lines: nearly opaque
  ImGui::Begin("##help", nullptr, kPanelFlags | ImGuiWindowFlags_AlwaysAutoResize);
  ImGui::PushFont(g_ui.fontBold, 21.0f);
  ImGui::TextUnformatted("Keys");
  ImGui::PopFont();
  ImGui::Dummy(ImVec2(0, U(4)));
  const float col = U(128);
  for (const ui::HelpRow& r : ui::HelpRows()) {
    float x0 = ImGui::GetCursorPosX(), y0 = ImGui::GetCursorPosY();
    std::string keys = r.keys;
    size_t a = 0;
    bool first = true;
    while (a < keys.size()) {
      size_t b = keys.find("  ", a);
      std::string k = keys.substr(a, b == std::string::npos ? std::string::npos : b - a);
      if (!first) ImGui::SameLine(0, U(6));
      KeyCap(k.c_str());
      first = false;
      if (b == std::string::npos) break;
      a = b + 2;
    }
    ImGui::SameLine(x0 + col);
    ImGui::SetCursorPosY(y0 + U(4));
    ImGui::TextUnformatted(r.what);
    ImGui::Dummy(ImVec2(0, U(1)));
  }
  ImGui::Dummy(ImVec2(0, U(6)));
  ImGui::PushFont(nullptr, 13.5f);
  ImGui::TextColored(kDim, "The overlay fades while a song plays. Move the mouse or press Tab to bring it back.");
  ImGui::TextColored(kDim, "H or Esc closes this card.");
  ImGui::PopFont();
  ImGui::End();
  ImGui::PopStyleVar();
}

static void DrawConfirm() {
  ImGuiIO& io = ImGui::GetIO();
  ImGui::SetNextWindowPos(ImVec2(io.DisplaySize.x * 0.5f, io.DisplaySize.y * 0.42f), ImGuiCond_Always, ImVec2(0.5f, 0.5f));
  ImGui::PushStyleVar(ImGuiStyleVar_WindowPadding, ImVec2(U(30), U(24)));
  ImGui::SetNextWindowBgAlpha(0.94f);
  ImGui::Begin("##quit", nullptr, kPanelFlags | ImGuiWindowFlags_AlwaysAutoResize);
  ImGui::PushFont(g_ui.fontBold, 21.0f);
  ImGui::TextUnformatted("Quit Dreamride?");
  ImGui::PopFont();
  ImGui::PushFont(nullptr, 14.5f);
  ImGui::TextColored(kDim, "%s", u8"Esc again to quit  ·  any other key to stay");
  ImGui::PopFont();
  ImGui::Dummy(ImVec2(0, U(8)));
  float bw = U(120);
  if (ImGui::Button("Stay", ImVec2(bw, U(36)))) ui::CancelQuit(g_ui.ov);
  ImGui::SameLine();
  ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(kAccent.x, kAccent.y, kAccent.z, 0.22f));
  ImGui::PushStyleColor(ImGuiCol_ButtonHovered, ImVec4(kAccent.x, kAccent.y, kAccent.z, 0.38f));
  if (ImGui::Button("Quit", ImVec2(bw, U(36)))) g_running = false;
  ImGui::PopStyleColor(2);
  ImGui::End();
  ImGui::PopStyleVar();
}

// Before the first song: a caption in the middle of the void (rendering, or how to choose one).
static void DrawCaption() {
  if (HaveSong()) return;
  std::string hint;
  if (g_loader.busy) hint = std::string(u8"Rendering ") + (g_loader.queued ? g_loader.qTitle : g_loader.title) + u8"…";
  else if (!g_ui.ov.menu) hint = "Tab opens the library";
  if (hint.empty()) return;
  ImGuiIO& io = ImGui::GetIO();
  float breathe = 0.55f + 0.25f * sinf((float)ImGui::GetTime() * 2.2f);
  ImGui::PushFont(g_ui.font, 22.0f);
  ImVec2 ts = ImGui::CalcTextSize(hint.c_str());
  float cx = g_ui.ov.menu ? (U(440) + io.DisplaySize.x) * 0.5f : io.DisplaySize.x * 0.5f;
  ImGui::GetBackgroundDrawList()->AddText(ImVec2(cx - ts.x * 0.5f, io.DisplaySize.y * 0.42f),
                                          ImGui::ColorConvertFloat4ToU32(ImVec4(kText.x, kText.y, kText.z, breathe)), hint.c_str());
  ImGui::PopFont();
}

// ---------------------------------------------------------------- the live equation (stream mode)
//
// The playing stage's definitions, typeset (math_layout.h) in a dark translucent panel in a corner clear of the
// draw-head: a stage counter and what changed, the definitions, the time names on one line, the voices playing. At a
// seam the old stage's equation crossfades into the new one and the terms that changed light up, then settle. Nothing
// ahead is shown: the next stage's equation exists in the engine, and stays there until it plays.

static ImU32 EqCol(const ImVec4& c, float a) { return ImGui::ColorConvertFloat4ToU32(ImVec4(c.x, c.y, c.z, c.w * a)); }

struct EqDef { mathui::Lines lines; bool changed = false, anyHl = false; };
struct EqStage {
  uint64_t index = UINT64_MAX; float size = 0, maxW = 0;
  std::vector<EqDef> defs;
  mathui::Lines times;
  std::vector<std::string> voices, changed;
  std::string summary;
  bool turn = false;
  float width = 0, height = 0;
};
static EqStage g_eqCache[2];

static mathui::Layouter MathLayouter() {
  mathui::Layouter L;
  L.measure = [](const std::string& t, bool italic, float size) {
    ImFont* f = italic ? g_ui.mathItalic : g_ui.math;
    return f->CalcTextSizeA(size, FLT_MAX, 0.0f, t.c_str()).x;
  };
  return L;
}

static bool HasHl(const mathui::Lines& ls) {
  for (auto& l : ls.lines) for (auto& g : l.glyphs) if (g.hl) return true;
  return false;
}

static float EqHeaderH(float size) { return size * 1.95f; }
static float EqGap(float size) { return size * 0.42f; }

static const EqStage* EqStageFor(uint64_t k, float size, float maxW) {
  for (auto& c : g_eqCache) if (c.index == k && c.size == size && c.maxW == maxW) return &c;
  LmStageInfo si{};
  if (!g_stream.s || !lm_stream_stage(g_stream.s, k, &si)) return nullptr;
  EqStage& c = g_eqCache[k % 2];
  c = EqStage{};
  c.index = k; c.size = size; c.maxW = maxW;
  c.summary = si.summary; c.turn = si.turn != 0;
  mathui::Layouter L = MathLayouter();
  std::string times;
  const char* e = si.equation;
  while (*e) {
    const char* nl = strchr(e, '\n');
    std::string line = nl ? std::string(e, nl - e) : std::string(e);
    e = nl ? nl + 1 : e + strlen(e);
    if (line.size() < 4 || line[2] != '\t') continue;
    std::string markup = line.substr(3);
    if (line[0] == 'T') { times += (times.empty() ? "" : u8",\\brk   ") + markup; continue; }
    EqDef d;
    d.lines = mathui::BreakLines(L, mathui::Parse(markup), size, maxW, size * 1.6f);
    d.changed = line[1] == '1';
    d.anyHl = HasHl(d.lines);
    c.defs.push_back(std::move(d));
  }
  if (!times.empty()) c.times = mathui::BreakLines(L, mathui::Parse(times), size * 0.74f, maxW, 0);
  auto split = [](const char* t) { std::vector<std::string> v; std::string cur; for (const char* q = t; ; ++q) {
    if (*q == ' ' || *q == 0) { if (!cur.empty()) v.push_back(cur); cur.clear(); if (!*q) break; } else cur += *q; } return v; };
  c.voices = split(si.voices);
  c.changed = split(si.changed);
  float w = 0, h = EqHeaderH(size);
  for (auto& d : c.defs) { w = std::max(w, d.lines.width); for (auto& l : d.lines.lines) h += l.asc + l.desc + EqGap(size) * 0.5f; h += EqGap(size) * 0.5f; }
  if (!c.times.lines.empty()) { w = std::max(w, c.times.width); for (auto& l : c.times.lines) h += l.asc + l.desc + EqGap(size) * 0.4f; }
  h += size * 1.6f;  // the voices row
  c.width = std::max(w, size * 16.0f);
  c.height = h;
  return &c;
}

static ImVec4 Mix4(const ImVec4& a, const ImVec4& b, float t) { return ImVec4(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t, a.z + (b.z - a.z) * t, a.w + (b.w - a.w) * t); }

// One laid line at a baseline, its changed terms lit by `lit` (0..1); `all` lights every term (a definition that is new).
static void DrawMathLine(ImDrawList* dl, const mathui::Laid& l, ImVec2 base, float alpha, float lit, bool all, const ImVec4& ink) {
  if (lit > 0.01f) {
    for (auto& g : l.glyphs) {
      if (!(g.hl || all)) continue;
      ImFont* f = g.italic ? g_ui.mathItalic : g_ui.math;
      float w = f->CalcTextSizeA(g.size, FLT_MAX, 0.0f, g.text.c_str()).x;
      float pad = g.size * 0.12f;
      dl->AddRectFilled(ImVec2(base.x + g.x - pad, base.y + g.y - g.size * 0.78f), ImVec2(base.x + g.x + w + pad, base.y + g.y + g.size * 0.28f),
                        EqCol(kAccent, 0.16f * lit * alpha), g.size * 0.22f);
    }
  }
  for (auto& s : l.strokes) {
    ImVec4 c = Mix4(ink, kAccent, (s.hl || all) ? lit : 0.0f);
    dl->AddLine(ImVec2(base.x + s.x0, base.y + s.y0), ImVec2(base.x + s.x1, base.y + s.y1), EqCol(c, alpha), std::max(1.0f, s.width));
  }
  for (auto& g : l.glyphs) {
    ImFont* f = g.italic ? g_ui.mathItalic : g_ui.math;
    float asc = f->GetFontBaked(g.size)->Ascent;
    ImVec4 c = Mix4(ink, kAccent, (g.hl || all) ? lit : 0.0f);
    dl->AddText(f, g.size, ImVec2(base.x + g.x, base.y + g.y - asc), EqCol(c, alpha), g.text.c_str());
  }
}

// A stage's content, its top-left at `o`.
static void DrawEqStage(ImDrawList* dl, const EqStage& c, ImVec2 o, float alpha, float lit) {
  float size = c.size;
  // The counter and what changed.
  ImGui::PushFont(g_ui.fontBold, size * 0.62f);
  char head[96]; snprintf(head, sizeof(head), "STAGE  %llu    SEED  %llu%s", (unsigned long long)c.index + 1,
                          (unsigned long long)g_stream.info.seed, g_stream.fromG ? "  (G)" : "");
  dl->AddText(ImVec2(o.x, o.y), EqCol(c.turn ? kAccent : kDim, alpha), head);
  float hw = ImGui::CalcTextSize(head).x;
  ImGui::PopFont();
  ImGui::PushFont(g_ui.font, size * 0.66f);
  std::string sum = c.index == 0 ? std::string("the seed") : c.summary;
  float room = c.width - hw - size * 0.8f;
  if (ImGui::CalcTextSize(sum.c_str()).x > room) {
    // Cut at a character boundary, with an ellipsis.
    while (!sum.empty() && ImGui::CalcTextSize((sum + u8"…").c_str()).x > room) {
      sum.pop_back();
      while (!sum.empty() && ((unsigned char)sum.back() & 0xC0) == 0x80) sum.pop_back();
      if (!sum.empty() && ((unsigned char)sum.back() & 0xC0) == 0xC0) sum.pop_back();
    }
    sum += u8"…";
  }
  dl->AddText(ImVec2(o.x + hw + size * 0.8f, o.y), EqCol(kDim, alpha), sum.c_str());
  ImGui::PopFont();
  float y = o.y + EqHeaderH(size);
  for (auto& d : c.defs) {
    bool first = true;
    for (auto& l : d.lines.lines) {
      y += l.asc;
      if (first && d.changed && lit > 0.01f)
        dl->AddRectFilled(ImVec2(o.x - size * 0.55f, y - l.asc * 0.8f), ImVec2(o.x - size * 0.40f, y + l.desc * 0.8f), EqCol(kAccent, lit * alpha), size * 0.08f);
      DrawMathLine(dl, l, ImVec2(o.x, y), alpha, lit, d.changed && !d.anyHl, kText);
      y += l.desc + EqGap(size) * 0.5f;
      first = false;
    }
    y += EqGap(size) * 0.5f;
  }
  for (auto& l : c.times.lines) {
    y += l.asc;
    DrawMathLine(dl, l, ImVec2(o.x, y), alpha * 0.8f, lit, false, kDim);
    y += l.desc + EqGap(size) * 0.4f;
  }
  // The voices playing: one that came in at this stage is lit.
  ImGui::PushFont(g_ui.font, size * 0.68f);
  float x = o.x;
  y += size * 0.35f;
  for (size_t i = 0; i < c.voices.size(); ++i) {
    const std::string& v = c.voices[i];
    bool joined = std::find(c.changed.begin(), c.changed.end(), v) != c.changed.end();
    std::string t = (i ? u8"  ·  " : "") + v;
    ImVec4 col = Mix4(kDim, kAccent, joined ? lit : 0.0f);
    dl->AddText(ImVec2(x, y), EqCol(col, alpha), t.c_str());
    x += ImGui::CalcTextSize(t.c_str()).x;
  }
  ImGui::PopFont();
}

// Where the draw-head is on screen: the present's column (x = 0) from the floor to the top of the wall, projected
// with the camera of this frame, with a margin.
static mathui::Rect DrawHeadRect(float W, float H) {
  View v = ViewAtSeconds(g_play.now);
  Camera cam = CameraAt(v);
  M4 vp = Mul(Perspective(kFovY, W / H, 0.05f), LookAt(cam.eye, cam.target, { 0, 1, 0 }));
  float top = PitchY(g_song, g_song.pMax) + 0.4f;
  mathui::Rect r{ 1e9f, 1e9f, -1e9f, -1e9f };
  for (float y : { 0.0f, top })
    for (float z : { -0.8f, 0.8f })
      for (float x : { -0.6f, 0.4f }) {
        float p[4] = { x, y, z, 1 }, c[4] = {};
        for (int i = 0; i < 4; ++i) for (int j = 0; j < 4; ++j) c[i] += vp.m[i][j] * p[j];
        if (c[3] <= 1e-4f) continue;
        float sx = (c[0] / c[3] * 0.5f + 0.5f) * W, sy = (0.5f - c[1] / c[3] * 0.5f) * H;
        r.x0 = std::min(r.x0, sx); r.y0 = std::min(r.y0, sy); r.x1 = std::max(r.x1, sx); r.y1 = std::max(r.y1, sy);
      }
  float m = U(48);
  r.x0 -= m; r.y0 -= m; r.x1 += m; r.y1 += m;
  return r;
}

// ---- the compact strip (E's default; mathui::FlowCompact and CompactStrip place it)

struct EqCompact {
  uint64_t index = UINT64_MAX;
  float size = 0;
  std::string summary;
  bool turn = false;
  std::vector<mathui::Laid> lines;  // each definition on one line, baseline at 0
  std::vector<char> changed, anyHl;
  std::vector<mathui::FlowItem> items;
};
static EqCompact g_eqCompact[2];

static const EqCompact* EqCompactFor(uint64_t k, float size) {
  for (auto& c : g_eqCompact) if (c.index == k && c.size == size) return &c;
  LmStageInfo si{};
  if (!g_stream.s || !lm_stream_stage(g_stream.s, k, &si)) return nullptr;
  EqCompact& c = g_eqCompact[k % 2];
  c = EqCompact{};
  c.index = k; c.size = size; c.summary = si.summary; c.turn = si.turn != 0;
  mathui::Layouter L = MathLayouter();
  for (const char* e = si.equation; *e;) {
    const char* nl = strchr(e, '\n');
    std::string line = nl ? std::string(e, nl - e) : std::string(e);
    e = nl ? nl + 1 : e + strlen(e);
    if (line.size() < 4 || line[2] != '\t') continue;
    mathui::Lines ls = mathui::BreakLines(L, mathui::Parse(line.substr(3)), size, 1e9f, 0);
    if (ls.lines.empty()) continue;
    bool changed = line[1] == '1';
    c.items.push_back({ ls.lines[0].w, ls.lines[0].asc, ls.lines[0].desc, mathui::CompactPriority(line[0], changed) });
    c.anyHl.push_back(ls.lines[0].hl);
    c.lines.push_back(std::move(ls.lines[0]));
    c.changed.push_back(changed);
  }
  return &c;
}

// `text` cut at a character boundary with an ellipsis to fit `room` px in the current font.
static std::string Ellipsize(std::string text, float room) {
  if (ImGui::CalcTextSize(text.c_str()).x <= room) return text;
  while (!text.empty() && ImGui::CalcTextSize((text + u8"…").c_str()).x > room) {
    text.pop_back();
    while (!text.empty() && ((unsigned char)text.back() & 0xC0) == 0x80) text.pop_back();
    if (!text.empty() && ((unsigned char)text.back() & 0xC0) == 0xC0) text.pop_back();
  }
  return text + u8"…";
}

static void DrawEquationCompact() {
  ImGuiIO& io = ImGui::GetIO();
  float W = io.DisplaySize.x, H = io.DisplaySize.y;
  float size = mathui::CompactSize(H, g_ui.scale);
  double now = g_play.now;
  uint64_t k = StageAtSeconds(now);
  double since = now - StageStartSeconds(k);
  const EqCompact* cur = EqCompactFor(k, size);
  if (!cur) return;
  float a = (float)mathui::NewAlpha(since);
  const EqCompact* old = (k > 0 && a < 1.0f) ? EqCompactFor(k - 1, size) : nullptr;
  if (old == cur) old = nullptr;
  float margin = U(18), pad = mathui::CompactPad(size), head = mathui::CompactHeaderH(size);
  mathui::Rect dh = DrawHeadRect(W, H);
  std::vector<mathui::Rect> avoid;
  if (g_ui.alpha > 0.01f) {
    float tw = std::min(U(860), W - U(40));
    avoid.push_back({ (W - tw) * 0.5f, H - U(200), (W + tw) * 0.5f, H });
    if (g_ui.ov.menu) avoid.push_back({ 0, 0, U(470), H });
  }
  // The top-left corner when it has room for the header at least (CompactStrip puts it there); else the side of the
  // draw-head with more room, as before.
  float tlRoom = mathui::CompactTopLeftRoom(W, margin, mathui::CompactMaxHeight(H), dh, avoid);
  float room = (tlRoom >= size * 30.0f + 2 * pad ? std::min(tlRoom, W * 0.62f)
                                                 : std::min(std::max(dh.x0, W - dh.x1) - 2 * margin, W * 0.62f)) - 2 * pad;
  float maxFlow = mathui::CompactMaxHeight(H) - head - 2 * pad;
  auto flow = [&](const EqCompact& c) {
    return mathui::FlowCompact(c.items, room, maxFlow, mathui::CompactGapX(size), mathui::CompactGapY(size));
  };
  mathui::Flow fc = flow(*cur), fo = old ? flow(*old) : mathui::Flow{};
  float w = std::max({ fc.width, fo.width, size * 30.0f }) + 2 * pad;
  float h = std::max(fc.height, fo.height) + head + 2 * pad;
  mathui::Rect r = mathui::CompactStrip(w, h, W, H, margin, dh, avoid);
  float A = g_stream.eqAlpha;
  ImDrawList* dl = ImGui::GetBackgroundDrawList();
  dl->AddRectFilled(ImVec2(r.x0, r.y0), ImVec2(r.x1, r.y1), EqCol(ImVec4(0.020f, 0.024f, 0.040f, 1.0f), 0.62f * A), U(10));

  // The header: the stage, what changed, and the seed (to play a good one again).
  char seed[96];
  if (g_stream.fromG) snprintf(seed, sizeof(seed), "from G  ·  seed %llu", (unsigned long long)g_stream.info.seed);
  else snprintf(seed, sizeof(seed), "seed %llu", (unsigned long long)g_stream.info.seed);
  std::string right = seed;
  if (fc.dropped > 0) right = "+" + std::to_string(fc.dropped) + u8" more (E)   ·   " + right;
  ImGui::PushFont(g_ui.fontBold, size * 0.95f);
  char stage[32]; snprintf(stage, sizeof(stage), "%llu", (unsigned long long)k + 1);
  float sw = ImGui::CalcTextSize(stage).x;
  dl->AddText(ImVec2(r.x0 + pad, r.y0 + pad), EqCol(cur->turn ? kAccent : kDim, A), stage);
  ImGui::PopFont();
  ImGui::PushFont(g_ui.font, size * 0.9f);
  float rw = ImGui::CalcTextSize(right.c_str()).x;
  dl->AddText(ImVec2(r.x1 - pad - rw, r.y0 + pad), EqCol(kDim, 0.75f * A), right.c_str());
  std::string sum = Ellipsize(cur->summary, (r.x1 - r.x0) - 2 * pad - sw - rw - size * 1.8f);
  dl->AddText(ImVec2(r.x0 + pad + sw + size * 0.7f, r.y0 + pad), EqCol(kDim, A), sum.c_str());
  ImGui::PopFont();

  // The definitions, one line each, in a dense block.
  ImVec2 o(r.x0 + pad, r.y0 + pad + head);
  auto draw = [&](const EqCompact& c, const mathui::Flow& f, float alpha, float lit) {
    for (auto& p : f.placed) {
      bool ch = c.changed[p.index] != 0;
      DrawMathLine(dl, c.lines[p.index], ImVec2(o.x + p.x, o.y + p.baseline), alpha, ch ? lit : 0.0f, ch && !c.anyHl[p.index],
                   ImVec4(kText.x, kText.y, kText.z, 0.88f));
    }
  };
  if (old) draw(*old, fo, A * (1.0f - a), 0.0f);
  draw(*cur, fc, A * a, (float)mathui::Highlight(k == 0 ? -1.0 : since));
  if (g_stream.toastUntil > now) {
    ImGui::PushFont(g_ui.font, size * 0.9f);
    float y = r.y0 > H * 0.5f ? r.y0 - U(6) - size : r.y1 + U(6);
    dl->AddText(ImVec2(r.x0 + pad, y), EqCol(g_stream.toastBad ? kWarn : kAccent, A), g_stream.toast.c_str());
    ImGui::PopFont();
  }
}

static void DrawEquation() {
  if (!g_streaming || !g_stream.s || g_stream.eqAlpha <= 0.002f) return;
  if (g_stream.eqMode == mathui::EqMode::Compact) { DrawEquationCompact(); return; }
  ImGuiIO& io = ImGui::GetIO();
  float W = io.DisplaySize.x, H = io.DisplaySize.y;
  float size = std::round(std::clamp(H * 0.0137f, 10.0f, 24.0f));  // about 70% of what it was (21 -> 15 px at 1080p), 2026-11-03
  float maxW = std::min(W * 0.40f, U(820));
  double now = g_play.now;
  uint64_t k = StageAtSeconds(now);
  double since = now - StageStartSeconds(k);
  const EqStage* cur = EqStageFor(k, size, maxW);
  if (!cur) return;
  float a = (float)mathui::NewAlpha(since);
  const EqStage* old = (k > 0 && a < 1.0f) ? EqStageFor(k - 1, size, maxW) : nullptr;
  if (old == cur) old = nullptr;
  float pad = size * 1.0f;
  float w = cur->width, h = cur->height;
  if (old) { w = std::max(w, old->width); h = std::max(h, old->height); }
  w += 2 * pad; h += 2 * pad;
  std::vector<mathui::Rect> avoid = { DrawHeadRect(W, H) };
  if (g_ui.alpha > 0.01f) {
    float tw = std::min(U(860), W - U(40));
    avoid.push_back({ (W - tw) * 0.5f, H - U(200), (W + tw) * 0.5f, H });
    if (g_ui.ov.menu) avoid.push_back({ 0, 0, U(470), H });
  }
  mathui::Rect r = mathui::PlacePanel(w, h, W, H, U(24), avoid);
  ImDrawList* dl = ImGui::GetBackgroundDrawList();
  float A = g_stream.eqAlpha;
  dl->AddRectFilled(ImVec2(r.x0, r.y0), ImVec2(r.x1, r.y1), EqCol(ImVec4(0.030f, 0.036f, 0.060f, 1.0f), 0.86f * A), U(14));
  dl->AddRect(ImVec2(r.x0, r.y0), ImVec2(r.x1, r.y1), EqCol(ImVec4(1, 1, 1, 1), 0.07f * A), U(14));
  ImVec2 o(r.x0 + pad, r.y0 + pad);
  if (old) DrawEqStage(dl, *old, o, A * (1.0f - a), 0.0f);
  DrawEqStage(dl, *cur, o, A * a, (float)mathui::Highlight(k == 0 ? -1.0 : since));
  if (g_stream.toastUntil > now) {
    ImGui::PushFont(g_ui.font, size * 0.66f);
    dl->AddText(ImVec2(r.x0 + pad, r.y1 + U(8)), EqCol(g_stream.toastBad ? kWarn : kAccent, A), g_stream.toast.c_str());
    ImGui::PopFont();
  }
}

// While the overlay is away: the song and the time, small and faint, in the corner.
static void DrawHud() {
  if (!g_ui.hud || !HaveSong() || g_ui.alpha > 0.98f) return;
  if (g_streaming && g_stream.eqAlpha > 0.5f) return;  // the equation panel shows the stage already
  float a = (1.0f - g_ui.alpha) * 0.5f;
  std::string s = g_lib.currentTitle + u8"   ·   " + ui::FormatTime(SongSecondsNow()) + " / " + ui::FormatTime(g_song.info.duration);
  if (g_streaming) s = g_lib.currentTitle + u8"   ·   stage " + std::to_string(StageAtSeconds(g_play.now) + 1) + u8"   ·   " + ui::FormatTime(g_play.now);
  ImGuiIO& io = ImGui::GetIO();
  ImGui::PushFont(g_ui.font, 14.0f);
  ImVec2 ts = ImGui::CalcTextSize(s.c_str());
  ImGui::GetForegroundDrawList()->AddText(ImVec2(U(24), io.DisplaySize.y - U(22) - ts.y),
                                          ImGui::ColorConvertFloat4ToU32(ImVec4(kText.x, kText.y, kText.z, a)), s.c_str());
  ImGui::PopFont();
}

// The ending card: the song is over (the stream ended on its own). Over the pulled-back trail, from the hold on.
static bool EndingCardShown() {
  return g_streaming && (g_stream.end == StreamMode::End::Hold || g_stream.end == StreamMode::End::Breath || g_stream.end == StreamMode::End::Waiting);
}
static void DrawEnding() {
  if (!EndingCardShown()) return;
  ImGuiIO& io = ImGui::GetIO();
  ImGui::SetNextWindowPos(ImVec2(io.DisplaySize.x * 0.5f, io.DisplaySize.y * 0.30f), ImGuiCond_Always, ImVec2(0.5f, 0.5f));
  ImGui::PushStyleVar(ImGuiStyleVar_WindowPadding, ImVec2(U(30), U(22)));
  ImGui::SetNextWindowBgAlpha(0.88f);
  ImGui::Begin("##ending", nullptr, kPanelFlags | ImGuiWindowFlags_AlwaysAutoResize);
  ImGui::PushFont(g_ui.fontBold, 21.0f);
  ImGui::TextUnformatted("The song has ended");
  ImGui::PopFont();
  ImGui::PushFont(nullptr, 14.5f);
  uint64_t stages = lm_stream_stage_at(g_stream.s, g_stream.endFrame.load() > 0 ? g_stream.endFrame.load() - 1 : 0) + 1;
  ImGui::TextColored(kDim, u8"%s  ·  %llu %s  ·  seed %llu  ·  kept in the History as a finished song", ui::FormatTime(g_stream.endSec).c_str(),
                     (unsigned long long)stages, stages == 1 ? "stage" : "stages", (unsigned long long)g_stream.info.seed);
  if (g_stream.end == StreamMode::End::Breath) {
    double left = std::max(0.0, kEndBreath - (HistNow() - g_stream.phaseAt));
    ImGui::TextColored(kAccent, "A new song in %.0f s", std::ceil(left));
  } else if (g_stream.end == StreamMode::End::Waiting && g_loader.busy) {
    ImGui::TextColored(kAccent, u8"The next song is on its way…");
  } else {
    ImGui::TextColored(kDim, g_autoNext ? "A new song follows in a moment." : "Next song: off (View).");
  }
  ImGui::PopFont();
  ImGui::Dummy(ImVec2(0, U(6)));
  float bw = U(150);
  ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(kAccent.x, kAccent.y, kAccent.z, 0.22f));
  ImGui::PushStyleColor(ImGuiCol_ButtonHovered, ImVec4(kAccent.x, kAccent.y, kAccent.z, 0.38f));
  bool replay = ImGui::Button("Replay this song", ImVec2(bw, U(36)));
  ImGui::PopStyleColor(2);
  ImGui::SameLine();
  bool next = ImGui::Button("Next song", ImVec2(bw, U(36)));
  ImGui::End();
  ImGui::PopStyleVar();
  if (replay) {
    if (g_hist.livePath.empty()) HistToast("This song was not recorded, so it cannot be replayed (see the History tab)", true);
    else {
      std::string path = g_hist.livePath, when;
      int i = hist::Find(g_hist.list, path);
      if (i >= 0) when = EntryWhen(g_hist.list[i]);
      ReplayStart(path, 0, SIZE_MAX, g_stream.info.seed, when);
      g_stream.end = StreamMode::End::Waiting; g_stream.phaseAt = 1e300;  // no auto-next over the replay
    }
  } else if (next) {
    g_stream.end = StreamMode::End::Waiting; g_stream.phaseAt = 1e300;
    StartNextStream();
  }
}

// The history's messages: a write that failed stays at the top until one succeeds; others for a few seconds. Shown
// over everything, even with the overlay faded, so a failure is never silent.
static void DrawHistoryNotice() {
  double now = HistNow();
  std::string text;
  ImVec4 col = kAccent;
  if (!g_hist.writeError.empty()) { text = g_hist.writeError; col = kWarn; }
  else if (g_hist.toastUntil > now) { text = g_hist.toast; col = g_hist.toastBad ? kWarn : kAccent; }
  if (text.empty()) return;
  ImGuiIO& io = ImGui::GetIO();
  ImGui::PushFont(g_ui.font, 15.0f);
  ImVec2 ts = ImGui::CalcTextSize(text.c_str(), nullptr, false, io.DisplaySize.x * 0.6f);
  ImVec2 p(io.DisplaySize.x * 0.5f - ts.x * 0.5f, U(18));
  ImDrawList* dl = ImGui::GetForegroundDrawList();
  dl->AddRectFilled(ImVec2(p.x - U(14), p.y - U(8)), ImVec2(p.x + ts.x + U(14), p.y + ts.y + U(8)), ImGui::GetColorU32(ImVec4(0.05f, 0.06f, 0.09f, 0.9f)), U(8));
  dl->AddText(g_ui.font, ImGui::GetFontSize(), p, ImGui::ColorConvertFloat4ToU32(col), text.c_str(), nullptr, io.DisplaySize.x * 0.6f);
  ImGui::PopFont();
}

static void DrawUi() {
  ImGuiIO& io = ImGui::GetIO();
  ImGui::GetStyle().Alpha = std::clamp(g_ui.alpha, 0.0f, 1.0f);
  g_ui.hovered = io.WantCaptureMouse && g_ui.alpha > 0.01f;
  DrawCaption();
  DrawEquation();
  if (g_ui.alpha > 0.01f) {
    float right = g_ui.ov.menu ? DrawMenu() : 0.0f;
    DrawTransport(right);
    DrawEnding();
    if (g_ui.ov.help) DrawHelp();
    if (g_ui.ov.confirmQuit) DrawConfirm();
  } else {
    g_ui.dragging = false;
    g_ui.volDrag = false;
  }
  DrawHud();
  DrawHistoryNotice();
}

// The overlay's opacity follows whether anything holds it up: no song playing, a song rendering,
// the quit question, a drag, the mouse over it, or input within kIdleSeconds. In fast, out slow.
static void UpdateOverlayFade(double dt) {
  g_ui.idle += dt;
  bool hold = !g_play.playing || g_loader.busy || (g_replay.active && (!g_replay.started || g_replay.waiting)) || g_ui.ov.confirmQuit || g_ui.dragging || g_ui.volDrag || g_ui.hovered || EndingCardShown() ||
              g_ui.idle < kIdleSeconds;
  float target = hold ? 1.0f : 0.0f;
  float rate = (float)dt / (target > g_ui.alpha ? 0.15f : 0.9f);
  g_ui.alpha += std::clamp(target - g_ui.alpha, -rate, rate);
  float eq = (g_streaming && g_stream.eqMode != mathui::EqMode::Hidden) ? 1.0f : 0.0f;  // E: it fades in and out in a third of a second
  g_stream.eqAlpha += std::clamp(eq - g_stream.eqAlpha, (float)-dt / 0.3f, (float)dt / 0.3f);
}

static void BuildWindowUi() {
  ImGui_ImplDX12_NewFrame();
  ImGui_ImplWin32_NewFrame();
  ImGui::NewFrame();
  DrawUi();
  ImGui::Render();
}

// Headless: the overlay as it stands at song time `secs`, no input, fully shown. ImGui lays a new
// window out over its first frames, so a capture builds a few and draws the last.
static void BuildHeadlessUi(double secs, int frames) {
  ImGuiIO& io = ImGui::GetIO();
  io.DisplaySize = ImVec2((float)g_t.w, (float)g_t.h);
  io.DeltaTime = 1.0f / 60.0f;
  io.AddMousePosEvent(-FLT_MAX, -FLT_MAX);
  g_play.now = secs;
  g_ui.alpha = g_ui.capEqOnly ? 0.0f : 1.0f;
  g_stream.eqAlpha = (g_streaming && g_stream.eqMode != mathui::EqMode::Hidden) ? 1.0f : 0.0f;
  for (int k = 0; k < frames; ++k) {
    ImGui_ImplDX12_NewFrame();
    ImGui::NewFrame();
    DrawUi();
    ImGui::Render();
  }
}

// --capture-menu's panels: the overlay state each one shows.
static const char* const kCapturePanels[] = { "transport", "library", "view", "theme", "help", "confirm", "loading", "equation", "equation-full",
                                              "history", "history-open", "history-delete", "ending" };
static bool IsCapturePanel(const std::string& p) {
  for (const char* k : kCapturePanels) if (p == k) return true;
  return false;
}
static void ApplyCapturePanel(const std::string& p) {
  g_ui.ov = ui::Overlay{};
  g_play.playing = true;
  if (p == "library" || p == "loading") { g_ui.ov.menu = true; g_ui.tab = kTabLibrary; }
  if (p == "view") { g_ui.ov.menu = true; g_ui.tab = kTabView; }
  if (p == "theme") { g_ui.ov.menu = true; g_ui.tab = kTabTheme; }
  if (p.rfind("history", 0) == 0) {
    // The history as --history-dir holds it (read before the capture); -open and -delete open the first entry.
    g_ui.ov.menu = true; g_ui.tab = kTabHistory;
    g_hist.open.clear(); g_hist.confirmDelete.clear();
    if (p != "history" && !g_hist.list.empty()) {
      g_hist.open = g_hist.list[0].path;
      g_hist.from = 1; g_hist.to = (int)std::max<uint64_t>(g_hist.list[0].stages, 1);
      if (p == "history-delete") g_hist.confirmDelete = g_hist.open;
    }
  }
  if (p == "ending" && g_streaming) {  // the ending card over the whole song's pulled-back trail, as at the hold
    g_stream.endFrame = (uint64_t)(g_play.now * g_stream.info.rate);
    g_stream.endSec = g_play.now;
    g_stream.end = StreamMode::End::Hold;
    uint64_t last = StageAtSeconds(g_play.now);
    std::string err;
    hist::Record r = StreamRecordUpTo(g_stream.s, last, err);
    auto whole = BuildWholeSong(r, g_song, g_theme, err);
    if (whole) { ShowWholeSong(std::move(whole), last); g_camSpan = g_camSpanTarget; }
    else fprintf(stderr, "capture: the whole song's trail: %s\n", err.c_str());
    g_camWide = 1.0f; g_ui.wide = true;
  }
  if (p == "help") g_ui.ov.help = true;
  if (p == "confirm") g_ui.ov.confirmQuit = true;
  g_ui.capEqOnly = p == "equation" || p == "equation-full";
  g_stream.eqMode = p == "equation-full" ? mathui::EqMode::Full : mathui::EqMode::Compact;
  if (p == "loading" && !g_lib.songs.empty()) {
    // The state while the next song renders (no thread runs: the loader only looks busy).
    int next = ui::StepIndex(g_lib.current, (int)g_lib.songs.size(), +1);
    g_loader.busy = true; g_loader.index = next; g_loader.title = g_lib.songs[next].title;
    g_loader.start = std::chrono::steady_clock::now() - std::chrono::milliseconds(2400);
  }
}
static void ResetCapturePanel() {
  g_ui.ov = ui::Overlay{};
  g_ui.capEqOnly = false;
  g_stream.eqMode = mathui::EqMode::Compact;
  g_ui.tab = kTabLibrary;
  g_loader.busy = false; g_loader.index = -1;
  g_hist.open.clear(); g_hist.confirmDelete.clear();
  if (g_stream.end != StreamMode::End::None) { g_stream.end = StreamMode::End::None; g_camWide = 0; g_ui.wide = false; g_camSpan = g_camSpanTarget = 0; }
}

// ---------------------------------------------------------------- the shortcuts

static void DoAction(ui::Action a) {
  bool shown = g_ui.alpha > 0.5f;
  int n = (int)g_lib.songs.size();
  switch (a) {
    case ui::Action::None: return;
    case ui::Action::PlayPause:
      if (HaveSong()) PlayerSetPlaying(!g_play.playing);
      else if (!g_loader.busy) RequestSong(g_lib.current >= 0 ? g_lib.current : 0);
      break;
    case ui::Action::Restart: if (g_streaming) RestartStream(); else if (HaveSong()) PlayerSeekTotal(0); break;
    case ui::Action::SeekBack: PlayerSeekSong(SongSecondsNow() - ui::kSeekStep); break;
    case ui::Action::SeekForward: PlayerSeekSong(SongSecondsNow() + ui::kSeekStep); break;
    case ui::Action::VolumeUp: PlayerSetVolume(ui::StepVolume(g_play.volume, +1)); break;
    case ui::Action::VolumeDown: PlayerSetVolume(ui::StepVolume(g_play.volume, -1)); break;
    case ui::Action::NextSong: RequestSong(ui::StepIndex(LibraryIndexNow(), n, +1)); break;
    case ui::Action::PrevSong: RequestSong(ui::StepIndex(LibraryIndexNow(), n, -1)); break;
    case ui::Action::ToggleLoop: PlayerToggleLoop(); break;
    case ui::Action::ToggleMenu: ui::OnMenuKey(g_ui.ov, shown); break;
    case ui::Action::ToggleHelp: ui::OnHelpKey(g_ui.ov, shown); break;
    case ui::Action::Fullscreen: ToggleFullscreen(); break;
    case ui::Action::Escape: if (ui::OnEscape(g_ui.ov, shown) == ui::EscResult::Quit) g_running = false; break;
    case ui::Action::ToggleEquation: g_stream.eqMode = mathui::NextEqMode(g_stream.eqMode); return;  // compact, full, hidden: the menu stays
    case ui::Action::SaveStream: SaveStreamStretch(g_play.now); return;
  }
  g_ui.idle = 0;  // the overlay comes back to show what the key did
}

// ================================================================ the window

static LRESULT CALLBACK WndProc(HWND h, UINT m, WPARAM w, LPARAM l) {
  // While the overlay is away, so is the cursor (over the picture only).
  if (m == WM_SETCURSOR && LOWORD(l) == HTCLIENT && g_ui.ready && g_ui.alpha <= 0.01f) { SetCursor(nullptr); return TRUE; }
  if (g_ui.ready && g_ui.platform && ImGui_ImplWin32_WndProcHandler(h, m, w, l)) return TRUE;
  switch (m) {
    case WM_SIZE: g_resize = true; return 0;
    case WM_MOUSEMOVE: {
      // Windows repeats WM_MOUSEMOVE without movement (when the cursor shows or hides): only real
      // movement wakes the overlay.
      int x = (int)(short)LOWORD(l), y = (int)(short)HIWORD(l);
      if (x != g_ui.lastMouseX || y != g_ui.lastMouseY) { g_ui.lastMouseX = x; g_ui.lastMouseY = y; g_ui.idle = 0; }
    } break;
    case WM_LBUTTONDOWN: case WM_RBUTTONDOWN: case WM_MBUTTONDOWN: case WM_MOUSEWHEEL: g_ui.idle = 0; break;
    case WM_KEYDOWN: {
      bool repeat = (l & (1 << 30)) != 0;
      ui::Action a = ui::KeyToAction((int)w, repeat);
      // "Quit?" is showing: Esc answers it (DoAction), any other key answers "stay" and does nothing else.
      if (g_ui.ov.confirmQuit && a != ui::Action::Escape) {
        if (!repeat && ui::CancelQuit(g_ui.ov)) g_ui.idle = 0;
        return 0;
      }
      DoAction(a);
      return 0;
    }
    case WM_DPICHANGED: {
      const RECT* r = (const RECT*)l;
      SetWindowPos(h, nullptr, r->left, r->top, r->right - r->left, r->bottom - r->top, SWP_NOZORDER | SWP_NOACTIVATE);
      g_ui.scale = HIWORD(w) / 96.0f;
      if (g_ui.ready) ApplyUiStyle(g_ui.scale);
      return 0;
    }
    case WM_CLOSE: case WM_DESTROY: g_running = false; return 0;
  }
  return DefWindowProcW(h, m, w, l);
}

static void CreateBackbuffers() {
  for (UINT i = 0; i < kBackBuffers; ++i) {
    CHECK(g_swap->GetBuffer(i, IID_PPV_ARGS(&g_back[i])), "GetBuffer");
    g_device->CreateRenderTargetView(g_back[i].Get(), nullptr, Rtv(1 + i));
  }
}

// visible = false makes a window that is never shown (the --present-test smoke test): it can
// neither appear nor take focus.
static void CreateWindowAndSwapchain(UINT w, UINT h, bool visible) {
  WNDCLASSW wc{}; wc.lpfnWndProc = WndProc; wc.hInstance = GetModuleHandleW(nullptr);
  wc.hCursor = LoadCursor(nullptr, IDC_ARROW); wc.lpszClassName = L"DreamrideWorld";
  wc.hIcon = LoadIconW(wc.hInstance, MAKEINTRESOURCEW(1));  // world.rc (the default icon if absent)
  wc.hbrBackground = (HBRUSH)GetStockObject(BLACK_BRUSH);
  RegisterClassW(&wc);
  RECT r{ 0, 0, (LONG)w, (LONG)h }; AdjustWindowRect(&r, WS_OVERLAPPEDWINDOW, FALSE);
  g_hwnd = CreateWindowW(wc.lpszClassName, L"Dreamride", WS_OVERLAPPEDWINDOW | (visible ? WS_VISIBLE : 0), CW_USEDEFAULT, CW_USEDEFAULT,
                         r.right - r.left, r.bottom - r.top, nullptr, nullptr, wc.hInstance, nullptr);
  if (!g_hwnd) Die("CreateWindow");
  DXGI_SWAP_CHAIN_DESC1 sd{};
  sd.Width = w; sd.Height = h; sd.Format = kLdrFmt; sd.SampleDesc = { 1, 0 };
  sd.BufferUsage = DXGI_USAGE_RENDER_TARGET_OUTPUT; sd.BufferCount = kBackBuffers;
  sd.SwapEffect = DXGI_SWAP_EFFECT_FLIP_DISCARD; sd.Scaling = DXGI_SCALING_STRETCH;
  sd.Flags = DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT;
  ComPtr<IDXGISwapChain1> sc1;
  CHECK(g_factory->CreateSwapChainForHwnd(g_queue.Get(), g_hwnd, &sd, nullptr, nullptr, &sc1), "CreateSwapChainForHwnd");
  CHECK(g_factory->MakeWindowAssociation(g_hwnd, DXGI_MWA_NO_ALT_ENTER), "MakeWindowAssociation");
  CHECK(sc1.As(&g_swap), "IDXGISwapChain3");
  CHECK(g_swap->SetMaximumFrameLatency(2), "SetMaximumFrameLatency");
  g_frameWait = g_swap->GetFrameLatencyWaitableObject();
  CreateBackbuffers();
}

static void Resize() {
  RECT rc; GetClientRect(g_hwnd, &rc);
  UINT w = std::max<LONG>(rc.right - rc.left, 8), h = std::max<LONG>(rc.bottom - rc.top, 8);
  if (w == g_t.w && h == g_t.h) return;
  WaitIdle();
  for (auto& b : g_back) b.Reset();
  CHECK(g_swap->ResizeBuffers(kBackBuffers, w, h, kLdrFmt, DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT), "ResizeBuffers");
  CreateTargets(w, h, false);
  CreateBackbuffers();
}

// Present one frame to the backbuffer. Vsync: Present(1) waits for the display's refresh, so the
// frame rate is the display's own. ui: draw the overlay built this frame.
static void PresentFrame(UINT fi, const View& v, UINT64* fenceOf, bool ui = false) {
  WaitFence(fenceOf[fi]);
  UINT bb = g_swap->GetCurrentBackBufferIndex();
  Tex back; back.r = g_back[bb]; back.st = D3D12_RESOURCE_STATE_PRESENT;
  To(back, D3D12_RESOURCE_STATE_RENDER_TARGET);  // recorded at the frame's first flush
  RecordFrame(fi, v, back.r.Get(), Rtv(1 + bb), false, ui);
  To(back, D3D12_RESOURCE_STATE_PRESENT);
  Flush();
  CHECK(g_cmd->Close(), "close");
  ID3D12CommandList* l[] = { g_cmd.Get() }; g_queue->ExecuteCommandLists(1, l);
  HRESULT hr = g_swap->Present(1, 0);
  if (FAILED(hr)) Die("Present", hr);
  fenceOf[fi] = Signal();
}

// A never-shown window: exercises the swapchain path (create, record into backbuffers, present,
// resize) without anything appearing on screen or taking focus.
static int PresentTest(UINT w, UINT h) {
  CreateWindowAndSwapchain(w, h, false);
  UINT64 fenceOf[kFrames] = {};
  for (int i = 0; i < 30; ++i) PresentFrame(i % kFrames, ViewAtSeconds(10.0 + i / 60.0), fenceOf);
  WaitIdle();
  printf("present test: 30 frames presented to a hidden %ux%u swapchain; device %s\n", w, h,
         SUCCEEDED(g_device->GetDeviceRemovedReason()) ? "ok" : "REMOVED");
  DestroyWindow(g_hwnd);
  return SUCCEEDED(g_device->GetDeviceRemovedReason()) ? 0 : 1;
}

// The window: opens at once on the void, with the song given (if any) rendering on the loader
// thread; openLibrary starts with the menu's library showing.
static int RunWindow(UINT w, UINT h, const std::string& song, bool openLibrary) {
  ImGui_ImplWin32_EnableDpiAwareness();
  float scale = ImGui_ImplWin32_GetDpiScaleForMonitor(MonitorFromPoint(POINT{ 0, 0 }, MONITOR_DEFAULTTOPRIMARY));
  if (!(scale > 0)) scale = 1;
  CreateWindowAndSwapchain((UINT)(w * scale), (UINT)(h * scale), true);
  scale = ImGui_ImplWin32_GetDpiScaleForHwnd(g_hwnd);
  InitUi(g_hwnd, scale > 0 ? scale : 1.0f);
  g_ui.ov.menu = openLibrary;
  if (!InitAudioEngine()) fprintf(stderr, "audio: playing silently (the clock runs on its own)\n");
  SetWindowTitleFor("");
  if (ui::IsStreamPath(song)) {
    int at = 0;  // the entry by its marker path (the library's order is Stream, Explore, Stream from G)
    for (int i = 0; i < (int)g_lib.songs.size(); ++i) if (g_lib.songs[i].path == song) { at = i; break; }
    StartLoad(song, g_lib.songs.empty() ? std::string("Stream") : g_lib.songs[at].title, at);
  }
  else if (!song.empty()) StartLoad(song, StemOf(song), ui::FindSong(g_lib.songs, song));
  g_lib.watch.Watch(g_lib.dirs);
  g_lib.rescan.poll = g_lib.watch.Unwatched();
  const auto opened = std::chrono::steady_clock::now();
  UINT64 fenceOf[kFrames] = {};
  UINT64 frame = 0;
  auto last = std::chrono::steady_clock::now();
  while (g_running) {
    MSG msg;
    while (PeekMessageW(&msg, nullptr, 0, 0, PM_REMOVE)) { TranslateMessage(&msg); DispatchMessageW(&msg); }
    if (!g_running) break;
    if (g_resize) { g_resize = false; Resize(); }
    WaitForSingleObjectEx(g_frameWait, 1000, TRUE);
    auto now = std::chrono::steady_clock::now();
    double dt = std::clamp(std::chrono::duration<double>(now - last).count(), 0.0, 0.25);
    last = now;
    PollLoader();
    PollHistory();
    PollReplay();
    PollLibrary(std::chrono::duration<double>(now - opened).count());
    double t = PlayerFrameTime(dt);
    PollStream(t);
    // The camera eases between follow and pull-back over about a second.
    float target = g_ui.wide ? 1.0f : 0.0f;
    g_camWide += std::clamp(target - g_camWide, (float)-dt / g_camEase, (float)dt / g_camEase);
    if (g_camWide <= 0 && g_stream.end == StreamMode::End::None) g_camSpan = g_camSpanTarget = 0;  // down from a song's end
    // At a song's end the framed span eases out (over a few seconds) when the whole song's trail comes in.
    if (g_camSpanTarget > 0) g_camSpan += (g_camSpanTarget - g_camSpan) * (1.0f - expf((float)-dt / 1.5f));
    UpdateOverlayFade(dt);
    BuildWindowUi();
    bool drawUi = g_ui.alpha > 0.002f || (g_ui.hud && HaveSong()) || (g_streaming && g_stream.eqAlpha > 0.002f) ||
                  !g_hist.writeError.empty() || g_hist.toastUntil > HistNow();
    PresentFrame((UINT)(frame++ % kFrames), ViewAtSeconds(g_ui.dragging ? DragTotal() : t), fenceOf, drawUi);
  }
  ShowWindow(g_hwnd, SW_HIDE);  // gone at once; a render still in flight finishes below
  StopStream();
  ShutdownReplay();
  g_wholeThreads.Join(true);  // a song's whole trail still building (seconds at most)
  if (g_hist.worker) { g_hist.worker->Flush(); PollHistory(); }  // the last word on the recording is on disk
  ShutdownLoader();
  ShutdownLibraryScan();
  WaitIdle();
  ShutdownUi();
  StopAudio();
  return 0;
}

// ================================================================ main

static void Usage() {
  printf("usage: luminesce-world [song.txt] [--library] [--songs DIR]... [--size W H]\n"
         "                      [--capture SECONDS OUT.png]... [--capture-menu PANEL SECONDS OUT.png]... [--ui-scale S] [--pull-back]\n"
         "                      [--bench [FRAMES]] [--bench-from SECONDS] [--bench-ui PANEL] [--audio-test] [--seek-test] [--present-test]\n"
         "                      [--stream [--seed N] [--stream-song SONG.txt]] [--stream-test [SECONDS]]\n"
         "                      [--history-dir DIR] [--history-test [DIR]] [--camera-seam-test [SEAMS]]\n"
         "  no flags: open Dreamride's window and play the song (default examples\\citypop_v4.txt)\n"
         "  --library: open with the song library showing; with no song given, nothing plays until one is chosen\n"
         "  --songs DIR: also list the *.txt songs in DIR (repeatable; examples\\ is always listed)\n"
         "  --stream: play stream mode, an endless song of eight-bar equations each evolved from the last: a new song\n"
         "            every launch (a grammar seed from the clock, shown in the equation panel); --seed N replays seed N,\n"
         "            --stream-g starts from G (the song kept from the blind test, s7-c38), and --stream-song SONG.txt\n"
         "            from that song (--seed then drives the evolution). Captures and benches work on it too (in time\n"
         "            order); E cycles the equation compact, full, hidden; S saves the last four stages as a song\n"
         "  keys: Space play/pause, R restart, Left/Right seek 5 s, Up/Down volume, N/P next/previous song,\n"
         "        L loop, Tab or M menu, H keys card, F11 fullscreen, Esc closes the menu, then quits (asks first)\n"
         "  --capture: render offscreen at that song time and write a PNG (repeatable); no window opens\n"
         "  --capture-menu: the same with the overlay drawn; PANEL is transport, library, view, theme, help,\n"
         "                  confirm, loading, equation (the stream's compact equation alone) or equation-full\n"
         "  --pull-back: the camera pulled back to show the trail (in the window: View, Pull back)\n"
         "  --ui-scale: the overlay's scale in headless captures and benches (default 1, as at 96 dpi)\n"
         "  --bench: render frames offscreen and report GPU and CPU frame times; no window opens\n"
         "  --bench-from: the song time the bench starts at (default 8 s)\n"
         "  --bench-ui PANEL: bench with the overlay built and drawn every frame\n"
         "  --audio-test: run the playback clock against XAudio2 at volume 0 for 3 s and report\n"
         "  --seek-test: seek a silent voice (volume 0) and check the audio position lands where asked\n"
         "  --present-test: render 30 frames to a never-shown window's swapchain\n"
         "  --stream-test [SECONDS]: play stream mode silently (volume 0, no window) for SECONDS (default 40) and check\n"
         "                  the audio never runs dry, keeps time, and the drawn window follows each seam\n"
         "  --history-dir DIR: keep the stream history in DIR (default %%APPDATA%%\\Dreamride\\history); headless runs\n"
         "                  record only with it, and --capture-menu history, history-open, history-delete show it\n"
         "  --camera-seam-test [SEAMS]: build a seeded stream's windows (seed 4821, or --seed N; --stream-g for G) over\n"
         "                  SEAMS seams (default 6) and check the camera and the wall never step or surge at a seam\n"
         "                  (no window, no audio, no GPU)\n"
         "  --history-test [DIR]: record a stream (seed 4821, or --seed N) silently, read it back, replay it and check\n"
         "                  it against the engine\n"
         "                  (no window, no audio; DIR keeps the recording)\n");
}

struct CaptureJob { double secs; std::string out, panel; };

int main(int argc, char** argv) {
  // Linked as a windowed app so no console opens with it. Headless runs (--capture, --bench, tests) still print:
  // redirected output keeps its inherited handles, and from a terminal we attach to the parent's console.
  if (!GetStdHandle(STD_OUTPUT_HANDLE) && AttachConsole(ATTACH_PARENT_PROCESS)) {
    FILE* f = nullptr;
    freopen_s(&f, "CONOUT$", "w", stdout);
    freopen_s(&f, "CONOUT$", "w", stderr);
  }
  setvbuf(stdout, nullptr, _IOFBF, 1 << 16);
  timeBeginPeriod(1);  // millisecond sleeps for the audio test and the frame wait
  g_exeDir = ExeDir();
  std::string song;
  UINT w = 0, h = 0;
  std::vector<CaptureJob> captures;
  std::vector<std::string> songDirs;
  std::string benchUi;
  float uiScale = 1.0f;
  int bench = 0; double benchFrom = 8.0; bool audioTest = false, presentTest = false, openLibrary = false, seekTest = false;
  bool streamMode = false, seedGiven = false, streamG = false; std::string streamSong;
  double streamTest = 0;
  std::string historyDir, historyTestDir, replayTest; bool historyTest = false, cameraSeamTest = false; int seams = 6;
  for (int i = 1; i < argc; ++i) {
    std::string a = argv[i];
    if (a == "--size" && i + 2 < argc) { w = (UINT)atoi(argv[i + 1]); h = (UINT)atoi(argv[i + 2]); i += 2; }
    else if (a == "--capture" && i + 2 < argc) { captures.push_back({ atof(argv[i + 1]), argv[i + 2], "" }); i += 2; }
    else if (a == "--capture-menu" && i + 3 < argc) {
      if (!IsCapturePanel(argv[i + 1])) { fprintf(stderr, "--capture-menu: no panel named '%s'\n", argv[i + 1]); Usage(); return 2; }
      captures.push_back({ atof(argv[i + 2]), argv[i + 3], argv[i + 1] }); i += 3;
    }
    else if (a == "--bench") { bench = 600; if (i + 1 < argc && argv[i + 1][0] != '-') bench = std::max(10, atoi(argv[++i])); }
    else if (a == "--bench-from" && i + 1 < argc) benchFrom = atof(argv[++i]);
    else if (a == "--bench-ui" && i + 1 < argc) {
      benchUi = argv[++i];
      if (!IsCapturePanel(benchUi)) { fprintf(stderr, "--bench-ui: no panel named '%s'\n", benchUi.c_str()); Usage(); return 2; }
    }
    else if (a == "--ui-scale" && i + 1 < argc) {
      uiScale = (float)atof(argv[++i]);
      if (!(uiScale >= 0.5f && uiScale <= 4.0f)) { fprintf(stderr, "--ui-scale takes 0.5 to 4\n"); return 2; }
    }
    else if (a == "--songs" && i + 1 < argc) songDirs.push_back(argv[++i]);
    else if (a == "--library") openLibrary = true;
    else if (a == "--stream") streamMode = true;
    else if (a == "--stream-g") { streamMode = true; streamG = true; }
    else if (a == "--seed" && i + 1 < argc) { g_streamCfg.seed = strtoull(argv[++i], nullptr, 10); seedGiven = true; }
    else if (a == "--stream-song" && i + 1 < argc) { streamSong = argv[++i]; streamMode = true; }
    else if (a == "--pull-back") { g_camWide = 1.0f; g_ui.wide = true; }  // the pull-back camera (captures, benches, the window)
    else if (a == "--audio-test") audioTest = true;
    else if (a == "--seek-test") seekTest = true;
    else if (a == "--present-test") presentTest = true;
    else if (a == "--stream-test") { streamTest = 40; if (i + 1 < argc && argv[i + 1][0] != '-') streamTest = std::max(1.0, atof(argv[++i])); }
    else if (a == "--history-dir" && i + 1 < argc) historyDir = argv[++i];
    else if (a == "--replay-test" && i + 1 < argc) replayTest = argv[++i];
    else if (a == "--camera-seam-test") { cameraSeamTest = true; if (i + 1 < argc && argv[i + 1][0] != '-') seams = std::max(1, atoi(argv[++i])); }
    else if (a == "--history-test") { historyTest = true; if (i + 1 < argc && argv[i + 1][0] != '-') historyTestDir = argv[++i]; }
    else if (a == "--help" || a == "-h") { Usage(); return 0; }
    else if (a.rfind("--", 0) == 0) { fprintf(stderr, "unknown option %s\n", a.c_str()); Usage(); return 2; }
    else song = a;
  }
  if (streamTest > 0) streamMode = true;
  bool headless = !captures.empty() || bench > 0 || audioTest || presentTest || seekTest || streamTest > 0 || historyTest || cameraSeamTest || !replayTest.empty();
  if (streamMode) {
    if (audioTest || seekTest) { fprintf(stderr, "--audio-test and --seek-test play a song, not a stream\n"); return 2; }
    if (!streamSong.empty()) {
      std::ifstream f(std::filesystem::u8path(streamSong), std::ios::binary);
      if (!f) { fprintf(stderr, "cannot read %s\n", streamSong.c_str()); return 1; }
      g_streamCfg.seedText.assign(std::istreambuf_iterator<char>(f), std::istreambuf_iterator<char>());
    } else if (seedGiven) {
      g_streamCfg.flags = LM_STREAM_GRAMMAR;
    }
    if (!streamSong.empty() || seedGiven) g_streamCfg.fresh = false;  // a given seed or song: the same stream every time
    song = streamG ? ui::kStreamGPath : ui::kStreamPath;
    // Captures go in time order: the engine keeps only the stages around the one being played.
    std::stable_sort(captures.begin(), captures.end(), [](const CaptureJob& a, const CaptureJob& b) { return a.secs < b.secs; });
  }
  if (song.empty() && (headless || !openLibrary)) {
    // The default: the repository's example, found from the exe in world\bin.
    char p[MAX_PATH]; WideCharToMultiByte(CP_UTF8, 0, (g_exeDir + L"..\\..\\examples\\citypop_v4.txt").c_str(), -1, p, MAX_PATH, nullptr, nullptr);
    song = p;
  }
  if (w == 0 || h == 0) { w = headless ? 1920 : 1600; h = headless ? 1080 : 900; }

  if (const char* th = getenv("LM_THEME")) {
    if (!ApplyThemeOverrides(th)) return 2;
    printf("theme overrides: %s\n", th);
  }
  g_themeStart = g_theme;

  // The library: the repository's examples, then every --songs folder.
  {
    char p[MAX_PATH]; WideCharToMultiByte(CP_UTF8, 0, (g_exeDir + L"..\\..\\examples").c_str(), -1, p, MAX_PATH, nullptr, nullptr);
    std::error_code ec;
    std::filesystem::path ex = std::filesystem::weakly_canonical(std::filesystem::u8path(p), ec);
    g_lib.dirs.push_back(ec ? std::string(p) : ex.u8string());
    for (auto& d : songDirs) g_lib.dirs.push_back(d);
    g_lib.songs = ui::WithStream(ui::ScanSongs(g_lib.dirs, &g_lib.errors));
    for (auto& e : g_lib.errors) fprintf(stderr, "library: %s\n", e.c_str());
  }

  // The stream history: %APPDATA%\Dreamride\history, or --history-dir. Headless runs read it (for captures of the
  // History tab) but record nothing unless --history-dir names a folder for them.
  if (historyTest) { int rc = HistoryTest(historyTestDir); fflush(stdout); return rc; }
  if (cameraSeamTest) { int rc = CameraSeamTest(seedGiven ? g_streamCfg.seed : 4821, seams, streamG ? 0u : (uint32_t)LM_STREAM_GRAMMAR); fflush(stdout); return rc; }
  {
    std::string err, dir = historyDir.empty() ? DefaultHistoryDir(err) : historyDir;
    HistInit(dir, !headless || !historyDir.empty());
    if (dir.empty()) { g_hist.writeError = "History is off: " + err; fprintf(stderr, "history: %s\n", err.c_str()); }
    if (headless) { g_hist.worker->Flush(); PollHistory(); }  // the list is read before any capture
  }

  if (streamTest > 0 || !replayTest.empty()) {
    MakeEmptySong(g_song, g_theme);  // the test starts the stream the way the window does, on the loader
  } else if (headless && streamMode) {
    std::string err;
    g_stream.s = StartStreamNow(StreamConfigFor(song), err);
    if (!g_stream.s || !LoadStreamWindowInto(g_song, g_stream.s, 0, 0, g_theme, err)) return 1;
    lm_stream_info(g_stream.s, &g_stream.info);
    g_streaming = true;
    g_stream.windowStage = 0;
    g_lib.current = 0;  // the entry by its marker path (the library's order is Stream, Explore, Stream from G)
    for (int i = 0; i < (int)g_lib.songs.size(); ++i) if (g_lib.songs[i].path == (streamG ? ui::kStreamGPath : ui::kStreamPath)) { g_lib.current = i; break; }
    g_stream.fromG = streamG;
    g_lib.currentTitle = g_lib.songs[g_lib.current].title;
    g_lib.currentDescription = g_lib.songs[g_lib.current].description;
  } else if (headless) {
    std::string err;
    if (!LoadSongInto(g_song, song.c_str(), g_theme, err)) return 1;
    g_lib.current = ui::FindSong(g_lib.songs, song);
    g_lib.currentTitle = StemOf(song);
    g_lib.currentDescription = g_lib.current >= 0 ? g_lib.songs[g_lib.current].description : std::string();
  } else {
    MakeEmptySong(g_song, g_theme);  // the window opens on the void; the song renders behind it
  }
  if (seekTest) { int rc = SeekTest(); fflush(stdout); if (rc || (captures.empty() && !bench && !presentTest && !audioTest)) return rc; }
  if (audioTest) { int rc = AudioClockTest(3.0); fflush(stdout); if (captures.empty() && !bench && !presentTest) return rc; }

  CreateDevice();
  LoadShaderSource();
  InitRenderer(w, h, headless);
  bool anyUi = !benchUi.empty();
  for (auto& c : captures) anyUi |= !c.panel.empty();
  if (headless && anyUi) InitUi(nullptr, uiScale);
  fflush(stdout);

  int rc = 0;
  for (auto& c : captures) {
    bool ui = !c.panel.empty();
    g_play.now = c.secs;  // panels that read the time (the ending) see the capture's
    if (g_streaming && !AdvanceHeadlessStream(c.secs)) { rc = 1; continue; }  // first: the ending reads its stages
    if (ui) ApplyCapturePanel(c.panel);
    if (!Capture(c.secs, c.out.c_str(), ui)) rc = 1;
    if (ui) ResetCapturePanel();
  }
  if (bench) {
    if (!benchUi.empty()) ApplyCapturePanel(benchUi);
    if (g_streaming && !AdvanceHeadlessStream(benchFrom)) rc = 1;
    Bench(bench, benchFrom, !benchUi.empty());
    if (!benchUi.empty()) ResetCapturePanel();
  }
  if (presentTest) rc |= PresentTest(w, h);
  if (streamTest > 0) rc |= StreamTest(streamTest);
  if (!replayTest.empty()) rc |= ReplayTest(replayTest);
  if (headless) ShutdownUi();
  if (!headless) rc = RunWindow(w, h, song, openLibrary);
  fflush(stdout);
  StopStream();
  if (g_song.lm) lm_free(g_song.lm);
  return rc;
}
