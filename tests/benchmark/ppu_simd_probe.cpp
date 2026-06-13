// Phase-2 SIMD probe for v0.3.11.
// Measures a PPU-representative operation (palette-index -> RGBA conversion
// over a full 256x240 frame) under scalar, SSE4.2-class and AVX2 compilation.
// This file does NOT enter the main emulator; it is a build-only benchmark.
#include <array>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <chrono>
#include <vector>

namespace {

constexpr size_t WIDTH = 256;
constexpr size_t HEIGHT = 240;
constexpr size_t PIXELS = WIDTH * HEIGHT;
constexpr int WARMUP = 10;
constexpr int ITERS = 100;

using Palette = std::array<uint32_t, 32>;

void fill_palette(Palette& pal) {
    for (size_t i = 0; i < pal.size(); ++i)
        pal[i] = static_cast<uint32_t>(i * 0x01020408u);
}

void fill_indices(std::vector<uint8_t>& idx) {
    for (size_t i = 0; i < idx.size(); ++i)
        idx[i] = static_cast<uint8_t>((i * 7 + i / 13) & 0x1F);
}

// Scalar baseline: deliberately prevented from auto-vectorizing.
void convert_scalar(const uint8_t* idx, const uint32_t* pal, uint32_t* out, size_t n) {
#ifdef SCALAR_PROBE
    #pragma loop(no_vector)
#endif
    for (size_t i = 0; i < n; ++i)
        out[i] = pal[idx[i] & 0x1F];
}

// Auto-vectorized variant: what the compiler can do with the chosen /arch.
void convert_auto(const uint8_t* idx, const uint32_t* pal, uint32_t* out, size_t n) {
    for (size_t i = 0; i < n; ++i)
        out[i] = pal[idx[i] & 0x1F];
}

double bench(void (*fn)(const uint8_t*, const uint32_t*, uint32_t*, size_t),
             const std::vector<uint8_t>& idx, const Palette& pal, std::vector<uint32_t>& out) {
    for (int w = 0; w < WARMUP; ++w)
        fn(idx.data(), pal.data(), out.data(), out.size());

    auto t0 = std::chrono::high_resolution_clock::now();
    for (int i = 0; i < ITERS; ++i)
        fn(idx.data(), pal.data(), out.data(), out.size());
    auto t1 = std::chrono::high_resolution_clock::now();

    std::chrono::duration<double> dt = t1 - t0;
    return dt.count() / ITERS; // seconds per frame
}

} // namespace

int main() {
    Palette pal;
    fill_palette(pal);
    std::vector<uint8_t> idx(PIXELS);
    fill_indices(idx);
    std::vector<uint32_t> out(PIXELS);

    // Touch output once to fault pages before timing.
    std::fill(out.begin(), out.end(), 0u);

    double t_scalar = bench(convert_scalar, idx, pal, out);
    double t_auto   = bench(convert_auto,   idx, pal, out);

    // Prevent the last result from being optimized away.
    volatile uint32_t sink = out[PIXELS / 2];
    (void)sink;

    std::printf("RESULT: SCALAR_MS_PER_FRAME=%.6f AUTO_MS_PER_FRAME=%.6f SPEEDUP=%.3f\n",
                t_scalar * 1000.0, t_auto * 1000.0, t_scalar / t_auto);
    return 0;
}
