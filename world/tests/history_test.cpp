// Tests for world/src/history.h: the stream history's file format, replay from the recorded text, deleting and
// undoing, the list's order and stars, and write failures. No window, no GPU, no engine. Built and run by
// world\build.bat test. (bin\luminesce-world.exe --history-test checks the same against the real engine.)
#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>

#include <cstdio>

#include "../src/history.h"

namespace fs = std::filesystem;
using namespace hist;

static int g_failed = 0, g_checks = 0;
static const char* g_test = "";
#define CHECK(cond) do { ++g_checks; if (!(cond)) { ++g_failed; printf("  FAIL %s: %s (line %d)\n", g_test, #cond, __LINE__); } } while (0)
#define CHECK_EQ(a, b) do { ++g_checks; if (!((a) == (b))) { ++g_failed; printf("  FAIL %s: %s == %s (line %d)\n", g_test, #a, #b, __LINE__); } } while (0)

struct Test { const char* name; std::function<void()> fn; };
static std::vector<Test>& Tests() { static std::vector<Test> t; return t; }
struct Reg { Reg(const char* n, std::function<void()> f) { Tests().push_back({ n, f }); } };
#define TEST(name) static void name(); static Reg reg_##name(#name, name); static void name()

struct TempDir {
  fs::path p;
  explicit TempDir(const char* tag) {
    p = fs::temp_directory_path() / (std::string("dreamride_hist_") + tag + "_" + std::to_string(GetCurrentProcessId()));
    fs::remove_all(p);
    fs::create_directories(p);
  }
  ~TempDir() { std::error_code ec; fs::remove_all(p, ec); }
  std::string s() const { return p.u8string(); }
};

// Two stages as the composer writes them (Equation::song_text): settings, beats, definitions, then the voices.
static const char* kStage1 =
  "# intro: the bass and the hats\n"
  "bpm 101\n"
  "reverb_size 0.7\n"
  "beats 32\n"
  "\n"
  "tri(v) = abs(v - floor(v + 0.5))\n"
  "c = mod(floor(3332 / 10^(3 - s)), 10)\n"
  "h(d) = floor((12d + c) / 7)\n"
  "n = floor(4t)\n"
  "s = floor(t / 16)\n"
  "\n"
  "voice bass {\n"
  "  wave = square\n"
  "  pitch = h(n) - 24\n"
  "  gate = 0.5 - frac(4t)\n"
  "  gain = 0.4\n"
  "}\n"
  "\n"
  "drum hats {\n"
  "  kind = hat\n"
  "  trigger = frac(2t)\n"
  "}\n";
static const char* kStage2 =
  "# build: the lead comes in; a new progression, 1 6 2 5\n"
  "bpm 101\n"
  "reverb_size 0.8\n"
  "beats 32\n"
  "\n"
  "tri(v) = abs(v - floor(v + 0.5))\n"
  "c = mod(floor(1625 / 10^(3 - s)), 10)\n"
  "h(d) = floor((12d + c) / 7)\n"
  "n = floor(4t)\n"
  "s = floor(t / 16)\n"
  "\n"
  "voice bass {\n"
  "  wave = square\n"
  "  pitch = h(n) - 24\n"
  "  gate = 0.5 - frac(4t)\n"
  "  gain = 0.5\n"
  "}\n"
  "\n"
  "voice lead {\n"
  "  wave = sine\n"
  "  pitch = h(n + tri(t)) + 12\n"
  "  gate = 1\n"
  "  amp = 0.8\n"
  "}\n";

static Record Sample() {
  Record r;
  r.started = "2026-11-03 14:05:12";
  r.startedUnix = 1793801112;
  r.seed = 955576;
  r.flags = kFlagGrammar;
  r.origin = "grammar seed 955576";
  r.composer = "luminesce.dll fnv1a64 0123456789abcdef 1268736";
  r.bpm = 101;
  r.stageBeats = 32;
  r.roster = { "bass", "hats", "lead" };
  r.stages.push_back({ 0, false, "intro: the bass and the hats", kStage1 });
  r.stages.push_back({ 1, true, "build: the lead comes in; a new progression, 1 6 2 5", kStage2 });
  r.heardStages = 2;
  r.heardSeconds = 21.5;
  return r;
}

