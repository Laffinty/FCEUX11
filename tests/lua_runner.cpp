// KagamiQA P3 — Headless Lua script runner.
//
// Runs a single .lua test script inside the fceux11-lua engine, headless
// (null driver, no Qt). The Lua script calls emu.frameadvance() which
// yields the coroutine; we resume it each frame via fceux11_lua_frame_boundary().
//
// Lifecycle:
//   null_driver_init() → fceu11::Initialize() → fceux11_lua_init()
//   → fceux11_lua_load_script(path) → frame loop → fceux11_lua_shutdown()
//   → fceu11::Kill()
//
// Usage:
//   fceux11_lua_runner <script_path> [--frames MAX] [--rom <path>]
//
// Output (stdout):
//   LUA_RESULT: script=<name> status=PASS|FAIL details=<output>
//
// Exit code: 0 = PASS, 1 = FAIL.
//
// P5: Assertion capture. Redirects stderr to a temporary file during
// script execution, then scans the captured output for Lua error/assert
// patterns to determine PASS/FAIL. This replaces the previous "didn't
// crash = PASS" heuristic with actual assertion-level signal extraction.

#include <cstdio>
#include <cstdlib>
#include <cstdint>
#include <cstring>
#include <string>
#include <ctime>
#include <sstream>
#include <fstream>

#include "types.h"
#include "fceu.h"
#include "driver.h"           // FCEUI_SetInput
#include "bus.h"              // ARead[], bus
#include "state.h"
#include "cart.h"
#include "sound.h"
#include "x6502.h"
#include "drivers/common/nes_shm.h"
#include "driver_callbacks.h"
#include "null_driver.h"

#ifdef _WIN32
#include <windows.h>
#endif

// ---------------------------------------------------------------------------
// Rust FFI — fceux11-lua extern "C" bridge (symbols exported from fceux11_rust.lib)
// ---------------------------------------------------------------------------
extern "C" {
    int  fceux11_lua_init();
    int  fceux11_lua_shutdown();
    int  fceux11_lua_load_script(const char* path, const char* arg);
    void fceux11_lua_frame_boundary();
    void fceux11_lua_stop();
    int  fceux11_lua_running();
}

// ---------------------------------------------------------------------------
// Minimal test harness (same pattern as blargg_runner.cpp)
// ---------------------------------------------------------------------------
static int g_init_count = 0;

static bool core_init() {
    if (g_init_count++ == 0) {
        if (!fceu11::Initialize()) {
            std::fprintf(stderr, "lua_runner: fceu11::Initialize() failed\n");
            g_init_count = 0;
            return false;
        }
        if (!nes_shm) {
            nes_shm = open_nes_shm();
        }
        FCEUI_SetInput(0,    static_cast<ESI>(SI_NONE),    nullptr, 0);
        FCEUI_SetInput(1,    static_cast<ESI>(SI_NONE),    nullptr, 0);
        FCEUI_SetInputFC(static_cast<ESIFC>(SIFC_NONE),    nullptr, 0);
        FCEUI_SetInputFourscore(false);
    }
    return true;
}

static void core_shutdown() {
    if (--g_init_count <= 0) {
        g_init_count = 0;
        fceu11::Kill();
    }
}

static FCEUGI* load_rom(const char* path) {
    FCEUGI* gi = fceu11::LoadGame(path, 1, true);
    if (!gi) {
        std::fprintf(stderr, "lua_runner: failed to load ROM '%s'\n", path);
    }
    return gi;
}

// ---------------------------------------------------------------------------
// Lua script result
// ---------------------------------------------------------------------------
struct LuaResult {
    std::string script_name;
    bool        passed;
    int         exit_code;
    std::string details;
    int64_t     duration_ms;
};

// ---------------------------------------------------------------------------
// P5 M2-fix: stdout + stderr capture for Lua assertion detection.
//
// Lua scripts write their test results to stdout (via print()), not stderr.
// Runtime Lua errors (assert/error) go to stderr. We must capture BOTH to
// correctly detect all failure modes:
//   - stdout: script-printed "FAIL: ..." lines (test logic failures)
//   - stderr: Lua runtime errors ("runtime error:", "stack traceback:")
// ---------------------------------------------------------------------------

