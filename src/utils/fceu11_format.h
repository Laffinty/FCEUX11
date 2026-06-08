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