static bool Same(const Record& a, const Record& b) {
  if (a.started != b.started || a.startedUnix != b.startedUnix || a.seed != b.seed || a.flags != b.flags || a.origin != b.origin ||
      a.composer != b.composer || a.bpm != b.bpm || a.stageBeats != b.stageBeats || a.roster != b.roster ||
      NormalizeText(a.seedSong) != b.seedSong || a.starred != b.starred || a.heardStages != b.heardStages ||
      std::fabs(a.heardSeconds - b.heardSeconds) > 1e-3 || a.complete != b.complete || a.completeStages != b.completeStages ||
      std::fabs(a.completeSeconds - b.completeSeconds) > 1e-3 || a.stages.size() != b.stages.size())
    return false;
  for (size_t i = 0; i < a.stages.size(); ++i)
    if (a.stages[i].index != b.stages[i].index || a.stages[i].turn != b.stages[i].turn || a.stages[i].summary != b.stages[i].summary ||
        NormalizeText(a.stages[i].text) != b.stages[i].text)
      return false;
  return true;
}

// ---------------------------------------------------------------- the format

TEST(a_record_round_trips_through_its_file_text) {
  Record r = Sample();
  r.seedSong = "# a seed song\r\nbpm 90\nbeats 16";  // CRLF and no final newline: kept as '\n' lines ending in '\n'
  r.flags = 0;
  r.starred = true;
  r.complete = true; r.completeStages = 2; r.completeSeconds = 38.0;
  std::string text = Serialize(r), err;
  Record back;
  CHECK(Parse(text, back, err));
  CHECK(err.empty());
  CHECK(Same(r, back));
  CHECK_EQ(Serialize(back), text);
}

TEST(a_recording_written_in_appended_pieces_reads_as_the_whole_record) {
  Record r = Sample();
  Record head = r; head.stages.clear(); head.heardStages = 0; head.heardSeconds = 0;
  std::string appended = HeaderText(head) + StageText(r.stages[0]) + HeardText(1, 9.0) + StageText(r.stages[1]) +
                         StarText(true) + StarText(false) + HeardText(2, 21.5);
  Record back; std::string err;
  CHECK(Parse(appended, back, err));
  CHECK(Same(r, back));  // the last heard and star lines count
}

TEST(a_stage_text_with_hash_lines_and_blank_lines_is_kept_byte_for_byte) {
  Record r = Sample();
  r.stages[0].text = "# heard\n\n#\nstage 9 turn 0 lines 1\nsummary not a header\nbpm 101\n";  // lines that look like keys
  Record back; std::string err;
  CHECK(Parse(Serialize(r), back, err));
  if (back.stages.size() == 2) CHECK_EQ(back.stages[0].text, r.stages[0].text);
}

TEST(parse_refuses_what_is_not_a_history_file_or_is_cut_short) {
  Record back; std::string err;
  CHECK(!Parse("", back, err));
  CHECK(!Parse("bpm 101\nbeats 32\n", back, err));
  CHECK(err.find("not a Dreamride history") != std::string::npos);
  std::string text = Serialize(Sample());
  CHECK(!Parse(text.substr(0, text.size() - 120), back, err));  // a stage cut short
  CHECK(!Parse(std::string(kMagic) + "\nseed 1\n", back, err));  // no start time
  CHECK(!Parse(text + "bogus 1\n", back, err));
  CHECK(err.find("unknown") != std::string::npos);
}

TEST(a_record_with_no_stages_round_trips) {
  Record r = Sample();
  r.stages.clear(); r.roster.clear(); r.heardStages = 0; r.heardSeconds = 0;
  Record back; std::string err;
  CHECK(Parse(Serialize(r), back, err));
  CHECK(Same(r, back));
}

// ---------------------------------------------------------------- replay from the recorded text

