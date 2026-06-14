// FCEUX11 v0.3.13 — Windows.Gaming.Input backend implementation.
//
// This file is only compiled when FCEUX11_WGI_BACKEND is defined.
// It tries to use C++/WinRT projection headers if they are available;
// otherwise the backend initializes to an empty/no-op state so that the
// build still succeeds on environments without the full WinRT SDK.

#include "wgi_backend.h"

#include <cstdio>

#if __has_include(<winrt/Windows.Gaming.Input.h>)
#define FCEUX11_WGI_CPPWINRT_AVAILABLE 1
#include <winrt/Windows.Gaming.Input.h>
#include <winrt/Windows.Foundation.Collections.h>
#endif

namespace fceu11 {
namespace input {

class WGIDevice : public InputDevice {
public:
    explicit WGIDevice(int slot) : slot_(slot) {}

    const char* name() const override {
        return connected_ ? "WGI Gamepad" : "WGI (disconnected)";
    }

    bool connected() const override { return connected_; }

    bool button(int buttonId) const override {
        if (!connected_ || buttonId < 0 || buttonId >= 16) return false;
        return (buttons_ & (1u << buttonId)) != 0;
    }

    void poll() override {
#if defined(FCEUX11_WGI_CPPWINRT_AVAILABLE)
        using namespace winrt::Windows::Gaming::Input;
        connected_ = false;
        auto gamepads = Gamepad::Gamepads();
        if (slot_ >= static_cast<int>(gamepads.Size())) return;

        Gamepad pad = gamepads.GetAt(static_cast<uint32_t>(slot_));
        if (!pad) return;

        GamepadReading reading = pad.GetCurrentReading();
        connected_ = true;

        // Map WGI buttons to a 16-bit mask.
        uint32_t mask = 0;
        if ((reading.Buttons & GamepadButtons::A) != GamepadButtons::None)
            mask |= 1u << 0;
        if ((reading.Buttons & GamepadButtons::B) != GamepadButtons::None)
            mask |= 1u << 1;
        if ((reading.Buttons & GamepadButtons::X) != GamepadButtons::None)
            mask |= 1u << 2;
        if ((reading.Buttons & GamepadButtons::Y) != GamepadButtons::None)
            mask |= 1u << 3;
        if ((reading.Buttons & GamepadButtons::LeftShoulder) != GamepadButtons::None)
            mask |= 1u << 4;
        if ((reading.Buttons & GamepadButtons::RightShoulder) != GamepadButtons::None)
            mask |= 1u << 5;
        if ((reading.Buttons & GamepadButtons::View) != GamepadButtons::None)
            mask |= 1u << 6;
        if ((reading.Buttons & GamepadButtons::Menu) != GamepadButtons::None)
            mask |= 1u << 7;
        if ((reading.Buttons & GamepadButtons::LeftThumbstick) != GamepadButtons::None)
            mask |= 1u << 8;
        if ((reading.Buttons & GamepadButtons::RightThumbstick) != GamepadButtons::None)
            mask |= 1u << 9;
        if ((reading.Buttons & GamepadButtons::DPadUp) != GamepadButtons::None)
            mask |= 1u << 10;
        if ((reading.Buttons & GamepadButtons::DPadDown) != GamepadButtons::None)
            mask |= 1u << 11;
        if ((reading.Buttons & GamepadButtons::DPadLeft) != GamepadButtons::None)
            mask |= 1u << 12;
        if ((reading.Buttons & GamepadButtons::DPadRight) != GamepadButtons::None)
            mask |= 1u << 13;
        buttons_ = mask;
#else
        connected_ = false;
#endif
    }

    void rumble(float lowFrequency, float highFrequency,
                uint32_t durationMs) override {
#if defined(FCEUX11_WGI_CPPWINRT_AVAILABLE)
        using namespace winrt::Windows::Gaming::Input;
        auto gamepads = Gamepad::Gamepads();
        if (slot_ >= static_cast<int>(gamepads.Size())) return;

        Gamepad pad = gamepads.GetAt(static_cast<uint32_t>(slot_));
        if (!pad) return;

        GamepadVibration vib{};
        vib.LeftMotor = lowFrequency;
        vib.RightMotor = highFrequency;
        vib.LeftTrigger = 0.0;
        vib.RightTrigger = 0.0;
        pad.Vibration(vib);
#else
        (void)lowFrequency;
        (void)highFrequency;
#endif
        (void)durationMs;
    }

private:
    int slot_;
    bool connected_ = false;
    uint32_t buttons_ = 0;
};

WindowsGamingInputBackend::WindowsGamingInputBackend() = default;

WindowsGamingInputBackend::~WindowsGamingInputBackend() {
    shutdown();
}

bool WindowsGamingInputBackend::initialize() {
    if (initialized_) return true;

#if !defined(FCEUX11_WGI_CPPWINRT_AVAILABLE)
    std::printf("WGI backend: C++/WinRT headers not available, disabling.\n");
    return false;
#endif

    for (int i = 0; i < MAX_DEVICES; ++i) {
        devices_[i] = std::make_unique<WGIDevice>(i);
    }

    initialized_ = true;
    return true;
}

void WindowsGamingInputBackend::shutdown() {
    for (int i = 0; i < MAX_DEVICES; ++i) {
        devices_[i].reset();
    }
    initialized_ = false;
}

void WindowsGamingInputBackend::enumerate() {
    // Device list is queried dynamically during pollAll().
}

InputDevice* WindowsGamingInputBackend::device(std::size_t index) {
    if (index >= static_cast<std::size_t>(MAX_DEVICES)) return nullptr;
    return devices_[index].get();
}

void WindowsGamingInputBackend::pollAll() {
    for (int i = 0; i < MAX_DEVICES; ++i) {
        if (devices_[i]) {
            devices_[i]->poll();
        }
    }
}

} // namespace input
} // namespace fceu11
