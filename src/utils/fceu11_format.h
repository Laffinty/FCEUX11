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
// FCEUX11 v0.3.3 — C++20 <format> wrapper for internal logging paths
// Public API (FCEU_printf) remains unchanged; this is for internal use only.

#ifndef FCEU11_FORMAT_H
#define FCEU11_FORMAT_H

#include <format>
#include <string>

namespace fceu11 {
    // v0.3.3: std::format-based string formatting (internal use only)
    template<typename... Args>
    std::string fmt(std::format_string<Args...> fmt_str, Args&&... args) {
        return std::format(fmt_str, std::forward<Args>(args)...);
    }
} // namespace fceu11

#endif // FCEU11_FORMAT_H