TEST(a_stage_read_back_from_disk_is_the_recorded_stage_text) {
  TempDir d("readback");
  Session s; s.dir = d.s(); s.rec = Sample();
  CHECK_EQ(Sync(s, ""), std::string());
  Record back; std::string err;
  CHECK(LoadRecord(s.rec.path, back, err));
  if (back.stages.size() == 2) {
    CHECK_EQ(back.stages[0].text, std::string(kStage1));
    CHECK_EQ(back.stages[1].text, std::string(kStage2));
  }
}

TEST(replaying_one_recorded_stage_plays_its_own_functions) {
  Record r = Sample();
  std::string err;
  std::string song = MergeStages(r, 0, 0, "Replayed", err);
  CHECK(err.empty());
  StageSong a, b;
  CHECK(ParseStageSong(kStage1, a, err));
  CHECK(ParseStageSong(song, b, err));
  // The same settings, and each definition the same with its stage's suffix and its own time T_0 = t - 0.
  CHECK(a.settings == b.settings);
  CHECK(song.find("beats 32\n") != std::string::npos);
  CHECK(song.find("T_0 = t - 0\n") != std::string::npos);
  CHECK(song.find("Q = floor(t / 32)\n") != std::string::npos);
  CHECK(song.find("tri(v) = abs(v - floor(v + 0.5))\n") != std::string::npos);  // free of t: shared, its own name
  CHECK(song.find("n_0 = floor(4T_0)\n") != std::string::npos);                 // 4t stays a product
  CHECK(song.find("h_0(d) = floor((12d + c_0) / 7)\n") != std::string::npos);   // its parameter kept
  const SongBlock* bass = b.voice("bass");
  CHECK(bass != nullptr);
  if (bass) {
    CHECK_EQ(*bass->get("pitch"), std::string("h_0(n_0) - 24"));
    CHECK_EQ(*bass->get("gate"), std::string("0.5 - frac(4T_0)"));
    CHECK_EQ(*bass->get("wave"), std::string("square"));
  }
  CHECK(b.voice("lead") == nullptr);  // a roster voice no stage has is left out
}

TEST(replaying_several_stages_picks_each_stage_by_q_and_takes_the_last_settings) {
  Record r = Sample();
  std::string err;
  std::string song = MergeStages(r, 0, 1, "Replayed", err);
  CHECK(err.empty());
  StageSong m;
  CHECK(ParseStageSong(song, m, err));
  CHECK(song.find("beats 64\n") != std::string::npos);
  CHECK(song.find("T_1 = t - 32\n") != std::string::npos);
  CHECK(song.find("c_1 = mod(floor(1625 / 10^(3 - s_1)), 10)\n") != std::string::npos);
  CHECK(std::find(m.settings.begin(), m.settings.end(), "reverb_size 0.8") != m.settings.end());
  const SongBlock* lead = m.voice("lead");
  CHECK(lead);
  if (lead) {
    CHECK_EQ(*lead->get("gate"), std::string("if(Q == 0, 0, 1)"));       // silent where it was not
    CHECK_EQ(*lead->get("amp"), std::string("if(Q == 0, 0, 0.8)"));
    CHECK_EQ(*lead->get("pitch"), std::string("if(Q == 0, 0, h_1(n_1 + tri(T_1)) + 12)"));
  }
  CHECK(song.find("# Stage 2: build: the lead comes in; a new progression, 1 6 2 5.\n") != std::string::npos);
  CHECK(MergeStages(r, 1, 2, "x", err).empty());  // out of range
  CHECK(!err.empty());
}

TEST(a_voice_kept_across_stages_with_one_instrument_is_one_voice) {
  Record r = Sample();
  std::string two = kStage1;
  two.replace(two.find("3332"), 4, "1625");  // a new progression: the same instruments, other functions
  r.stages[1].text = two;
  std::string err;
  StageSong m;
  CHECK(ParseStageSong(MergeStages(r, 0, 1, "Replayed", err), m, err));
  const SongBlock* bass = m.voice("bass");
  CHECK(bass != nullptr);
  CHECK(m.voices.size() == 2);  // bass and hats, no second bass
  if (bass) CHECK_EQ(*bass->get("pitch"), std::string("if(Q == 0, h_0(n_0) - 24, h_1(n_1) - 24)"));
}

