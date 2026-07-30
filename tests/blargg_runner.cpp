// KagamiQA P2 — blargg $6000 test ROM runner.
//
// Runs one or more blargg test ROMs headless (console-only), reads the
// $6000 result code after N frames, and prints a machine-parseable result
// line. This is the Oracle B executor — it answers "does FCEUX11 behave
// the same as real hardware for this ROM?"
//
// Blargg $6000 protocol:
//   $6000 = 0x00 → PASS
//   $6000 = 0x01-0xFF → FAIL (code indicates specific failure category)
//   $6001-$6003 = optional diagnostic bytes (printed for FAIL cases)
//
// Usage:
//   fceux11_blargg_runner --rom <path> --frames <N>
//     Run a single ROM. Exit code 0 = PASS, 1 = FAIL.
//
//   fceux11_blargg_runner --manifest <path.json>
//     Run all ROMs listed in the manifest. Prints JSON array of results.
//     Exit code 0 if all ROMs PASS, 1 if any FAIL.
//
// Links against fceux11_drivers_qt (existing test infrastructure pattern).
// Does NOT create any GUI window — console-only operation.

#include <cstdio>
#include <cstdlib>
#include <cstdint>
#include <cstring>
#include <string>
#include <vector>

#include "types.h"
#include "fceu.h"
#include "driver.h"           // FCEUI_SetInput
#include "x6502.h"
#include "bus.h"              // ARead[]
#include "state.h"
#include "cart.h"
#include "sound.h"
#include "drivers/common/nes_shm.h"
#include "driver_callbacks.h"

// ---------------------------------------------------------------------------
// Minimal test harness (avoids pulling in all of test_helpers.h to keep
// this file self-contained and auditable for Oracle B correctness).
// ---------------------------------------------------------------------------

static int g_init_count = 0;
/// Stage-2 PR E-2: --reset-after N sets g_reset_after_frames = N.
/// After running N frames, the runner calls fceu11::ResetNES() so ROMs
/// that gate their actual test behind a manual reset (apu_reset_*,
/// mmc3_irq*) can complete. Default -1 = disabled (legacy behaviour).
static int g_reset_after_frames = -1;

static bool core_init() {
    if (g_init_count++ == 0) {
        if (!fceu11::Initialize()) {
            std::fprintf(stderr, "blargg_runner: fceu11::Initialize() failed\n");
            g_init_count = 0;
            return false;
        }
        if (!nes_shm) {
            nes_shm = open_nes_shm();
        }
        // No input devices needed for blargg ROMs (they run autonomously).
        FCEUI_SetInput(0,    static_cast<ESI>(SI_NONE),    nullptr, 0);
        FCEUI_SetInput(1,    static_cast<ESI>(SI_NONE),    nullptr, 0);
        FCEUI_SetInputFC(static_cast<ESIFC>(SIFC_NONE),    nullptr, 0);
        FCEUI_SetInputFourscore(false);

        // P4-bridge: Enable new PPU before any LoadGame.
        // Setting newppu=1 early lets FCEU_ResetVidSys() compute
        // normalscanlines=241 and disable overclocking correctly.
        newppu = 1;
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
        std::fprintf(stderr, "blargg_runner: failed to load ROM '%s'\n", path);
    }
    return gi;
}

static void emulate_n(int n) {
    uint8* xbuf = nullptr;
    int32* sbuf = nullptr;
    int   sbuf_size = 0;
    for (int i = 0; i < n; ++i) {
        fceu11::Emulate(&xbuf, &sbuf, &sbuf_size, 0);
    }
}

// ---------------------------------------------------------------------------
// $6000 result reader. Uses ARead[] — the canonical read-dispatch table.
// Returns the 4-byte probe tuple: {$6000, $6001, $6002, $6003}.
// ---------------------------------------------------------------------------
struct ProbeResult {
    uint8_t code;           // $6000: 0x00=PASS, other=FAIL
    uint8_t diag[3];        // $6001-$6003: diagnostic detail
};

static ProbeResult read_probe() {
    ProbeResult r;
    r.code    = ARead[0x6000](0x6000);
    r.diag[0] = ARead[0x6001](0x6001);
    r.diag[1] = ARead[0x6002](0x6002);
    r.diag[2] = ARead[0x6003](0x6003);
    return r;
}

