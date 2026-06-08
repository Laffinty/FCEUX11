// FCEUX11 v0.3.3 — tl::expected wrapper for fceu11 namespace
// Polyfill until MSVC supports std::expected (C++23)

#ifndef FCEU11_EXPECTED_H
#define FCEU11_EXPECTED_H

#include "tl/expected.hpp"
#include <string>

namespace fceu11 {
    template<typename T, typename E = int>
    using expected = tl::expected<T, E>;

    template<typename E = int>
    using unexpected = tl::unexpected<E>;

    // v0.3.3: New expected-style API wrappers (6 initial APIs)
    // These wrap existing bool+outparam patterns.

    // 1. File byte loading (wraps EMUFILE::readAllBytes)
    expected<std::vector<uint8_t>> load_file_bytes(const std::string& path);

    // 2. Core initialization (wraps FCEUI_Initialize)
    expected<void> initialize_core();

    // 3. Movie info retrieval (wraps FCEUI_MovieGetInfo)
    struct MovieInfo {
        int length;
        int rerecord_count;
        std::string author;
        bool read_only;
    };
    expected<MovieInfo> get_movie_info(const std::string& path, bool skipFrameCount = false);

    // 4. Save state serialization (wraps FCEUSS_SaveMS)
    expected<std::vector<uint8_t>> save_state(int compressionLevel = 0);

    // 5. Save state deserialization (wraps FCEUSS_LoadFP)
    expected<void> load_state(const std::vector<uint8_t>& data);

    // 6. IPS filename construction (wraps FCEU_MakeIpsFilename)
    expected<std::string> make_ips_filename(const std::string& baseDir,
                                            const std::string& baseName,
                                            const std::string& ext);
} // namespace fceu11

#endif // FCEU11_EXPECTED_H
