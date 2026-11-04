// math_layout.h: the live equation's layout, with no window and no GPU, so it can be tested alone
// (world/tests/math_layout_test.cpp). Stream mode's overlay shows the playing stage's definitions typeset
// as math: fractions stacked, floor brackets and parentheses sized to what they hold, exponents raised,
// names in italic, and the terms that changed at this stage marked.
//
// The engine writes each definition as a small markup (src/compose/display.rs): plain text and the
// commands \frac{a}{b} \floor{x} \ceil{x} \abs{x} \sqrt{x} \paren{x} \sup{x} \sub{x} \it{x} \rm{x}
// \hl{x} \, (a thin space) and \brk (a place a long line may break). Parse() reads it into boxes;
// Layout() places them for a font, given only a way to measure text; BreakLines() wraps a definition
// at its \brk points to a width. The drawing (ImGui) lives in main.cpp and only replays the result.
//
// Coordinates: x right, y down, in pixels; a laid box's origin is its baseline's left end.
#pragma once

#include <algorithm>
#include <cmath>
#include <functional>
#include <memory>
#include <string>
#include <vector>

namespace mathui {

enum class Kind { Row, Text, Frac, Floor, Ceil, Abs, Sqrt, Paren, Sup, Sub, Space, Break };

struct Node {
  Kind kind = Kind::Row;
  std::string text;       // Text
  bool italic = false;    // Text inside \it
  bool hl = false;        // inside \hl (a term that changed)
  std::vector<Node> kids; // Row: its items; Frac: numerator, denominator; the others: one Row
};

// ---------------------------------------------------------------- parsing

struct Parser {
  const std::string& s;
  size_t i = 0;

