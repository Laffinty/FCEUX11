#include "null_driver.h"
#include "driver_callbacks.h"

int dendy = 0;
int pal_emulation = 0;

void null_driver_init() {
    // Register a fully-null DriverCallbacks. Every FCEUD_* forwarding
    // function in driver_callbacks.cpp checks `if (fn) fn(...)` before
    // calling, so all-nullptr is safe: the engine runs without a GUI.
    fceu11::register_driver(fceu11::DriverCallbacks{});
}

// ---------------------------------------------------------------------------
// P3: Headless stubs — symbols that fceux11_core references but are normally
// provided by the Qt driver. We provide zero/default stubs so headless
// binaries can link without Qt.
// ---------------------------------------------------------------------------

// fceu.cpp references (frame advance / throttle)
unsigned int frameAdvHoldTimer = 0;
int KillFCEUXonFrame = 0;
bool turbo = false;

void RefreshThrottleFPS() {}

// AVI / movie recording stubs
namespace fceu11 {
    bool AviIsRecording() { return false; }
    bool AviEnableHUDrecording() { return false; }
    bool AviDisableMovieMessages() { return false; }
    void AviVideoUpdate(const unsigned char*) {}
}

// TAS editor stubs
bool isTaseditorRecording() { return false; }
int closeFinishedMovie = 0;

// Input stubs
namespace fceu11 {
    void UseInputPreset(int) {}
}
unsigned int* GetKeyboard() {
    static unsigned int dummy[256] = {};
    return dummy;
}
void GetMouseData(unsigned int (&md)[3]) {
    md[0] = md[1] = md[2] = 0;
}

// Misc globals referenced by core
int eoptions = 0;
const char* getRomFile() { return ""; }
