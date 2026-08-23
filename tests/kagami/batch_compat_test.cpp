// FCEUX11 v2.0_hotfix1 — Batch ROM Compatibility Tester
//
// Headless tool that loads every .nes file from a directory, runs N frames,
// and classifies each ROM as PASS / FAIL / SKIP with detailed diagnostics.
//
// Usage:
//   batch_compat_test <rom_dir> [--frames N] [--output report.json]
//
// Exit codes:
//   0 = all ROMs passed
//   1 = some ROMs failed
//   2 = fatal error (init failure, etc.)

#include "kagami_bridge.h"

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>
#include <vector>
#include <algorithm>
#include <chrono>
#include <cmath>

#ifdef _WIN32
#include <windows.h>
#else
#include <dirent.h>
#include <sys/stat.h>
#endif

// ---------------------------------------------------------------------------
// ROM entry
// ---------------------------------------------------------------------------
struct RomResult {
    std::string filename;
    std::string path;
    int load_status;       // 0=ok, -2=load_fail, -1=init_fail
    int frames_run;
    int frames_blanked;    // frames where XBuf was all 0x80 (blank)
    uint16_t first_pc;     // CPU PC after power-on
    uint16_t last_pc;      // CPU PC after last frame
    bool pc_in_rom;        // PC in 0x8000-0xFFFF (normal ROM execution)
    bool pc_stuck;         // PC didn't change across frames (potential hang)
    bool video_active;     // at least one frame had non-blank video
    int mapper;
    int prg_size_kb;
    int chr_size_kb;
    double elapsed_ms;
    std::string failure_reason;
};

// ---------------------------------------------------------------------------
// Read iNES header to extract mapper number and sizes
// ---------------------------------------------------------------------------
struct INesInfo {
    int mapper;
    int prg_banks;   // 16KB units
    int chr_banks;   // 8KB units
    int prg_kb;
    int chr_kb;
    bool valid;
    bool ines2;
};

static INesInfo read_ines_header(const std::string& path) {
    INesInfo info = {-1, 0, 0, 0, 0, false, false};
    FILE* f = fopen(path.c_str(), "rb");
    if (!f) return info;

    uint8_t header[16];
    if (fread(header, 1, 16, f) != 16) { fclose(f); return info; }
    fclose(f);

    // Check "NES\x1A" magic
    if (header[0] != 'N' || header[1] != 'E' || header[2] != 'S' || header[3] != 0x1A)
        return info;

    info.prg_banks = header[4];
    info.chr_banks = header[5];
    info.prg_kb = info.prg_banks * 16;
    info.chr_kb = info.chr_banks * 8;
    info.mapper = (header[6] >> 4) | (header[7] & 0xF0);
    info.ines2 = (header[7] & 0x0C) == 0x08;
    info.valid = true;
    return info;
}

// ---------------------------------------------------------------------------
// Scan directory for .nes files
// ---------------------------------------------------------------------------
static std::vector<std::string> scan_rom_dir(const std::string& dir) {
    std::vector<std::string> result;

#ifdef _WIN32
    std::string pattern = dir + "\\*.nes";
    WIN32_FIND_DATAA fd;
    HANDLE hFind = FindFirstFileA(pattern.c_str(), &fd);
    if (hFind == INVALID_HANDLE_VALUE) return result;
    do {
        if (!(fd.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY)) {
            result.push_back(fd.cFileName);
        }
    } while (FindNextFileA(hFind, &fd));
    FindClose(hFind);
#else
    DIR* d = opendir(dir.c_str());
    if (!d) return result;
    struct dirent* ent;
    while ((ent = readdir(d)) != nullptr) {
        std::string name = ent->d_name;
        if (name.size() > 4 && name.substr(name.size() - 4) == ".nes") {
            result.push_back(name);
        }
    }
    closedir(d);
#endif

    // Sort for deterministic ordering
    std::sort(result.begin(), result.end());
    return result;
}

// ---------------------------------------------------------------------------
// JSON escape
// ---------------------------------------------------------------------------
static std::string json_escape(const std::string& s) {
    std::string out;
    out.reserve(s.size() + 8);
    for (char c : s) {
        switch (c) {
            case '"':  out += "\\\""; break;
            case '\\': out += "\\\\"; break;
            case '\n': out += "\\n"; break;
            case '\r': out += "\\r"; break;
            case '\t': out += "\\t"; break;
            default:   out += c; break;
        }
    }
    return out;
}