  Node Row(bool italic, bool hl, bool stopAtBrace) {
    Node row; row.kind = Kind::Row; row.italic = italic; row.hl = hl;
    std::string run;
    auto flush = [&]() {
      if (run.empty()) return;
      Node t; t.kind = Kind::Text; t.text = run; t.italic = italic; t.hl = hl;
      row.kids.push_back(t);
      run.clear();
    };
    while (i < s.size()) {
      char c = s[i];
      if (c == '}') { if (stopAtBrace) break; ++i; continue; }  // a stray brace is dropped
      if (c == '{') { ++i; flush(); Node g = Row(italic, hl, true); if (i < s.size()) ++i; row.kids.push_back(g); continue; }
      if (c != '\\') { run += c; ++i; continue; }
      flush();
      ++i;
      if (i < s.size() && s[i] == ',') { ++i; Node sp; sp.kind = Kind::Space; sp.hl = hl; row.kids.push_back(sp); continue; }
      size_t a = i;
      while (i < s.size() && s[i] >= 'a' && s[i] <= 'z') ++i;
      std::string cmd = s.substr(a, i - a);
      auto arg = [&](bool it, bool h) {
        if (i < s.size() && s[i] == '{') { ++i; Node g = Row(it, h, true); if (i < s.size()) ++i; return g; }
        return Node{};
      };
      if (cmd == "brk") { Node b; b.kind = Kind::Break; row.kids.push_back(b); continue; }
      if (cmd == "frac") {
        Node f; f.kind = Kind::Frac; f.hl = hl;
        f.kids.push_back(arg(italic, hl));
        f.kids.push_back(arg(italic, hl));
        row.kids.push_back(f);
        continue;
      }
      if (cmd == "it" || cmd == "rm") { Node g = arg(cmd == "it", hl); for (auto& k : g.kids) row.kids.push_back(k); continue; }
      if (cmd == "hl") { Node g = arg(italic, true); g.hl = true; row.kids.push_back(g); continue; }
      Kind k = cmd == "floor" ? Kind::Floor : cmd == "ceil" ? Kind::Ceil : cmd == "abs" ? Kind::Abs : cmd == "sqrt" ? Kind::Sqrt
             : cmd == "paren" ? Kind::Paren : cmd == "sup" ? Kind::Sup : cmd == "sub" ? Kind::Sub : Kind::Row;
      Node n; n.kind = k; n.hl = hl;
      n.kids.push_back(arg(italic, hl));
      row.kids.push_back(n);
    }
    flush();
    return row;
  }
};

inline Node Parse(const std::string& markup) { Parser p{ markup }; return p.Row(false, false, false); }

// The text of a markup with its commands removed (for logs, tests and a plain fallback).
inline std::string PlainText(const Node& n) {
  switch (n.kind) {
    case Kind::Text: return n.text;
    case Kind::Space: return " ";
    case Kind::Break: return "";
    case Kind::Frac: return "(" + PlainText(n.kids[0]) + ")/(" + PlainText(n.kids[1]) + ")";
    case Kind::Floor: return "floor(" + PlainText(n.kids[0]) + ")";
    case Kind::Ceil: return "ceil(" + PlainText(n.kids[0]) + ")";
    case Kind::Abs: return "|" + PlainText(n.kids[0]) + "|";
    case Kind::Sqrt: return "sqrt(" + PlainText(n.kids[0]) + ")";
    case Kind::Paren: return "(" + PlainText(n.kids[0]) + ")";
    case Kind::Sup: return "^" + PlainText(n.kids[0]);
    case Kind::Sub: return "_" + PlainText(n.kids[0]);
    case Kind::Row: { std::string s; for (auto& k : n.kids) s += PlainText(k); return s; }
  }
  return "";
}

// ---------------------------------------------------------------- layout

// Measuring text: its width in pixels at a size, upright or italic. The tests pass a fixed-width font.
using Measure = std::function<float(const std::string& utf8, bool italic, float size)>;

// What the drawing replays: text at a baseline, and strokes (fraction bars, brackets, the radical).
struct Glyph { float x, y; float size; bool italic, hl; std::string text; };  // (x, y) is the baseline's left end
struct Stroke { float x0, y0, x1, y1; float width; bool hl; };
struct Laid {
  float w = 0, asc = 0, desc = 0;  // width, height above and depth below the baseline
  std::vector<Glyph> glyphs;
  std::vector<Stroke> strokes;
  bool hl = false;                 // it holds a changed term
  void Append(const Laid& o, float dx, float dy) {
    for (auto g : o.glyphs) { g.x += dx; g.y += dy; glyphs.push_back(g); }
    for (auto s : o.strokes) { s.x0 += dx; s.x1 += dx; s.y0 += dy; s.y1 += dy; strokes.push_back(s); }
    hl = hl || o.hl;
  }
};

// The shape of text in this font, as fractions of its size (Cambria: caps reach about 0.67, the fraction bar sits
// at the minus sign's height, about 0.25).
struct Metrics {
  float asc = 0.74f, desc = 0.24f, axis = 0.25f, thin = 0.17f;
  float scriptScale = 0.7f, supRise = 0.42f, subDrop = 0.18f, fracScale = 0.86f, rule = 0.055f;
};

struct Layouter {
  Measure measure;
  Metrics m;

  Laid Text(const std::string& t, bool italic, bool hl, float size) const {
    Laid l; l.w = measure(t, italic, size); l.asc = m.asc * size; l.desc = m.desc * size; l.hl = hl;
    l.glyphs.push_back({ 0, 0, size, italic, hl, t });
    return l;
  }

  Laid RowOf(const std::vector<Node>& kids, float size, int depth) const {
    Laid row;
    for (const Node& k : kids) {
      if (k.kind == Kind::Break) continue;
      Laid b = Box(k, size, depth);
      if (k.kind == Kind::Sup || k.kind == Kind::Sub) b.w += 0.04f * size;
      row.Append(b, row.w, 0);
      row.w += b.w;
      row.asc = std::max(row.asc, b.asc);
      row.desc = std::max(row.desc, b.desc);
    }
    if (kids.empty()) { row.asc = m.asc * size; row.desc = m.desc * size; }
    return row;
  }

