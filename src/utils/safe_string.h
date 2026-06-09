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
