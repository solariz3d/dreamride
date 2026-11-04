// Tests for world/src/math_layout.h (the live equation's layout: parsing the engine's markup, placing
// fractions, brackets and scripts, breaking long lines, the timing at a seam, where the panel goes) and
// the stream helpers in world/src/ui_logic.h. No window, no GPU. Built and run by `world\build.bat test`.
#include <cstdio>
#include <functional>
#include <set>
#include <string>
#include <vector>

#include "../src/math_layout.h"
#include "../src/ui_logic.h"

using namespace mathui;

static int g_failed = 0, g_checks = 0;
static const char* g_test = "";
#define CHECK(cond) do { ++g_checks; if (!(cond)) { ++g_failed; printf("  FAIL %s: %s (line %d)\n", g_test, #cond, __LINE__); } } while (0)

struct Test { const char* name; std::function<void()> fn; };
static std::vector<Test>& Tests() { static std::vector<Test> t; return t; }
struct Reg { Reg(const char* n, std::function<void()> f) { Tests().push_back({ n, f }); } };
#define TEST(name) static void name(); static Reg reg_##name(#name, name); static void name()

// A monospaced stand-in for the font: every character (code point) is half the size wide, italic or not.
static float Mono(const std::string& s, bool, float size) {
  int n = 0;
  for (unsigned char c : s) n += (c & 0xC0) != 0x80;
  return 0.5f * size * n;
}
static Layouter L() { return Layouter{ Mono, Metrics{} }; }

// Markup as the engine writes it (src/compose/display.rs; the Rust tests pin these forms).
static const char* kLens = "\\it{h}\\paren{\\it{d}} = \\floor{\\frac{12\\it{d} + \\it{c}}{7}}";
static const char* kRoots = "\\it{R}\\paren{\\it{i}} = \\rm{mod}\\paren{\\floor{\\frac{\\it{N}}{10\\sup{7 − \\rm{mod}\\paren{\\it{i}, 8}}}}, 10}\\brk − 5";
static const char* kMelody =
  "\\it{M}\\paren{\\it{k}} = \\it{R}\\paren{\\floor{\\frac{\\it{k}}{16}}}\\brk + 1\\,\\rm{sq}\\paren{\\frac{\\it{k}}{\\hl{4}} + 0.25}"
  "\\brk + 2\\,\\rm{sq}\\paren{\\frac{\\it{k}}{16}}\\brk + 4\\,\\rm{sq}\\paren{\\frac{\\it{k}}{2} + 0.5}";

static void Collect(const Node& n, std::vector<const Node*>& out) {
  out.push_back(&n);
  for (auto& k : n.kids) Collect(k, out);
}
static const Node* FindText(const Node& n, const std::string& t) {
  std::vector<const Node*> all; Collect(n, all);
  for (auto* p : all) if (p->kind == Kind::Text && p->text == t) return p;
  return nullptr;
}

// ---------------------------------------------------------------- parsing

TEST(the_markup_parses_to_the_definition_it_spells) {
  CHECK(PlainText(Parse(kLens)) == "h(d) = floor((12d + c)/(7))");
  CHECK(PlainText(Parse(kRoots)) == "R(i) = mod(floor((N)/(10^7 − mod(i, 8))), 10) − 5");
}

TEST(names_are_italic_and_numbers_upright) {
  Node n = Parse(kLens);
  const Node* h = FindText(n, "h");
  const Node* twelve = FindText(n, "12");
  CHECK(h && h->italic);
  CHECK(twelve && !twelve->italic);
  Node r = Parse(kRoots);
  const Node* mod = FindText(r, "mod");
  CHECK(mod && !mod->italic);
}

TEST(only_the_highlighted_term_is_marked) {
  Node n = Parse(kMelody);
  std::vector<const Node*> all; Collect(n, all);
  int lit = 0;
  for (auto* p : all) if (p->kind == Kind::Text && p->hl) { ++lit; CHECK(p->text == "4"); }
  CHECK(lit == 1);
}

TEST(stray_or_unclosed_braces_do_not_crash_or_loop) {
  for (const char* bad : { "}", "{", "\\frac{a}", "\\frac", "\\hl{", "a}}b{{", "\\", "\\unknown{x}", "" }) {
    Node n = Parse(bad);
    Laid l = L().Layout(n, 20);
    CHECK(l.w >= 0);
  }
}

// ---------------------------------------------------------------- layout

