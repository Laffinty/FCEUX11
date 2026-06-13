// FCEUX11 v0.3.3 — tl::expected wrapper implementations

#include "fceu11_expected.h"
#include "fceu11_format.h"
#include "emufile.h"
#include "fceu.h"
#include "driver.h"
#include "state.h"
#include "movie.h"
#include "file.h"
#include "types.h"

namespace fceu11 {

expected<std::vector<uint8_t>> load_file_bytes(const std::string& path) {
    std::vector<uint8_t> buf;
    if (!EMUFILE::readAllBytes(&buf, path)) {
        return unexpected(1);
    }
    return buf;
}

expected<void> initialize_core() {
    if (!FCEUI_Initialize()) {
        return unexpected(2);
    }
    return {};
}

expected<MovieInfo> get_movie_info(const std::string& path, bool skipFrameCount) {
    FCEUFILE* fp = FCEU_fopen(path.c_str(), nullptr, "rb", nullptr, 0, nullptr);
    if (!fp) {
        return unexpected(3);
    }
    MOVIE_INFO info{};
    bool ok = FCEUI_MovieGetInfo(fp, info, skipFrameCount);
    FCEU_fclose(fp);
    if (!ok) {
        return unexpected(4);
    }
    MovieInfo mi{};
    mi.length = static_cast<int>(info.num_frames);
    mi.rerecord_count = static_cast<int>(info.rerecord_count);
    mi.author = info.comments.empty() ? "" : std::string(info.comments[0].begin(), info.comments[0].end());
    mi.read_only = info.poweron;
    return mi;
}

expected<std::vector<uint8_t>> save_state(int compressionLevel) {
    EMUFILE_MEMORY ms;
    if (!FCEUSS_SaveMS(&ms, compressionLevel)) {
        return unexpected(5);
    }
    // v0.3.10: ms.buf() returns std::byte*; convert to uint8_t* for vector init.
    std::vector<uint8_t> buf(reinterpret_cast<const uint8_t*>(ms.buf()),
                             reinterpret_cast<const uint8_t*>(ms.buf()) + ms.size());
    return buf;
}

expected<void> load_state(const std::vector<uint8_t>& data) {
    EMUFILE_MEMORY ms(const_cast<void*>(static_cast<const void*>(data.data())), data.size());
    if (!FCEUSS_LoadFP(&ms, SSLOADPARAM_NOBACKUP)) {
        return unexpected(6);
    }
    return {};
}

expected<std::string> make_ips_filename(const std::string& baseDir,
                                        const std::string& baseName,
                                        const std::string& ext) {
    // Inline IPS filename construction (avoids dependency on file.cpp internal)
    std::string ret = baseDir + PSS + baseName + ext + ".ips";
    return ret;
}

} // namespace fceu11