/// Read a NUL-terminated ASCII diagnostic string starting at `addr`.
/// Blargg instr_test ROMs write error detail strings at $6004+.
/// Returns empty string if the first byte is 0x00 or non-printable.
static std::string read_probe_string(uint16_t addr, size_t max_len = 512) {
    std::string s;
    s.reserve(max_len);
    for (size_t i = 0; i < max_len; ++i) {
        uint8_t b = ARead[addr + static_cast<uint16_t>(i)](addr + static_cast<uint16_t>(i));
        if (b == 0x00 || b == 0xFF) break;
        if (b < 0x20 && b != 0x0A && b != 0x0D) break;  // non-printable (allow LF/CR)
        s.push_back(static_cast<char>(b));
    }
    return s;
}

// ---------------------------------------------------------------------------
// Single ROM runner
// ---------------------------------------------------------------------------
struct SingleResult {
    std::string rom_name;
    uint16_t    probe_addr;
    uint8_t     value;
    uint8_t     diag[3];
    bool        passed;
    int64_t     duration_ms;
    std::string diag_string;   // $6004+ ASCII diagnostic (blargg error detail)
};

static SingleResult run_one_rom(const char* rom_path, int frames) {
    SingleResult res;
    res.probe_addr = 0x6000;

    // Extract ROM name from path for reporting.
    const char* base = std::strrchr(rom_path, '/');
    if (!base) base = std::strrchr(rom_path, '\\');
    res.rom_name = base ? (base + 1) : rom_path;

    auto t0 = std::clock();

    if (!core_init()) {
        res.value  = 0xFF;
        res.passed = false;
        res.duration_ms = 0;
        return res;
    }

    FCEUGI* gi = load_rom(rom_path);
    if (!gi) {
        core_shutdown();
        res.value  = 0xFE;  // 0xFE = ROM load failure (distinct from $6000 codes)
        res.passed = false;
        res.duration_ms = 0;
        return res;
    }

    // Stage-2 Phase E / PR E-2 + E-3 root cause: many blargg ROMs (notably
    // apu_reset_* / mmc3_irq*) require a manual soft-reset AFTER power-on
    // to enter their actual test. Without it, the ROM displays "Press
    // RESET" and writes $6000=0x81 forever, which the runner previously
    // mis-reported as FAIL. If --reset-after N is set, run N frames, then
    // ResetNES(), then run the remaining frames.
    if (frames > 0 && g_reset_after_frames >= 0
        && g_reset_after_frames < frames)
    {
        emulate_n(g_reset_after_frames);
        fceu11::ResetNES();
        emulate_n(frames - g_reset_after_frames);
    } else {
        // Run frames. Blargg ROMs are self-checking — they write PASS/FAIL to
        // $6000-$6003 and then loop. The frame count is tuned so the ROM has
        // time to complete its test sequence and write the result.
        emulate_n(frames);
    }

    // Read $6000-$6003. We read AFTER the full frame run because blargg
    // ROMs run their tests during NMI / game loop and the result is
    // stable once written (the ROM loops at end).
    ProbeResult probe = read_probe();

    res.value    = probe.code;
    res.diag[0]  = probe.diag[0];
    res.diag[1]  = probe.diag[1];
    res.diag[2]  = probe.diag[2];
    res.passed   = (probe.code == 0x00);

    // Read diagnostic string from $6004+ if the test failed.
    // Blargg instr_test ROMs write error detail (opcode + expected/actual) here.
    // We sample twice (now, and again 1 frame later) because some ROMs
    // overwrite the text AFTER writing $6000 — without the second sample
    // we'd see only a partial / empty detail. PR E-2.
    if (!res.passed) {
        res.diag_string = read_probe_string(0x6004);
        if (res.diag_string.empty()) {
            emulate_n(1);
            res.diag_string = read_probe_string(0x6004);
        }
    }

    fceu11::CloseGame();
    core_shutdown();

    res.duration_ms = (std::clock() - t0) * 1000 / CLOCKS_PER_SEC;

    return res;
}

// ---------------------------------------------------------------------------
// JSON manifest reader (minimal — avoids a JSON library dependency).
// Reads the blargg_manifest.json format:
//   { "roms": [ { "name":..., "path":..., "frames":..., "probe_addr":... }, ... ] }
// ---------------------------------------------------------------------------

#include <fstream>
#include <sstream>

struct ManifestEntry {
    std::string name;
    std::string path;
    int         frames;
    uint32_t    probe_addr;
    std::string description;
};

