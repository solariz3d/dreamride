// Tests for world/src/ui_logic.h: the song library, the shortcut map, seek and volume steps, and
// the Esc/Tab/H panel logic. No window, no GPU. Built and run by world/test.bat.
#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>

#include <cstdio>
#include <fstream>
#include <functional>
#include <set>

#include "../src/ui_logic.h"
#include "../src/folder_watch.h"

namespace fs = std::filesystem;
using namespace ui;

static int g_failed = 0, g_checks = 0;
static const char* g_test = "";
#define CHECK(cond) do { ++g_checks; if (!(cond)) { ++g_failed; printf("  FAIL %s: %s (line %d)\n", g_test, #cond, __LINE__); } } while (0)
#define CHECK_EQ(a, b) do { ++g_checks; if (!((a) == (b))) { ++g_failed; printf("  FAIL %s: %s == %s (line %d)\n", g_test, #a, #b, __LINE__); } } while (0)

struct Test { const char* name; std::function<void()> fn; };
static std::vector<Test>& Tests() { static std::vector<Test> t; return t; }
struct Reg { Reg(const char* n, std::function<void()> f) { Tests().push_back({ n, f }); } };
#define TEST(name) static void name(); static Reg reg_##name(#name, name); static void name()

// A fresh temporary folder per test, removed afterwards.
struct TempDir {
  fs::path p;
  explicit TempDir(const char* tag) {
    p = fs::temp_directory_path() / (std::string("dreamride_test_") + tag + "_" + std::to_string(GetCurrentProcessId()));
    fs::remove_all(p);
    fs::create_directories(p);
  }
  ~TempDir() { std::error_code ec; fs::remove_all(p, ec); }
  void Write(const char* name, const std::string& text) const { std::ofstream(p / name, std::ios::binary) << text; }
};

// ---------------------------------------------------------------- descriptions

TEST(description_is_the_first_comment_line_without_the_hash) {
  CHECK_EQ(DescriptionOf("# City-pop v4: a night drive.\n# more\nbpm = 94\n"), std::string("City-pop v4: a night drive."));
}
TEST(description_ignores_a_crlf_a_bom_and_surrounding_spaces) {
  CHECK_EQ(DescriptionOf("\xEF\xBB\xBF#   Dark v3   \r\nbpm = 144\r\n"), std::string("Dark v3"));
  CHECK_EQ(DescriptionOf("  ## Two hashes\n"), std::string("Two hashes"));
}
TEST(description_is_empty_for_empty_files_code_first_and_bare_hashes) {
  CHECK_EQ(DescriptionOf(""), std::string(""));
  CHECK_EQ(DescriptionOf("\n# second line\n"), std::string(""));
  CHECK_EQ(DescriptionOf("bpm = 120\n# later\n"), std::string(""));
  CHECK_EQ(DescriptionOf("#\n"), std::string(""));
  CHECK_EQ(DescriptionOf("#   \n"), std::string(""));
}
TEST(description_of_a_single_line_without_newline) {
  CHECK_EQ(DescriptionOf("# only line"), std::string("only line"));
}

// ---------------------------------------------------------------- scanning

