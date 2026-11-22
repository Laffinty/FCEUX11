/* FCE Ultra - NES/Famicom Emulator
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program; if not, write to the Free Software
 * Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA  02110-1301  USA
 */
// FCEUX11 v0.3.13 — InputManager implementation.

#include "input_manager.h"

#include "sdl_backend.h"
#include "xinput_backend.h"

#ifdef FCEUX11_WGI_BACKEND
#include "wgi_backend.h"
#endif

#include <algorithm>
#include <cstdio>

namespace fceu11 {
namespace input {

InputManager& InputManager::instance() {
    static InputManager manager;
    return manager;
}

void InputManager::registerBackend(std::unique_ptr<InputBackend> backend) {
    if (!backend) return;

    if (!backend->initialize()) {
        std::printf("Input backend '%s' failed to initialize, skipping.\n",
                    backend->name());
        backend->shutdown();
        return;
    }

    backend->enumerate();
    backends_.push_back(std::move(backend));
}

void InputManager::registerDefaultBackends() {
    // Registration order must match BackendId enum values because
    // backend(BackendId) indexes directly into the vector.
    registerBackend(std::make_unique<SDLBackend>());            // BackendId::SDL == 0
    registerBackend(std::make_unique<XInputBackend>());         // BackendId::XInput == 1

#ifdef FCEUX11_WGI_BACKEND
    registerBackend(std::make_unique<WindowsGamingInputBackend>()); // BackendId::WGI == 2
#endif
}

void InputManager::initializeAll() {
    for (auto& backend : backends_) {
        backend->initialize();
        backend->enumerate();
    }
}

void InputManager::shutdownAll() {
    for (auto& backend : backends_) {
        backend->shutdown();
    }
    backends_.clear();
}

void InputManager::enumerateAll() {
    for (auto& backend : backends_) {
        backend->enumerate();
    }
}

void InputManager::pollAll() {
    for (auto& backend : backends_) {
        backend->pollAll();
    }
}

bool InputManager::testButton(int deviceNum, int buttonId) const {
    BackendId backendId = kBackendFromDeviceNum(deviceNum);
    int localIdx = kLocalDeviceIndex(deviceNum);

    InputBackend* be = backend(backendId);
    if (!be) return false;

    std::size_t idx = static_cast<std::size_t>(localIdx);
    if (idx >= be->deviceCount()) return false;

    InputDevice* dev = be->device(idx);
    if (!dev || !dev->connected()) return false;

    return dev->button(buttonId);
}

void InputManager::rumble(int deviceNum, float lowFrequency, float highFrequency,
                          uint32_t durationMs) {
    BackendId backendId = kBackendFromDeviceNum(deviceNum);
    int localIdx = kLocalDeviceIndex(deviceNum);

    InputBackend* be = backend(backendId);
    if (!be) return;

    std::size_t idx = static_cast<std::size_t>(localIdx);
    if (idx >= be->deviceCount()) return;

    InputDevice* dev = be->device(idx);
    if (!dev || !dev->connected()) return;

    dev->rumble(lowFrequency, highFrequency, durationMs);
}

InputBackend* InputManager::backend(BackendId id) const {
    int index = static_cast<int>(id);
    if (index < 0 || index >= static_cast<int>(backends_.size())) {
        return nullptr;
    }
    return backends_[index].get();
}

} // namespace input
} // namespace fceu11
