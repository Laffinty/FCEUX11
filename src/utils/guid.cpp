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
/// \file
/// \brief GUID wrapper — delegates to Rust.
///
/// Phase 2 (v0.2.3): Rust module provides UUID v4 generation via the `uuid` crate,
/// replacing the original `rand()`-based GUID generation with RFC 4122 compliant UUIDs.

#include "guid.h"
#include "../rust/fceux11_rust.h"

void FCEU_Guid::newGuid()
{
    fceux11_rust_guid_new(reinterpret_cast<FceuGuid*>(this));
}

std::string FCEU_Guid::toString()
{
    const char* ptr = fceux11_rust_guid_to_string(reinterpret_cast<const FceuGuid*>(this));
    if (!ptr) return std::string();
    return std::string(ptr);
}

FCEU_Guid FCEU_Guid::fromString(std::string str)
{
    // v2.0.4 (V19): value-initialize so a failed parse can never return
    // stack garbage. Upstream master does the same via
    // memset(data,0,sizeof(data)) on the failure path of scan(); here the
    // guarantee lives in the initialization because the parse itself is
    // delegated to Rust (fceux11_rust_guid_scan), which zeroes its own
    // failure paths — this `ret{}` is the belt-and-suspenders underneath.
    // See docs/plans/FCEUX11-v2.0.4_残留精度与调试待办-构建计划.md §V19.
    FCEU_Guid ret{};
    ret.scan(str);
    return ret;
}

void FCEU_Guid::scan(std::string& str)
{
    fceux11_rust_guid_scan(reinterpret_cast<FceuGuid*>(this), str.c_str());
}