TEST(scan_lists_txt_songs_sorted_with_titles_and_descriptions) {
  TempDir d("scan");
  d.Write("b_song.txt", "# Bee\nbpm = 1\n");
  d.Write("A_song.TXT", "# Ay\n");
  d.Write("c_song.txt", "bpm = 3\n");
  d.Write("render.wav", "RIFF");
  d.Write("notes.md", "# not a song\n");
  fs::create_directories(d.p / "sub");
  std::ofstream(d.p / "sub" / "nested.txt") << "# nested\n";
  std::vector<std::string> errs;
  auto s = ScanSongs({ d.p.u8string() }, &errs);
  CHECK(errs.empty());
  CHECK_EQ(s.size(), (size_t)3);
  if (s.size() == 3) {
    CHECK_EQ(s[0].title, std::string("A_song"));
    CHECK_EQ(s[0].description, std::string("Ay"));
    CHECK_EQ(s[1].title, std::string("b_song"));
    CHECK_EQ(s[1].description, std::string("Bee"));
    CHECK_EQ(s[2].title, std::string("c_song"));
    CHECK_EQ(s[2].description, std::string(""));
    CHECK(fs::exists(fs::u8path(s[1].path)));
  }
}
TEST(scan_lists_seed_files_beside_songs) {
  TempDir d("seeds");
  d.Write("song.txt", "# a song\n");
  d.Write("stream_s152637_v1.seed", "# Dreamride seed: the automaton species, seed 152637\nseed 152637\nkind species\nversion 1\n");
  d.Write("LOUD.SEED", "seed 1\nkind book\nversion 2\n");
  d.Write("stream.seeds", "not a seed file\n");
  auto s = ScanSongs({ d.p.u8string() });
  CHECK_EQ(s.size(), (size_t)3);
  int seeds = 0;
  for (const auto& e : s) if (IsSeedPath(e.path)) ++seeds;
  CHECK_EQ(seeds, 2);
  for (const auto& e : s)
    if (e.title == "stream_s152637_v1") CHECK_EQ(e.description, std::string("Dreamride seed: the automaton species, seed 152637"));
  CHECK(!IsSeedPath("song.txt"));
  CHECK(!IsSeedPath("a.seed.txt"));
  CHECK(IsSeedPath("C:\\songs\\x.Seed"));
}
TEST(scan_keeps_folder_order_and_lists_a_folder_given_twice_once) {
  TempDir a("scanA"), b("scanB");
  a.Write("zz.txt", "# from a\n");
  b.Write("aa.txt", "# from b\n");
  auto s = ScanSongs({ a.p.u8string(), b.p.u8string(), a.p.u8string() });
  CHECK_EQ(s.size(), (size_t)2);
  if (s.size() == 2) { CHECK_EQ(s[0].title, std::string("zz")); CHECK_EQ(s[1].title, std::string("aa")); }
}
TEST(scan_reports_a_missing_folder_and_still_loads_the_rest) {
  TempDir a("scanMissing");
  a.Write("one.txt", "# one\n");
  std::vector<std::string> errs;
  auto s = ScanSongs({ (a.p / "does_not_exist").u8string(), a.p.u8string() }, &errs);
  CHECK_EQ(s.size(), (size_t)1);
  CHECK_EQ(errs.size(), (size_t)1);
  if (!errs.empty()) CHECK(errs[0].find("does_not_exist") != std::string::npos);
}
TEST(scan_of_an_empty_folder_and_of_no_folders_is_empty) {
  TempDir a("scanEmpty");
  std::vector<std::string> errs;
  CHECK(ScanSongs({ a.p.u8string() }, &errs).empty());
  CHECK(errs.empty());
  CHECK(ScanSongs({}, &errs).empty());
}
TEST(scan_reads_a_long_first_line_and_non_ascii_names) {
  TempDir a("scanLong");
  std::string longLine = "# " + std::string(300, 'x');
  a.Write("long.txt", longLine + "\n");
  std::ofstream(a.p / fs::u8path(u8"café.txt")) << "# accent\n";
  auto s = ScanSongs({ a.p.u8string() });
  CHECK_EQ(s.size(), (size_t)2);
  if (s.size() == 2) {
    CHECK_EQ(s[0].title, std::string(u8"café"));
    CHECK_EQ(s[1].description.size(), (size_t)300);
  }
}
TEST(find_song_matches_paths_regardless_of_case_and_separators) {
  TempDir a("find");
  a.Write("Song.txt", "# s\n");
  auto s = ScanSongs({ a.p.u8string() });
  std::string p = (a.p / "song.txt").u8string();
  std::replace(p.begin(), p.end(), '\\', '/');
  CHECK_EQ(FindSong(s, p), 0);
  CHECK_EQ(FindSong(s, (a.p / "other.txt").u8string()), -1);
  CHECK_EQ(FindSong({}, p), -1);
}
// ---------------------------------------------------------------- the library refreshing itself

static int IndexOfTitle(const std::vector<SongEntry>& s, const char* title) {
  for (size_t i = 0; i < s.size(); ++i) if (s[i].title == title) return (int)i;
  return -1;
}