// The stream (src/stream.rs) releases a voice whose instrument changes at a seam and strikes the new one fresh: the
// replay does the same, as two voices, each with its own settings.
TEST(a_voice_whose_instrument_changes_is_released_and_a_new_one_struck) {
  Record r = Sample();  // the bass's gain is 0.4 in stage 1, 0.5 in stage 2
  std::string err;
  StageSong m;
  CHECK(ParseStageSong(MergeStages(r, 0, 1, "Replayed", err), m, err));
  const SongBlock* a = m.voice("bass");
  const SongBlock* b = m.voice("bass_2");
  CHECK(a && b);
  if (a && b) {
    CHECK_EQ(*a->get("gain"), std::string("0.4"));
    CHECK_EQ(*b->get("gain"), std::string("0.5"));
    CHECK_EQ(*a->get("gate"), std::string("if(Q == 0, 0.5 - frac(4T_0), 0)"));            // released at the seam
    CHECK_EQ(*a->get("pitch"), std::string("if(Q == 0, h_0(n_0) - 24, h_0(n_0) - 24)"));  // ringing on its own function
    CHECK_EQ(*b->get("gate"), std::string("if(Q == 0, 0, 0.5 - frac(4T_1))"));
    CHECK_EQ(*b->get("pitch"), std::string("if(Q == 0, 0, h_1(n_1) - 24)"));
  }
}

// A voice that leaves is released with its own functions, as the stream releases it: its pitch goes on from where it
// was, not to 0.
TEST(a_voice_that_leaves_rings_out_with_its_own_functions) {
  Record r = Sample();
  std::swap(r.stages[0].text, r.stages[1].text);  // the lead plays in the first stage and leaves
  std::string err;
  StageSong m;
  CHECK(ParseStageSong(MergeStages(r, 0, 1, "Replayed", err), m, err));
  const SongBlock* lead = m.voice("lead");
  const SongBlock* hats = m.voice("hats");
  CHECK(lead && hats);
  if (lead) {
    CHECK_EQ(*lead->get("gate"), std::string("if(Q == 0, 1, 0)"));
    CHECK_EQ(*lead->get("pitch"), std::string("if(Q == 0, h_0(n_0 + tri(T_0)) + 12, h_0(n_0 + tri(T_0)) + 12)"));
    CHECK_EQ(*lead->get("amp"), std::string("if(Q == 0, 0.8, 0.8)"));
  }
  if (hats) CHECK_EQ(*hats->get("trigger"), std::string("if(Q == 0, 0, frac(2T_1))"));  // not struck before it came
}

// The move book names a function T (the blancmange curve, T(u)): renamed per stage it would be T_0, the merge's own
// time name. The time names step aside, as Q does.
TEST(a_stage_defining_t_keeps_its_function_and_the_times_take_another_name) {
  Record r = Sample();
  std::string two = kStage2;
  two.replace(two.find("tri(v) = abs"), 0, "T(u) = tri(u) + 0.5 tri(2u)\n");
  two.replace(two.find("h(n + tri(t))"), 13, "h(n + T(t))");
  r.stages[1].text = two;
  std::string err;
  std::string song = MergeStages(r, 0, 1, "Replayed", err);
  CHECK(err.empty());
  StageSong m;
  CHECK(ParseStageSong(song, m, err));
  CHECK(song.find("T_1(u) = tri(u) + 0.5 tri(2u)\n") != std::string::npos);
  CHECK(song.find("T_0 = t") == std::string::npos);
  CHECK(song.find("T2_1 = t - 32\n") != std::string::npos);
  const SongBlock* lead = m.voice("lead");
  CHECK(lead != nullptr);
  if (lead) CHECK_EQ(*lead->get("pitch"), std::string("if(Q == 0, 0, h_1(n_1 + T_1(T2_1)) + 12)"));
  std::vector<std::string> names;
  for (auto& d : m.defs) names.push_back(d.name);
  std::sort(names.begin(), names.end());
  CHECK(std::adjacent_find(names.begin(), names.end()) == names.end());  // no name defined twice
}

