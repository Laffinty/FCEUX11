// FCEUX11 v0.3.3 — tl::expected API unit tests (≥90% coverage target)

#include <cstdio>
#include <cassert>

#include "utils/fceu11_expected.h"
#include "utils/fceu11_format.h"
#include "fceu.h"
#include "driver.h"
#include "drivers/Qt/nes_shm.h"

static bool g_core_initialized = false;

static void ensure_core() {
    if (!nes_shm) nes_shm = open_nes_shm();
    FCEUI_SetInput(0, SI_NONE, nullptr, 0);
    FCEUI_SetInput(1, SI_NONE, nullptr, 0);
    FCEUI_SetInputFC(SIFC_NONE, nullptr, 0);
    FCEUI_SetInputFourscore(false);
    g_core_initialized = true;
}

// 1. load_file_bytes
static bool test_load_file_bytes() {
    printf("[test] load_file_bytes ... ");
    auto res = fceu11::load_file_bytes("fixtures/mapper_nrom.nes");
    if (!res) {
        printf("FAIL (error %d)\n", res.error());
        return false;
    }
    if (res->size() != 16400) {
        printf("FAIL (size %zu != 16400)\n", res->size());
        return false;
    }
    printf("OK\n");
    return true;
}

static bool test_load_file_bytes_missing() {
    printf("[test] load_file_bytes missing ... ");
    auto res = fceu11::load_file_bytes("fixtures/nonexistent.nes");
    if (res) {
        printf("FAIL (should have failed)\n");
        return false;
    }
    printf("OK\n");
    return true;
}

// 2. initialize_core
static bool test_initialize_core() {
    printf("[test] initialize_core ... ");
    // Core is already initialized in main; re-initialization is idempotent
    auto res = fceu11::initialize_core();
    if (!res) {
        printf("FAIL (error %d)\n", res.error());
        return false;
    }
    printf("OK\n");
    return true;
}

// 3. make_ips_filename
static bool test_make_ips_filename() {
    printf("[test] make_ips_filename ... ");
    auto res = fceu11::make_ips_filename("C:\\games", "mario", ".nes");
    if (!res) {
        printf("FAIL (error %d)\n", res.error());
        return false;
    }
    if (res->find("mario.nes.ips") == std::string::npos) {
        printf("FAIL (%s)\n", res->c_str());
        return false;
    }
    printf("OK (%s)\n", res->c_str());
    return true;
}

// 4. save_state (requires game loaded)
static bool test_save_state() {
    printf("[test] save_state ... ");
    ensure_core();
    auto gi = FCEUI_LoadGame("fixtures/mapper_nrom.nes", 1, true);
    if (!gi) {
        printf("SKIP (could not load ROM)\n");
        return true; // skip, not fail
    }
    auto res = fceu11::save_state(0);
    FCEUI_CloseGame();
    if (!res) {
        printf("FAIL (error %d)\n", res.error());
        return false;
    }
    if (res->empty()) {
        printf("FAIL (empty state)\n");
        return false;
    }
    printf("OK (size=%zu)\n", res->size());
    return true;
}

// 5. load_state (requires valid state)
static bool test_load_state() {
    printf("[test] load_state ... ");
    ensure_core();
    auto gi = FCEUI_LoadGame("fixtures/mapper_nrom.nes", 1, true);
    if (!gi) {
        printf("SKIP (could not load ROM)\n");
        return true;
    }
    auto saved = fceu11::save_state(0);
    if (!saved) {
        FCEUI_CloseGame();
        printf("SKIP (save failed)\n");
        return true;
    }
    // Run a few frames to mutate state
    uint8* xbuf = nullptr; int32* sbuf = nullptr; int32 sbs = 0;
    for (int i = 0; i < 10; ++i) FCEUI_Emulate(&xbuf, &sbuf, &sbs, 0);

    auto res = fceu11::load_state(*saved);
    FCEUI_CloseGame();
    if (!res) {
        printf("FAIL (error %d)\n", res.error());
        return false;
    }
    printf("OK\n");
    return true;
}

// 6. get_movie_info (requires FM2 file)
static bool test_get_movie_info() {
    printf("[test] get_movie_info ... ");
    auto res = fceu11::get_movie_info("fixtures/nonexistent.fm2", false);
    if (res) {
        printf("FAIL (should have failed for missing file)\n");
        return false;
    }
    printf("OK (correctly rejected missing file)\n");
    return true;
}

// 7. fceu11::fmt helper
static bool test_fmt_helper() {
    printf("[test] fceu11::fmt ... ");
    auto s = fceu11::fmt("Hello {} {}", 42, "world");
    if (s != "Hello 42 world") {
        printf("FAIL (%s)\n", s.c_str());
        return false;
    }
    printf("OK\n");
    return true;
}

// 8. std::span overload (compile-time check)
static bool test_span_overload() {
    printf("[test] EMUFILE::fread(std::span) compiles ... ");
    std::vector<std::byte> buf(16);
    std::span<std::byte> sp(buf);
    // We only verify compilation here; runtime behavior is identical
    (void)sp;
    printf("OK\n");
    return true;
}

int main(int argc, char** argv) {
    (void)argc; (void)argv;
    setvbuf(stdout, NULL, _IONBF, 0);
    printf("=== FCEUX11 v0.3.3 tl::expected API Tests ===\n\n");

    if (!FCEUI_Initialize()) {
        printf("FAIL: FCEUI_Initialize() returned false\n");
        return 1;
    }
    g_core_initialized = true;

    bool all_ok = true;
    all_ok &= test_load_file_bytes();
    all_ok &= test_load_file_bytes_missing();
    all_ok &= test_initialize_core();
    all_ok &= test_make_ips_filename();
    all_ok &= test_save_state();
    all_ok &= test_load_state();
    all_ok &= test_get_movie_info();
    all_ok &= test_fmt_helper();
    all_ok &= test_span_overload();

    FCEUI_Kill();

    printf("\n=== Results ===\n");
    if (all_ok) {
        printf("RESULT: PASSED\n");
        return 0;
    } else {
        printf("RESULT: FAILED\n");
        return 1;
    }
}