TEST(rescan_marks_an_added_song_new_and_keeps_the_others_as_they_were) {
  TempDir d("rescanAdd");
  d.Write("b.txt", "# bee\n");
  auto before = ScanSongs({ d.p.u8string() });
  d.Write("a.txt", "# ay\n");
  Rescan r = MergeRescan(before, ScanSongs({ d.p.u8string() }));
  CHECK(r.changed);
  CHECK_EQ(r.songs.size(), (size_t)2);
  int a = IndexOfTitle(r.songs, "a"), b = IndexOfTitle(r.songs, "b");
  CHECK(a >= 0 && r.songs[a].isNew);
  CHECK(b >= 0 && !r.songs[b].isNew);
  if (a >= 0) CHECK_EQ(r.songs[a].description, std::string("ay"));
}
TEST(rescan_marks_a_rewritten_song_new) {
  TempDir d("rescanEdit");
  d.Write("s.txt", "# one\nbpm 1\n");
  auto before = ScanSongs({ d.p.u8string() });
  d.Write("s.txt", "# two, longer\nbpm 1\n");
  Rescan r = MergeRescan(before, ScanSongs({ d.p.u8string() }));
  CHECK(r.changed);
  CHECK_EQ(r.songs.size(), (size_t)1);
  if (r.songs.size() == 1) { CHECK(r.songs[0].isNew); CHECK_EQ(r.songs[0].description, std::string("two, longer")); }
}
TEST(rescan_marks_a_song_saved_at_the_same_size_new_by_its_time) {
  TempDir d("rescanTime");
  d.Write("s.txt", "# aaaa\n");
  auto before = ScanSongs({ d.p.u8string() });
  d.Write("s.txt", "# bbbb\n");
  fs::last_write_time(d.p / "s.txt", fs::last_write_time(d.p / "s.txt") + std::chrono::seconds(5));
  Rescan r = MergeRescan(before, ScanSongs({ d.p.u8string() }));
  CHECK(r.changed);
  if (r.songs.size() == 1) CHECK(r.songs[0].isNew);
}
TEST(rescan_drops_a_deleted_song) {
  TempDir d("rescanDel");
  d.Write("a.txt", "# a\n");
  d.Write("b.txt", "# b\n");
  auto before = ScanSongs({ d.p.u8string() });
  fs::remove(d.p / "a.txt");
  Rescan r = MergeRescan(before, ScanSongs({ d.p.u8string() }));
  CHECK(r.changed);
  CHECK_EQ(r.songs.size(), (size_t)1);
  if (r.songs.size() == 1) { CHECK_EQ(r.songs[0].title, std::string("b")); CHECK(!r.songs[0].isNew); }
}
TEST(rescan_with_nothing_changed_reports_no_change_and_keeps_new_marks) {
  TempDir d("rescanSame");
  d.Write("a.txt", "# a\n");
  auto before = ScanSongs({ d.p.u8string() });
  before[0].isNew = true;  // marked by an earlier rescan, not yet played
  d.Write("render.wav", "RIFF");  // a non-song file changing must not count
  Rescan r = MergeRescan(before, ScanSongs({ d.p.u8string() }));
  CHECK(!r.changed);
  if (r.songs.size() == 1) CHECK(r.songs[0].isNew);
}
TEST(rescan_of_empty_lists) {
  CHECK(!MergeRescan({}, {}).changed);
  TempDir d("rescanEmpty");
  d.Write("a.txt", "# a\n");
  auto s = ScanSongs({ d.p.u8string() });
  Rescan r = MergeRescan(s, {});
  CHECK(r.changed);
  CHECK(r.songs.empty());
  Rescan r2 = MergeRescan({}, s);
  CHECK(r2.changed);
  if (r2.songs.size() == 1) CHECK(r2.songs[0].isNew);
}
TEST(remap_index_follows_a_song_to_its_new_place) {
  TempDir d("remap");
  d.Write("b.txt", "# b\n");
  d.Write("c.txt", "# c\n");
  auto before = ScanSongs({ d.p.u8string() });
  d.Write("a.txt", "# a\n");
  auto after = MergeRescan(before, ScanSongs({ d.p.u8string() })).songs;
  CHECK_EQ(RemapIndex(before, 0, after), 1);   // b moved down one
  CHECK_EQ(RemapIndex(before, 1, after), 2);   // c too
  CHECK_EQ(RemapIndex(before, -1, after), -1); // none stays none
  CHECK_EQ(RemapIndex(before, -2, after), -2);
  CHECK_EQ(RemapIndex(before, 7, after), -1);  // out of range: none
  fs::remove(d.p / "c.txt");
  auto gone = ScanSongs({ d.p.u8string() });
  CHECK_EQ(RemapIndex(before, 1, gone), -1);   // deleted: no longer in the library
}
TEST(rescan_clock_waits_for_quiet_after_a_change) {
  RescanClock c;
  CHECK(!c.Due(0.0));
  c.Signal(1.0);
  CHECK(!c.Due(1.1));
  c.Signal(1.2);                       // an editor saving in several writes: wait for the last
  CHECK(!c.Due(1.2 + kRescanQuiet - 0.01));
  CHECK(c.Due(1.2 + kRescanQuiet));
  CHECK(!c.Due(5.0));                  // once only
}
TEST(rescan_clock_polls_on_a_period_when_a_folder_cannot_be_watched) {
  RescanClock c;
  c.poll = true;
  CHECK(!c.Due(0.5));
  CHECK(c.Due(kRescanPoll));
  CHECK(!c.Due(kRescanPoll + 0.1));
  CHECK(c.Due(2 * kRescanPoll + 0.01));
}
TEST(folder_watch_sees_a_song_written_into_a_watched_folder) {
  TempDir d("watch");
  FolderWatch w({ d.p.u8string() });
  CHECK(!w.Unwatched());
  CHECK(!w.Changed());
  d.Write("fresh.txt", "# fresh\n");
  bool seen = false;
  for (int i = 0; i < 200 && !seen; ++i) { seen = w.Changed(); if (!seen) Sleep(10); }
  CHECK(seen);
}
TEST(folder_watch_reports_a_folder_it_cannot_watch) {
  TempDir d("watchMissing");
  FolderWatch w({ (d.p / "nope").u8string() });
  CHECK(w.Unwatched());
  CHECK(!w.Changed());
}