TEST(a_fraction_stacks_its_numerator_over_the_bar_over_its_denominator) {
  Laid l = L().Layout(Parse("\\frac{12d + c}{7}"), 20);
  CHECK(l.strokes.size() == 1);
  float bar = l.strokes[0].y0;
  CHECK(bar < 0);  // the bar sits above the baseline, at the axis
  bool num = false, den = false;
  for (auto& g : l.glyphs) {
    if (g.text == "12d + c") { num = true; CHECK(g.y < bar); CHECK(g.x > 0); }
    if (g.text == "7") { den = true; CHECK(g.y > bar); CHECK(g.x + Mono("7", false, g.size) <= l.w); }
  }
  CHECK(num && den);
  CHECK(l.w >= Mono("12d + c", false, 20));
  CHECK(l.asc > 20 * Metrics{}.asc);  // taller than a line of text
}

TEST(floor_brackets_reach_the_full_height_of_what_they_hold) {
  Laid l = L().Layout(Parse(kLens), 20);
  float top = 0, bot = 0;
  for (auto& s : l.strokes) { top = std::min(top, std::min(s.y0, s.y1)); bot = std::max(bot, std::max(s.y0, s.y1)); }
  Laid frac = L().Layout(Parse("\\frac{12\\it{d} + \\it{c}}{7}"), 20);
  CHECK(top <= -frac.asc);
  CHECK(bot >= frac.desc);
  // Two vertical strokes, two feet, the fraction bar (h(d)'s parentheses are the font's glyphs).
  int vertical = 0, feet = 0;
  for (auto& s : l.strokes) {
    if (s.x0 == s.x1 && s.y1 - s.y0 > frac.asc) ++vertical;
    if (s.y0 == s.y1 && s.y0 > 0) ++feet;
  }
  CHECK(vertical == 2);
  CHECK(feet == 2);
}

TEST(an_exponent_is_raised_and_smaller) {
  Laid l = L().Layout(Parse("10\\sup{7 − i}"), 20);
  const Glyph* base = nullptr; const Glyph* sup = nullptr;
  for (auto& g : l.glyphs) { if (g.text == "10") base = &g; if (g.text == "7 − i") sup = &g; }
  CHECK(base && sup);
  if (base && sup) {
    CHECK(sup->y < base->y);
    CHECK(sup->size < base->size);
    CHECK(sup->x >= base->x + Mono("10", false, 20) - 0.01f);
  }
}

TEST(tall_parentheses_are_drawn_as_arcs_sized_to_their_content) {
  Laid flat = L().Layout(Parse("\\paren{x}"), 20);
  Laid tall = L().Layout(Parse("\\paren{\\frac{a}{b}}"), 20);
  CHECK(tall.asc + tall.desc > flat.asc + flat.desc);
  CHECK(!tall.strokes.empty());
  // Around one line of text, the font's own glyphs: no strokes, "(" first and ")" last.
  CHECK(flat.strokes.empty());
  CHECK(flat.glyphs.size() == 3 && flat.glyphs.front().text == "(" && flat.glyphs.back().text == ")");
}

TEST(every_glyph_stays_inside_its_box) {
  for (const char* m : { kLens, kRoots, kMelody }) {
    Laid l = L().Layout(Parse(m), 20);
    for (auto& g : l.glyphs) {
      CHECK(g.x >= -0.01f);
      CHECK(g.x + Mono(g.text, g.italic, g.size) <= l.w + 0.01f);
      CHECK(g.y - Metrics{}.asc * g.size >= -l.asc - 0.01f);
      CHECK(g.y + Metrics{}.desc * g.size <= l.desc + 0.01f);
    }
  }
}

// ---------------------------------------------------------------- line breaking

static size_t Glyphs(const Lines& ls) { size_t n = 0; for (auto& l : ls.lines) n += l.glyphs.size(); return n; }

