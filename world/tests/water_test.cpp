// Tests for world/src/water.h (the bass on the water: a note's frequency to the drawn ripple wavelength, and how a
// voice's coupling into the water falls off with its height). No window, no GPU. Built and run by
// `world\build.bat test`.
#include <cmath>
#include <cstdio>
#include <functional>
#include <vector>

#include "../src/water.h"

static int g_failed = 0, g_checks = 0;
static const char* g_test = "";
#define CHECK(cond) do { ++g_checks; if (!(cond)) { ++g_failed; printf("  FAIL %s: %s (line %d)\n", g_test, #cond, __LINE__); } } while (0)

struct Test { const char* name; std::function<void()> fn; };
static std::vector<Test>& Tests() { static std::vector<Test> t; return t; }
struct Reg { Reg(const char* n, std::function<void()> f) { Tests().push_back({ n, f }); } };
#define TEST(name) static void name(); static Reg reg_##name(#name, name); static void name()

static const double kScale = 90.0;  // the void theme's bassWaveScale

TEST(the_dispersion_root_solves_the_capillary_gravity_relation) {
  for (double w : { 1.0, 10.0, 172.8, 691.0, 30000.0 }) {
    double k = water::DispersionK(w);
    CHECK(k > 0);
    CHECK(fabs(water::kSigmaRho * k * k * k + water::kG * k - w * w) <= 1e-9 * w * w);
  }
}

TEST(the_dispersion_matches_both_limits) {
  // Long waves are gravity waves (k = w^2 / g), short ones capillary (k = (w^2 rho / sigma)^(1/3)).
  double wLong = 0.5, wShort = 1e5;
  CHECK(fabs(water::DispersionK(wLong) / (wLong * wLong / water::kG) - 1.0) < 1e-3);
  CHECK(fabs(water::DispersionK(wShort) / std::cbrt(wShort * wShort / water::kSigmaRho) - 1.0) < 1e-3);
}

TEST(a_higher_note_draws_finer_ripples_everywhere) {
  double prev = water::FaradayWavelength(20.0, kScale);
  for (double hz = 20.5; hz <= 4000.0; hz *= 1.0 + 1.0 / 64.0) {  // a sixteenth of a semitone apart, roughly
    double l = water::FaradayWavelength(hz, kScale);
    CHECK(l < prev);
    prev = l;
  }
}

TEST(bass_wavelengths_are_in_a_drawable_range) {
  // The bass register and beyond, 30 Hz to 500 Hz: between 10 cm and 1.5 m at the default scale.
  for (double hz = 30.0; hz <= 500.0; hz += 1.0) {
    double l = water::FaradayWavelength(hz, kScale);
    CHECK(l >= 0.10 && l <= 1.5);
  }
  // The anchors the header and the README quote.
  CHECK(fabs(water::FaradayWavelength(55.0, kScale) - 0.83) < 0.02);
  CHECK(fabs(water::FaradayWavelength(110.0, kScale) - 0.50) < 0.02);
  CHECK(fabs(water::FaradayWavelength(220.0, kScale) - 0.31) < 0.02);
}

TEST(an_octave_up_is_about_the_capillary_two_thirds_power) {
  // 2^(-2/3) = 0.63 where surface tension rules; gravity (k = w^2 / g, an octave 2^-2) pulls the low end shorter,
  // so the ratio rises toward 0.63 with frequency and never passes it.
  double prev = 0;
  for (double hz : { 40.0, 80.0, 160.0, 320.0, 640.0 }) {
    double r = water::FaradayWavelength(2.0 * hz, kScale) / water::FaradayWavelength(hz, kScale);
    CHECK(r > 0.55 && r <= pow(2.0, -2.0 / 3.0) + 1e-9);
    CHECK(r > prev);
    prev = r;
  }
  CHECK(fabs(prev - pow(2.0, -2.0 / 3.0)) < 0.005);
}

TEST(the_wavelength_scales_linearly_with_the_theme) {
  CHECK(fabs(water::FaradayWavelength(110.0, 2.0 * kScale) - 2.0 * water::FaradayWavelength(110.0, kScale)) < 1e-12);
}

