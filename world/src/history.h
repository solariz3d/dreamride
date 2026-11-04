// history.h: Dreamride's stream history. Every stream that starts is recorded as it is composed: its seed and where it
// came from, the composer that wrote it, and every stage's full song text, so it can be replayed exactly and saved as a
// song after the composer has changed. One plain-text file per stream under %APPDATA%\Dreamride\history, written on a
// worker thread so no frame waits for the disk. Deleting moves a file into history\deleted (undoable); nothing is ever
// pruned on its own.
//
// No window and no GPU here, so world/tests/history_test.cpp tests all of it: the file format, merging recorded stages
// into one song (replay and "Save as song"), the list's order, deleting and undoing, and write failures.
//
// The file format (dreamride-history 1). Line based; a stage's song (and a seed song) follows its header line, framed
// by its line count. Lines after the stages may repeat `heard` and `star`; the last one counts (a recording appends
// them as it goes, so a crash loses nothing written before it). Unknown keys are an error, so a damaged file says so.
//
//   # Dreamride stream history: ...
//   dreamride-history 1
//   started 2026-11-03 14:05:12 1793812345     local time, then seconds since 1970 (UTC)
//   seed 955576
//   flags 1                                    the lm_stream_start flags (1: a grammar seed)
//   origin grammar seed 955576                 the engine's own words for where it came from
//   composer luminesce.dll fnv1a64 0123456789abcdef 1268736
//   bpm 101
//   stage_beats 32
//   roster bass hats kick lead
//   seed_song 0                                the seed song's line count, then its lines (0: none)
//   star 0
//   stage 1 turn 0 lines 27                    stage numbers count from 1
//   summary intro: the bass and the hats
//   ...27 lines: the stage as a song file...
//   heard 1 15.2                               stages heard, and seconds
//   complete 7 312.500                         the stream ended on its own: a finished song, its stages and length
#pragma once

#include <algorithm>
#include <atomic>
#include <cerrno>
#include <cmath>
#include <condition_variable>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <ctime>
#include <deque>
#include <filesystem>
#include <fstream>
#include <functional>
#include <map>
#include <memory>
#include <mutex>
#include <string>
#include <system_error>
#include <thread>
#include <vector>

