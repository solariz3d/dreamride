// ui_logic.h: the parts of Dreamride's UI that need no window and no GPU, so they can be tested
// alone (world/tests/ui_logic_test.cpp, run by world/test.bat): the song library (with its Stream entry), the
// keyboard shortcuts, seek and volume steps, and what Esc, Tab and H do to the open panels.
#pragma once

#include <algorithm>
#include <cmath>
#include <cstdio>
#include <filesystem>
#include <fstream>
#include <functional>
#include <string>
#include <system_error>
#include <vector>

namespace ui {

// ================================================================ actions and keys

enum class Action {
  None, PlayPause, Restart, SeekBack, SeekForward, VolumeUp, VolumeDown, NextSong, PrevSong,
  ToggleLoop, ToggleMenu, ToggleHelp, Fullscreen, Escape, ToggleEquation, SaveStream, MarkBad,
};

// Windows virtual-key codes, by value so this header needs no windows.h (the test checks them
// against VK_*).
enum Key : int {
  kKeyTab = 0x09, kKeyEscape = 0x1B, kKeySpace = 0x20, kKeyLeft = 0x25, kKeyUp = 0x26, kKeyRight = 0x27,
  kKeyDown = 0x28, kKeyB = 'B', kKeyE = 'E', kKeyS = 'S', kKeyH = 'H', kKeyL = 'L', kKeyM = 'M', kKeyN = 'N', kKeyP = 'P', kKeyR = 'R', kKeyF11 = 0x7A,
};

// repeats: whether holding the key acts again (seek and volume do; toggles do not).
struct Shortcut { int key; Action action; bool repeats; };

inline const std::vector<Shortcut>& Shortcuts() {
  static const std::vector<Shortcut> s = {
    { kKeySpace, Action::PlayPause, false }, { kKeyR, Action::Restart, false },
    { kKeyLeft, Action::SeekBack, true },    { kKeyRight, Action::SeekForward, true },
    { kKeyUp, Action::VolumeUp, true },      { kKeyDown, Action::VolumeDown, true },
    { kKeyN, Action::NextSong, false },      { kKeyP, Action::PrevSong, false },
    { kKeyL, Action::ToggleLoop, false },    { kKeyTab, Action::ToggleMenu, false },
    { kKeyM, Action::ToggleMenu, false },    { kKeyH, Action::ToggleHelp, false },
    { kKeyF11, Action::Fullscreen, false },  { kKeyEscape, Action::Escape, false },
    { kKeyE, Action::ToggleEquation, false }, { kKeyS, Action::SaveStream, false },
    { kKeyB, Action::MarkBad, false },
  };
  return s;
}

// The action for a key press; None for an unmapped key, and for an auto-repeat of a key that does
// not repeat (holding Space must not flicker play and pause).
inline Action KeyToAction(int key, bool isRepeat = false) {
  for (const Shortcut& s : Shortcuts())
    if (s.key == key) return (isRepeat && !s.repeats) ? Action::None : s.action;
  return Action::None;
}

// The help card's rows: the keys as a person reads them, and every action each row covers. The
// test checks that every shortcut's action appears in some row, so the card cannot drift.
struct HelpRow { const char* keys; const char* what; std::vector<Action> actions; };

inline const std::vector<HelpRow>& HelpRows() {
  static const std::vector<HelpRow> r = {
    { "Space", "Play / pause", { Action::PlayPause } },
    { "R", "Restart the song", { Action::Restart } },
    { u8"←  →", "Seek 5 seconds", { Action::SeekBack, Action::SeekForward } },
    { u8"↑  ↓", "Volume", { Action::VolumeUp, Action::VolumeDown } },
    { "N  P", "Next / previous song", { Action::NextSong, Action::PrevSong } },
    { "L", "Loop the song on / off", { Action::ToggleLoop } },
    { "Tab  M", "Menu", { Action::ToggleMenu } },
    { "H", "This card", { Action::ToggleHelp } },
    { "E", "The live equation (Stream): compact, full, hidden", { Action::ToggleEquation } },
    { "S", "Save the last stages as a song (Stream)", { Action::SaveStream } },
    { "B", "Mark the last stages as bad, for the generator to learn from (Stream)", { Action::MarkBad } },
    { "F11", "Fullscreen", { Action::Fullscreen } },
    { "Esc", "Close the menu, then quit (asks first)", { Action::Escape } },
    { "Wheel", "Over the seek bar: scrub", {} },
  };
  return r;
}

// ================================================================ seek, volume, the song list

constexpr double kSeekStep = 5.0;        // seconds, the arrow keys
constexpr double kWheelSeekStep = 2.0;   // seconds per wheel notch over the seek bar
constexpr double kSeekEndMargin = 0.05;  // a seek stops this short of the end, so it still plays
constexpr float kVolumeStep = 0.1f;

// A position in the song, held inside [0, duration - margin]. A bad duration or position (zero,
// negative, NaN) gives 0: the start is the only position every song has.
inline double ClampSeek(double t, double duration) {
  if (!(duration > 0) || !std::isfinite(duration) || !std::isfinite(t)) return 0.0;
  return std::clamp(t, 0.0, std::max(0.0, duration - kSeekEndMargin));
}

// Volume one step up or down, in [0, 1], on the tenths (repeated steps do not drift).
inline float StepVolume(float v, int dir) {
  if (!std::isfinite(v)) v = 1.0f;
  float n = std::round((v + dir * kVolumeStep) * 10.0f) / 10.0f;
  return std::clamp(n, 0.0f, 1.0f);
}

// The next (dir > 0) or previous song index, wrapping round the list. -1 when the list is empty;
// from no song (-1), next is the first and previous the last.
inline int StepIndex(int cur, int count, int dir) {
  if (count <= 0) return -1;
  if (cur < 0 || cur >= count) return dir >= 0 ? 0 : count - 1;
  return ((cur + (dir >= 0 ? 1 : -1)) % count + count) % count;
}

// "m:ss" (and "h:mm:ss" past an hour). Negative or NaN shows as 0:00.
inline std::string FormatTime(double s) {
  if (!std::isfinite(s) || s < 0) s = 0;
  long t = (long)std::floor(s);
  char b[32];
  if (t >= 3600) snprintf(b, sizeof(b), "%ld:%02ld:%02ld", t / 3600, (t / 60) % 60, t % 60);
  else snprintf(b, sizeof(b), "%ld:%02ld", t / 60, t % 60);
  return b;
}

// ================================================================ the panels and Esc

struct Overlay {
  bool menu = false;         // the side panel (library, view, theme)
  bool help = false;         // the keys card
  bool confirmQuit = false;  // "Quit Dreamride?"
};

enum class EscResult { Closed, AskQuit, Quit };

// Esc closes what is open, the topmost first, and quits only when nothing is: the first Esc with
// nothing open asks, the second quits. `shown` is whether the overlay is visible now: a panel that
// has faded away counts as closed, so Esc never spends itself closing something unseen.
inline EscResult OnEscape(Overlay& o, bool shown) {
  if (o.confirmQuit) return EscResult::Quit;
  if (!shown) { o.menu = false; o.help = false; }
  if (o.help) { o.help = false; return EscResult::Closed; }
  if (o.menu) { o.menu = false; return EscResult::Closed; }
  o.confirmQuit = true;
  return EscResult::AskQuit;
}

// Any other key while "Quit?" shows answers "stay", and does nothing else. True when it did.
inline bool CancelQuit(Overlay& o) {
  if (!o.confirmQuit) return false;
  o.confirmQuit = false;
  return true;
}

// Tab or M: a faded overlay comes back showing the menu; a visible one toggles it.
inline void OnMenuKey(Overlay& o, bool shown) { o.menu = shown ? !o.menu : true; }
// H: the same, for the keys card.
inline void OnHelpKey(Overlay& o, bool shown) { o.help = shown ? !o.help : true; }

// ================================================================ the song library

struct SongEntry {
  std::string path;         // UTF-8
  std::string title;        // the file name without .txt
  std::string description;  // the first line, when it is a comment
  long long size = -1;      // bytes, and the last write time (the clock's ticks): together the file's stamp,
  long long written = 0;    //   so a rescan can tell a rewritten song from an unchanged one
  bool isNew = false;       // appeared or changed while Dreamride was running, and not played since
};

// A song's description: its first line, when that line is a `#` comment, without the `#`. Empty
// when the file is empty or starts with code. A UTF-8 byte-order mark and a CR are ignored.
inline std::string DescriptionOf(const std::string& text) {
  size_t p = 0;
  if (text.size() >= 3 && (unsigned char)text[0] == 0xEF && (unsigned char)text[1] == 0xBB && (unsigned char)text[2] == 0xBF) p = 3;
  size_t e = text.find('\n', p);
  std::string line = text.substr(p, e == std::string::npos ? std::string::npos : e - p);
  if (!line.empty() && line.back() == '\r') line.pop_back();
  size_t q = line.find_first_not_of(" \t");
  if (q == std::string::npos || line[q] != '#') return "";
  q = line.find_first_not_of("# \t", q);
  if (q == std::string::npos) return "";
  size_t last = line.find_last_not_of(" \t");
  return line.substr(q, last - q + 1);
}

inline bool HasTxtExtension(const std::filesystem::path& p) {
  std::string x = p.extension().u8string();
  std::transform(x.begin(), x.end(), x.begin(), [](unsigned char c) { return (char)std::tolower(c); });
  return x == ".txt";
}

// A seed file (since 2026-11-04): a stream's seed, kind and generator version; opening it replays the whole stream.
inline bool HasSeedExtension(const std::filesystem::path& p) {
  std::string x = p.extension().u8string();
  std::transform(x.begin(), x.end(), x.begin(), [](unsigned char c) { return (char)std::tolower(c); });
  return x == ".seed";
}
inline bool IsSeedPath(const std::string& p) { return HasSeedExtension(std::filesystem::u8path(p)); }

// Every *.txt song directly in each folder (not in subfolders), with its description, in folder
// order and then by name (case-insensitive). A file found twice (the same folder given twice) is
// listed once. A folder that does not exist or cannot be read, and a file that cannot be read, is
// reported in `errors` (when given) and skipped; the rest of the library still loads.
inline std::vector<SongEntry> ScanSongs(const std::vector<std::string>& dirs, std::vector<std::string>* errors = nullptr) {
  namespace fs = std::filesystem;
  std::vector<SongEntry> out;
  std::vector<std::string> seen;
  auto lower = [](std::string s) { std::transform(s.begin(), s.end(), s.begin(), [](unsigned char c) { return (char)std::tolower(c); }); return s; };
  for (const std::string& d : dirs) {
    std::error_code ec;
    fs::path dir = fs::u8path(d);
    if (!fs::is_directory(dir, ec)) {
      if (errors) errors->push_back("no song folder " + d);
      continue;
    }
    std::vector<SongEntry> here;
    fs::directory_iterator it(dir, ec), end;
    if (ec) { if (errors) errors->push_back("cannot read " + d + ": " + ec.message()); continue; }
    for (; it != end; it.increment(ec)) {
      if (ec) { if (errors) errors->push_back("cannot read " + d + ": " + ec.message()); break; }
      std::error_code fe;
      if (!it->is_regular_file(fe) || !(HasTxtExtension(it->path()) || HasSeedExtension(it->path()))) continue;
      fs::path canon = fs::weakly_canonical(it->path(), fe);
      std::string key = lower((fe ? it->path() : canon).u8string());
      if (std::find(seen.begin(), seen.end(), key) != seen.end()) continue;
      std::ifstream f(it->path(), std::ios::binary);
      if (!f) { if (errors) errors->push_back("cannot open " + it->path().u8string()); continue; }
      std::string head(4096, '\0');
      f.read(&head[0], (std::streamsize)head.size());
      head.resize((size_t)f.gcount());
      seen.push_back(key);
      SongEntry e{ it->path().u8string(), it->path().stem().u8string(), DescriptionOf(head) };
      std::error_code se, te;
      auto size = fs::file_size(it->path(), se);
      auto when = fs::last_write_time(it->path(), te);
      e.size = se ? -1 : (long long)size;
      e.written = te ? 0 : (long long)when.time_since_epoch().count();
      here.push_back(std::move(e));
    }
    std::sort(here.begin(), here.end(), [&](const SongEntry& a, const SongEntry& b) { return lower(a.title) < lower(b.title); });
    out.insert(out.end(), here.begin(), here.end());
  }
  return out;
}

// The library index of a song path (compared without case, separators normalised); -1 if absent.
inline int FindSong(const std::vector<SongEntry>& songs, const std::string& path) {
  namespace fs = std::filesystem;
  std::error_code ec;
  auto norm = [&](const std::string& p) {
    fs::path c = fs::weakly_canonical(fs::u8path(p), ec);
    std::string s = (ec ? fs::u8path(p) : c).u8string();
    std::transform(s.begin(), s.end(), s.begin(), [](unsigned char ch) { return ch == '/' ? '\\' : (char)std::tolower(ch); });
    return s;
  };
  std::string want = norm(path);
  for (size_t i = 0; i < songs.size(); ++i) if (norm(songs[i].path) == want) return (int)i;
  return -1;
}

// ---------------------------------------------------------------- the Stream entries

// The library's first three entries are not files: they start stream mode, an endless song whose eight-bar
// equations the composer writes as it plays. "Stream" is a new song every time it starts (a species from a fresh
// seed, its harmony one of the written progressions, shown in the equation panel so a good one can be played again
// with --stream --seed N); "Explore (strange tunings)" beside it is the same from the species of strange tunings
// (19-tone loops, Bohlen-Pierce, quarter-tone maqam, just ratios; since 2026-11-03); "Stream from G" starts from the
// song kept from the blind test. Their paths are markers no file can have.
constexpr const char* kStreamPath = "<stream>";
constexpr const char* kStreamExplorePath = "<stream:explore>";
constexpr const char* kStreamGPath = "<stream:G>";

inline bool IsStreamPath(const std::string& p) { return p == kStreamPath || p == kStreamExplorePath || p == kStreamGPath; }
inline bool IsStream(const SongEntry& e) { return IsStreamPath(e.path); }

inline SongEntry StreamEntry() {
  SongEntry e;
  e.path = kStreamPath;
  e.title = "Stream";
  e.description = u8"A new endless song every time: eight-bar equations, each a change you can hear, written as it plays";
  return e;
}

inline SongEntry StreamExploreEntry() {
  SongEntry e;
  e.path = kStreamExplorePath;
  e.title = "Explore (strange tunings)";
  e.description = u8"A new endless song in a strange tuning: 19-tone loops, Bohlen-Pierce, quarter-tone maqam, just ratios";
  return e;
}

inline SongEntry StreamGEntry() {
  SongEntry e;
  e.path = kStreamGPath;
  e.title = "Stream from G";
  e.description = u8"The endless song grown from G, the song kept from the blind test (binary counting, the pluck lead)";
  return e;
}

// A scan with the Stream entries first (once each, whatever the folders hold).
inline std::vector<SongEntry> WithStream(std::vector<SongEntry> songs) {
  songs.erase(std::remove_if(songs.begin(), songs.end(), IsStream), songs.end());
  songs.insert(songs.begin(), { StreamEntry(), StreamExploreEntry(), StreamGEntry() });
  return songs;
}

// A fresh stream seed from any entropy (the clock): 1 .. 999999, short enough to type after --seed.
// SplitMix64's finaliser, so neighbouring clock values give unrelated seeds.
inline unsigned long long FreshStreamSeed(unsigned long long entropy) {
  unsigned long long z = entropy + 0x9E3779B97F4A7C15ull;
  z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9ull;
  z = (z ^ (z >> 27)) * 0x94D049BB133111EBull;
  z ^= z >> 31;
  return 1 + z % 999999ull;
}

// A file name for a saved stretch of a stream: stream_s<seed>_<first>-<last>.txt (stages counted from 1),
// with _2, _3 ... added when `exists` says the name is taken.
inline std::string StreamSaveName(unsigned long long seed, unsigned long long first, unsigned long long last,
                                  const std::function<bool(const std::string&)>& exists) {
  char b[96];
  snprintf(b, sizeof(b), "stream_s%llu_%llu-%llu", seed, first, last);
  std::string base = b, name = base + ".txt";
  for (int k = 2; exists && exists(name) && k < 1000; ++k) name = base + "_" + std::to_string(k) + ".txt";
  return name;
}

// ---------------------------------------------------------------- the library refreshing itself

// A fresh scan merged with the library as it stood: a song not there before, or whose size or write
// time differs, is marked new; an unchanged one keeps its mark. `changed` is whether the list a
// person sees differs at all (a song added, removed, rewritten, or its description changed).
struct Rescan { std::vector<SongEntry> songs; bool changed = false; };

// Two scans of the same folders spell each path the same way, so a rescan matches songs by the
// path's text (without case, either separator) and touches no disk: FindSong canonicalises, which
// is a file-system call per comparison.
inline std::string PathKey(std::string p) {
  std::transform(p.begin(), p.end(), p.begin(), [](unsigned char c) { return c == '/' ? '\\' : (char)std::tolower(c); });
  return p;
}
inline int FindByKey(const std::vector<SongEntry>& songs, const std::string& key) {
  for (size_t i = 0; i < songs.size(); ++i) if (PathKey(songs[i].path) == key) return (int)i;
  return -1;
}

inline Rescan MergeRescan(const std::vector<SongEntry>& old, std::vector<SongEntry> fresh) {
  Rescan r;
  r.changed = old.size() != fresh.size();
  for (size_t i = 0; i < fresh.size(); ++i) {
    SongEntry& e = fresh[i];
    int j = FindByKey(old, PathKey(e.path));
    if (j < 0) { e.isNew = true; r.changed = true; continue; }
    const SongEntry& o = old[j];
    if (o.size != e.size || o.written != e.written || o.description != e.description) {
      e.isNew = true;
      r.changed = true;
    } else {
      e.isNew = o.isNew;
    }
    if (j != (int)i) r.changed = true;
  }
  r.songs = std::move(fresh);
  return r;
}

// An index into `from` as an index into `to`, by the song's path: -1 when the song is gone or the
// index is out of range; a negative index (none, or a song outside the library) is kept as it is.
inline int RemapIndex(const std::vector<SongEntry>& from, int index, const std::vector<SongEntry>& to) {
  if (index < 0) return index;
  if (index >= (int)from.size()) return -1;
  return FindByKey(to, PathKey(from[index].path));
}

// When to rescan: kRescanQuiet seconds after the last change signal (an editor saves in several
// writes), and, for folders that cannot be watched, every kRescanPoll seconds.
constexpr double kRescanQuiet = 0.3;
constexpr double kRescanPoll = 2.0;

struct RescanClock {
  bool poll = false;     // some folder cannot be watched: rescan on a period as well
  double due = -1;       // the time a signalled rescan is due (-1: none)
  double lastPoll = 0;

  void Signal(double now) { due = now + kRescanQuiet; }
  // True once when a rescan is due at `now`.
  bool Due(double now) {
    if (due >= 0 && now >= due) { due = -1; lastPoll = now; return true; }
    if (poll && now - lastPoll >= kRescanPoll) { lastPoll = now; return true; }
    return false;
  }
};

}  // namespace ui