// Crude JSON string value extractor. Returns the string value for `"key"`.
// Handles escaped quotes and backslashes minimally. NOT a general-purpose
// parser — tuned for the blargg_manifest.json format produced by the
// KagamiQA build system.
static std::string json_extract_string(const std::string& json, const std::string& key) {
    std::string search = "\"" + key + "\"";
    size_t pos = json.find(search);
    if (pos == std::string::npos) return "";

    // Find the colon separating key from value.
    size_t colon = json.find(':', pos + search.size());
    if (colon == std::string::npos) return "";
    // Find the opening quote of the string value.
    size_t val_start = json.find('"', colon + 1);
    if (val_start == std::string::npos) return "";
    size_t val_end = json.find('"', val_start + 1);
    if (val_end == std::string::npos) return "";

    return json.substr(val_start + 1, val_end - val_start - 1);
}

static int json_extract_int(const std::string& json, const std::string& key) {
    std::string search = "\"" + key + "\"";
    size_t pos = json.find(search);
    if (pos == std::string::npos) return 0;

    pos = json.find(':', pos + search.size());
    if (pos == std::string::npos) return 0;

    // Skip whitespace.
    pos++;
    while (pos < json.size() && (json[pos] == ' ' || json[pos] == '\t' || json[pos] == '\n')) {
        pos++;
    }

    // Parse integer.
    int val = 0;
    while (pos < json.size() && json[pos] >= '0' && json[pos] <= '9') {
        val = val * 10 + (json[pos] - '0');
        pos++;
    }
    return val;
}

static std::vector<ManifestEntry> load_manifest(const char* manifest_path) {
    std::vector<ManifestEntry> entries;

    std::ifstream f(manifest_path);
    if (!f.is_open()) {
        std::fprintf(stderr, "blargg_runner: cannot open manifest '%s'\n", manifest_path);
        return entries;
    }

    std::stringstream buf;
    buf << f.rdbuf();
    std::string json = buf.str();

    // Find the "roms" array and iterate through objects.
    size_t roms_start = json.find("\"roms\"");
    if (roms_start == std::string::npos) {
        std::fprintf(stderr, "blargg_runner: no 'roms' key in manifest\n");
        return entries;
    }

    size_t array_start = json.find('[', roms_start);
    if (array_start == std::string::npos) return entries;

    // Manually find each object between { and } in the array.
    size_t pos = array_start + 1;
    while (pos < json.size()) {
        size_t obj_start = json.find('{', pos);
        if (obj_start == std::string::npos) break;
        size_t obj_end = json.find('}', obj_start);
        if (obj_end == std::string::npos) break;

        std::string obj = json.substr(obj_start, obj_end - obj_start + 1);

        ManifestEntry e;
        e.name        = json_extract_string(obj, "name");
        e.path        = json_extract_string(obj, "path");
        e.frames      = json_extract_int(obj, "frames");
        e.probe_addr  = (uint32_t)json_extract_int(obj, "probe_addr");
        e.description = json_extract_string(obj, "description");

        if (!e.name.empty() && !e.path.empty()) {
            entries.push_back(e);
        }

        pos = obj_end + 1;
    }

    return entries;
}

// ---------------------------------------------------------------------------
// Print helpers
// ---------------------------------------------------------------------------

// Single-ROM output line (machine-parseable).
static void print_single_result(const SingleResult& r) {
    std::printf("BLARGG_RESULT: rom=%s addr=0x%04X value=0x%02X diag=[0x%02X,0x%02X,0x%02X] status=%s duration_ms=%lld",
        r.rom_name.c_str(),
        r.probe_addr,
        r.value,
        r.diag[0], r.diag[1], r.diag[2],
        r.passed ? "PASS" : "FAIL",
        static_cast<long long>(r.duration_ms));
    if (!r.diag_string.empty()) {
        std::printf(" diag_string=\"%s\"", r.diag_string.c_str());
    }
    std::printf("\n");
}