namespace hist {

namespace fs = std::filesystem;

constexpr const char* kMagic = "dreamride-history 1";
constexpr uint32_t kFlagGrammar = 1;  // LM_STREAM_GRAMMAR

// ================================================================ the record

struct Stage {
  uint64_t index = 0;      // from 0 (the file writes it from 1)
  bool turn = false;       // a bigger change
  std::string summary;     // what changed, one line
  std::string text;        // the stage as a song file: '\n' line ends, ending in '\n'
};

struct Record {
  std::string path;                 // the file (UTF-8); empty until written
  std::string started;              // "2026-11-03 14:05:12", local time
  long long startedUnix = 0;        // seconds since 1970, UTC
  uint64_t seed = 0;
  uint32_t flags = 0;
  std::string origin;               // where it came from, in the engine's words
  std::string composer;             // which composer wrote it (CurrentComposer)
  double bpm = 0;
  double stageBeats = 32;
  std::vector<std::string> roster;  // every voice the stream can have, in order
  std::string seedSong;             // the seed song's text (empty: none given, or the engine's default)
  bool starred = false;
  uint64_t heardStages = 0;
  double heardSeconds = 0;
  bool complete = false;            // the stream ended on its own: a finished song
  uint64_t completeStages = 0;      // its stages and length then
  double completeSeconds = 0;
  std::vector<Stage> stages;
};

// Text with '\n' line ends and a final '\n' (empty stays empty): how every song text is kept, so what is read back is
// byte for byte what was recorded.
inline std::string NormalizeText(const std::string& t) {
  std::string o;
  o.reserve(t.size() + 1);
  for (char c : t) if (c != '\r') o += c;
  if (!o.empty() && o.back() != '\n') o += '\n';
  return o;
}

inline std::string OneLine(std::string s) {
  for (char& c : s) if (c == '\n' || c == '\r') c = ' ';
  while (!s.empty() && s.back() == ' ') s.pop_back();
  return s;
}

inline size_t LineCount(const std::string& normalized) {
  return (size_t)std::count(normalized.begin(), normalized.end(), '\n');
}

inline std::string FormatSeconds(double s) {
  char b[32];
  snprintf(b, sizeof(b), "%.3f", std::isfinite(s) && s > 0 ? s : 0.0);
  return b;
}

// The header: everything before the stages.
inline std::string HeaderText(const Record& r) {
  std::string o = "# Dreamride stream history: one stream as it was composed. Each stage's song follows its 'stage' line.\n";
  o += std::string(kMagic) + "\n";
  o += "started " + OneLine(r.started) + " " + std::to_string(r.startedUnix) + "\n";
  o += "seed " + std::to_string(r.seed) + "\n";
  o += "flags " + std::to_string(r.flags) + "\n";
  o += "origin " + OneLine(r.origin) + "\n";
  o += "composer " + OneLine(r.composer) + "\n";
  char b[64];
  snprintf(b, sizeof(b), "%.17g", r.bpm);
  o += std::string("bpm ") + b + "\n";
  snprintf(b, sizeof(b), "%.17g", r.stageBeats);
  o += std::string("stage_beats ") + b + "\n";
  o += "roster";
  for (auto& n : r.roster) o += " " + n;
  o += "\n";
  std::string seedSong = NormalizeText(r.seedSong);
  o += "seed_song " + std::to_string(LineCount(seedSong)) + "\n" + seedSong;
  o += std::string("star ") + (r.starred ? "1" : "0") + "\n";
  return o;
}

inline std::string StageText(const Stage& s) {
  std::string text = NormalizeText(s.text);
  return "stage " + std::to_string(s.index + 1) + " turn " + (s.turn ? "1" : "0") + " lines " + std::to_string(LineCount(text)) +
         "\nsummary " + OneLine(s.summary) + "\n" + text;
}

inline std::string HeardText(uint64_t stages, double seconds) {
  return "heard " + std::to_string(stages) + " " + FormatSeconds(seconds) + "\n";
}
inline std::string CompleteText(uint64_t stages, double seconds) {
  return "complete " + std::to_string(stages) + " " + FormatSeconds(seconds) + "\n";
}
inline std::string StarText(bool starred) { return std::string("star ") + (starred ? "1" : "0") + "\n"; }

// The whole record as one file.
inline std::string Serialize(const Record& r) {
  std::string o = HeaderText(r);
  for (auto& s : r.stages) o += StageText(s);
  o += HeardText(r.heardStages, r.heardSeconds);
  if (r.complete) o += CompleteText(r.completeStages, r.completeSeconds);
  return o;
}

// Reads a history file. keepText false drops the stages' song texts (the list needs only the summaries). False with
// `err` (and the line) when it is not a history file or is damaged.
inline bool Parse(const std::string& text, Record& r, std::string& err, bool keepText = true) {
  r = Record{};
  std::vector<std::string> lines;
  {
    size_t a = 0;
    while (a < text.size()) {
      size_t e = text.find('\n', a);
      std::string l = text.substr(a, e == std::string::npos ? std::string::npos : e - a);
      if (!l.empty() && l.back() == '\r') l.pop_back();
      lines.push_back(std::move(l));
      if (e == std::string::npos) break;
      a = e + 1;
    }
  }
  size_t i = 0;
  auto fail = [&](const std::string& why) { err = "line " + std::to_string(i + 1) + ": " + why; return false; };
  while (i < lines.size() && (lines[i].empty() || lines[i][0] == '#')) ++i;
  if (i >= lines.size() || lines[i] != kMagic) { err = "not a Dreamride history file (no '" + std::string(kMagic) + "' line)"; return false; }
  ++i;
  auto rest = [](const std::string& l, size_t n) { return l.size() > n ? l.substr(n + 1) : std::string(); };
  auto takeLines = [&](size_t n, std::string& out) {
    if (i + n > lines.size()) return false;
    out.clear();
    for (size_t k = 0; k < n; ++k) { out += lines[i + k]; out += '\n'; }
    i += n;
    return true;
  };
  bool sawStarted = false, sawSeed = false;
  for (; i < lines.size();) {
    const std::string& l = lines[i];
    if (l.empty() || l[0] == '#') { ++i; continue; }
    std::string key = l.substr(0, l.find(' '));
    std::string v = rest(l, key.size());
    try {
      if (key == "started") {
        size_t sp = v.rfind(' ');
        if (sp == std::string::npos) return fail("'started' needs a time and seconds");
        r.started = v.substr(0, sp);
        r.startedUnix = std::stoll(v.substr(sp + 1));
        sawStarted = true;
      } else if (key == "seed") { r.seed = std::stoull(v); sawSeed = true; }
      else if (key == "flags") r.flags = (uint32_t)std::stoul(v);
      else if (key == "origin") r.origin = v;
      else if (key == "composer") r.composer = v;
      else if (key == "bpm") r.bpm = std::stod(v);
      else if (key == "stage_beats") r.stageBeats = std::stod(v);
      else if (key == "roster") {
        r.roster.clear();
        size_t a = 0;
        while (a < v.size()) {
          size_t e = v.find(' ', a);
          std::string n = v.substr(a, e == std::string::npos ? std::string::npos : e - a);
          if (!n.empty()) r.roster.push_back(n);
          if (e == std::string::npos) break;
          a = e + 1;
        }
      } else if (key == "seed_song") {
        size_t n = std::stoul(v);
        ++i;
        if (!takeLines(n, r.seedSong)) return fail("the seed song is cut short");
        continue;
      } else if (key == "star") r.starred = v == "1";
      else if (key == "heard") {
        size_t sp = v.find(' ');
        if (sp == std::string::npos) return fail("'heard' needs stages and seconds");
        r.heardStages = std::stoull(v.substr(0, sp));
        r.heardSeconds = std::stod(v.substr(sp + 1));
      } else if (key == "complete") {
        size_t sp = v.find(' ');
        if (sp == std::string::npos) return fail("'complete' needs stages and seconds");
        r.complete = true;
        r.completeStages = std::stoull(v.substr(0, sp));
        r.completeSeconds = std::stod(v.substr(sp + 1));
      } else if (key == "stage") {
        Stage s;
        unsigned long long num = 0, turn = 0, n = 0;
        if (sscanf_s(v.c_str(), "%llu turn %llu lines %llu", &num, &turn, &n) != 3 || num == 0) return fail("expected 'stage N turn 0|1 lines L'");
        s.index = num - 1;
        s.turn = turn != 0;
        if (!r.stages.empty() && s.index <= r.stages.back().index) return fail("stage " + std::to_string(num) + " is out of order");
        ++i;
        if (i >= lines.size() || lines[i].rfind("summary", 0) != 0) return fail("a stage needs its 'summary' line");
        s.summary = rest(lines[i], 7);
        ++i;
        std::string t;
        if (!takeLines((size_t)n, t)) return fail("stage " + std::to_string(num) + " is cut short");
        if (keepText) s.text = std::move(t);
        r.stages.push_back(std::move(s));
        continue;
      } else {
        return fail("unknown line '" + l.substr(0, 40) + "'");
      }
    } catch (const std::exception&) {
      return fail("bad value in '" + l.substr(0, 60) + "'");
    }
    ++i;
  }
  if (!sawStarted || !sawSeed) { err = "the header is incomplete (no 'started' or 'seed')"; return false; }
  return true;
}

// ================================================================ files

inline std::string ReadFile(const fs::path& p, std::string& err) {
  std::ifstream f(p, std::ios::binary);
  if (!f) { err = "cannot open " + p.u8string(); return {}; }
  std::string s((std::istreambuf_iterator<char>(f)), std::istreambuf_iterator<char>());
  if (f.bad()) err = "cannot read " + p.u8string();
  return s;
}

inline bool LoadRecord(const std::string& path, Record& r, std::string& err, bool keepText = true) {
  std::string e;
  std::string text = ReadFile(fs::u8path(path), e);
  if (!e.empty()) { err = e; return false; }
  if (!Parse(text, r, e, keepText)) { err = fs::u8path(path).filename().u8string() + ": " + e; return false; }
  r.path = path;
  return true;
}

inline std::string ErrnoText(int e) {
  char b[256] = {};
  strerror_s(b, sizeof(b), e);
  return b;
}

// Writes (append false: replaces) `text` to `p`. "" on success, else why not.
inline std::string WriteText(const fs::path& p, const std::string& text, bool append) {
  FILE* f = nullptr;
  errno_t e = _wfopen_s(&f, p.c_str(), append ? L"ab" : L"wb");
  if (e || !f) return "cannot open " + p.u8string() + " for writing: " + ErrnoText(e ? e : errno);
  size_t n = text.empty() ? 0 : fwrite(text.data(), 1, text.size(), f);
  int we = errno;
  bool bad = n != text.size() || fflush(f) != 0;
  if (bad) we = errno;
  if (fclose(f) != 0 && !bad) { bad = true; we = errno; }
  if (bad) return "cannot write " + p.u8string() + ": " + ErrnoText(we);
  return "";
}

// Creates the folder (and its parents). "" on success.
inline std::string EnsureDir(const fs::path& d) {
  std::error_code ec;
  if (fs::is_directory(d, ec)) return "";
  fs::create_directories(d, ec);
  if (ec || !fs::is_directory(d)) return "cannot create the folder " + d.u8string() + (ec ? ": " + ec.message() : std::string(" (a file is in the way)"));
  return "";
}

// `stem.txt` in `dir`, or `stem_2.txt`, `stem_3.txt`, ... when taken.
inline fs::path UniquePath(const fs::path& dir, const std::string& stem) {
  std::error_code ec;
  fs::path p = dir / fs::u8path(stem + ".txt");
  for (int k = 2; fs::exists(p, ec) && k < 100000; ++k) p = dir / fs::u8path(stem + "_" + std::to_string(k) + ".txt");
  return p;
}

// The file a recording starts as: <date>_<seed>.txt (the date is the start's local date).
inline std::string FileStem(const Record& r) {
  std::string date = r.started.substr(0, 10);
  if (date.size() != 10) date = "undated";
  return date + "_" + std::to_string(r.seed);
}

// "2026-11-03 14:05:12" and the seconds since 1970, for now.
inline void Now(std::string& local, long long& unixSecs) {
  std::time_t t = std::time(nullptr);
  std::tm tm{};
  localtime_s(&tm, &t);
  char b[32];
  strftime(b, sizeof(b), "%Y-%m-%d %H:%M:%S", &tm);
  local = b;
  unixSecs = (long long)t;
}

// The composer's version: a hash of the engine that composes (luminesce.dll), so a seed recorded with another build is
// known not to replay the same. FNV-1a 64 over the file's bytes, and its size. Empty with `err` if it cannot be read.
inline std::string ComposerVersionOf(const fs::path& dll, std::string& err) {
  std::ifstream f(dll, std::ios::binary);
  if (!f) { err = "cannot read " + dll.u8string() + " to tell the composer's version"; return {}; }
  uint64_t h = 0xcbf29ce484222325ull, size = 0;
  std::vector<char> buf(1 << 16);
  while (f) {
    f.read(buf.data(), (std::streamsize)buf.size());
    std::streamsize n = f.gcount();
    for (std::streamsize k = 0; k < n; ++k) { h ^= (unsigned char)buf[(size_t)k]; h *= 0x100000001b3ull; }
    size += (uint64_t)n;
  }
  char b[96];
  snprintf(b, sizeof(b), "%s fnv1a64 %016llx %llu", dll.filename().u8string().c_str(), (unsigned long long)h, (unsigned long long)size);
  return b;
}

// ================================================================ merging stages into one song

// A stage's song text, read the way the engine's song format reads it (src/song.rs), as far as merging needs.
struct SongDef { std::string name, line; std::vector<std::string> params; std::string rhs; };
struct SongBlock { std::string kind, name; std::vector<std::pair<std::string, std::string>> fields;
  const std::string* get(const std::string& k) const { for (auto& f : fields) if (f.first == k) return &f.second; return nullptr; } };
struct StageSong {
  std::vector<std::string> settings;  // setting lines, without beats and loop
  std::vector<SongDef> defs;
  std::vector<SongBlock> voices;
  const SongDef* def(const std::string& n) const { for (auto& d : defs) if (d.name == n) return &d; return nullptr; }
  const SongBlock* voice(const std::string& n) const { for (auto& b : voices) if (b.name == n) return &b; return nullptr; }
};

inline std::string Trim(const std::string& s) {
  size_t a = s.find_first_not_of(" \t\r");
  if (a == std::string::npos) return "";
  size_t b = s.find_last_not_of(" \t\r");
  return s.substr(a, b - a + 1);
}

inline bool ParseStageSong(const std::string& text, StageSong& out, std::string& err) {
  static const char* kSettings[] = { "bpm", "beats", "rate", "reference", "reverb_size", "reverb_damp", "reverb_width", "reverb_predelay",
                                     "delay_beats", "delay_feedback", "delay_tone", "wow", "wow_rate", "warmth", "loop" };
  out = StageSong{};
  SongBlock* open = nullptr;
  SongBlock cur;
  size_t a = 0, line = 0;
  while (a <= text.size()) {
    size_t e = text.find('\n', a);
    std::string raw = text.substr(a, e == std::string::npos ? std::string::npos : e - a);
    ++line;
    a = e == std::string::npos ? text.size() + 1 : e + 1;
    size_t hash = raw.find('#');
    std::string t = Trim(hash == std::string::npos ? raw : raw.substr(0, hash));
    if (t.empty()) continue;
    std::string first = t.substr(0, t.find_first_of(" \t"));
    auto fail = [&](const std::string& why) { err = "line " + std::to_string(line) + ": " + why; return false; };
    if (open) {
      if (t == "}") { out.voices.push_back(cur); open = nullptr; continue; }
      size_t eq = t.find('=');
      if (eq == std::string::npos) return fail("expected 'key = value' in a voice");
      cur.fields.push_back({ Trim(t.substr(0, eq)), Trim(t.substr(eq + 1)) });
      continue;
    }
    if (std::find_if(std::begin(kSettings), std::end(kSettings), [&](const char* s) { return first == s; }) != std::end(kSettings)) {
      if (first != "beats" && first != "loop") out.settings.push_back(t);
      continue;
    }
    if (first == "voice" || first == "drum") {
      std::string rest = Trim(t.substr(first.size()));
      if (rest.empty() || rest.back() != '{') return fail("expected '" + first + " <name> {'");
      cur = SongBlock{ first, Trim(rest.substr(0, rest.size() - 1)), {} };
      open = &cur;
      continue;
    }
    size_t eq = t.find('=');
    if (eq == std::string::npos) return fail("expected a definition 'name = ...'");
    SongDef d;
    d.line = t;
    std::string lhs = Trim(t.substr(0, eq));
    d.rhs = Trim(t.substr(eq + 1));
    size_t par = lhs.find('(');
    d.name = Trim(lhs.substr(0, par));
    if (par != std::string::npos) {
      std::string ps = lhs.substr(par + 1);
      size_t b = 0;
      while (b < ps.size()) {
        size_t c = ps.find_first_of(",)", b);
        std::string p = Trim(ps.substr(b, c == std::string::npos ? std::string::npos : c - b));
        if (!p.empty()) d.params.push_back(p);
        if (c == std::string::npos) break;
        b = c + 1;
      }
    }
    out.defs.push_back(std::move(d));
  }
  if (open) { err = "a voice is not closed with '}'"; return false; }
  return true;
}

// The identifiers in an expression, as the engine's lexer reads them: [start, end) byte spans. Numbers (digits with one
// dot) are skipped, so the `t` in `4t` is a name (a product), as the engine reads it.
struct Ident { size_t start, end; };
inline std::vector<Ident> Identifiers(const std::string& s) {
  std::vector<Ident> out;
  size_t i = 0;
  auto alpha = [](char c) { return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || c == '_'; };
  auto digit = [](char c) { return c >= '0' && c <= '9'; };
  while (i < s.size()) {
    char c = s[i];
    if (digit(c) || (c == '.' && i + 1 < s.size() && digit(s[i + 1]))) {
      bool dot = false;
      while (i < s.size() && (digit(s[i]) || (s[i] == '.' && !dot))) { dot |= s[i] == '.'; ++i; }
    } else if (alpha(c)) {
      size_t a = i;
      while (i < s.size() && (alpha(s[i]) || digit(s[i]))) ++i;
      out.push_back({ a, i });
    } else {
      ++i;
    }
  }
  return out;
}

// Renames in an expression: `t` becomes `time`, every name in `map` its new name, a function's own parameters stay.
inline std::string Rename(const std::string& expr, const std::map<std::string, std::string>& map, const std::string& time,
                          const std::vector<std::string>& params) {
  std::string out;
  size_t at = 0;
  for (const Ident& id : Identifiers(expr)) {
    std::string name = expr.substr(id.start, id.end - id.start);
    if (std::find(params.begin(), params.end(), name) != params.end()) continue;
    std::string to;
    if (name == "t") to = time;
    else { auto f = map.find(name); if (f == map.end()) continue; to = f->second; }
    out += expr.substr(at, id.start - at);
    out += to;
    at = id.end;
  }
  out += expr.substr(at);
  return out;
}

// Recorded stages first..last (positions in r.stages) as one ordinary song, the way the engine saves a stretch of a
// live stream (src/stream.rs, save_text): stage i's definitions get the suffix _i (those identical in every stage and
// free of t are shared and keep their names), its time is T_i = t - 32 i (T2_i when a stage defines a T), and every
// voice's functions choose stage i's by Q = floor(t / 32). The song's settings are the last stage's. A voice is one
// voice for as long as its instrument (every setting but its time functions) stays the same; where the instrument
// changes, the old one is released and a second voice (name_N, struck in stage N) starts fresh, as the stream does. A
// voice not in a stage after it has played is released there on the functions of the stage it last played in.
// `what` opens the first comment line ("Replayed" or "Saved"). Empty with `err` when a stage's text cannot be read.
inline std::string MergeStages(const Record& r, size_t first, size_t last, const std::string& what, std::string& err) {
  if (r.stages.empty() || first > last || last >= r.stages.size()) { err = "no such stages"; return {}; }
  const size_t count = last - first + 1;
  const double sb = r.stageBeats > 0 ? r.stageBeats : 32;
  auto num = [](double v) { char b[32]; snprintf(b, sizeof(b), "%.17g", v); return std::string(b); };
  std::vector<StageSong> st(count);
  for (size_t i = 0; i < count; ++i) {
    std::string e;
    if (!ParseStageSong(r.stages[first + i].text, st[i], e)) { err = "stage " + std::to_string(r.stages[first + i].index + 1) + ": " + e; return {}; }
  }
  const StageSong& L = st.back();
  // Shared: an identical line in every stage, reading neither t nor a definition that is not itself shared.
  std::vector<std::string> shared;
  for (auto& d : L.defs) {
    bool all = true;
    for (auto& s : st) { const SongDef* x = s.def(d.name); if (!x || x->line != d.line) { all = false; break; } }
    if (all) shared.push_back(d.name);
  }
  for (;;) {
    std::vector<std::string> snap = shared, keep;
    for (auto& n : shared) {
      const SongDef* d = L.def(n);
      bool ok = true;
      for (const Ident& id : Identifiers(d->rhs)) {
        std::string x = d->rhs.substr(id.start, id.end - id.start);
        if (std::find(d->params.begin(), d->params.end(), x) != d->params.end()) continue;
        if (x == "t") { ok = false; break; }
        if (L.def(x) && std::find(snap.begin(), snap.end(), x) == snap.end()) { ok = false; break; }
      }
      if (ok) keep.push_back(n);
    }
    bool same = keep.size() == shared.size();
    shared = keep;
    if (same) break;
  }
  auto isShared = [&](const std::string& n) { return std::find(shared.begin(), shared.end(), n) != shared.end(); };
  std::vector<std::string> taken;
  for (auto& s : st) for (auto& d : s.defs) taken.push_back(d.name);
  std::string q = "Q";
  for (int k = 2; std::find(taken.begin(), taken.end(), q) != taken.end(); ++k) q = "Q" + std::to_string(k);
  // The stages' times, tp_i: "T" unless a stage has a definition its renaming would collide with (the move book's
  // blancmange curve is T(u), renamed T_i).
  auto clashes = [&](const std::string& p) {
    for (auto& n : taken) if (n == p || n.rfind(p + "_", 0) == 0) return true;
    return false;
  };
  std::string tp = "T";
  for (int k = 2; clashes(tp); ++k) tp = "T" + std::to_string(k);

  const Stage& a = r.stages[first];
  const Stage& b = r.stages[last];
  std::string text = "# " + what + " from Dreamride's history (" + (r.origin.empty() ? std::string("a stream") : r.origin) + ", seed " +
                     std::to_string(r.seed) + ", recorded " + r.started + "): stages " + std::to_string(a.index + 1) + " to " +
                     std::to_string(b.index + 1) + ", " + num(sb / 4 * count) + " bars.\n#\n";
  for (size_t i = 0; i < count; ++i) {
    std::string s = OneLine(r.stages[first + i].summary);
    while (!s.empty() && s.back() == '.') s.pop_back();
    text += "# Stage " + std::to_string(r.stages[first + i].index + 1) + ": " + s + ".\n";
  }
  text += "#\n# Each stage's definitions carry its number (M_0, M_1, ...) and its own time " + tp + "_i = t - " + num(sb) + " i; " + q +
          " = floor(t / " + num(sb) + ") picks the stage.\n\n";
  for (auto& s : L.settings) text += s + "\n";
  text += "beats " + num(sb * count) + "\n\n";
  std::vector<std::string> lines{ q + " = floor(t / " + num(sb) + ")" };
  for (auto& d : L.defs) if (isShared(d.name)) lines.push_back(d.line);
  std::vector<std::map<std::string, std::string>> maps(count);
  std::vector<std::string> times(count);
  for (size_t i = 0; i < count; ++i) {
    times[i] = tp + "_" + std::to_string(i);
    lines.push_back(times[i] + " = t - " + num(sb * i));
    for (auto& d : st[i].defs) if (!isShared(d.name)) maps[i][d.name] = d.name + "_" + std::to_string(i);
    for (auto& d : st[i].defs) {
      if (isShared(d.name)) continue;
      std::string lhs = d.name + "_" + std::to_string(i);
      if (!d.params.empty()) {
        lhs += "(";
        for (size_t k = 0; k < d.params.size(); ++k) lhs += (k ? ", " : "") + d.params[k];
        lhs += ")";
      }
      lines.push_back(lhs + " = " + Rename(d.rhs, maps[i], times[i], d.params));
    }
  }
  for (auto& l : lines) text += l + "\n";
  // The voices: the roster's order, then any other in the order they appear.
  std::vector<std::string> names = r.roster;
  for (auto& s : st) for (auto& v : s.voices) if (std::find(names.begin(), names.end(), v.name) == names.end()) names.push_back(v.name);
  static const char* kExpr[] = { "pitch", "gate", "amp", "trigger", "cutoff" };
  auto isExpr = [](const std::string& k) { return std::find_if(std::begin(kExpr), std::end(kExpr), [&](const char* e) { return k == e; }) != std::end(kExpr); };
  // A voice's instrument: everything but its time functions, and which of amp and cutoff it has. The stream releases a
  // voice whose instrument changes at a seam and strikes the new one fresh (src/stream.rs, same_instrument), so a
  // change of instrument here is another voice: the old one released at the seam, the new one starting from rest.
  auto instrument = [&](const SongBlock& b) {
    std::vector<std::string> parts{ b.kind };
    for (auto& f : b.fields) parts.push_back(isExpr(f.first) ? ((f.first == "amp" || f.first == "cutoff") ? f.first : std::string()) : f.first + "=" + f.second);
    std::sort(parts.begin() + 1, parts.end());
    std::string k;
    for (auto& p : parts) k += p + "\n";
    return k;
  };
  std::vector<std::string> used = names;
  for (auto& name : names) {
    // Runs of one instrument, in order: each run's stages (a voice may leave and come back on the same instrument).
    struct Run { std::string key, name; std::vector<const SongBlock*> at; const SongBlock* tmpl = nullptr; };
    std::vector<Run> runs;
    for (size_t i = 0; i < count; ++i) {
      const SongBlock* v = st[i].voice(name);
      if (!v) continue;
      std::string key = instrument(*v);
      if (runs.empty() || runs.back().key != key) {
        Run run; run.key = key; run.at.assign(count, nullptr);
        run.name = name;
        if (!runs.empty()) {  // later instruments: the name and the stage it is struck in (unique in this song)
          run.name = name + "_" + std::to_string(r.stages[first + i].index + 1);
          for (int k = 2; std::find(used.begin(), used.end(), run.name) != used.end(); ++k)
            run.name = name + "_" + std::to_string(r.stages[first + i].index + 1) + "_" + std::to_string(k);
          used.push_back(run.name);
        }
        runs.push_back(run);
      }
      runs.back().at[i] = v;
      runs.back().tmpl = v;
    }
    for (auto& run : runs) {
      text += "\n" + run.tmpl->kind + " " + run.name + " {\n";
      for (auto& f : run.tmpl->fields) {
        if (!isExpr(f.first)) { text += "  " + f.first + " = " + f.second + "\n"; continue; }
        // Per stage: its own function where this instrument plays; after it has played, released (gate and trigger
        // 0) with the functions of the stage it last played in, on that stage's time, as the stream releases a
        // voice; before it first plays, nothing.
        std::vector<std::string> h(count);
        size_t last = SIZE_MAX;
        for (size_t i = 0; i < count; ++i) {
          const SongBlock* v = run.at[i];
          if (v) last = i;
          const SongBlock* from = v ? v : (last != SIZE_MAX ? run.at[last] : nullptr);
          const std::string* here = from ? from->get(f.first) : nullptr;
          if (!v && from && (f.first == "gate" || f.first == "trigger")) h[i] = "0";
          else if (here) h[i] = Rename(*here, maps[v ? i : last], times[v ? i : last], {});
          else h[i] = (f.first == "amp" && from) ? "1" : "0";
        }
        std::string e;
        for (size_t i = count; i-- > 0;) e = e.empty() ? h[i] : "if(" + q + " == " + std::to_string(i) + ", " + h[i] + ", " + e + ")";
        text += "  " + f.first + " = " + e + "\n";
      }
      text += "}\n";
    }
  }
  return text;
}

// ================================================================ the list

// A history entry as the History tab lists it.
struct Summary {
  std::string path, started, origin, composer, note;
  long long startedUnix = 0;
  uint64_t seed = 0, stages = 0, heardStages = 0;
  double heardSeconds = 0, bpm = 0, stageBeats = 32;
  bool starred = false;
  bool complete = false;
  double completeSeconds = 0;
};

// A stage's summary as a few words: "build: the kick comes in; a new progression, 1 6 2 5" gives "kick in" and
// "new progression"; the first stage gives its role ("intro").
inline std::vector<std::string> ShortChanges(const std::string& summary, bool first) {
  std::string s = OneLine(summary);
  size_t colon = s.find(':');
  std::string role = colon == std::string::npos ? std::string() : Trim(s.substr(0, colon));
  if (first && !role.empty() && role.find(' ') == std::string::npos) return { role };
  std::string body = colon == std::string::npos ? s : s.substr(colon + 1);
  std::vector<std::string> out;
  size_t a = 0;
  while (a <= body.size()) {
    size_t e = body.find(';', a);
    std::string c = Trim(body.substr(a, e == std::string::npos ? std::string::npos : e - a));
    a = e == std::string::npos ? body.size() + 1 : e + 1;
    size_t comma = c.find(',');
    if (comma != std::string::npos) c = Trim(c.substr(0, comma));
    while (!c.empty() && c.back() == '.') c.pop_back();
    for (const char* art : { "the ", "a ", "an " }) if (c.rfind(art, 0) == 0) { c = c.substr(strlen(art)); break; }
    auto swap = [&](const char* from, const char* to) { size_t p = c.find(from); if (p != std::string::npos) c.replace(p, strlen(from), to); };
    swap(" comes in", " in"); swap(" come in", " in"); swap(" leaves", " out"); swap(" leave", " out"); swap(" goes", " out");
    if (!c.empty()) out.push_back(c);
  }
  if (out.empty() && !role.empty()) out.push_back(role);
  return out;
}

// The note under an entry: its stages' changes in a few words, at most `maxChars` bytes, "…" when cut.
inline std::string StageNote(const std::vector<Stage>& stages, size_t maxChars = 90) {
  std::vector<std::string> parts;
  for (size_t i = 0; i < stages.size(); ++i)
    for (auto& c : ShortChanges(stages[i].summary, i == 0))
      if (parts.empty() || parts.back() != c) parts.push_back(c);
  std::string o;
  for (size_t i = 0; i < parts.size(); ++i) {
    std::string next = (i ? ", " : "") + parts[i];
    if (o.size() + next.size() > maxChars) return o + u8"…";
    o += next;
  }
  return o;
}

inline Summary Summarize(const Record& r) {
  Summary s;
  s.path = r.path; s.started = r.started; s.origin = r.origin; s.composer = r.composer;
  s.startedUnix = r.startedUnix; s.seed = r.seed; s.stages = r.stages.size();
  s.heardStages = r.heardStages; s.heardSeconds = r.heardSeconds; s.bpm = r.bpm; s.stageBeats = r.stageBeats;
  s.starred = r.starred;
  s.complete = r.complete; s.completeSeconds = r.completeSeconds;
  s.note = StageNote(r.stages);
  return s;
}

// Starred first, then the newest first (by start, then by file name).
inline void Sort(std::vector<Summary>& v) {
  std::stable_sort(v.begin(), v.end(), [](const Summary& a, const Summary& b) {
    if (a.starred != b.starred) return a.starred;
    if (a.startedUnix != b.startedUnix) return a.startedUnix > b.startedUnix;
    return a.path > b.path;
  });
}

inline int Find(const std::vector<Summary>& v, const std::string& path) {
  for (size_t i = 0; i < v.size(); ++i) if (v[i].path == path) return (int)i;
  return -1;
}

inline bool IsTxt(const fs::path& p) {
  std::string x = p.extension().u8string();
  for (char& c : x) c = (char)tolower((unsigned char)c);
  return x == ".txt";
}

struct Scan { std::vector<Summary> entries; std::vector<std::string> errors; size_t deleted = 0; };

// Every history file directly in `dir`, sorted; a file that is not one or is damaged is listed in `errors` and
// skipped. `deleted` counts the files in dir\deleted. A missing folder is an empty history, not an error.
inline Scan ScanDir(const std::string& dir) {
  Scan s;
  std::error_code ec;
  fs::path d = fs::u8path(dir);
  if (!fs::is_directory(d, ec)) return s;
  fs::directory_iterator it(d, ec), end;
  if (ec) { s.errors.push_back("cannot read " + dir + ": " + ec.message()); return s; }
  for (; it != end; it.increment(ec)) {
    if (ec) { s.errors.push_back("cannot read " + dir + ": " + ec.message()); break; }
    std::error_code fe;
    if (!it->is_regular_file(fe) || !IsTxt(it->path())) continue;
    Record r;
    std::string err;
    if (!LoadRecord(it->path().u8string(), r, err, false)) { s.errors.push_back(err); continue; }
    s.entries.push_back(Summarize(r));
  }
  fs::directory_iterator del(d / "deleted", ec);
  if (!ec) for (; del != end; del.increment(ec)) { if (ec) break; std::error_code fe; if (del->is_regular_file(fe)) ++s.deleted; }
  Sort(s.entries);
  return s;
}

// ================================================================ deleting, undoing, emptying

// Moves a history file into <its folder>\deleted (a free name there). The new path in `moved`. "" on success.
inline std::string MoveToDeleted(const std::string& path, std::string& moved) {
  fs::path p = fs::u8path(path);
  std::error_code ec;
  if (!fs::is_regular_file(p, ec)) return "no such file: " + path;
  fs::path bin = p.parent_path() / "deleted";
  std::string e = EnsureDir(bin);
  if (!e.empty()) return e;
  fs::path to = UniquePath(bin, p.stem().u8string());
  fs::rename(p, to, ec);
  if (ec) return "cannot move " + path + " to " + to.u8string() + ": " + ec.message();
  moved = to.u8string();
  return "";
}

// Puts a deleted file back where it was (a free name there if that one has been taken since). The path in `restored`.
inline std::string Restore(const std::string& deletedPath, const std::string& originalPath, std::string& restored) {
  fs::path from = fs::u8path(deletedPath), orig = fs::u8path(originalPath);
  std::error_code ec;
  if (!fs::is_regular_file(from, ec)) return "the deleted file is gone: " + deletedPath;
  std::string e = EnsureDir(orig.parent_path());
  if (!e.empty()) return e;
  fs::path to = fs::exists(orig, ec) ? UniquePath(orig.parent_path(), orig.stem().u8string()) : orig;
  fs::rename(from, to, ec);
  if (ec) return "cannot restore " + to.u8string() + ": " + ec.message();
  restored = to.u8string();
  return "";
}

// Removes every file in dir\deleted for good. How many in `removed`. "" on success.
inline std::string EmptyDeleted(const std::string& dir, size_t& removed) {
  removed = 0;
  fs::path bin = fs::u8path(dir) / "deleted";
  std::error_code ec;
  if (!fs::is_directory(bin, ec)) return "";
  std::vector<fs::path> files;
  fs::directory_iterator it(bin, ec), end;
  if (ec) return "cannot read " + bin.u8string() + ": " + ec.message();
  for (; it != end; it.increment(ec)) { if (ec) break; std::error_code fe; if (it->is_regular_file(fe)) files.push_back(it->path()); }
  for (auto& f : files) {
    if (!fs::remove(f, ec) || ec) return "cannot remove " + f.u8string() + (ec ? ": " + ec.message() : std::string());
    ++removed;
  }
  return "";
}

// ================================================================ the writer

// One thread that does the history's file work in order, so the frame never waits on the disk. A job returns what the
// main thread should hear: an error ("" for none) and, optionally, something to run on the main thread (Take).
class Worker {
 public:
  struct Done { std::string error; std::function<void()> onMain; bool sessionWrite = false; };
  Worker() : thread_([this] { Run(); }) {}
  ~Worker() {
    { std::lock_guard<std::mutex> l(m_); stop_ = true; }
    cv_.notify_all();
    if (thread_.joinable()) thread_.join();
  }
  Worker(const Worker&) = delete;
  Worker& operator=(const Worker&) = delete;

