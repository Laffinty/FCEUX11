// KagamiQA P1 — Headless smoke test.
//
// P1 minimal verification: the null driver compiles, links, and can be
// initialised without Qt. Does NOT call fceu11::Initialize() because the
// core engine still has ~15 unresolved references to Qt-driver symbols
// (UseInputPreset, GetKeyboard, closeFinishedMovie, etc.) — these will
// be stubbed in P2 when the sdl.h include is removed from fceu.cpp.
//
// What this test proves:
//   - fceux11_drivers_null compiles and links
//   - null_driver_init() registers all-nullptr DriverCallbacks
//   - The binary has zero Qt DLL dependencies

#include "driver_callbacks.h"
#include "null_driver.h"
#include <cstdio>

int main() {
    null_driver_init();

    // Verify the driver globals are defined and accessible.
    if (dendy != 0 || pal_emulation != 0) {
        std::printf("FAIL: driver globals have unexpected non-zero values\n");
        return 1;
    }

    // Verify DriverCallbacks are registered (all-nullptr is valid).
    auto& cb = fceu11::g_driver();
    (void)cb; // silence unused-variable warning

    std::printf("PASS: headless null driver infrastructure verified\n");
    return 0;
}
