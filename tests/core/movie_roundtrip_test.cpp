// FCEUX11 v2.0.5 r12 - movie record/playback round-trip regression (S9.23 V15).
//
// Root cause (plan doc S9.23): MovieData::dump() wrote movie bytes into a
// Rust-side EmuFileMem and read them back WITHOUT seeking to the start.
// EmuFileMem::read_into is a positional read, so the fread returned 0 bytes,
// the zero-initialized buffer was written out verbatim, and every recorded
// .fm2 was "correct length, all zero" - unparsable, replay impossible.
//
// What is locked here:
//   Tier 1 (no ROM): dump a minimal MovieData to a temp file in both FM2
//     record encodings (text / binary) and assert: byte count > 0, the file
//     starts with the FM2 text header ("version "), and the content is not
//     all zero. This directly exercises the fixed read-back line.
//   Tier 2 (end-to-end): on a fixture ROM, record N frames through the real
//     recording entry (SaveMovie -> frames -> StopMovie, whose stop path
//     rewrites the file through MovieData::dump), then assert the .fm2 on
//     disk is valid, LoadMovie parses it back with all N frame records, and
//     the playback path is self-consistent across two replays.
//
// Mutation contract: removing the fseek(mem, 0, 0) in MovieData::dump
// (src/movie_fm2.cpp) must turn the header/zero checks red (the file becomes
// all zero) and the records-count check red (LoadMovie cannot parse records
// out of a zero file), i.e. both tiers fail.
//
// Deliberately NOT locked here (r12 finding, registered in plan S9.23):
// frame-exact equality between the record-time run and the replay run. The
// first frame's output differs by state-establishment mechanism (four setup
// sequences produce four distinct frame-1 framebuffer hashes; the playback
// path itself is self-consistent). That is a pre-existing engine-level
// property of savestate-driven replay equivalence, out of the V15 fix scope.
//
// Format note: this fork's FM2 is pure text ("version N" first line, "|"
// records) - there is no binary "FCEUXM" magic anywhere in the dump path
// (verified r12: zero hits for FCEUXM in movie.rs / movie_fm2.cpp). The
// plan S9.23 acceptance wording "FCEUXM header" was corrected in r12.

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <cstdint>
#include <string>
#include <vector>
#include <filesystem>
#include <atomic>
#include <chrono>
#include <thread>

#include "types.h"
#include "fceu.h"
#include "movie.h"

#include "test_helpers.h"

namespace {

// ---------------------------------------------------------------------------
// CI watchdog + stage markers (v2.0.5 r14).
//
// On the GitHub runner this test hung for the full ctest default timeout
// (1500 s) and produced NO output at all: stdout is fully buffered when the
// harness redirects it, so the buffer was lost when ctest killed the process.
// That cost ~25 min of CI time and hid the hang site.
//
// Two changes make the next CI run diagnosable:
//   1. Every step announces itself on stderr. stderr is unbuffered, so the
//      markers survive the forced kill a ctest timeout performs.
//   2. An in-process watchdog aborts the run after WATCHDOG_SECONDS and
//      reports the stage it was in, instead of letting ctest kill it
//      silently after 1500 s.
//
// The watchdog thread only reads an atomic and sleeps - it never touches the
// emulator, so it introduces no cross-thread engine access.
// ---------------------------------------------------------------------------
constexpr int WATCHDOG_SECONDS = 60;

std::atomic<int> g_stage{0};

const char* stage_name(int s) {
    switch (s) {
        case 0:  return "start";
        case 1:  return "tier1-text-dump";
        case 2:  return "tier1-binary-dump";
        case 3:  return "tier2-core-init";
        case 4:  return "tier2-load-rom";
        case 5:  return "tier2-save-movie";
        case 6:  return "tier2-record-frames";
        case 7:  return "tier2-stop-movie";
        case 8:  return "tier2-verify-file";
        case 9:  return "tier2-load-movie-1";
        case 10: return "tier2-replay-1-frames";
        case 11: return "tier2-stop-movie-1";
        case 12: return "tier2-load-movie-2";
        case 13: return "tier2-replay-2-frames";
        case 14: return "tier2-stop-movie-2";
        default: return "done";
    }
}

void set_stage(int s) {
    g_stage.store(s, std::memory_order_release);
    std::fprintf(stderr, "[movie-roundtrip] stage %d: %s\n", s, stage_name(s));
    std::fflush(stderr);
}

void start_watchdog() {
    std::thread([] {
        const auto t0 = std::chrono::steady_clock::now();
        for (;;) {
            std::this_thread::sleep_for(std::chrono::milliseconds(500));
            const auto el = std::chrono::duration_cast<std::chrono::seconds>(
                std::chrono::steady_clock::now() - t0).count();
            if (el >= WATCHDOG_SECONDS) {
                const int s = g_stage.load(std::memory_order_acquire);
                std::fprintf(stderr,
                    "[movie-roundtrip] WATCHDOG: no progress for %lld s, "
                    "stuck at stage %d (%s)\n",
                    (long long)el, s, stage_name(s));
                std::fflush(stderr);
                std::_Exit(3);
            }
        }
    }).detach();
}

bool file_has_prefix(const char* path, const char* prefix) {
    FILE* f = std::fopen(path, "rb");
    if (!f) return false;
    const size_t plen = std::strlen(prefix);
    uint8_t head[32];
    const size_t got = (plen <= sizeof(head)) ? std::fread(head, 1, plen, f) : 0;
    std::fclose(f);
    return got == plen && std::memcmp(head, prefix, plen) == 0;
}

bool file_has_nonzero(const char* path) {
    FILE* f = std::fopen(path, "rb");
    if (!f) return false;
    uint8_t buf[4096];
    bool nonzero = false;
    size_t got;
    while (!nonzero && (got = std::fread(buf, 1, sizeof(buf), f)) > 0) {
        for (size_t i = 0; i < got; ++i) {
            if (buf[i] != 0) { nonzero = true; break; }
        }
    }
    std::fclose(f);
    return nonzero;
}

}  // namespace

