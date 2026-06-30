// FCEUX11 — v1.6 Resonance §3: ExpansionAudio interface + EXPSOUND adapter.
//
// Abstract base class for mapper/expansion-board audio devices (VRC6,
// VRC7, FDS, MMC5, Namco163, Sunsoft5B, etc.). v1.6 only defines the
// interface and the EXPSOUND adapter; actual chip subclassing is
// deferred to v1.8, except for the VRC6 proof-of-concept in Phase E.
//
// The EXPSOUND struct is kept as the v1.0 ABI: board files continue to
// fill its function pointers. The new `expansion` field lets the core
// prefer a C++ object when one is provided, while the function-pointer
// fields remain as the fallback path.

#ifndef FCEU11_EXPANSION_AUDIO_H
#define FCEU11_EXPANSION_AUDIO_H

#include <cstdint>
#include <cstddef>

namespace fceu11 {

class ExpansionAudio {
public:
    virtual ~ExpansionAudio() = default;

    // Low-quality (LQ) mix path. Equivalent to EXPSOUND::Fill.
    virtual void fill(int32_t count) = 0;

    // High-quality (HQ) fill path. Equivalent to EXPSOUND::HiFill.
    virtual void hi_fill() = 0;

    // HQ sample-rate resync after filtering. Equivalent to EXPSOUND::HiSync.
    virtual void hi_sync(int32_t ts) = 0;

    // Called when region / sample rate changes. Equivalent to EXPSOUND::RChange.
    virtual void region_changed() = 0;

    // Cleanup (e.g. VRC7 OPLL_free). Equivalent to EXPSOUND::Kill.
    virtual void kill() = 0;

    // VRC7-specific LQ path. Kept out of the pure interface so that other
    // chips are not forced to implement it. Default no-op.
    virtual void neo_fill(int32_t* wave, int32_t count) {}
};

} // namespace fceu11

// v1.0 expansion-sound adapter. Board files populate the function pointers
// as before. The optional `expansion` field points to a C++ subclass that
// the core will prefer when non-null.
typedef struct {
    void (*Fill)(int Count);           // LQ mix path
    void (*NeoFill)(int32 *Wave, int Count); // VRC7 LQ path
    void (*HiFill)(void);              // HQ fill path
    void (*HiSync)(int32 ts);          // HQ resync
    void (*RChange)(void);             // region / rate change
    void (*Kill)(void);                // cleanup

    // v1.6 Resonance: optional object-oriented backend. When non-null,
    // FlushEmulateSound routes through this object instead of the function
    // pointers above.
    fceu11::ExpansionAudio* expansion;
} EXPSOUND;

// ---------------------------------------------------------------------------
// Adapter helpers: core code calls these instead of dereferencing the function
// pointers directly. They prefer an installed ExpansionAudio object, falling
// back to the legacy function pointers when no object is present.
// ---------------------------------------------------------------------------

static inline void FCEU11_ExpFill(EXPSOUND* es, int count) {
    if (es && es->expansion) {
        es->expansion->fill(count);
    } else if (es && es->Fill) {
        es->Fill(count);
    }
}

static inline void FCEU11_ExpNeoFill(EXPSOUND* es, int32_t* wave, int count) {
    if (es && es->expansion) {
        es->expansion->neo_fill(wave, count);
    } else if (es && es->NeoFill) {
        es->NeoFill(wave, count);
    }
}

static inline void FCEU11_ExpHiFill(EXPSOUND* es) {
    if (es && es->expansion) {
        es->expansion->hi_fill();
    } else if (es && es->HiFill) {
        es->HiFill();
    }
}

static inline void FCEU11_ExpHiSync(EXPSOUND* es, int32_t ts) {
    if (es && es->expansion) {
        es->expansion->hi_sync(ts);
    } else if (es && es->HiSync) {
        es->HiSync(ts);
    }
}

static inline void FCEU11_ExpRegionChanged(EXPSOUND* es) {
    if (es && es->expansion) {
        es->expansion->region_changed();
    } else if (es && es->RChange) {
        es->RChange();
    }
}

static inline void FCEU11_ExpKill(EXPSOUND* es) {
    if (es && es->expansion) {
        es->expansion->kill();
    } else if (es && es->Kill) {
        es->Kill();
    }
}

// C-style callback suitable for passing to code that only understands function
// pointers (e.g. the Rust neo filter). It forwards to the installed
// ExpansionAudio::neo_fill() or EXPSOUND::NeoFill via GameExpSound.
void FCEU11_ExpNeoFillCallback(int32_t* wave, int count);

#endif // FCEU11_EXPANSION_AUDIO_H
