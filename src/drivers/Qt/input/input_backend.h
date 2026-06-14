// FCEUX11 v0.3.13 — input backend plugin interface.
//
// An InputBackend owns all devices provided by one underlying API
// (SDL_GameController, XInput, Windows.Gaming.Input, ...).  It is
// responsible for enumeration, lifetime management and per-frame polling.

#ifndef FCEUX11_INPUT_BACKEND_H
#define FCEUX11_INPUT_BACKEND_H

#include "input_device.h"

#include <cstddef>
#include <memory>

namespace fceu11 {
namespace input {

class InputBackend {
public:
    virtual ~InputBackend() = default;

    // Backend name, e.g. "SDL", "XInput", "WGI".
    virtual const char* name() const = 0;

    // One-time initialization.  Returns false if the backend cannot be used
    // on this system (e.g. XInput DLL missing).  On false the backend will
    // not be registered with the manager.
    virtual bool initialize() = 0;

    // Release all resources held by this backend.
    virtual void shutdown() = 0;

    // Re-enumerate connected devices.  Called on hot-plug events and at
    // startup after initialize().
    virtual void enumerate() = 0;

    // Number of currently known devices.
    virtual std::size_t deviceCount() const = 0;

    // Access a device by index.  The pointer remains valid until the next
    // enumerate() call or until the backend is shut down.
    virtual InputDevice* device(std::size_t index) = 0;

    // Poll all devices owned by this backend.
    virtual void pollAll() = 0;
};

} // namespace input
} // namespace fceu11

#endif // FCEUX11_INPUT_BACKEND_H
