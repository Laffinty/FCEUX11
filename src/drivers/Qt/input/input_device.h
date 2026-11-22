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
// FCEUX11 v0.3.13 — unified low-level input device abstraction.
//
// An InputDevice represents a single physical controller (one gamepad).
// Backends such as SDL, XInput and WGI each expose their devices through
// this interface so the rest of the emulator only needs to know about
// backend id + device index + button id.

#ifndef FCEUX11_INPUT_DEVICE_H
#define FCEUX11_INPUT_DEVICE_H

#include <cstdint>
#include <cstddef>

namespace fceu11 {
namespace input {

class InputDevice {
public:
    virtual ~InputDevice() = default;

    // Human-readable name, e.g. "Xbox Controller" or "SDL GameController 0".
    virtual const char* name() const = 0;

    // Whether this device is currently connected/available.
    virtual bool connected() const = 0;

    // Query the pressed state of a logical button.  The meaning of
    // buttonId is backend-specific and is documented in each backend.
    virtual bool button(int buttonId) const = 0;

    // Poll the hardware once.  The backend's InputBackend::pollAll()
    // is responsible for calling this at the right time.
    virtual void poll() = 0;

    // Trigger vibration.  Frequencies are normalized [0.0, 1.0].
    // durationMs is the requested duration in milliseconds; 0 means stop.
    virtual void rumble(float lowFrequency, float highFrequency, uint32_t durationMs) = 0;
};

} // namespace input
} // namespace fceu11

#endif // FCEUX11_INPUT_DEVICE_H