TEST(rename_leaves_numbers_parameters_and_longer_names_alone) {
  std::map<std::string, std::string> m{ { "c", "c_0" }, { "s", "s_0" } };
  CHECK_EQ(Rename("cs + c + s2 + 1.5t + t_x", m, "T_0", {}), std::string("cs + c_0 + s2 + 1.5T_0 + t_x"));
  CHECK_EQ(Rename("c * s", m, "T_0", { "s" }), std::string("c_0 * s"));
}

// ---------------------------------------------------------------- the list

TEST(the_list_puts_stars_first_then_the_newest) {
  std::vector<Summary> v(4);
  v[0].path = "a"; v[0].startedUnix = 100;
  v[1].path = "b"; v[1].startedUnix = 300;
  v[2].path = "c"; v[2].startedUnix = 200; v[2].starred = true;
  v[3].path = "d"; v[3].startedUnix = 50; v[3].starred = true;
  Sort(v);
  CHECK_EQ(v[0].path, std::string("c"));
  CHECK_EQ(v[1].path, std::string("d"));
  CHECK_EQ(v[2].path, std::string("b"));
  CHECK_EQ(v[3].path, std::string("a"));
}

TEST(starring_an_entry_on_disk_moves_it_to_the_top_of_a_rescan) {
  TempDir d("star");
  Session older; older.dir = d.s(); older.rec = Sample(); older.rec.startedUnix = 10; older.rec.seed = 1;
  Session newer; newer.dir = d.s(); newer.rec = Sample(); newer.rec.startedUnix = 20; newer.rec.seed = 2;
  CHECK_EQ(Sync(older, ""), std::string());
  CHECK_EQ(Sync(newer, ""), std::string());
  Scan s = ScanDir(d.s());
  CHECK_EQ(s.entries.size(), (size_t)2);
  if (s.entries.size() == 2) CHECK_EQ(s.entries[0].seed, (uint64_t)2);
  CHECK_EQ(WriteText(fs::u8path(older.rec.path), StarText(true), true), std::string());
  s = ScanDir(d.s());
  if (s.entries.size() == 2) { CHECK_EQ(s.entries[0].seed, (uint64_t)1); CHECK(s.entries[0].starred); CHECK(!s.entries[1].starred); }
  CHECK_EQ(WriteText(fs::u8path(older.rec.path), StarText(false), true), std::string());
  s = ScanDir(d.s());
  if (s.entries.size() == 2) CHECK_EQ(s.entries[0].seed, (uint64_t)2);
}

TEST(a_scan_skips_damaged_files_with_an_error_and_counts_the_deleted) {
  TempDir d("scan");
  Session a; a.dir = d.s(); a.rec = Sample();
  CHECK_EQ(Sync(a, ""), std::string());
  CHECK_EQ(WriteText(d.p / "notes.txt", "just notes\n", false), std::string());
  fs::create_directories(d.p / "deleted");
  CHECK_EQ(WriteText(d.p / "deleted" / "old.txt", "x", false), std::string());
  Scan s = ScanDir(d.s());
  CHECK_EQ(s.entries.size(), (size_t)1);
  CHECK_EQ(s.errors.size(), (size_t)1);
  CHECK_EQ(s.deleted, (size_t)1);
  if (!s.entries.empty()) { CHECK_EQ(s.entries[0].stages, (uint64_t)2); CHECK_EQ(s.entries[0].heardStages, (uint64_t)2); }
  CHECK(ScanDir((d.p / "missing").u8string()).entries.empty());
}

TEST(the_note_names_the_stages_in_a_few_words) {
  std::vector<Stage> st(3);
  st[0].summary = "intro: the bass and the hats";
  st[1].summary = "build: the kick comes in; a new progression, 1 6 2 5 1 6 2 5.";
  st[2].summary = "breakdown: the lead leaves";
  CHECK_EQ(StageNote(st), std::string("intro, kick in, new progression, lead out"));
  std::string cut = StageNote(st, 20);
  CHECK(cut.size() <= 20 + 3);
  CHECK(cut.find(u8"…") != std::string::npos);
  CHECK_EQ(StageNote({}), std::string());
}