// ---------------------------------------------------------------------------
// Write JSON report
// ---------------------------------------------------------------------------
static void write_report(const char* path, const std::vector<RomResult>& results,
                         double total_seconds) {
    FILE* f = fopen(path, "w");
    if (!f) {
        fprintf(stderr, "ERROR: cannot open %s for writing\n", path);
        return;
    }

    int pass = 0, fail = 0, skip = 0;
    for (auto& r : results) {
        if (r.load_status == -2) skip++;
        else if (r.failure_reason.empty()) pass++;
        else fail++;
    }

    fprintf(f, "{\n");
    fprintf(f, "  \"version\": \"v2.0_hotfix1\",\n");
    fprintf(f, "  \"test_date\": \"2026-08-24\",\n");
    fprintf(f, "  \"total_roms\": %d,\n", (int)results.size());
    fprintf(f, "  \"pass\": %d,\n", pass);
    fprintf(f, "  \"fail\": %d,\n", fail);
    fprintf(f, "  \"skip\": %d,\n", skip);
    fprintf(f, "  \"pass_rate\": \"%.1f%%\",\n", results.empty() ? 0.0 : 100.0 * pass / results.size());
    fprintf(f, "  \"elapsed_seconds\": %.1f,\n", total_seconds);
    fprintf(f, "  \"results\": [\n");

    for (size_t i = 0; i < results.size(); ++i) {
        const auto& r = results[i];
        fprintf(f, "    {\n");
        fprintf(f, "      \"file\": \"%s\",\n", json_escape(r.filename).c_str());
        fprintf(f, "      \"status\": \"%s\",\n",
                r.load_status == -2 ? "SKIP" :
                r.failure_reason.empty() ? "PASS" : "FAIL");
        fprintf(f, "      \"load_ok\": %s,\n", r.load_status == 0 ? "true" : "false");
        if (r.load_status == 0) {
            fprintf(f, "      \"mapper\": %d,\n", r.mapper);
            fprintf(f, "      \"prg_kb\": %d,\n", r.prg_size_kb);
            fprintf(f, "      \"chr_kb\": %d,\n", r.chr_size_kb);
            fprintf(f, "      \"frames_run\": %d,\n", r.frames_run);
            fprintf(f, "      \"frames_blank\": %d,\n", r.frames_blanked);
            fprintf(f, "      \"video_active\": %s,\n", r.video_active ? "true" : "false");
            fprintf(f, "      \"first_pc\": \"0x%04X\",\n", r.first_pc);
            fprintf(f, "      \"last_pc\": \"0x%04X\",\n", r.last_pc);
            fprintf(f, "      \"pc_in_rom\": %s,\n", r.pc_in_rom ? "true" : "false");
            fprintf(f, "      \"pc_stuck\": %s,\n", r.pc_stuck ? "true" : "false");
            fprintf(f, "      \"elapsed_ms\": %.1f,\n", r.elapsed_ms);
        }
        if (!r.failure_reason.empty()) {
            fprintf(f, "      \"reason\": \"%s\"\n", json_escape(r.failure_reason).c_str());
        } else {
            fprintf(f, "      \"reason\": null\n");
        }
        fprintf(f, "    }%s\n", i + 1 < results.size() ? "," : "");
    }

    fprintf(f, "  ]\n");
    fprintf(f, "}\n");
    fclose(f);
    printf("Report written to: %s\n", path);
}

// ---------------------------------------------------------------------------
// Emulation loop with SEH crash protection (separate function to allow __try)
// Returns: empty string on success, error description on failure
// ---------------------------------------------------------------------------
struct EmuDiag {
    int frames_run;
    int frames_blanked;
    uint16_t first_pc;
    uint16_t last_pc;
    bool pc_in_rom;
    bool video_active;
    int stuck_count;
    int error_code;        // 0=ok, -1=frame_error, -2=crash
    char error_msg[128];   // C-style for SEH compatibility
};

