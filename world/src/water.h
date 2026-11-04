// water.h: the bass on the water, the numbers with no GPU, so they can be tested alone
// (world/tests/water_test.cpp). A voice low on the wall, near the water, shakes it: its note drives the surface
// and the surface answers in standing waves whose wavelength is set by the note's pitch (cymatics).
//
// The physics, at true scale, then one visual scale:
// - A liquid surface shaken at frequency f answers at f / 2 (Faraday waves, the parametric resonance).
// - A ripple at angular frequency w on deep water has the wavenumber k that solves the capillary-gravity
//   dispersion relation  w^2 = g k + (sigma / rho) k^3  (water: g 9.81 m/s^2, sigma / rho 7.28e-5 m^3/s^2).
// - Its wavelength 2 pi / k is millimetres for a bass note (55 Hz: 9.2 mm), so the picture multiplies it by the
//   theme's wavelength scale (bassWaveScale, 90 in the void theme: 55 Hz is drawn 0.83 m, 110 Hz 0.50 m, 220 Hz 0.31 m).
//   One factor for every note, so the ratios are the water's own: an octave up is 0.6 to 0.63 times the wavelength
//   (f^-2/3 where surface tension rules, a little less in the low bass where gravity joins in). Higher note,
//   finer ripples, always (monotonic).
// - Time is slowed by kBassTimeScale too (55 Hz answers at 27.5 Hz, drawn at 6.9 Hz), and held under
//   kBassMaxVisualHz so a crest never moves more than about a sixth of its wavelength between frames at 60 fps
//   (from about 80 Hz up, every note moves at that rate; its wavelength still follows its pitch).
// - A ring's front, where its energy has reached, moves at the group velocity dw/dk, which on water is not the
//   crests' speed: 1.5 times it where surface tension rules, 0.5 where gravity does (GroupOverPhase).
// - How hard a voice shakes the water falls off with its height above it: exp(-h / reach).
#pragma once

#include <algorithm>
#include <cmath>

namespace water {

static const double kG = 9.81;               // m/s^2
static const double kSigmaRho = 0.0728 / 1000.0;  // water's surface tension over its density, m^3/s^2
static const double kBassTimeScale = 4.0;    // the water's response is drawn this many times slower
static const double kBassMaxVisualHz = 10.0;  // never faster than this on screen (60 fps: 6 frames a cycle)
static const double kMinDriveHz = 1.0, kMaxDriveHz = 20000.0;

// The wavenumber (rad/m, true scale) of a ripple at angular frequency w (rad/s) on deep water: the one positive
// root of (sigma/rho) k^3 + g k - w^2, which increases with k, so Newton from above converges. 0 for w <= 0.
inline double DispersionK(double w) {
  if (!(w > 0) || !std::isfinite(w)) return 0;
  double w2 = w * w;
  double k = std::max(w2 / kG, std::cbrt(w2 / kSigmaRho));  // above the root: each term alone overshoots
  for (int i = 0; i < 60; ++i) {
    double f = kSigmaRho * k * k * k + kG * k - w2;
    double d = 3.0 * kSigmaRho * k * k + kG;
    double nk = k - f / d;
    if (fabs(nk - k) <= 1e-12 * k) { k = nk; break; }
    k = nk;
  }
  return k;
}

// The drawn wavelength (m) of the standing waves a note at driveHz raises: Faraday's half frequency, the water's
// dispersion, times the visual scale. 0 (draw nothing) for a frequency that is not finite and positive or a scale
// that is not; the frequency is held to [1 Hz, 20 kHz].
inline double FaradayWavelength(double driveHz, double scale) {
  if (!(driveHz > 0) || !std::isfinite(driveHz) || !(scale > 0) || !std::isfinite(scale)) return 0;
  double f = std::clamp(driveHz, kMinDriveHz, kMaxDriveHz) * 0.5;
  double k = DispersionK(2.0 * 3.14159265358979 * f);
  return k > 0 ? scale * 2.0 * 3.14159265358979 / k : 0;
}

// How often (Hz, drawn) the surface under a note at driveHz moves: Faraday's half frequency, slowed, capped.
inline double VisualHz(double driveHz) {
  if (!(driveHz > 0) || !std::isfinite(driveHz)) return 0;
  return std::min(std::clamp(driveHz, kMinDriveHz, kMaxDriveHz) * 0.5 / kBassTimeScale, kBassMaxVisualHz);
}

// The group velocity over the phase velocity of the ripple a note at driveHz raises, at true scale:
// (dw/dk) / (w/k) = k (g + 3 (sigma/rho) k^2) / (2 w^2), from 0.5 (gravity) to 1.5 (capillary). 0 for an invalid
// frequency. The drawn front moves at this times the drawn crests' speed.
inline double GroupOverPhase(double driveHz) {
  if (!(driveHz > 0) || !std::isfinite(driveHz)) return 0;
  double w = 2.0 * 3.14159265358979 * std::clamp(driveHz, kMinDriveHz, kMaxDriveHz) * 0.5;
  double k = DispersionK(w);
  return k * (kG + 3.0 * kSigmaRho * k * k) / (2.0 * w * w);
}

// How strongly a voice at height h (m) above the water couples into it: 1 at the surface, exp(-h / reach) above it.
// 0 for a reach that is not positive or an h that is not finite.
inline double Coupling(double h, double reach) {
  if (!(reach > 0) || !std::isfinite(h) || !std::isfinite(reach)) return 0;
  return exp(-std::max(h, 0.0) / reach);
}

}  // namespace water