TEST(a_new_recording_is_named_by_its_date_and_seed_and_never_overwrites) {
  TempDir d("names");
  Session a; a.dir = d.s(); a.rec = Sample();
  Session b; b.dir = d.s(); b.rec = Sample();
  CHECK_EQ(Sync(a, ""), std::string());
  CHECK_EQ(Sync(b, ""), std::string());
  CHECK_EQ(fs::u8path(a.rec.path).filename().u8string(), std::string("2026-11-03_955576.txt"));
  CHECK_EQ(fs::u8path(b.rec.path).filename().u8string(), std::string("2026-11-03_955576_2.txt"));
}

// ---------------------------------------------------------------- deleting and undoing

TEST(delete_moves_the_file_into_deleted_and_undo_puts_it_back) {
  TempDir d("delete");
  Session a; a.dir = d.s(); a.rec = Sample();
  CHECK_EQ(Sync(a, ""), std::string());
  std::string rerr;
  std::string original = ReadFile(fs::u8path(a.rec.path), rerr);
  std::string moved;
  CHECK_EQ(MoveToDeleted(a.rec.path, moved), std::string());
  CHECK(!fs::exists(fs::u8path(a.rec.path)));
  CHECK(fs::exists(fs::u8path(moved)));
  CHECK_EQ(fs::u8path(moved).parent_path().filename().u8string(), std::string("deleted"));
  CHECK(ScanDir(d.s()).entries.empty());
  CHECK_EQ(ScanDir(d.s()).deleted, (size_t)1);
  std::string restored, e;
  CHECK_EQ(Restore(moved, a.rec.path, restored), std::string());
  CHECK_EQ(restored, a.rec.path);
  CHECK_EQ(ReadFile(fs::u8path(restored), e), original);
  CHECK_EQ(ScanDir(d.s()).entries.size(), (size_t)1);
  CHECK_EQ(ScanDir(d.s()).deleted, (size_t)0);
}

TEST(deleting_twice_the_same_name_keeps_both_and_undo_never_overwrites) {
  TempDir d("delete2");
  Session a; a.dir = d.s(); a.rec = Sample();
  CHECK_EQ(Sync(a, ""), std::string());
  std::string first, second, restored;
  CHECK_EQ(MoveToDeleted(a.rec.path, first), std::string());
  Session b; b.dir = d.s(); b.rec = Sample();  // the same date and seed again
  CHECK_EQ(Sync(b, ""), std::string());
  CHECK_EQ(b.rec.path, a.rec.path);
  CHECK_EQ(MoveToDeleted(b.rec.path, second), std::string());
  CHECK(first != second);
  CHECK(fs::exists(fs::u8path(first)) && fs::exists(fs::u8path(second)));
  Session c; c.dir = d.s(); c.rec = Sample();  // the name taken again before the undo
  CHECK_EQ(Sync(c, ""), std::string());
  CHECK_EQ(Restore(first, a.rec.path, restored), std::string());
  CHECK(restored != c.rec.path);
  CHECK(fs::exists(fs::u8path(c.rec.path)) && fs::exists(fs::u8path(restored)));
  CHECK(MoveToDeleted((d.p / "nope.txt").u8string(), first) != std::string());
}

TEST(empty_deleted_removes_only_the_deleted_files) {
  TempDir d("empty");
  Session a; a.dir = d.s(); a.rec = Sample();
  Session b; b.dir = d.s(); b.rec = Sample(); b.rec.seed = 7;
  CHECK_EQ(Sync(a, ""), std::string());
  CHECK_EQ(Sync(b, ""), std::string());
  std::string moved;
  CHECK_EQ(MoveToDeleted(a.rec.path, moved), std::string());
  size_t removed = 9;
  CHECK_EQ(EmptyDeleted(d.s(), removed), std::string());
  CHECK_EQ(removed, (size_t)1);
  CHECK(!fs::exists(fs::u8path(moved)));
  CHECK(fs::exists(fs::u8path(b.rec.path)));
  CHECK_EQ(EmptyDeleted(d.s(), removed), std::string());  // nothing there: fine
  CHECK_EQ(removed, (size_t)0);
}