TEST(a_long_definition_breaks_only_before_a_top_level_plus_or_minus) {
  Layouter lay = L();
  Node n = Parse(kMelody);
  Laid whole = lay.Layout(n, 20);
  Lines one = BreakLines(lay, n, 20, 1e9f, 30);
  CHECK(one.lines.size() == 1);
  float maxW = whole.w * 0.55f;
  Lines ls = BreakLines(lay, n, 20, maxW, 30);
  CHECK(ls.lines.size() >= 2);
  CHECK(Glyphs(ls) == whole.glyphs.size());  // nothing lost, nothing doubled
  for (size_t i = 0; i < ls.lines.size(); ++i) {
    CHECK(ls.lines[i].w <= maxW + 0.01f);
    float left = 1e9f;
    for (auto& g : ls.lines[i].glyphs) left = std::min(left, g.x);
    if (i == 0) CHECK(left == 0);
    else {
      CHECK(left >= 30);  // a continuation is indented
      // and it starts with the operator it broke before
      const Glyph* first = &ls.lines[i].glyphs[0];
      for (auto& g : ls.lines[i].glyphs) if (g.x < first->x) first = &g;
      CHECK(first->text.rfind(" + ", 0) == 0 || first->text.rfind(" − ", 0) == 0);
    }
  }
}

TEST(a_segment_wider_than_the_line_stays_whole) {
  Layouter lay = L();
  Node n = Parse("\\it{x} = \\frac{a + b + c + d + e + f}{2}\\brk + 1");
  Lines ls = BreakLines(lay, n, 20, 40, 10);
  CHECK(ls.lines.size() == 2);
  CHECK(ls.lines[0].w > 40);
  CHECK(Glyphs(ls) == lay.Layout(n, 20).glyphs.size());
}

TEST(an_empty_definition_lays_out_as_nothing) {
  Lines ls = BreakLines(L(), Parse(""), 20, 100, 10);
  CHECK(ls.lines.size() <= 1);
  CHECK(Glyphs(ls) == 0);
}

// ---------------------------------------------------------------- timing at a seam

TEST(the_new_equation_crossfades_in_over_the_seam) {
  CHECK(NewAlpha(0.0) == 0.0);
  CHECK(NewAlpha(kCrossfade) == 1.0);
  CHECK(NewAlpha(100.0) == 1.0);
  CHECK(NewAlpha(-1.0) == 1.0);  // no seam yet: the first stage shows fully
  double prev = -1;
  for (double t = 0; t <= kCrossfade; t += 0.01) { CHECK(NewAlpha(t) >= prev); prev = NewAlpha(t); }
}

TEST(changed_terms_light_up_hold_and_fade_out) {
  CHECK(Highlight(-0.5) == 0.0);
  CHECK(Highlight(0.0) == 0.0);
  CHECK(Highlight(kHlRise) == 1.0);
  CHECK(Highlight(1.0) == 1.0);
  CHECK(Highlight(kHlHold) == 1.0);
  CHECK(Highlight(kHlEnd) == 0.0);
  CHECK(Highlight(60.0) == 0.0);
  CHECK(Highlight(std::nan("")) == 0.0);
  for (double t = 0; t < kHlEnd + 1; t += 0.01) CHECK(Highlight(t) >= 0 && Highlight(t) <= 1);
  for (double t = kHlHold; t + 0.01 < kHlEnd; t += 0.01) CHECK(Highlight(t + 0.01) <= Highlight(t));
}

// ---------------------------------------------------------------- placement

TEST(the_panel_takes_the_first_corner_clear_of_the_draw_head) {
  float W = 1920, H = 1080, m = 24, w = 600, h = 300;
  Rect free = PlacePanel(w, h, W, H, m, {});
  CHECK(free.x0 == m && free.y0 == m);  // top left by default (since 2026-11-03)
  Rect headHighLeft{ 100, 100, 200, 200 };
  Rect r = PlacePanel(w, h, W, H, m, { headHighLeft });
  CHECK(!r.Overlaps(headHighLeft));
  CHECK(r.x0 == m && r.y1 == H - m);  // bottom left
  Rect wideLeft{ 0, 0, 700, 1080 };
  r = PlacePanel(w, h, W, H, m, { wideLeft });
  CHECK(!r.Overlaps(wideLeft));
  CHECK(r.x1 == W - m);
}

TEST(with_every_corner_covered_the_panel_covers_the_least) {
  float W = 1000, H = 800, m = 10, w = 300, h = 200;
  std::vector<Rect> avoid = { { 0, 0, 1000, 120 }, { 0, 700, 1000, 800 } };  // top and bottom bands
  Rect r = PlacePanel(w, h, W, H, m, avoid);
  // The bottom corners overlap the bottom band by 90 px of height, the top corners the top band by 110: bottom wins.
  CHECK(r.y1 == H - m);
}

// ---------------------------------------------------------------- the compact equation