  // Brackets drawn as strokes sized to what they hold: kind is Floor, Ceil, Abs or Paren.
  Laid Bracketed(const Node& n, float size, int depth) const {
    Laid in = Box(n.kids[0], size, depth);
    if (n.kind == Kind::Paren && in.asc <= m.asc * size * 1.02f && in.desc <= m.desc * size * 1.3f) {
      // Around a single line of text the font's own parentheses look right; only taller content gets drawn ones.
      Laid open = Text("(", false, n.hl, size), close = Text(")", false, n.hl, size);
      Laid l; l.hl = n.hl || in.hl;
      l.Append(open, 0, 0); l.w = open.w;
      l.Append(in, l.w, 0); l.w += in.w;
      l.Append(close, l.w, 0); l.w += close.w;
      l.asc = std::max(open.asc, in.asc); l.desc = std::max(open.desc, in.desc);
      return l;
    }
    float pad = 0.06f * size;
    float top = in.asc + pad, bot = in.desc + pad;
    float lw = m.rule * size * 1.1f;
    float bw = n.kind == Kind::Paren ? 0.26f * size + 0.04f * (top + bot) : 0.24f * size;
    float gap = n.kind == Kind::Paren ? 0.05f * size : 0.08f * size;
    Laid l; l.hl = n.hl || in.hl;
    auto line = [&](float x0, float y0, float x1, float y1) { l.strokes.push_back({ x0, y0, x1, y1, lw, n.hl }); };
    float xl = lw, xr = bw + gap + in.w + gap + bw - lw;
    if (n.kind == Kind::Paren) {
      // Each bracket is a shallow arc, as short line pieces: a flat parabola from top to bottom.
      const int pieces = 10;
      float h = top + bot, bulge = bw - lw * 1.5f;
      for (int side = 0; side < 2; ++side)
        for (int k = 0; k < pieces; ++k) {
          float u0 = (float)k / pieces, u1 = (float)(k + 1) / pieces;
          auto x = [&](float u) { float d = bulge * (1.0f - 4.0f * (u - 0.5f) * (u - 0.5f)); return side == 0 ? xl + bulge - d : xr - bulge + d; };
          line(x(u0), -top + h * u0, x(u1), -top + h * u1);
        }
    } else {
      line(xl, -top, xl, bot);
      line(xr, -top, xr, bot);
      if (n.kind == Kind::Floor) { line(xl, bot, xl + bw * 0.75f, bot); line(xr, bot, xr - bw * 0.75f, bot); }
      if (n.kind == Kind::Ceil) { line(xl, -top, xl + bw * 0.75f, -top); line(xr, -top, xr - bw * 0.75f, -top); }
    }
    l.Append(in, bw + gap, 0);
    l.w = xr + lw;
    l.asc = top + lw;
    l.desc = bot + lw;
    return l;
  }

  Laid Box(const Node& n, float size, int depth) const {
    switch (n.kind) {
      case Kind::Text: return Text(n.text, n.italic, n.hl, size);
      case Kind::Space: { Laid l; l.w = m.thin * size; return l; }
      case Kind::Break: return Laid{};
      case Kind::Row: return RowOf(n.kids, size, depth);
      case Kind::Frac: {
        float s = depth > 0 ? size * m.fracScale : size;
        Laid a = Box(n.kids[0], s, depth + 1), b = Box(n.kids[1], s, depth + 1);
        float pad = 0.12f * size, gap = 0.12f * size, axis = m.axis * size, lw = m.rule * size;
        Laid l; l.hl = n.hl;
        l.w = std::max(a.w, b.w) + 2 * pad;
        l.strokes.push_back({ 0, -axis, l.w, -axis, lw, n.hl });
        l.Append(a, (l.w - a.w) * 0.5f, -axis - gap - a.desc);
        l.Append(b, (l.w - b.w) * 0.5f, -axis + gap + b.asc);
        l.asc = axis + gap + a.desc + a.asc;
        l.desc = std::max(0.0f, gap + b.asc + b.desc - axis);
        return l;
      }
      case Kind::Floor: case Kind::Ceil: case Kind::Abs: case Kind::Paren: return Bracketed(n, size, depth);
      case Kind::Sqrt: {
        Laid in = Box(n.kids[0], size, depth);
        float lw = m.rule * size, pad = 0.08f * size, top = in.asc + pad, bot = in.desc;
        float hook = 0.42f * size;
        Laid l; l.hl = n.hl || in.hl;
        l.strokes.push_back({ 0, -0.3f * size, hook * 0.3f, -0.36f * size, lw, n.hl });
        l.strokes.push_back({ hook * 0.3f, -0.36f * size, hook * 0.6f, bot, lw * 1.4f, n.hl });
        l.strokes.push_back({ hook * 0.6f, bot, hook, -top, lw, n.hl });
        l.strokes.push_back({ hook, -top, hook + pad + in.w + pad, -top, lw, n.hl });
        l.Append(in, hook + pad, 0);
        l.w = hook + in.w + 2 * pad;
        l.asc = top + lw;
        l.desc = bot;
        return l;
      }
      case Kind::Sup: case Kind::Sub: {
        float s = size * m.scriptScale;
        Laid in = Box(n.kids[0], s, depth + 1);
        float dy = n.kind == Kind::Sup ? -m.supRise * size : m.subDrop * size;
        Laid l; l.hl = n.hl || in.hl;
        l.Append(in, 0, dy);
        l.w = in.w;
        l.asc = std::max(0.0f, in.asc - dy);
        l.desc = std::max(0.0f, in.desc + dy);
        return l;
      }
    }
    return Laid{};
  }