// ---------------------------------------------------------------- writing, and failing to

TEST(a_recording_appends_each_stage_and_reads_back_whole) {
  TempDir d("append");
  Session s; s.dir = d.s(); s.rec = Sample();
  Stage second = s.rec.stages[1];
  s.rec.stages.pop_back(); s.rec.heardStages = 0; s.rec.heardSeconds = 0;
  CHECK_EQ(Sync(s, ""), std::string());
  s.rec.stages.push_back(second);
  CHECK_EQ(Sync(s, StageText(second)), std::string());
  s.rec.heardStages = 2; s.rec.heardSeconds = 21.5;
  CHECK_EQ(Sync(s, HeardText(2, 21.5)), std::string());
  Record back; std::string err;
  CHECK(LoadRecord(s.rec.path, back, err));
  CHECK(Same(Sample(), back));
}

TEST(a_write_failure_is_reported_and_the_next_write_recovers_everything) {
  TempDir d("fail");
  fs::path blocker = d.p / "history";
  CHECK_EQ(WriteText(blocker, "a file where the folder should be", false), std::string());
  Session s; s.dir = blocker.u8string(); s.rec = Sample();
  std::string e = Sync(s, "");
  CHECK(!e.empty());
  CHECK(e.find("cannot create the folder") != std::string::npos);
  CHECK(!s.written);
  // Through the worker, the error reaches the main thread.
  Worker w;
  auto sess = std::make_shared<Session>(s);
  w.Post([sess] { Worker::Done d; d.error = Sync(*sess, StageText(sess->rec.stages[0])); d.sessionWrite = true; return d; });
  w.Flush();
  auto done = w.Take();
  CHECK_EQ(done.size(), (size_t)1);
  if (!done.empty()) CHECK(!done[0].error.empty());
  // The way cleared, the next write puts the whole record down.
  fs::remove(blocker);
  w.Post([sess] { Worker::Done d; d.error = Sync(*sess, HeardText(2, 21.5)); return d; });
  w.Flush();
  done = w.Take();
  if (!done.empty()) CHECK(done[0].error.empty());
  Record back; std::string err;
  CHECK(LoadRecord(sess->rec.path, back, err));
  CHECK(Same(Sample(), back));
}

TEST(writing_into_a_missing_drive_or_a_read_only_file_says_why) {
  CHECK(!WriteText(fs::u8path("Q:\\no\\such\\place\\x.txt"), "x", false).empty());
  TempDir d("ro");
  fs::path f = d.p / "ro.txt";
  CHECK_EQ(WriteText(f, "x", false), std::string());
  SetFileAttributesW(f.c_str(), FILE_ATTRIBUTE_READONLY);
  std::string e = WriteText(f, "y", true);
  CHECK(!e.empty());
  CHECK(e.find("ro.txt") != std::string::npos);
  SetFileAttributesW(f.c_str(), FILE_ATTRIBUTE_NORMAL);
}

TEST(the_composer_version_is_a_hash_of_the_engine_file) {
  TempDir d("ver");
  std::string err;
  CHECK_EQ(WriteText(d.p / "luminesce.dll", "abc", false), std::string());
  std::string v1 = ComposerVersionOf(d.p / "luminesce.dll", err);
  CHECK(err.empty());
  CHECK_EQ(v1, std::string("luminesce.dll fnv1a64 e71fa2190541574b 3"));  // FNV-1a 64 of "abc"
  CHECK_EQ(WriteText(d.p / "luminesce.dll", "abd", false), std::string());
  CHECK(ComposerVersionOf(d.p / "luminesce.dll", err) != v1);
  CHECK(ComposerVersionOf(d.p / "missing.dll", err).empty());
  CHECK(!err.empty());
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