#ifdef _WIN32
// SEH-protected emulation loop. No C++ objects with destructors allowed here.
// Simulates Start button press at frames 10-11 to skip title screens.
static int run_frames_seh(int frames_to_run, EmuDiag* diag) {
    uint16_t prev_pc = 0;
    diag->stuck_count = 0;
    diag->frames_blanked = 0;
    diag->first_pc = 0;
    diag->last_pc = 0;
    diag->pc_in_rom = false;
    diag->video_active = false;
    diag->frames_run = 0;
    diag->error_code = 0;
    diag->error_msg[0] = 0;

    __try {
        for (int f = 0; f < frames_to_run; ++f) {
            // Aggressive input simulation to get past title/menu screens
            // NES buttons: 0x01=A, 0x02=B, 0x04=Select, 0x08=Start
            if (f >= 10 && f <= 15) {
                kagami_bridge_set_joypad(0x08); // Hold Start (frames 10-15)
            } else if (f >= 20 && f <= 22) {
                kagami_bridge_set_joypad(0x01); // Press A (frames 20-22)
            } else if (f >= 30 && f <= 32) {
                kagami_bridge_set_joypad(0x08); // Press Start again (frames 30-32)
            } else if (f >= 40 && f <= 42) {
                kagami_bridge_set_joypad(0x01); // Press A again (frames 40-42)
            } else {
                kagami_bridge_set_joypad(0x00); // No input
            }

            int er = kagami_bridge_emulate_frame();
            if (er != 0) {
                diag->error_code = -1;
                _snprintf_s(diag->error_msg, sizeof(diag->error_msg), _TRUNCATE,
                            "emulate_frame_error_frame_%d", f);
                return -1;
            }

            uint16_t pc = kagami_bridge_get_cpu_pc();
            if (f == 0) {
                diag->first_pc = pc;
                prev_pc = pc;
                diag->pc_in_rom = (pc >= 0x8000);
            }
            diag->last_pc = pc;

            if (f > 0 && pc == prev_pc) {
                diag->stuck_count++;
            }
            prev_pc = pc;

            uint8_t frame_buf[256 * 240];
            if (kagami_bridge_extract_frame_buffer(frame_buf, sizeof(frame_buf)) == 0) {
                int all_blank = 1;
                for (int i = 0; i < 256 * 240; i += 64) {
                    if (frame_buf[i] != 0x80) {
                        all_blank = 0;
                        break;
                    }
                }
                if (all_blank) {
                    diag->frames_blanked++;
                } else {
                    diag->video_active = true;
                }
            }
        }
        diag->frames_run = frames_to_run;
        return 0;
    } __except(EXCEPTION_EXECUTE_HANDLER) {
        diag->error_code = -2;
        _snprintf_s(diag->error_msg, sizeof(diag->error_msg), _TRUNCATE,
                    "crash_seh_0x%08X", GetExceptionCode());
        return -2;
    }
}
#endif

