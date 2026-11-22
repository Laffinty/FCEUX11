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
#ifndef FCEU_SAFE_STRING_H
#define FCEU_SAFE_STRING_H

// v0.3.5: Safe string operations — replaces strcpy/strcat/sprintf

#include <cstddef>
#include <cstring>
#include <cstdio>
#include <utility>

/// Safe string copy (strlcpy semantics). Guarantees null-termination.
inline void FCEU_strlcpy(char* dst, size_t dstSize, const char* src) {
    if (dstSize == 0) return;
    std::strncpy(dst, src, dstSize - 1);
    dst[dstSize - 1] = '\0';
}

/// Safe string concatenation. Guarantees null-termination.
inline void safe_strcat(char* dst, size_t dstSize, const char* src) {
    size_t dstLen = std::strlen(dst);
    if (dstLen >= dstSize - 1) return;
    std::strncat(dst, src, dstSize - dstLen - 1);
    dst[dstSize - 1] = '\0';
}

/// Safe formatted output to a fixed-size char buffer. Guarantees null-termination.
template<size_t N, typename... Args>
void safe_format(char(&buf)[N], const char* fmt, Args&&... args) {
    std::snprintf(buf, N, fmt, std::forward<Args>(args)...);
    buf[N - 1] = '\0';
}

#endif // FCEU_SAFE_STRING_H