  Laid Layout(const Node& n, float size) const { return Box(n, size, 0); }
};

// ---------------------------------------------------------------- lines

// A definition laid out in one or more lines no wider than maxWidth where it can be helped: the top-level
// row splits at its \brk points (before a + or −), segments go greedily onto lines, and continuation
// lines are indented by `indent`. A single segment wider than the width stays whole on its own line.
struct Lines {
  std::vector<Laid> lines;   // each line's baseline at y = 0, its x from 0 (continuations already indented)
  float width = 0;           // the widest line, indent included
};

inline Lines BreakLines(const Layouter& L, const Node& top, float size, float maxWidth, float indent) {
  std::vector<std::vector<Node>> segs(1);
  for (const Node& k : top.kids) {
    if (k.kind == Kind::Break) segs.emplace_back();
    else segs.back().push_back(k);
  }
  Lines out;
  Laid cur; bool empty = true;
  float x0 = 0;  // the current line's indent
  auto finish = [&]() {
    Laid line; line.Append(cur, x0, 0);
    line.w = cur.w + x0; line.asc = cur.asc; line.desc = cur.desc;
    out.lines.push_back(line);
  };
  for (auto& seg : segs) {
    Laid s = L.RowOf(seg, size, 0);
    if (!empty && x0 + cur.w + s.w > maxWidth) { finish(); cur = Laid{}; empty = true; x0 = indent; }
    cur.Append(s, cur.w, 0);
    cur.w += s.w;
    cur.asc = std::max(cur.asc, s.asc);
    cur.desc = std::max(cur.desc, s.desc);
    empty = false;
  }
  if (!empty) finish();
  for (auto& l : out.lines) out.width = std::max(out.width, l.w);
  return out;
}

// ---------------------------------------------------------------- timing at a seam

// Seconds a new stage's equation takes to replace the old one (the old fades out as the new fades in).
constexpr double kCrossfade = 0.6;
// The changed terms: lit within kHlRise, held until kHlHold, gone by kHlEnd (seconds after the seam).
constexpr double kHlRise = 0.25, kHlHold = 2.2, kHlEnd = 4.0;

inline double Smooth01(double u) { u = std::clamp(u, 0.0, 1.0); return u * u * (3.0 - 2.0 * u); }

// The new stage's opacity t seconds after its seam (the old stage's is 1 minus it); 1 before any seam.
inline double NewAlpha(double t) { return t < 0 ? 1.0 : Smooth01(t / kCrossfade); }
// How strongly the changed terms are lit t seconds after the seam: 0 before it and after kHlEnd.
inline double Highlight(double t) {
  if (!(t >= 0) || t >= kHlEnd) return 0.0;
  if (t < kHlRise) return Smooth01(t / kHlRise);
  if (t <= kHlHold) return 1.0;
  return 1.0 - Smooth01((t - kHlHold) / (kHlEnd - kHlHold));
}

// ---------------------------------------------------------------- placement

struct Rect {
  float x0, y0, x1, y1;
  bool Overlaps(const Rect& o) const { return x0 < o.x1 && o.x0 < x1 && y0 < o.y1 && o.y0 < y1; }
};

// Where the panel goes: the first of the four corners (top left, bottom left, top right, bottom right) where it
// covers none of `avoid` (the draw-head, with a margin, and any open panel). If every corner is covered, the one
// covering the least area of them. Top left first since 2026-11-03.
inline Rect PlacePanel(float w, float h, float screenW, float screenH, float margin, const std::vector<Rect>& avoid) {
  Rect c[4] = {
    { margin, margin, margin + w, margin + h },
    { margin, screenH - margin - h, margin + w, screenH - margin },
    { screenW - margin - w, margin, screenW - margin, margin + h },
    { screenW - margin - w, screenH - margin - h, screenW - margin, screenH - margin },
  };
  int best = 0; float bestArea = 1e30f;
  for (int i = 0; i < 4; ++i) {
    float area = 0;
    for (const Rect& a : avoid)
      if (c[i].Overlaps(a))
        area += (std::min(c[i].x1, a.x1) - std::max(c[i].x0, a.x0)) * (std::min(c[i].y1, a.y1) - std::max(c[i].y0, a.y0));
    if (area == 0) return c[i];
    if (area < bestArea) { bestArea = area; best = i; }
  }
  return c[best];
}

// ---------------------------------------------------------------- the compact equation (stream v2)
//
// The full panel took up too much space for what it conveys. E now cycles three ways. Compact (the default) is one quiet strip in the top-left corner, at
// most a fifth of the screen high and never over the draw-head: a one-line header, then the definitions in
// a dense block, each on one line, flowing left to right. What it keeps when not everything fits: the lines
// that changed at this stage first, then the generators (the structures the music is made of), then the
// lines the voices read, then the lens. The time names are left out. Full is the old panel; Hidden is none.

enum class EqMode { Compact, Full, Hidden };

inline EqMode NextEqMode(EqMode m) {
  return m == EqMode::Compact ? EqMode::Full : m == EqMode::Full ? EqMode::Hidden : EqMode::Compact;
}

// The order a compact strip keeps definitions in when space runs out (lower first), from the engine's
// display kind ('G' generator or stored integer, 'V' a voice line, 'L' the lens, 'T' a time name) and
// whether it changed at this stage. -1: never shown (a time name).
inline int CompactPriority(char kind, bool changed) {
  if (kind == 'T') return -1;
  if (changed) return 0;
  return kind == 'G' ? 1 : kind == 'V' ? 2 : 3;
}

// The compact strip's type size (px): 0.875% of the screen height, never under 9.5 px times the UI scale, at least
// 10 and at most 21: about 70% of what it was (13.5 -> 10 px at 1080p, 20 -> 14 at 150%; smaller
// since 2026-11-03). The math is drawn from the font at this size, so it stays crisp, not scaled.
inline float CompactSize(float screenH, float uiScale) {
  return std::round(std::clamp(std::max(screenH * 0.00875f, 9.5f * uiScale), 10.0f, 21.0f));
}
inline float CompactMaxHeight(float screenH) { return screenH / 5.0f; }  // the whole strip, padding included
inline float CompactHeaderH(float size) { return size * 1.45f; }         // the one-line header above the block
inline float CompactPad(float size) { return size * 0.6f; }
inline float CompactGapX(float size) { return size * 1.6f; }
inline float CompactGapY(float size) { return size * 0.35f; }

struct FlowItem { float w = 0, asc = 0, desc = 0; int priority = 0; };  // priority < 0: left out
struct FlowPlace { int index; float x, baseline; };                     // x from the block's left, baseline from its top
struct Flow { std::vector<FlowPlace> placed; float width = 0, height = 0; int dropped = 0; };

// Items laid in their own order, left to right in rows `width` wide (`gapX` between items, `gapY`
// between rows), keeping the most important that fit in `maxH`: an item is added in priority order
// (ties by position) only if the whole block still fits. An item wider than the row is dropped.
inline Flow FlowCompact(const std::vector<FlowItem>& items, float width, float maxH, float gapX, float gapY) {
  auto layout = [&](const std::vector<char>& on) {
    Flow f;
    float x = 0, top = 0, rowAsc = 0, rowDesc = 0;
    std::vector<FlowPlace> row;
    auto flush = [&]() {
      if (row.empty()) return;
      for (auto& p : row) { p.baseline = top + rowAsc; f.placed.push_back(p); }
      top += rowAsc + rowDesc + gapY;
      row.clear(); x = 0; rowAsc = rowDesc = 0;
    };
    for (size_t i = 0; i < items.size(); ++i) {
      if (!on[i]) continue;
      const FlowItem& it = items[i];
      if (!row.empty() && x + gapX + it.w > width) flush();
      float at = row.empty() ? 0 : x + gapX;
      row.push_back({ (int)i, at, 0 });
      x = at + it.w;
      f.width = std::max(f.width, x);
      rowAsc = std::max(rowAsc, it.asc); rowDesc = std::max(rowDesc, it.desc);
    }
    flush();
    f.height = f.placed.empty() ? 0 : top - gapY;
    return f;
  };
  std::vector<int> order;
  for (size_t i = 0; i < items.size(); ++i) if (items[i].priority >= 0) order.push_back((int)i);
  std::stable_sort(order.begin(), order.end(), [&](int a, int b) { return items[a].priority < items[b].priority; });
  std::vector<char> on(items.size(), 0);
  int dropped = 0;
  for (int i : order) {
    if (items[i].w > width) { ++dropped; continue; }
    on[i] = 1;
    if (layout(on).height > maxH) { on[i] = 0; ++dropped; }
  }
  Flow f = layout(on);
  f.dropped = dropped;
  return f;
}

// The compact strip's left edge in the top-left corner: the margin, or beside a panel down the left edge (the open
// menu: anything in `avoid` that starts at the screen's top-left).
inline float CompactLeft(float margin, const std::vector<Rect>& avoid) {
  float x = margin;
  for (const Rect& a : avoid) if (a.x0 <= margin && a.y0 <= margin) x = std::max(x, a.x1 + margin);
  return x;
}

// How wide the strip can be in the top-left corner, `bandH` high: to the screen's right margin, or short of the
// draw-head when the head reaches up into that band. 0 when the head covers the corner.
inline float CompactTopLeftRoom(float screenW, float margin, float bandH, const Rect& head, const std::vector<Rect>& avoid) {
  float x0 = CompactLeft(margin, avoid), x1 = screenW - margin;
  if (head.y0 < margin + bandH && head.x1 > x0 && head.x0 < screenW) x1 = std::min(x1, head.x0 - margin);
  return std::max(0.0f, x1 - x0);
}

// Where the compact strip goes: the top-left corner (since 2026-11-03), `w` wide and `h`
// high, clear of the draw-head and of everything in `avoid`. If the top left cannot hold it whole, the old places:
// along the bottom edge (or the top, when the bottom is covered), on the side of the draw-head with more room.
// Returns a rect whose width may be less than `w` (the room there).
inline Rect CompactStrip(float w, float h, float screenW, float screenH, float margin, const Rect& head,
                         const std::vector<Rect>& avoid) {
  {
    float x0 = CompactLeft(margin, avoid);
    Rect tl{ x0, margin, x0 + w, margin + h };
    bool clear = w <= CompactTopLeftRoom(screenW, margin, h, head, avoid) && !tl.Overlaps(head);
    for (const Rect& a : avoid) clear = clear && !tl.Overlaps(a);
    if (clear) return tl;
  }
  float leftRoom = head.x0 - 2 * margin, rightRoom = screenW - head.x1 - 2 * margin;
  bool left = leftRoom >= rightRoom;
  float room = std::max(0.0f, left ? leftRoom : rightRoom);
  if (head.x1 <= 0 || head.x0 >= screenW) { left = true; room = screenW - 2 * margin; }  // the head is off screen
  float ww = std::min(w, room);
  float x0 = left ? margin : screenW - margin - ww;
  Rect cands[2] = { { x0, screenH - margin - h, x0 + ww, screenH - margin }, { x0, margin, x0 + ww, margin + h } };
  for (const Rect& c : cands) {
    bool clear = !c.Overlaps(head);
    for (const Rect& a : avoid) clear = clear && !c.Overlaps(a);
    if (clear) return c;
  }
  return cands[0];
}

}  // namespace mathui