// ---------------------------------------------------------------------------
// Test a single ROM (with crash resilience)
// ---------------------------------------------------------------------------
static RomResult test_rom(const std::string& dir, const std::string& filename, int frames_to_run) {
    RomResult r;
    r.filename = filename;
    r.path = dir + "\\" + filename;
    r.load_status = -1;
    r.frames_run = 0;
    r.frames_blanked = 0;
    r.first_pc = 0;
    r.last_pc = 0;
    r.pc_in_rom = false;
    r.pc_stuck = false;
    r.video_active = false;
    r.mapper = -1;
    r.prg_size_kb = 0;
    r.chr_size_kb = 0;
    r.elapsed_ms = 0;

    // Read iNES header for mapper/size info
    INesInfo ines = read_ines_header(r.path);
    if (ines.valid) {
        r.mapper = ines.mapper;
        r.prg_size_kb = ines.prg_kb;
        r.chr_size_kb = ines.chr_kb;
    }

    auto t0 = std::chrono::steady_clock::now();

    // kagami_bridge_load_rom already closes previous ROM — no full_reset needed.
    int lr = kagami_bridge_load_rom(r.path.c_str());
    if (lr != 0) {
        // Try recovery: kill + init + retry
        kagami_bridge_kill();
        if (kagami_bridge_init() != 0) {
            r.failure_reason = "recovery_init_failed";
            r.elapsed_ms = std::chrono::duration<double, std::milli>(
                std::chrono::steady_clock::now() - t0).count();
            return r;
        }
        lr = kagami_bridge_load_rom(r.path.c_str());
        if (lr != 0) {
            r.load_status = -2;
            r.failure_reason = "load_failed";
            r.elapsed_ms = std::chrono::duration<double, std::milli>(
                std::chrono::steady_clock::now() - t0).count();
            return r;
        }
    }

    r.load_status = 0;

    // Run frames with SEH crash protection
    EmuDiag diag = {};
    diag.error_code = 0;
    diag.error_msg[0] = 0;
#ifdef _WIN32
    int emu_rc = run_frames_seh(frames_to_run, &diag);
    if (emu_rc == -2) {
        // Crash — recover emulator
        r.failure_reason = diag.error_msg;
        kagami_bridge_kill();
        kagami_bridge_init();
    } else if (emu_rc == -1) {
        r.failure_reason = diag.error_msg;
    }
#else
    // Non-Windows: no SEH, just run directly
    diag.frames_run = 0;
    diag.stuck_count = 0;
    diag.frames_blanked = 0;
    diag.first_pc = 0;
    diag.last_pc = 0;
    diag.pc_in_rom = false;
    diag.video_active = false;
    uint16_t prev_pc = 0;
    for (int f = 0; f < frames_to_run; ++f) {
        // Aggressive input simulation
        if (f >= 10 && f <= 15) {
            kagami_bridge_set_joypad(0x08); // Hold Start
        } else if (f >= 20 && f <= 22) {
            kagami_bridge_set_joypad(0x01); // Press A
        } else if (f >= 30 && f <= 32) {
            kagami_bridge_set_joypad(0x08); // Press Start again
        } else if (f >= 40 && f <= 42) {
            kagami_bridge_set_joypad(0x01); // Press A again
        } else {
            kagami_bridge_set_joypad(0x00);
        }
        if (kagami_bridge_emulate_frame() != 0) {
            _snprintf_s(diag.error_msg, sizeof(diag.error_msg), _TRUNCATE,
                        "emulate_frame_error_frame_%d", f);
            diag.error_code = -1;
            break;
        }
        uint16_t pc = kagami_bridge_get_cpu_pc();
        if (f == 0) { diag.first_pc = pc; prev_pc = pc; diag.pc_in_rom = (pc >= 0x8000); }
        diag.last_pc = pc;
        if (f > 0 && pc == prev_pc) diag.stuck_count++;
        prev_pc = pc;
        uint8_t frame_buf[256 * 240];
        if (kagami_bridge_extract_frame_buffer(frame_buf, sizeof(frame_buf)) == 0) {
            bool all_blank = true;
            for (int i = 0; i < 256 * 240; i += 64) {
                if (frame_buf[i] != 0x80) { all_blank = false; break; }
            }
            if (all_blank) diag.frames_blanked++; else diag.video_active = true;
        }
    }
    diag.frames_run = frames_to_run;
    if (diag.error_code != 0) r.failure_reason = diag.error_msg;
#endif

    r.frames_run = diag.frames_run;
    r.frames_blanked = diag.frames_blanked;
    r.first_pc = diag.first_pc;
    r.last_pc = diag.last_pc;
    r.pc_in_rom = diag.pc_in_rom;
    r.video_active = diag.video_active;

    // Classify issues
    if (r.failure_reason.empty()) {
        char hexbuf[16];
        if (diag.stuck_count > (frames_to_run - 10) * 0.8 && frames_to_run > 10) {
            r.pc_stuck = true;
            _snprintf_s(hexbuf, sizeof(hexbuf), _TRUNCATE, "%04X", r.last_pc);
            r.failure_reason = std::string("cpu_stuck_pc=0x") + hexbuf;
        }
        else if (!r.video_active && frames_to_run >= 10) {
            r.failure_reason = "no_video_output";
        }
        else if (!r.pc_in_rom && r.first_pc < 0x4020) {
            _snprintf_s(hexbuf, sizeof(hexbuf), _TRUNCATE, "%04X", r.first_pc);
            r.failure_reason = std::string("pc_not_in_rom_space_first=0x") + hexbuf;
        }
    }

    r.elapsed_ms = std::chrono::duration<double, std::milli>(
        std::chrono::steady_clock::now() - t0).count();

    return r;
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------
int main(int argc, char** argv) {
    setvbuf(stdout, NULL, _IONBF, 0);
    setvbuf(stderr, NULL, _IONBF, 0);

    if (argc < 2) {
        fprintf(stderr, "Usage: %s <rom_dir> [--frames N] [--output report.json]\n", argv[0]);
        return 2;
    }

    std::string rom_dir = argv[1];
    int frames = 60;
    const char* output_path = "compat_report.json";

    for (int i = 2; i < argc; ++i) {
        if (strcmp(argv[i], "--frames") == 0 && i + 1 < argc) {
            frames = atoi(argv[++i]);
        } else if (strcmp(argv[i], "--output") == 0 && i + 1 < argc) {
            output_path = argv[++i];
        }
    }

    printf("=== FCEUX11 Batch ROM Compatibility Test (v2.0_hotfix1) ===\n");
    printf("ROM directory: %s\n", rom_dir.c_str());
    printf("Frames per ROM: %d\n", frames);
    printf("Output: %s\n\n", output_path);

    // Scan for ROMs
    auto rom_files = scan_rom_dir(rom_dir);
    printf("Found %d ROM files\n\n", (int)rom_files.size());

    if (rom_files.empty()) {
        fprintf(stderr, "No .nes files found in %s\n", rom_dir.c_str());
        return 2;
    }

    // Initialize emulator once
    printf("Initializing emulator...\n");
    if (kagami_bridge_init() != 0) {
        fprintf(stderr, "FATAL: kagami_bridge_init() failed\n");
        return 2;
    }
    // P1: Enable new PPU for better NMI/IRQ timing accuracy.
    // The legacy PPU has known timing issues that cause many games
    // to hang in VBlank wait loops. The new PPU passes blargg timing tests.
    kagami_bridge_set_newppu(1);
    printf("Emulator initialized (new PPU enabled).\n\n");

    // Run tests
    auto global_start = std::chrono::steady_clock::now();
    std::vector<RomResult> results;
    results.reserve(rom_files.size());

    int pass_count = 0, fail_count = 0, skip_count = 0;

    for (size_t i = 0; i < rom_files.size(); ++i) {
        RomResult r = test_rom(rom_dir, rom_files[i], frames);
        results.push_back(r);

        const char* status_str;
        if (r.load_status == -2) {
            status_str = "SKIP";
            skip_count++;
        } else if (r.failure_reason.empty()) {
            status_str = "PASS";
            pass_count++;
        } else {
            status_str = "FAIL";
            fail_count++;
        }

        // Progress: print every ROM with pass/fail indicator
        printf("[%4d/%d] %-5s %-50s",
               (int)(i + 1), (int)rom_files.size(), status_str, rom_files[i].c_str());
        if (r.load_status == 0) {
            printf(" mapper=%3d %4dKB %4.0fms", r.mapper, r.prg_size_kb, r.elapsed_ms);
        }
        if (!r.failure_reason.empty()) {
            printf(" [%s]", r.failure_reason.c_str());
        }
        printf("\n");

        // Periodic progress summary every 100 ROMs
        if ((i + 1) % 100 == 0) {
            auto elapsed = std::chrono::duration<double>(
                std::chrono::steady_clock::now() - global_start).count();
            printf("  --- Progress: %d/%d done, %d PASS / %d FAIL / %d SKIP, %.0fs elapsed ---\n\n",
                   (int)(i + 1), (int)rom_files.size(), pass_count, fail_count, skip_count, elapsed);
        }
    }

    auto global_end = std::chrono::steady_clock::now();
    double total_seconds = std::chrono::duration<double>(global_end - global_start).count();

    // Summary
    printf("\n=== SUMMARY ===\n");
    printf("Total ROMs:  %d\n", (int)results.size());
    printf("PASS:        %d (%.1f%%)\n", pass_count, 100.0 * pass_count / results.size());
    printf("FAIL:        %d (%.1f%%)\n", fail_count, 100.0 * fail_count / results.size());
    printf("SKIP:        %d (%.1f%%)\n", skip_count, 100.0 * skip_count / results.size());
    printf("Elapsed:     %.1f seconds\n", total_seconds);

    // Failure breakdown
    if (fail_count > 0) {
        printf("\n=== FAILURE BREAKDOWN ===\n");
        // Count by failure category
        int cat_load = 0, cat_crash = 0, cat_stuck = 0, cat_novideo = 0, cat_pc = 0, cat_other = 0;
        for (auto& r : results) {
            if (r.failure_reason.empty() || r.load_status == -2) continue;
            if (r.failure_reason.find("load_failed") != std::string::npos) cat_load++;
            else if (r.failure_reason.find("crash") != std::string::npos) cat_crash++;
            else if (r.failure_reason.find("stuck") != std::string::npos) cat_stuck++;
            else if (r.failure_reason.find("no_video") != std::string::npos) cat_novideo++;
            else if (r.failure_reason.find("pc_not") != std::string::npos) cat_pc++;
            else cat_other++;
        }
        if (cat_crash) printf("  Frame crash:       %d\n", cat_crash);
        if (cat_stuck) printf("  CPU stuck/hang:    %d\n", cat_stuck);
        if (cat_novideo) printf("  No video output:   %d\n", cat_novideo);
        if (cat_pc) printf("  PC not in ROM:     %d\n", cat_pc);
        if (cat_other) printf("  Other:             %d\n", cat_other);
    }

    // Write JSON report
    write_report(output_path, results, total_seconds);

    // Cleanup
    kagami_bridge_kill();

    return fail_count > 0 ? 1 : 0;
}
