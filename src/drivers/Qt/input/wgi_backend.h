// FCEUX11 v0.3.13 — Windows.Gaming.Input (WGI) backend plugin.
//
// Optional backend controlled by FCEUX11_WGI_BACKEND.  When enabled it
// provides high-precision vibration via IGamepad::SetVibration.

#ifndef FCEUX11_WGI_BACKEND_H
#define FCEUX11_WGI_BACKEND_H

#include "input_backend.h"

#include <array>
#include <memory>

namespace fceu11 {
namespace input {

class WGIDevice;

class WindowsGamingInputBackend : public InputBackend {
public:
    static constexpr int MAX_DEVICES = 4;

    WindowsGamingInputBackend();
    ~WindowsGamingInputBackend() override;

    const char* name() const override { return "WGI"; }
    bool initialize() override;
    void shutdown() override;
    void enumerate() override;
    std::size_t deviceCount() const override { return MAX_DEVICES; }
    InputDevice* device(std::size_t index) override;
    void pollAll() override;

private:
    bool initialized_ = false;
    std::array<std::unique_ptr<WGIDevice>, MAX_DEVICES> devices_;
};

} // namespace input
} // namespace fceu11

#endif // FCEUX11_WGI_BACKEND_H