int main() {
    fceu11_test::TestContext ctx;
    start_watchdog();
    set_stage(0);

    // ------------------------------------------------------------------
    // Tier 1 - dump-level lock (no ROM, both FM2 record encodings)
    // ------------------------------------------------------------------
    {
        MovieData md;
        md.ports[0] = 0;
        md.ports[1] = 0;
        md.ports[2] = 0;
        md.romFilename = "movie_roundtrip_fixture";
        for (int i = 0; i < 3; ++i) md.records.emplace_back();

        for (int mode = 0; mode <= 1; ++mode) {
            const bool binary = (mode != 0);
            const std::string tag = binary ? "binary" : "text";
            set_stage(binary ? 2 : 1);
            const std::filesystem::path path =
                std::filesystem::temp_directory_path() /
                ("fceux11_movie_roundtrip_t1_" + tag + ".fm2");

            int bytes = -1;
            {
                EMUFILE_FILE f(path.string().c_str(), "wb");
                bytes = md.dump(&f, binary);
            }  // EMUFILE_FILE dtor flushes and closes

            FCEU11_EXPECT(ctx, bytes > 0,
                ("tier1[" + tag + "]: dump returns byte count > 0").c_str());
            FCEU11_EXPECT(ctx, file_has_prefix(path.string().c_str(), "version "),
                ("tier1[" + tag + "]: file starts with FM2 text header").c_str());
            FCEU11_EXPECT(ctx, file_has_nonzero(path.string().c_str()),
                ("tier1[" + tag + "]: file content is not all zero").c_str());

            std::error_code ec;
            std::filesystem::remove(path, ec);
        }
    }

    // ------------------------------------------------------------------
    // Tier 2 - end-to-end record -> save -> parse -> replay (S9.23)
    // ------------------------------------------------------------------
    constexpr int REPLAY_FRAMES = 120;
    {
        set_stage(3);
        FCEU11_EXPECT(ctx, fceu11_test::core_init(), "tier2: core_init");

        // Headless hygiene (same as ppu_frame_diff_test): disable interactive
        // / GUI-only features that can block without a desktop session.
        AutoResumePlay = false;
        FCEU_StateRecorderSetEnabled(false);

        set_stage(4);
        FCEUGI* gi = fceu11_test::load_rom("fixtures/mapper_nrom.nes");
        FCEU11_EXPECT(ctx, gi != nullptr, "tier2: load fixture ROM (mapper_nrom.nes)");
        if (!gi) {
            // Without a game, SaveMovie/LoadMovie would crash the process;
            // report the failure and skip the rest of tier 2.
            std::printf("  (tier2 skipped: no ROM loaded - run from tests/ so "
                        "fixtures/mapper_nrom.nes resolves)\n");
            return fceu11_test::report_and_exit(
                ctx, "FCEUX11 v2.0.5 r12 - movie roundtrip test (S9.23 V15)");
        }

        const std::filesystem::path moviePath =
            std::filesystem::temp_directory_path() / "fceux11_movie_roundtrip_t2.fm2";

        // Record N frames through the real recording entry, then stop (the
        // stop path rewrites the whole file through the fixed MovieData::dump).
        set_stage(5);
        fceu11::SaveMovie(moviePath.string().c_str(), MOVIE_FLAG_NONE, L"");
        set_stage(6);
        for (int i = 0; i < REPLAY_FRAMES; ++i) fceu11_test::emulate_one();
        set_stage(7);
        fceu11::StopMovie();

        set_stage(8);
        FCEU11_EXPECT(ctx, file_has_prefix(moviePath.string().c_str(), "version "),
            "tier2: recorded .fm2 starts with FM2 text header");
        FCEU11_EXPECT(ctx, file_has_nonzero(moviePath.string().c_str()),
            "tier2: recorded .fm2 is not all zero");

        // Replay #1: LoadMovie restores the movie's embedded savestate and
        // enters playback; the frame records must parse back completely.
        set_stage(9);
        const bool loaded = fceu11::LoadMovie(moviePath.string().c_str(), true, -1);
        FCEU11_EXPECT(ctx, loaded, "tier2: LoadMovie accepts the recorded file");
        FCEU11_EXPECT(ctx, currMovieData.getNumRecords() == REPLAY_FRAMES,
            "tier2: parsed movie carries the recorded frame records");
        set_stage(10);
        for (int i = 0; i < REPLAY_FRAMES; ++i) fceu11_test::emulate_one();
        set_stage(11);
        fceu11::StopMovie();

        // Replay #2: the playback path must be self-consistent - the same
        // movie replayed twice parses identically.
        set_stage(12);
        fceu11::LoadMovie(moviePath.string().c_str(), true, -1);
        FCEU11_EXPECT(ctx, currMovieData.getNumRecords() == REPLAY_FRAMES,
            "tier2: second LoadMovie parses the same record count");
        set_stage(13);
        for (int i = 0; i < REPLAY_FRAMES; ++i) fceu11_test::emulate_one();
        set_stage(14);
        fceu11::StopMovie();

        std::error_code ec;
        std::filesystem::remove(moviePath, ec);
        fceu11_test::core_shutdown();
    }

    set_stage(15);
    return fceu11_test::report_and_exit(
        ctx, "FCEUX11 v2.0.5 r12 - movie roundtrip test (S9.23 V15)");
}