TEST(e_cycles_compact_full_hidden) {
  CHECK(NextEqMode(EqMode::Compact) == EqMode::Full);
  CHECK(NextEqMode(EqMode::Full) == EqMode::Hidden);
  CHECK(NextEqMode(EqMode::Hidden) == EqMode::Compact);
}

TEST(the_compact_strip_keeps_changed_lines_then_generators_then_voice_lines_then_the_lens) {
  CHECK(CompactPriority('T', false) < 0 && CompactPriority('T', true) < 0);  // time names never
  CHECK(CompactPriority('L', true) == 0 && CompactPriority('V', true) == 0 && CompactPriority('G', true) == 0);
  CHECK(CompactPriority('G', false) < CompactPriority('V', false));
  CHECK(CompactPriority('V', false) < CompactPriority('L', false));
}

TEST(the_compact_block_fits_its_height_in_order_without_overlap) {
  // Ten items in two sizes; the height allows two rows of a 300 px block.
  std::vector<FlowItem> items;
  for (int i = 0; i < 10; ++i) items.push_back({ 90.0f + 10 * (i % 3), 14, 6, i % 4 });
  items[7].priority = 0;  // a changed line late in the equation
  float width = 300, maxH = 2 * 20 + 5;
  Flow f = FlowCompact(items, width, maxH, 10, 5);
  CHECK(f.height <= maxH);
  CHECK(!f.placed.empty() && f.dropped == 10 - (int)f.placed.size());
  bool seven = false;
  for (size_t i = 0; i < f.placed.size(); ++i) {
    const FlowPlace& p = f.placed[i];
    seven |= p.index == 7;
    CHECK(p.x >= 0 && p.x + items[p.index].w <= width + 1e-3f);
    if (i > 0) CHECK(p.index > f.placed[i - 1].index);  // the equation's own order
    for (size_t j = 0; j < i; ++j) {
      const FlowPlace& q = f.placed[j];
      bool sameRow = q.baseline == p.baseline;
      if (sameRow) CHECK(q.x + items[q.index].w <= p.x);
    }
  }
  CHECK(seven);  // the changed line is kept
  // Every kept item is at least as important as every dropped one that would have fitted alone.
  int worstKept = 0;
  for (auto& p : f.placed) worstKept = std::max(worstKept, items[p.index].priority);
  for (int i = 0; i < 10; ++i) {
    bool kept = false;
    for (auto& p : f.placed) kept |= p.index == i;
    if (!kept) CHECK(items[i].priority >= 1);
  }
  CHECK(worstKept <= 3);
}

TEST(a_line_wider_than_the_strip_is_dropped_and_counted) {
  std::vector<FlowItem> items = { { 50, 10, 4, 1 }, { 900, 10, 4, 0 }, { 60, 10, 4, 2 }, { 40, 10, 4, -1 } };
  Flow f = FlowCompact(items, 400, 100, 8, 4);
  CHECK(f.placed.size() == 2);
  CHECK(f.dropped == 1);  // the wide one; the time name (-1) is not counted as dropped
  CHECK(f.placed[0].index == 0 && f.placed[1].index == 2);
  CHECK(FlowCompact({}, 400, 100, 8, 4).height == 0);
}

// The stage as the engine marks it (kind, changed, markup), laid like main.cpp lays it.
static Flow CompactFor(float H, float scale, float stripW, float* sizeOut) {
  float size = CompactSize(H, scale);
  *sizeOut = size;
  struct Def { char kind; bool changed; const char* markup; };
  std::vector<Def> defs = {
    { 'G', false, "\\rm{sq}\\paren{\\it{v}} = \\floor{2\\,\\rm{frac}\\paren{\\it{v}}}" },
    { 'L', false, "\\it{c} = 2 + \\it{s}" },
    { 'L', false, kLens },
    { 'G', true, "\\it{N} = 14531453" },
    { 'G', false, kRoots },
    { 'G', false, kMelody },
    { 'V', false, "\\rm{lead}\\paren{\\it{k}} = \\it{h}\\paren{\\it{M}\\paren{\\it{k}}} − 1" },
    { 'G', false, "\\it{P} = 43690" },
    { 'V', false, "\\rm{on}\\paren{\\it{k}} = \\rm{mod}\\paren{\\floor{\\frac{\\it{P}}{2\\sup{\\rm{mod}\\paren{\\it{k}, 16}}}}, 2}" },
    { 'T', false, "\\it{n} = \\floor{4\\it{t}}" },
  };
  std::vector<FlowItem> items;
  for (auto& d : defs) {
    Lines ls = BreakLines(L(), Parse(d.markup), size, 1e9f, 0);
    const Laid& l = ls.lines[0];
    items.push_back({ l.w, l.asc, l.desc, CompactPriority(d.kind, d.changed) });
  }
  float maxFlow = CompactMaxHeight(H) - CompactHeaderH(size) - 2 * CompactPad(size);
  return FlowCompact(items, stripW - 2 * CompactPad(size), maxFlow, CompactGapX(size), CompactGapY(size));
}