TEST(the_repository_examples_all_have_descriptions) {
  // world/tests -> examples: every example in the repository says what it is on its first line.
  fs::path ex = fs::path(__FILE__).parent_path() / ".." / ".." / "examples";
  std::vector<std::string> errs;
  auto s = ScanSongs({ ex.u8string() }, &errs);
  CHECK(errs.empty());
  CHECK(s.size() >= 9);
  for (auto& e : s) CHECK(!e.description.empty());
}

// ---------------------------------------------------------------- shortcuts

TEST(key_codes_are_the_windows_virtual_keys) {
  CHECK_EQ((int)kKeyTab, VK_TAB); CHECK_EQ((int)kKeyEscape, VK_ESCAPE); CHECK_EQ((int)kKeySpace, VK_SPACE);
  CHECK_EQ((int)kKeyLeft, VK_LEFT); CHECK_EQ((int)kKeyRight, VK_RIGHT); CHECK_EQ((int)kKeyUp, VK_UP);
  CHECK_EQ((int)kKeyDown, VK_DOWN); CHECK_EQ((int)kKeyF11, VK_F11);
}
TEST(every_documented_key_maps_to_its_action) {
  CHECK(KeyToAction(VK_SPACE) == Action::PlayPause);
  CHECK(KeyToAction('R') == Action::Restart);
  CHECK(KeyToAction(VK_LEFT) == Action::SeekBack);
  CHECK(KeyToAction(VK_RIGHT) == Action::SeekForward);
  CHECK(KeyToAction(VK_UP) == Action::VolumeUp);
  CHECK(KeyToAction(VK_DOWN) == Action::VolumeDown);
  CHECK(KeyToAction('N') == Action::NextSong);
  CHECK(KeyToAction('P') == Action::PrevSong);
  CHECK(KeyToAction('L') == Action::ToggleLoop);
  CHECK(KeyToAction(VK_TAB) == Action::ToggleMenu);
  CHECK(KeyToAction('M') == Action::ToggleMenu);
  CHECK(KeyToAction('H') == Action::ToggleHelp);
  CHECK(KeyToAction(VK_F11) == Action::Fullscreen);
  CHECK(KeyToAction(VK_ESCAPE) == Action::Escape);
}
TEST(unmapped_keys_do_nothing) {
  CHECK(KeyToAction('Q') == Action::None);
  CHECK(KeyToAction(VK_RETURN) == Action::None);
  CHECK(KeyToAction(0) == Action::None);
  CHECK(KeyToAction(-1) == Action::None);
}
TEST(held_keys_repeat_only_for_seek_and_volume) {
  CHECK(KeyToAction(VK_LEFT, true) == Action::SeekBack);
  CHECK(KeyToAction(VK_RIGHT, true) == Action::SeekForward);
  CHECK(KeyToAction(VK_UP, true) == Action::VolumeUp);
  CHECK(KeyToAction(VK_DOWN, true) == Action::VolumeDown);
  CHECK(KeyToAction(VK_SPACE, true) == Action::None);
  CHECK(KeyToAction(VK_TAB, true) == Action::None);
  CHECK(KeyToAction(VK_ESCAPE, true) == Action::None);
  CHECK(KeyToAction(VK_F11, true) == Action::None);
}
TEST(no_key_is_bound_twice) {
  std::set<int> keys;
  for (auto& s : Shortcuts()) { CHECK(!keys.count(s.key)); keys.insert(s.key); }
}
TEST(the_help_card_covers_every_shortcut) {
  for (auto& s : Shortcuts()) {
    bool found = false;
    for (auto& r : HelpRows()) for (Action a : r.actions) if (a == s.action) found = true;
    CHECK(found);
  }
}

// ---------------------------------------------------------------- seek, volume, index, time

