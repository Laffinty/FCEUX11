// FCEUX11 — v1.7 Cartograph §1: fceu11::Cart class implementation shell.
//
// Phase B only provides the global instance wiring and trivial scaffolding
// bodies. The real state migration (SaveGame, ROM metadata, lifecycle
// virtualization, expansion-audio hook) lands in Phase C~E.

#include "cart_class.h"

#include <cstring>

namespace fceu11 {

namespace {

// Placeholder cart used until a concrete mapper subclass is assigned during
// ROM loading. It implements the abstract Cart lifecycle methods as no-ops.
// Keeping g_cart non-null lets fceu.cpp::Initialize() call attach_bus()
// unconditionally without a null-pointer guard.
class CartPlaceholder : public Cart {
public:
    void on_power() noexcept override {}
    void on_reset() noexcept override {}
    void on_close() noexcept override {}
};

CartPlaceholder g_cart_placeholder;

} // namespace

Cart* g_cart = &g_cart_placeholder;

Cart::Cart() noexcept = default;

// ---------------------------------------------------------------------------
// ROM metadata setters
// ---------------------------------------------------------------------------

void Cart::set_md5(const uint8_t (&md5)[16]) noexcept {
    std::memcpy(md5_.data(), md5, md5_.size());
}

// ---------------------------------------------------------------------------
// Battery save (Phase C1 stub)
// ---------------------------------------------------------------------------

bool Cart::has_battery() const noexcept {
    return battery_present();
}

bool Cart::save_battery(const std::filesystem::path& /*path*/) const {
    // Phase C1: serialize CartInfo::SaveGame buffers to disk.
    return false;
}

bool Cart::load_battery(const std::filesystem::path& /*path*/) const {
    // Phase C1: deserialize CartInfo::SaveGame buffers from disk.
    return false;
}

void Cart::clear_battery() noexcept {
    // Phase C1: clear registered save-game buffers.
}

// ---------------------------------------------------------------------------
// SaveGame buffer registration (Phase C1 stub)
// ---------------------------------------------------------------------------

void Cart::addSaveGameBuf(uint8_t* bufptrIn, uint32_t buflenIn,
                          void (*resetFuncIn)(void)) {
    SaveGame_t tmp;
    tmp.bufptr = bufptrIn;
    tmp.buflen = buflenIn;
    tmp.resetFunc = resetFuncIn;
    save_games_.push_back(tmp);
}

// ---------------------------------------------------------------------------
// Expansion-audio installation (Phase E stub)
// ---------------------------------------------------------------------------

void Cart::install_expansion_audio(class Apu& /*apu*/) noexcept {
    // Default: no expansion audio. Vrc6Cart and friends override in Phase E.
}

// ---------------------------------------------------------------------------
// Savestate chunk registration (Phase D stub)
// ---------------------------------------------------------------------------

void Cart::register_state_chunk(const char* /*name*/, void* /*data*/,
                                uint32_t /*size*/, uint32_t /*count*/) {
    // Phase D: forward to the real SFORMAT registration in state.cpp.
}

} // namespace fceu11