TEST(the_compact_strip_is_top_left_at_most_a_fifth_high_and_clear_of_the_draw_head_at_1080p_and_150_percent) {
  float W = 1920, H = 1080, m = 24;
  Rect head{ 1050, 200, 1500, 1080 };  // where the draw-head stands in the stream captures, with its margin
  for (float scale : { 1.0f, 1.5f }) {
    float size = 0;
    float room = head.x0 - 2 * m;
    Flow f = CompactFor(H, scale, room, &size);
    float h = f.height + CompactHeaderH(size) + 2 * CompactPad(size);
    Rect r = CompactStrip(f.width + 2 * CompactPad(size), h, W, H, m, head, {});
    CHECK(size >= 10);                              // readable
    CHECK(size <= (scale == 1.0f ? 10 : 14));       // about 70% of the old 14 and 20 px (smaller since 2026-11-03)
    CHECK(scale == 1.0f || size >= 14);             // larger at 150%
    CHECK(r.y1 - r.y0 <= CompactMaxHeight(H));      // a fifth of the screen at most
    CHECK(!r.Overlaps(head));                       // never over the draw-head
    CHECK(r.x0 == m && r.y0 == m);                  // in the top-left corner
    bool changed = false, melody = false;
    for (auto& p : f.placed) { changed |= p.index == 3; melody |= p.index == 5; CHECK(p.index != 9); }
    CHECK(changed);                                 // the line that changed is shown
    CHECK(melody);                                  // the melody's generator is shown
    printf("    compact at 1080p, scale %.1f: %.0f px type, %zu of 9 lines, %.0f x %.0f px (%.0f%% of the height)\n", scale, size,
           f.placed.size(), r.x1 - r.x0, r.y1 - r.y0, 100.0f * (r.y1 - r.y0) / H);
  }
}

TEST(the_compact_strip_sits_beside_an_open_menu_and_leaves_the_top_left_only_when_the_head_covers_it) {
  float W = 1920, H = 1080, m = 24;
  Rect headRight{ 1300, 200, 1700, 1000 };
  Rect menu{ 0, 0, 470, H };
  Rect r = CompactStrip(900, 150, W, H, m, headRight, { menu });
  CHECK(r.x0 == 470 + m && r.y0 == m && !r.Overlaps(menu));  // top left, beside the menu
  CHECK(CompactTopLeftRoom(W, m, 150, headRight, { menu }) == W - m - (470 + m));  // the head is below the band
  Rect headHigh{ 1300, 100, 1700, 1000 };                                           // up into the band: stop short of it
  CHECK(CompactTopLeftRoom(W, m, 150, headHigh, { menu }) == 1300 - m - (470 + m));
  Rect headTopLeft{ 200, 0, 600, 1080 };  // the head reaches up through the top-left corner: the roomier side instead
  r = CompactStrip(900, 150, W, H, m, headTopLeft, {});
  CHECK(!r.Overlaps(headTopLeft) && r.x1 == W - m);
  Rect headLow{ 200, 600, 600, 1080 };    // a head below the strip's band leaves the top left whole
  r = CompactStrip(900, 150, W, H, m, headLow, {});
  CHECK(r.x0 == m && r.y0 == m && r.x1 == m + 900);
}

TEST(the_compact_strip_takes_the_roomier_side_and_the_top_when_the_bottom_is_covered) {
  float W = 1920, H = 1080, m = 24;
  Rect r;
  Rect transport{ 0, H - 200, W, H };
  Rect headTopLeft{ 200, 0, 600, 1080 };  // the top left is covered, so the old places: the roomier side, the top there
  r = CompactStrip(900, 150, W, H, m, headTopLeft, { transport });
  CHECK(r.y0 == m && r.x1 == W - m && !r.Overlaps(headTopLeft) && !r.Overlaps(transport));
  Rect narrow{ 30, 0, 1900, 1080 };  // no room either side: width shrinks to nothing, never negative
  r = CompactStrip(900, 150, W, H, m, narrow, {});
  CHECK(r.x1 >= r.x0);
}

