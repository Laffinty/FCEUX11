// FCEUX11 v0.3.13 — XInput backend plugin.
//
// Uses xinput1_4.dll via LoadLibrary/GetProcAddress so that the executable
// does not statically link xinput.lib and can degrade gracefully when the
// DLL is absent.

#ifndef FCEUX11_XINPUT_BACKEND_H
#define FCEUX11_XINPUT_BACKEND_H

#include "input_backend.h"

#include <windows.h>
#include <xinput.h>

#include <array>
#include <memory>

namespace fceu11 {
namespace input {

class XInputDevice;

class XInputBackend : public InputBackend {
public:
    static constexpr int MAX_DEVICES = XUSER_MAX_COUNT; // 4

    XInputBackend();
    ~XInputBackend() override;

    const char* name() const override { return "XInput"; }
    bool initialize() override;
    void shutdown() override;
    void enumerate() override;
    std::size_t deviceCount() const override { return MAX_DEVICES; }
    InputDevice* device(std::size_t index) override;
    void pollAll() override;

    // Direct access for legacy paths and diagnostics.
    bool getState(int index, XINPUT_STATE& state) const;
    void setVibration(int index, WORD leftMotorSpeed, WORD rightMotorSpeed);

    using XInputGetStateProc = DWORD(WINAPI*)(DWORD, XINPUT_STATE*);
    using XInputSetStateProc = DWORD(WINAPI*)(DWORD, XINPUT_VIBRATION*);

private:
    HMODULE dll_ = nullptr;
    XInputGetStateProc getState_ = nullptr;
    XInputSetStateProc setState_ = nullptr;
    std::array<std::unique_ptr<XInputDevice>, MAX_DEVICES> devices_;
};

} // namespace input
} // namespace fceu11

#endif // FCEUX11_XINPUT_BACKEND_H