TEST(invalid_frequencies_and_scales_draw_nothing) {
  CHECK(water::FaradayWavelength(0.0, kScale) == 0);
  CHECK(water::FaradayWavelength(-55.0, kScale) == 0);
  CHECK(water::FaradayWavelength(NAN, kScale) == 0);
  CHECK(water::FaradayWavelength(INFINITY, kScale) == 0);
  CHECK(water::FaradayWavelength(110.0, 0.0) == 0);
  CHECK(water::FaradayWavelength(110.0, NAN) == 0);
  CHECK(water::VisualHz(NAN) == 0);
  CHECK(water::VisualHz(-1.0) == 0);
  // Out of range but valid: held to the ends, still finite.
  CHECK(water::FaradayWavelength(0.01, kScale) == water::FaradayWavelength(1.0, kScale));
  CHECK(water::FaradayWavelength(1e6, kScale) == water::FaradayWavelength(20000.0, kScale));
}

TEST(the_drawn_motion_rises_with_pitch_and_is_capped) {
  CHECK(water::VisualHz(55.0) < water::VisualHz(110.0));
  CHECK(fabs(water::VisualHz(55.0) - 27.5 / water::kBassTimeScale) < 1e-12);
  CHECK(water::VisualHz(20000.0) == water::kBassMaxVisualHz);
  // At the cap, a crest moves at most a sixth of its wavelength a frame at 60 fps.
  CHECK(water::kBassMaxVisualHz / 60.0 <= 1.0 / 6.0 + 1e-12);
}

TEST(the_front_moves_at_the_group_velocity_between_gravity_and_capillary) {
  // (dw/dk) / (w/k): 0.5 for gravity waves, 1.5 for capillary ones, rising with frequency in between.
  CHECK(fabs(water::GroupOverPhase(1.0) - 0.5) < 0.01);
  CHECK(fabs(water::GroupOverPhase(20000.0) - 1.5) < 0.01);
  double prev = 0;
  for (double hz = 1.0; hz <= 20000.0; hz *= 1.25) {
    double r = water::GroupOverPhase(hz);
    CHECK(r > prev && r >= 0.5 - 1e-9 && r <= 1.5 + 1e-9);
    prev = r;
  }
  // Against a numerical derivative of the dispersion relation at a bass note.
  double w = 2.0 * 3.14159265358979 * 27.5, dw = 1e-3 * w;
  double cg = 2.0 * dw / (water::DispersionK(w + dw) - water::DispersionK(w - dw));
  double vp = w / water::DispersionK(w);
  CHECK(fabs(water::GroupOverPhase(55.0) - cg / vp) < 1e-5);
  CHECK(water::GroupOverPhase(0.0) == 0);
  CHECK(water::GroupOverPhase(NAN) == 0);
}

TEST(coupling_is_one_at_the_surface_and_falls_with_height) {
  const double reach = 0.8;
  CHECK(water::Coupling(0.0, reach) == 1.0);
  CHECK(water::Coupling(-0.2, reach) == 1.0);  // below the surface counts as at it
  double prev = 1.0;
  for (double h = 0.05; h <= 6.0; h += 0.05) {
    double c = water::Coupling(h, reach);
    CHECK(c < prev && c > 0);
    prev = c;
  }
  CHECK(fabs(water::Coupling(reach, reach) - exp(-1.0)) < 1e-12);  // one e-fold per reach
  CHECK(water::Coupling(5.6, reach) < 1e-3);  // the top of the wall barely touches it
}

TEST(a_longer_reach_couples_more_at_every_height) {
  for (double h = 0.1; h <= 5.0; h += 0.3) CHECK(water::Coupling(h, 1.2) > water::Coupling(h, 0.6));
}

TEST(invalid_coupling_inputs_couple_nothing) {
  CHECK(water::Coupling(0.5, 0.0) == 0);
  CHECK(water::Coupling(0.5, -1.0) == 0);
  CHECK(water::Coupling(NAN, 0.8) == 0);
  CHECK(water::Coupling(0.5, NAN) == 0);
}

int main() {
  for (auto& t : Tests()) {
    g_test = t.name;
    int before = g_failed;
    t.fn();
    printf("%s %s\n", g_failed == before ? "ok  " : "FAIL", t.name);
  }
  printf("  (55 Hz: %.3f m, 110 Hz: %.3f m, 220 Hz: %.3f m at scale %.0f)\n", water::FaradayWavelength(55, kScale),
         water::FaradayWavelength(110, kScale), water::FaradayWavelength(220, kScale), kScale);
  printf("\n%zu tests, %d checks, %d failed\n", Tests().size(), g_checks, g_failed);
  return g_failed ? 1 : 0;
}