  void Post(std::function<Done()> job) {
    { std::lock_guard<std::mutex> l(m_); jobs_.push_back(std::move(job)); }
    cv_.notify_all();
  }
  // The results so far (main thread; never waits).
  std::vector<Done> Take() {
    std::lock_guard<std::mutex> l(m_);
    std::vector<Done> d;
    d.swap(done_);
    return d;
  }
  // Waits until every job posted so far has run (tests, headless captures, and quitting).
  void Flush() {
    std::unique_lock<std::mutex> l(m_);
    idle_.wait(l, [this] { return jobs_.empty() && !running_; });
  }
  bool Busy() { std::lock_guard<std::mutex> l(m_); return running_ || !jobs_.empty(); }

 private:
  void Run() {
    for (;;) {
      std::function<Done()> job;
      {
        std::unique_lock<std::mutex> l(m_);
        cv_.wait(l, [this] { return stop_ || !jobs_.empty(); });
        if (jobs_.empty()) return;  // stopping, and nothing left to write
        job = std::move(jobs_.front());
        jobs_.pop_front();
        running_ = true;
      }
      Done d;
      try { d = job(); } catch (const std::exception& e) { d.error = std::string("history: ") + e.what(); }
      {
        std::lock_guard<std::mutex> l(m_);
        done_.push_back(std::move(d));
        running_ = false;
      }
      idle_.notify_all();
    }
  }
  std::mutex m_;
  std::condition_variable cv_, idle_;
  std::deque<std::function<Done()>> jobs_;
  std::vector<Done> done_;
  bool stop_ = false, running_ = false;
  std::thread thread_;
};

// A recording, as the worker keeps it: the record so far, and whether the file on disk holds all of it. Only the
// worker touches it (its jobs run in order on one thread).
struct Session {
  std::string dir;
  Record rec;
  bool written = false;  // the file at rec.path holds rec
};

// After a change to s.rec: append `chunk` when the file is up to date, else write the whole record (the first write,
// or after a failure: a failed write loses nothing, the next one writes everything). "" on success.
inline std::string Sync(Session& s, const std::string& chunk) {
  std::error_code ec;
  if (s.written && !fs::is_regular_file(fs::u8path(s.rec.path), ec)) s.written = false;  // gone since: write it whole again
  if (s.written) {
    std::string e = WriteText(fs::u8path(s.rec.path), chunk, true);
    if (e.empty()) return "";
    s.written = false;
    return e;
  }
  std::string e = EnsureDir(fs::u8path(s.dir));
  if (!e.empty()) return e;
  if (s.rec.path.empty()) s.rec.path = UniquePath(fs::u8path(s.dir), FileStem(s.rec)).u8string();
  e = WriteText(fs::u8path(s.rec.path), Serialize(s.rec), false);
  s.written = e.empty();
  return e;
}

}  // namespace hist