// Batch JSON output.
static void print_batch_json(const std::vector<SingleResult>& results) {
    std::printf("{\n  \"runner\": \"kagami-qa-blargg-runner\",\n");
    std::printf("  \"protocol\": \"$6000\",\n");
    std::printf("  \"results\": [\n");
    for (size_t i = 0; i < results.size(); ++i) {
        const auto& r = results[i];
        const char* comma = (i + 1 < results.size()) ? "," : "";
        std::printf(
            "    {\"rom\":\"%s\",\"addr\":\"0x%04X\",\"value\":\"0x%02X\","
            "\"diag\":[%d,%d,%d],\"status\":\"%s\",\"duration_ms\":%lld",
            r.rom_name.c_str(),
            r.probe_addr,
            r.value,
            r.diag[0], r.diag[1], r.diag[2],
            r.passed ? "PASS" : "FAIL",
            static_cast<long long>(r.duration_ms));
        if (!r.diag_string.empty()) {
            // Escape JSON special characters in diag_string.
            std::printf(",\"diag_string\":\"");
            for (char c : r.diag_string) {
                if (c == '"') std::printf("\\\"");
                else if (c == '\\') std::printf("\\\\");
                else if (c == '\n') std::printf("\\n");
                else if (c == '\r') std::printf("\\r");
                else if (c == '\t') std::printf("\\t");
                else std::putchar(c);
            }
            std::printf("\"");
        }
        std::printf("}%s\n", comma);
    }
    std::printf("  ]\n}\n");
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------
int main(int argc, char** argv) {
    std::setvbuf(stdout, nullptr, _IONBF, 0);

    // Parse arguments.
    const char* rom_path      = nullptr;
    int         frames        = 300;  // default: generous for most blargg ROMs
    const char* manifest_path = nullptr;

    for (int i = 1; i < argc; ++i) {
        if (std::strcmp(argv[i], "--rom") == 0 && i + 1 < argc) {
            rom_path = argv[++i];
        } else if (std::strcmp(argv[i], "--frames") == 0 && i + 1 < argc) {
            frames = std::atoi(argv[++i]);
        } else if (std::strcmp(argv[i], "--manifest") == 0 && i + 1 < argc) {
            manifest_path = argv[++i];
        } else if (std::strcmp(argv[i], "--reset-after") == 0 && i + 1 < argc) {
            // Stage-2 PR E-2: press RESET after N frames, then continue.
            g_reset_after_frames = std::atoi(argv[++i]);
        } else if (argv[i][0] != '-') {
            // Positional: first arg = rom_path, second = frames (for backward compat).
            if (!rom_path) {
                rom_path = argv[i];
            } else if (frames == 300) {
                frames = std::atoi(argv[i]);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Batch mode: --manifest <path>
    // -----------------------------------------------------------------------
    if (manifest_path) {
        auto entries = load_manifest(manifest_path);
        if (entries.empty()) {
            std::fprintf(stderr, "blargg_runner: no valid entries in manifest\n");
            return 1;
        }

        std::vector<SingleResult> results;
        results.reserve(entries.size());

        int fail_count = 0;
        for (const auto& e : entries) {
            std::fprintf(stderr, "  [%s] %d frames...", e.name.c_str(), e.frames);
            auto r = run_one_rom(e.path.c_str(), e.frames);
            results.push_back(r);
            std::fprintf(stderr, " %s (0x%02X) %lldms\n",
                r.passed ? "PASS" : "FAIL",
                r.value,
                static_cast<long long>(r.duration_ms));
            if (!r.passed) fail_count++;
        }

        print_batch_json(results);

        int total = (int)results.size();
        std::fprintf(stderr, "\n=== Blargg Suite Summary ===\n");
        std::fprintf(stderr, "Total:  %d\n", total);
        std::fprintf(stderr, "Passed: %d\n", total - fail_count);
        std::fprintf(stderr, "Failed: %d\n", fail_count);
        return fail_count > 0 ? 1 : 0;
    }

    // -----------------------------------------------------------------------
    // Single ROM mode: --rom <path> [--frames N]
    // -----------------------------------------------------------------------
    if (!rom_path) {
        std::fprintf(stderr,
            "Usage: fceux11_blargg_runner --rom <path> [--frames N] [--reset-after N]\n"
            "       fceux11_blargg_runner --manifest <path.json>\n"
            "       fceux11_blargg_runner <rom_path> [frames]\n"
            "\n"
            "--reset-after N: press RESET after N frames, then continue.\n"
            "                 Required for apu_reset_* / mmc3_irq_* ROMs that\n"
            "                 wait for a manual soft-reset before their test runs.\n");
        return 1;
    }

    auto r = run_one_rom(rom_path, frames);
    print_single_result(r);
    return r.passed ? 0 : 1;
}
