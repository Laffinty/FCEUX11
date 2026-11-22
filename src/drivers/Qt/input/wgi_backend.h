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