#include <cstdlib>
#include <cstdio>

static std::string make_temp_path(const char* prefix) {
#ifdef _WIN32
    char tmp_path[MAX_PATH];
    char tmp_file[MAX_PATH];
    if (GetTempPathA(MAX_PATH, tmp_path) == 0) { tmp_path[0] = '.'; tmp_path[1] = '\\'; tmp_path[2] = 0; }
    if (GetTempFileNameA(tmp_path, prefix, 0, tmp_file) == 0) {
        return std::string(tmp_path) + prefix + "_temp.txt";
    }
    return std::string(tmp_file);
#else
    return std::string("/tmp/lua_runner_") + prefix + "_" + std::to_string(std::time(nullptr)) + ".txt";
#endif
}

static LuaResult run_lua_script(const char* script_path, const char* rom_path,
                                 int max_frames) {
    LuaResult res;
    res.passed     = false;
    res.exit_code  = 0;

    // Extract script name from path.
    const char* base = std::strrchr(script_path, '/');
    if (!base) base = std::strrchr(script_path, '\\');
    res.script_name = base ? (base + 1) : script_path;

    // P5: Capture BOTH stdout and stderr to temp files.
    std::string temp_stdout_path = make_temp_path("luaout");
    std::string temp_stderr_path = make_temp_path("luaerr");
    FILE* captured_stdout = std::freopen(temp_stdout_path.c_str(), "w", stdout);
    FILE* captured_stderr = std::freopen(temp_stderr_path.c_str(), "w", stderr);

    auto t0 = std::clock();

    // --- Initialize engine ---
    if (!core_init()) {
        res.exit_code = -1;
        res.details   = "core_init failed";
        res.duration_ms = 0;
        return res;
    }

    // --- Load ROM (optional) ---
    if (rom_path && rom_path[0] != '\0') {
        FCEUGI* gi = load_rom(rom_path);
        if (!gi) {
            core_shutdown();
            res.exit_code = -2;
            res.details   = "ROM load failed";
            res.duration_ms = 0;
            return res;
        }
        // Run one warm-up frame so mapper banking settles.
        uint8* xbuf = nullptr;
        int32* sbuf = nullptr;
        int    sbuf_size = 0;
        fceu11::Emulate(&xbuf, &sbuf, &sbuf_size, 0);
    }

    // --- Initialize Lua engine ---
    if (fceux11_lua_init() != 0) {
        core_shutdown();
        res.exit_code = -3;
        res.details   = "fceux11_lua_init failed";
        res.duration_ms = (std::clock() - t0) * 1000 / CLOCKS_PER_SEC;
        return res;
    }

    // --- Load Lua script ---
    if (fceux11_lua_load_script(script_path, nullptr) != 0) {
        fceux11_lua_shutdown();
        core_shutdown();
        res.exit_code = -4;
        res.details   = "fceux11_lua_load_script failed";
        res.duration_ms = (std::clock() - t0) * 1000 / CLOCKS_PER_SEC;
        return res;
    }

    // --- Frame loop ---
    int frame_count = 0;
    while (fceux11_lua_running()) {
        if (max_frames > 0 && frame_count >= max_frames) {
            std::fprintf(stderr, "lua_runner: timeout after %d frames\n", max_frames);
            fceux11_lua_stop();
            break;
        }

        // Emulate one frame.
        uint8* xbuf = nullptr;
        int32* sbuf = nullptr;
        int    sbuf_size = 0;
        fceu11::Emulate(&xbuf, &sbuf, &sbuf_size, 0);

        // Resume Lua coroutine (handles callbacks + script continuation).
        fceux11_lua_frame_boundary();

        frame_count++;
    }

    // --- Shutdown Lua ---
    fceux11_lua_shutdown();

    // --- Restore stdout/stderr (best-effort) ---
    if (captured_stdout) { std::fclose(captured_stdout); }
    if (captured_stderr) { std::fclose(captured_stderr); }
    std::freopen("CONOUT$", "w", stdout);
    std::freopen("CONOUT$", "w", stderr);

    // --- M2-fix: Read BOTH captured stdout and stderr ---
    std::string captured_stdout_str;
    std::string captured_stderr_str;
    {
        std::ifstream out_file(temp_stdout_path);
        if (out_file.is_open()) {
            std::stringstream ss;
            ss << out_file.rdbuf();
            captured_stdout_str = ss.str();
            out_file.close();
        }
        std::remove(temp_stdout_path.c_str());
    }
    {
        std::ifstream err_file(temp_stderr_path);
        if (err_file.is_open()) {
            std::stringstream ss;
            ss << err_file.rdbuf();
            captured_stderr_str = ss.str();
            err_file.close();
        }
        std::remove(temp_stderr_path.c_str());
    }

    // --- Close game and shutdown core ---
    if (rom_path && rom_path[0] != '\0') {
        fceu11::CloseGame();
    }
    core_shutdown();

    res.duration_ms = (std::clock() - t0) * 1000 / CLOCKS_PER_SEC;

    // --- M2-fix: Parse BOTH stdout and stderr for failure signals ---
    // Script-printed FAIL: / ERROR: lines go to stdout (Lua print()).
    // Lua runtime errors go to stderr.
    // Combine both streams for comprehensive detection.
    std::string combined = captured_stdout_str + "\n" + captured_stderr_str;

    bool has_error = false;
    std::string error_detail;

    // Check stdout for script-printed failure markers.
    //
    // Stage-2 fix: previously this also matched the bare substring "failed",
    // which produced false positives whenever a script's summary line
    // contained that word — e.g. test_bit.lua's "bit library: X passed, Y
    // failed" was treated as an error even when all assertions passed.
    // Removed the bare "failed" check; rely on the more specific FAIL: /
    // FAIL / ERROR: line-level patterns instead. The "failed" summary
    // count is not a useful failure signal at this layer.
    if (captured_stdout_str.find("FAIL:") != std::string::npos ||
        captured_stdout_str.find("FAIL ") != std::string::npos ||
        captured_stdout_str.find("ERROR:") != std::string::npos) {
        has_error = true;
    }

    // Check stderr for Lua runtime errors.
    if (captured_stderr_str.find("runtime error") != std::string::npos ||
        captured_stderr_str.find("stack traceback") != std::string::npos ||
        captured_stderr_str.find("assertion failed") != std::string::npos ||
        captured_stderr_str.find("ERROR:") != std::string::npos ||
        captured_stderr_str.find("PANIC:") != std::string::npos ||
        captured_stderr_str.find("syntax error") != std::string::npos) {
        has_error = true;
    }

    // Count FAIL lines to produce a meaningful detail string.
    int fail_count = 0;
    {
        std::istringstream iss(combined);
        std::string line;
        while (std::getline(iss, line)) {
            if (line.find("FAIL:") != std::string::npos ||
                line.find("FAIL ") != std::string::npos) {
                fail_count++;
                if (error_detail.empty() && line.size() > 6) {
                    error_detail = line;
                }
            }
        }
    }

    if (has_error || fail_count > 0) {
        res.passed  = false;
        if (!error_detail.empty()) {
            res.details = error_detail;
            if (res.details.size() > 200) {
                res.details = res.details.substr(0, 197) + "...";
            }
        } else if (fail_count > 0) {
            res.details = std::to_string(fail_count) + " test failure(s) detected";
        } else {
            res.details = "Lua error/assert detected";
        }
    } else {
        res.passed  = true;
        res.details = "script completed (all assertions passed)";
    }

    // P2 Phase 3 Step 3.2 桶 G3 debug aid (2026-08-05): dump the captured
    // script output (stdout + stderr) to a fixed path so a failing script's
    // exact assertion line is observable even when the parent redirects
    // stdio. Guarded by env FCEUX11_LUA_CAPTURE_DUMP=1 to stay silent in
    // normal runs. The temp files are normally deleted; re-open the path
    // only if the files still exist (they are removed by run_lua_script).
#ifdef _WIN32
    if (std::getenv("FCEUX11_LUA_CAPTURE_DUMP") && std::getenv("FCEUX11_LUA_CAPTURE_DUMP")[0] == '1') {
        FILE* dump = std::fopen("C:\\Temp\\lua_capture_dump.txt", "w");
        if (dump) {
            std::fprintf(dump, "=== script=%s passed=%d ===\n%s\n--- stderr ---\n%s\n",
                res.script_name.c_str(), res.passed ? 1 : 0,
                captured_stdout_str.c_str(), captured_stderr_str.c_str());
            std::fclose(dump);
        }
    }
#else
    if (std::getenv("FCEUX11_LUA_CAPTURE_DUMP") && std::getenv("FCEUX11_LUA_CAPTURE_DUMP")[0] == '1') {
        FILE* dump = std::fopen("/tmp/lua_capture_dump.txt", "w");
        if (dump) {
            std::fprintf(dump, "=== script=%s passed=%d ===\n%s\n--- stderr ---\n%s\n",
                res.script_name.c_str(), res.passed ? 1 : 0,
                captured_stdout_str.c_str(), captured_stderr_str.c_str());
            std::fclose(dump);
        }
    }
#endif

    return res;
}

