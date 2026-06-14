// FCEUX11 v0.3.13 — central input backend manager.
//
// Owns all registered InputBackend instances and exposes a single point
// for polling and button queries.  Device numbers are partitioned by
// backend so that existing ButtConfig::DeviceNum values keep working
// without changing the structure layout.

#ifndef FCEUX11_INPUT_MANAGER_H
#define FCEUX11_INPUT_MANAGER_H

#include "input_backend.h"

#include <cstdint>
#include <memory>
#include <vector>

namespace fceu11 {
namespace input {

// Backend identifiers and per-backend device index offsets.
// These are intentionally stable: adding a backend must not shift the
// offsets of an existing backend, or saved input configurations would break.
enum class BackendId : int {
    SDL = 0,
    XInput = 1,
    WGI = 2,

    Count = 3
};

inline constexpr int kMaxDevicesPerBackend = 32;
inline constexpr int kBackendOffset(BackendId id) {
    return static_cast<int>(id) * kMaxDevicesPerBackend;
}
inline constexpr BackendId kBackendFromDeviceNum(int deviceNum) {
    if (deviceNum < 0) return BackendId::SDL;
    int id = deviceNum / kMaxDevicesPerBackend;
    if (id >= static_cast<int>(BackendId::Count)) return BackendId::SDL;
    return static_cast<BackendId>(id);
}
inline constexpr int kLocalDeviceIndex(int deviceNum) {
    return deviceNum % kMaxDevicesPerBackend;
}

class InputManager {
public:
    static InputManager& instance();

    // Non-copyable, non-movable.
    InputManager(const InputManager&) = delete;
    InputManager& operator=(const InputManager&) = delete;

    // Register a backend.  The manager takes ownership.  initialize() is
    // called immediately; if it fails the backend is dropped.
    void registerBackend(std::unique_ptr<InputBackend> backend);

    // Convenience: register the standard backends in priority order
    // (XInput, SDL, optionally WGI).  WGI is only registered if the
    // FCEUX11_WGI_BACKEND macro is defined at compile time.
    void registerDefaultBackends();

    // Initialize all registered backends and enumerate their devices.
    void initializeAll();

    // Shut all backends down and release resources.
    void shutdownAll();

    // Re-enumerate devices on all backends.
    void enumerateAll();

    // Poll all backends once per frame.
    void pollAll();

    // Query a button.  deviceNum and buttonId use the existing ButtConfig
    // encoding.  Returns false for disconnected/unknown devices.
    bool testButton(int deviceNum, int buttonId) const;

    // Trigger rumble on a specific device.
    void rumble(int deviceNum, float lowFrequency, float highFrequency, uint32_t durationMs);

    // Direct backend access for configuration UIs.
    InputBackend* backend(BackendId id) const;

private:
    InputManager() = default;
    ~InputManager() = default;

    std::vector<std::unique_ptr<InputBackend>> backends_;
};

} // namespace input
} // namespace fceu11

#endif // FCEUX11_INPUT_MANAGER_H