// ---------------------------------------------------------------- the stream's library entries and saved names

TEST(the_library_lists_the_three_streams_first_once) {
  std::vector<ui::SongEntry> songs(2);
  songs[0].path = "a.txt"; songs[0].title = "a";
  songs[1].path = "b.txt"; songs[1].title = "b";
  auto with = ui::WithStream(songs);
  CHECK(with.size() == 5);
  CHECK(ui::IsStream(with[0]) && ui::IsStream(with[1]) && ui::IsStream(with[2]) && !ui::IsStream(with[3]));
  CHECK(with[0].title == "Stream" && with[0].path == ui::kStreamPath);
  CHECK(with[1].title == "Explore (strange tunings)" && with[1].path == ui::kStreamExplorePath);
  CHECK(with[2].title == "Stream from G" && with[2].path == ui::kStreamGPath);
  auto twice = ui::WithStream(with);
  CHECK(twice.size() == 5);
  CHECK(ui::IsStream(twice[2]) && !ui::IsStream(twice[3]));
  auto empty = ui::WithStream({});
  CHECK(empty.size() == 3 && ui::IsStream(empty[0]) && ui::IsStream(empty[1]) && ui::IsStream(empty[2]));
}

TEST(a_fresh_stream_seed_is_new_each_time_and_short_enough_to_type) {
  std::set<unsigned long long> seen;
  for (unsigned long long t = 1730600000000ull; t < 1730600000000ull + 2000; ++t) {
    unsigned long long s = ui::FreshStreamSeed(t);
    CHECK(s >= 1 && s <= 999999);
    seen.insert(s);
  }
  CHECK(seen.size() >= 1995);  // neighbouring clock values give different seeds
  CHECK(ui::FreshStreamSeed(42) == ui::FreshStreamSeed(42));
  CHECK(ui::FreshStreamSeed(0) >= 1);
}

TEST(a_rescan_keeps_the_stream_entry_and_does_not_mark_it_new) {
  auto old = ui::WithStream({});
  auto fresh = ui::WithStream({});
  ui::Rescan r = ui::MergeRescan(old, fresh);
  CHECK(!r.changed);
  CHECK(!r.songs[0].isNew && !r.songs[1].isNew && !r.songs[2].isNew);
  CHECK(ui::RemapIndex(old, 0, r.songs) == 0);
  CHECK(ui::RemapIndex(old, 1, r.songs) == 1);
  CHECK(ui::RemapIndex(old, 2, r.songs) == 2);
}

TEST(a_saved_stretch_gets_a_name_that_is_free) {
  std::set<std::string> taken;
  auto exists = [&](const std::string& n) { return taken.count(n) > 0; };
  CHECK(ui::StreamSaveName(1, 2, 5, exists) == "stream_s1_2-5.txt");
  taken.insert("stream_s1_2-5.txt");
  CHECK(ui::StreamSaveName(1, 2, 5, exists) == "stream_s1_2-5_2.txt");
  taken.insert("stream_s1_2-5_2.txt");
  CHECK(ui::StreamSaveName(1, 2, 5, exists) == "stream_s1_2-5_3.txt");
  CHECK(ui::StreamSaveName(7, 1, 1, nullptr) == "stream_s7_1-1.txt");
}

TEST(e_and_s_are_stream_keys_and_on_the_card) {
  CHECK(ui::KeyToAction('E') == ui::Action::ToggleEquation);
  CHECK(ui::KeyToAction('S') == ui::Action::SaveStream);
  CHECK(ui::KeyToAction('S', true) == ui::Action::None);  // holding S saves once
  bool e = false, s = false;
  for (auto& row : ui::HelpRows())
    for (auto a : row.actions) { e |= a == ui::Action::ToggleEquation; s |= a == ui::Action::SaveStream; }
  CHECK(e && s);
}

int main() {
  for (auto& t : Tests()) {
    g_test = t.name;
    int before = g_failed;
    t.fn();
    printf("%s %s\n", g_failed == before ? "ok  " : "FAIL", t.name);
  }
  printf("\n%zu tests, %d checks, %d failed\n", Tests().size(), g_checks, g_failed);
  return g_failed ? 1 : 0;
}