// ---------------------------------------------------------------------------
// Print result
// ---------------------------------------------------------------------------
static void print_result(const LuaResult& r) {
    // P2 Phase 3 Step 3.2 桶 G3 (2026-08-05): emit LUA_RESULT reliably.
    // The run_lua_script() body `freopen`s both stdout and stderr to temp
    // files for script output capture, then tries to restore them with
    // `freopen("CONOUT$", ...)`. That restore silently fails when the
    // process's stdio is itself redirected (PowerShell `> file 2>&1`,
    // CI runners), leaving the FILE* pointing at the now-deleted temp
    // file — so `printf`/`fprintf` here produce nothing.
    //   Fix: write the result through the OS-level original handles
    // (GetStdHandle) on Windows, which are unaffected by stdio freopen.
    // stdout is also re-attempted for non-Windows / console attach paths.
    char buf[512];
    int n = std::snprintf(buf, sizeof(buf),
        "LUA_RESULT: script=%s status=%s duration_ms=%lld details=%s\n",
        r.script_name.c_str(),
        r.passed ? "PASS" : "FAIL",
        static_cast<long long>(r.duration_ms),
        r.details.c_str());
    if (n < 0) return;
    if (n >= static_cast<int>(sizeof(buf))) n = static_cast<int>(sizeof(buf)) - 1;
#ifdef _WIN32
    HANDLE hOut = GetStdHandle(STD_OUTPUT_HANDLE);
    if (hOut && hOut != INVALID_HANDLE_VALUE) {
        DWORD written = 0;
        WriteFile(hOut, buf, static_cast<DWORD>(n), &written, nullptr);
        return;
    }
#endif
    std::fwrite(buf, 1, static_cast<size_t>(n), stdout);
    std::fflush(stdout);
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------
int main(int argc, char** argv) {
    std::setvbuf(stdout, nullptr, _IONBF, 0);

    const char* script_path = nullptr;
    const char* rom_path    = nullptr;
    int         max_frames  = 300;   // default: generous timeout

    for (int i = 1; i < argc; ++i) {
        if (std::strcmp(argv[i], "--rom") == 0 && i + 1 < argc) {
            rom_path = argv[++i];
        } else if (std::strcmp(argv[i], "--frames") == 0 && i + 1 < argc) {
            max_frames = std::atoi(argv[++i]);
        } else if (argv[i][0] != '-') {
            // First positional arg = script path.
            if (!script_path) {
                script_path = argv[i];
            } else if (!rom_path) {
                rom_path = argv[i];
            }
        }
    }

    if (!script_path) {
        std::fprintf(stderr,
            "Usage: fceux11_lua_runner <script_path> [--rom <path>] [--frames N]\n"
            "  Runs a Lua test script inside the fceux11-lua engine (headless).\n");
        return 1;
    }

    // Initialize null driver (must be before core_init).
    null_driver_init();

    auto result = run_lua_script(script_path, rom_path, max_frames);
    print_result(result);

    return result.passed ? 0 : 1;
}
