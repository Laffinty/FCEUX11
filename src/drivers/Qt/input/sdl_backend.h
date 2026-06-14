// FCEUX11 v0.3.13 — SDL input backend plugin.

#ifndef FCEUX11_SDL_BACKEND_H
#define FCEUX11_SDL_BACKEND_H

#include "input_backend.h"

#include "Qt/sdl.h"
#include "Qt/sdl-joystick.h"

#include <array>
#include <memory>

namespace fceu11 {
namespace input {

// Legacy storage lives in sdl-joystick.cpp so existing GamePadConf and
// mapping code can keep accessing jsDev_t directly.
extern jsDev_t g_sdlJsDev[MAX_JOYSTICKS];
extern int g_sdlJInited;

class SDLDevice;

class SDLBackend : public InputBackend {
public:
    static constexpr int MAX_DEVICES = MAX_JOYSTICKS;

    SDLBackend();
    ~SDLBackend() override;

    const char* name() const override { return "SDL"; }
    bool initialize() override;
    void shutdown() override;
    void enumerate() override;
    std::size_t deviceCount() const override { return MAX_DEVICES; }
    InputDevice* device(std::size_t index) override;
    void pollAll() override;

    jsDev_t* rawDevice(int index);

private:
    bool initialized_ = false;
    std::array<std::unique_ptr<SDLDevice>, MAX_DEVICES> devices_;
};

// Legacy accessor used by sdl-joystick.cpp free functions.
SDLBackend* GetSDLBackend();

} // namespace input
} // namespace fceu11

#endif // FCEUX11_SDL_BACKEND_H