TEST(seek_is_clamped_into_the_song) {
  CHECK_EQ(ClampSeek(12.0, 60.0), 12.0);
  CHECK_EQ(ClampSeek(-3.0, 60.0), 0.0);
  CHECK_EQ(ClampSeek(75.0, 60.0), 60.0 - kSeekEndMargin);
  CHECK_EQ(ClampSeek(60.0, 60.0), 60.0 - kSeekEndMargin);
}
TEST(seek_with_a_bad_duration_or_position_goes_to_the_start) {
  CHECK_EQ(ClampSeek(5.0, 0.0), 0.0);
  CHECK_EQ(ClampSeek(5.0, -1.0), 0.0);
  CHECK_EQ(ClampSeek(5.0, NAN), 0.0);
  CHECK_EQ(ClampSeek(NAN, 60.0), 0.0);
  CHECK_EQ(ClampSeek(INFINITY, 60.0), 0.0);
  CHECK_EQ(ClampSeek(0.01, 0.02), 0.0);  // shorter than the end margin
}
TEST(volume_steps_by_tenths_within_zero_and_one) {
  CHECK_EQ(StepVolume(0.5f, +1), 0.6f);
  CHECK_EQ(StepVolume(0.5f, -1), 0.4f);
  CHECK_EQ(StepVolume(1.0f, +1), 1.0f);
  CHECK_EQ(StepVolume(0.0f, -1), 0.0f);
  float v = 0.0f;
  for (int i = 0; i < 7; ++i) v = StepVolume(v, +1);
  CHECK_EQ(v, 0.7f);
  CHECK_EQ(StepVolume(NAN, -1), 0.9f);
}
TEST(song_index_wraps_and_handles_empty_and_none) {
  CHECK_EQ(StepIndex(0, 3, +1), 1);
  CHECK_EQ(StepIndex(2, 3, +1), 0);
  CHECK_EQ(StepIndex(0, 3, -1), 2);
  CHECK_EQ(StepIndex(-1, 3, +1), 0);
  CHECK_EQ(StepIndex(-1, 3, -1), 2);
  CHECK_EQ(StepIndex(7, 3, +1), 0);
  CHECK_EQ(StepIndex(0, 0, +1), -1);
  CHECK_EQ(StepIndex(0, 1, +1), 0);
}
TEST(time_formats_as_minutes_and_seconds) {
  CHECK_EQ(FormatTime(0), std::string("0:00"));
  CHECK_EQ(FormatTime(65.9), std::string("1:05"));
  CHECK_EQ(FormatTime(3725), std::string("1:02:05"));
  CHECK_EQ(FormatTime(-4), std::string("0:00"));
  CHECK_EQ(FormatTime(NAN), std::string("0:00"));
}

// ---------------------------------------------------------------- Esc, Tab, H

TEST(esc_closes_help_then_menu_then_asks_then_quits) {
  Overlay o; o.menu = true; o.help = true;
  CHECK(OnEscape(o, true) == EscResult::Closed); CHECK(!o.help); CHECK(o.menu);
  CHECK(OnEscape(o, true) == EscResult::Closed); CHECK(!o.menu);
  CHECK(OnEscape(o, true) == EscResult::AskQuit); CHECK(o.confirmQuit);
  CHECK(OnEscape(o, true) == EscResult::Quit);
}
TEST(esc_with_nothing_open_asks_before_quitting) {
  Overlay o;
  CHECK(OnEscape(o, false) == EscResult::AskQuit);
  CHECK(o.confirmQuit);
}
TEST(another_key_answers_stay_to_the_quit_question) {
  Overlay o;
  OnEscape(o, true);
  CHECK(CancelQuit(o)); CHECK(!o.confirmQuit);
  CHECK(!CancelQuit(o));
  CHECK(OnEscape(o, true) == EscResult::AskQuit);  // asks again rather than quitting
}
TEST(esc_counts_a_faded_menu_as_closed) {
  Overlay o; o.menu = true; o.help = true;
  CHECK(OnEscape(o, false) == EscResult::AskQuit);
  CHECK(!o.menu); CHECK(!o.help);
}
TEST(tab_toggles_a_visible_menu_and_wakes_a_faded_one) {
  Overlay o;
  OnMenuKey(o, true); CHECK(o.menu);
  OnMenuKey(o, true); CHECK(!o.menu);
  OnMenuKey(o, false); CHECK(o.menu);
  OnMenuKey(o, false); CHECK(o.menu);  // faded and open: Tab brings it back, it does not close it
}
TEST(h_toggles_a_visible_card_and_wakes_a_faded_one) {
  Overlay o;
  OnHelpKey(o, true); CHECK(o.help);
  OnHelpKey(o, true); CHECK(!o.help);
  o.help = true;
  OnHelpKey(o, false); CHECK(o.help);
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
