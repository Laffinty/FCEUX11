// FCEUX11 v0.3.13 — SDL input backend implementation.

#include "sdl_backend.h"

#include <cstdio>
#include <cstring>

namespace fceu11 {
namespace input {

// Legacy storage shared with sdl-joystick.cpp.
jsDev_t g_sdlJsDev[MAX_JOYSTICKS];
int g_sdlJInited = 0;

// Global pointer for legacy sdl-joystick.cpp free functions to reach the
// registered SDL backend.  Set on successful initialize(), cleared on
// shutdown().  The lifetime is managed by InputManager.
static SDLBackend* g_sdlBackend = nullptr;

class SDLDevice : public InputDevice {
public:
    explicit SDLDevice(jsDev_t* dev) : dev_(dev) {}

    const char* name() const override {
        if (!dev_ || !dev_->isConnected()) return "SDL (disconnected)";
        if (dev_->isGameController()) {
            const char* n = SDL_GameControllerName(dev_->gc);
            return n ? n : "SDL GameController";
        }
        const char* n = SDL_JoystickName(dev_->getJS());
        return n ? n : "SDL Joystick";
    }

    bool connected() const override {
        return dev_ && dev_->isConnected();
    }

    bool button(int buttonId) const override {
        if (!dev_ || !dev_->isConnected()) return false;
        if (buttonId < 0) return false;

        SDL_Joystick* js = dev_->getJS();
        if (!js) return false;

        if (buttonId & 0x2000) {
            // Hat "button"
            return (SDL_JoystickGetHat(js, (buttonId >> 8) & 0x1F) &
                    (buttonId & 0xFF)) != 0;
        }
        if (buttonId & 0x8000) {
            // Axis "button"
            int pos = SDL_JoystickGetAxis(js, buttonId & 0x3FFF);
            if ((buttonId & 0x4000) && pos <= -16383) return true;
            if (!(buttonId & 0x4000) && pos >= 16363) return true;
            return false;
        }
        return SDL_JoystickGetButton(js, buttonId) != 0;
    }

    void poll() override {
        // SDL state is maintained by the event loop in input.cpp.
        // Explicit polling is not required.
    }

    void rumble(float lowFrequency, float highFrequency,
                uint32_t durationMs) override {
        if (!dev_ || !dev_->isGameController()) return;
        SDL_GameControllerRumble(
            dev_->gc,
            static_cast<Uint16>(lowFrequency * 65535.0f),
            static_cast<Uint16>(highFrequency * 65535.0f),
            durationMs);
    }

private:
    jsDev_t* dev_;
};

SDLBackend::SDLBackend() = default;

SDLBackend::~SDLBackend() {
    shutdown();
}

bool SDLBackend::initialize() {
    if (initialized_) return true;

    if (SDL_InitSubSystem(SDL_INIT_JOYSTICK) < 0) {
        std::printf("SDLBackend: SDL_InitSubSystem(JOYSTICK) failed: %s\n",
                    SDL_GetError());
        return false;
    }

    for (int i = 0; i < MAX_DEVICES; ++i) {
        devices_[i] = std::make_unique<SDLDevice>(&g_sdlJsDev[i]);
    }

    initialized_ = true;
    g_sdlJInited = 1;
    g_sdlBackend = this;
    return true;
}

void SDLBackend::shutdown() {
    if (!initialized_) return;

    for (int i = 0; i < MAX_DEVICES; ++i) {
        g_sdlJsDev[i].close();
        devices_[i].reset();
    }

    SDL_QuitSubSystem(SDL_INIT_JOYSTICK);
    initialized_ = false;
    g_sdlJInited = 0;
    if (g_sdlBackend == this) g_sdlBackend = nullptr;
}

void SDLBackend::enumerate() {
    if (!initialized_) return;

    // Close any previously opened devices before re-enumerating.
    for (int i = 0; i < MAX_DEVICES; ++i) {
        g_sdlJsDev[i].close();
    }

    int total = SDL_NumJoysticks();
    if (total > MAX_DEVICES) total = MAX_DEVICES;

    for (int i = 0; i < total; ++i) {
        if (SDL_IsGameController(i)) {
            g_sdlJsDev[i].gc = SDL_GameControllerOpen(i);
            if (g_sdlJsDev[i].gc) {
                g_sdlJsDev[i].js = SDL_GameControllerGetJoystick(g_sdlJsDev[i].gc);
                g_sdlJsDev[i].init(i);
            } else {
                std::printf("SDLBackend: Could not open game controller %d: %s.\n",
                            i, SDL_GetError());
            }
        } else {
            g_sdlJsDev[i].js = SDL_JoystickOpen(i);
            if (g_sdlJsDev[i].js) {
                g_sdlJsDev[i].init(i);
            } else {
                std::printf("SDLBackend: Could not open joystick %d: %s.\n",
                            i, SDL_GetError());
            }
        }
    }
}

InputDevice* SDLBackend::device(std::size_t index) {
    if (index >= static_cast<std::size_t>(MAX_DEVICES)) return nullptr;
    return devices_[index].get();
}

void SDLBackend::pollAll() {
    // Pump SDL events so joystick state stays current.  The actual event
    // dispatch loop lives in input.cpp; this just keeps SDL's internal
    // joystick state fresh for queries.
    SDL_PumpEvents();
}

jsDev_t* SDLBackend::rawDevice(int index) {
    if (index < 0 || index >= MAX_DEVICES) return nullptr;
    return &g_sdlJsDev[index];
}

SDLBackend* GetSDLBackend() {
    return g_sdlBackend;
}

} // namespace input
} // namespace fceu11
