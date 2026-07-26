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
