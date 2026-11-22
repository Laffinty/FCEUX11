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
