// FCEUX11 v2.1.1.7 Step B.5 (D3-a) - `newppu` neutrality gate.
//
// The legacy `newppu` switch used to select between the C++ PPU cores.
// Since B.5 it is a compatibility switch only: the Rust PPU is the engine
// regardless of its value. This test proves that at runtime by running the
// same ROM twice (newppu = 0 / 1) and requiring the frame-buffer CRC32 of
// the same frame index to be identical.
//
// The fixture ROM corpus is gitignored (AGENTS.md gotcha 5), so the test
// reports SKIP (exit 0) when the ROM is absent instead of failing.

#include <cstdio>
#include <cstdint>

extern "C" {
#include "kagami_bridge.h"
}

namespace {

constexpr uint32_t kFrames = 60;
constexpr uint32_t kFrameBytes = 256 * 240;  // visible slice of XBuf

// Portable CRC32 (IEEE 802.3) - avoids a zlib link dependency in this
// headless target.
uint32_t Crc32(const uint8_t* data, uint32_t len) {
    uint32_t crc = 0xFFFFFFFFu;
    for (uint32_t i = 0; i < len; ++i) {
        crc ^= data[i];
        for (int bit = 0; bit < 8; ++bit) {
            const uint32_t mask = 0u - (crc & 1u);
            crc = (crc >> 1) ^ (0xEDB88320u & mask);
        }
    }
    return ~crc;
}

bool FileExists(const char* path) {
    FILE* f = std::fopen(path, "rb");
    if (f == nullptr) {
        return false;
    }
    std::fclose(f);
    return true;
}

// Returns 0 on success and writes the CRC of the visible frame buffer.
int RunAndCrc(const char* rom, int newppu, uint32_t* out_crc) {
    if (kagami_bridge_full_reset() != 0) {
        std::fprintf(stderr, "FAIL: kagami_bridge_full_reset\n");
        return 1;
    }
    kagami_bridge_set_newppu(newppu);
    if (kagami_bridge_load_rom(rom) != 0) {
        std::fprintf(stderr, "FAIL: kagami_bridge_load_rom('%s')\n", rom);
        return 1;
    }
    for (uint32_t i = 0; i < kFrames; ++i) {
        if (kagami_bridge_emulate_frame() != 0) {
            std::fprintf(stderr, "FAIL: emulate_frame %u\n", i);
            return 1;
        }
    }
    static uint8_t frame[kFrameBytes];
    if (kagami_bridge_extract_frame_buffer(frame, sizeof(frame)) != 0) {
        std::fprintf(stderr, "FAIL: extract_frame_buffer\n");
        return 1;
    }
    *out_crc = Crc32(frame, sizeof(frame));
    return 0;
}

}  // namespace

int main() {
    const char* rom = "fixtures/nestest.nes";
    if (!FileExists(rom)) {
        std::printf("SKIP: %s not present (gitignored ROM corpus)\n", rom);
        return 0;
    }

    if (kagami_bridge_init() != 0) {
        std::fprintf(stderr, "FAIL: kagami_bridge_init\n");
        return 1;
    }

    uint32_t crc_old_ppu = 0;
    uint32_t crc_new_ppu = 0;
    if (RunAndCrc(rom, 0, &crc_old_ppu) != 0) {
        kagami_bridge_kill();
        return 1;
    }
    if (RunAndCrc(rom, 1, &crc_new_ppu) != 0) {
        kagami_bridge_kill();
        return 1;
    }
    kagami_bridge_kill();

    std::printf("newppu=0 frame%u crc=%08X\n", kFrames, crc_old_ppu);
    std::printf("newppu=1 frame%u crc=%08X\n", kFrames, crc_new_ppu);

    if (crc_old_ppu != crc_new_ppu) {
        std::fprintf(stderr,
                     "FAIL: the newppu switch still changes the emulated output\n");
        return 1;
    }
    std::printf("PASS: newppu is engine-neutral\n");
    return 0;
}