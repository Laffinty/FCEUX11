// FCEUX11 — v1.7 Cartograph §1: fceu11::Cart class implementation shell.
//
// Phase B only provides the global instance wiring and trivial scaffolding
// bodies. The real state migration (SaveGame, ROM metadata, lifecycle
// virtualization, expansion-audio hook) lands in Phase C~E.

#include "cart_class.h"

#include "cart.h"             // currCartInfo, CartInfo::SaveGame_t
#include "boards/nrom_cart.h" // v1.7 Phase E: NromCart PoC subclass
#include "boards/vrc6_cart.h" // v1.7 Phase E: Vrc6Cart PoC subclass (24/26)
#include "boards/mmc1_cart.h" // v1.7 Phase F: Mmc1Cart PoC subclass
#include "boards/mmc3_cart.h" // v1.7 Phase F: Mmc3Cart PoC subclass
#include "rust/fceux11_rust.h" // FceuSaveGameEntry, battery Rust functions

#include <cstring>
#include <vector>

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

// Owning pointer for the concrete cart returned by create_cart_for_mapper().
// When nullptr, g_cart points at g_cart_placeholder (above). When set, g_cart
// aliases g_cart_owner.get() so the existing raw-pointer API surface
// (fceu11::g_cart, currCartInfo->cart_obj) keeps working unchanged.
std::unique_ptr<Cart> g_cart_owner;

} // namespace

Cart* g_cart = &g_cart_placeholder;

std::unique_ptr<Cart> create_cart_for_mapper(uint32_t mapper_no, Bus& bus) {
    // Phase E/F: NROM (0) + VRC6 (24, 26) + MMC1 (1) + MMC3 (4) are the PoC
    // subclasses. All other mappers return nullptr, which keeps them on the
    // legacy CartInfo function-pointer path through the v1.7 compat layer.
    switch (mapper_no) {
    case 0:
        return std::make_unique<NromCart>(bus);
    case 1:
        return std::make_unique<Mmc1Cart>(bus);
    case 4:
        return std::make_unique<Mmc3Cart>(bus);
    case 24:
    case 26:
        return std::make_unique<Vrc6Cart>(bus, mapper_no);
    default:
        return nullptr;
    }
}

void assign_cart(std::unique_ptr<Cart> cart) {
    g_cart_owner = std::move(cart);
    g_cart = g_cart_owner ? g_cart_owner.get() : &g_cart_placeholder;
}

Cart::Cart() noexcept = default;

// ---------------------------------------------------------------------------
// ROM metadata setters
// ---------------------------------------------------------------------------

void Cart::set_md5(const uint8_t (&md5)[16]) noexcept {
    std::memcpy(md5_.data(), md5, md5_.size());
    if (currCartInfo) {
        std::memcpy(currCartInfo->MD5, md5, sizeof(currCartInfo->MD5));
    }
}

void Cart::set_crc32(uint32_t v) noexcept {
    crc32_ = v;
    if (currCartInfo) currCartInfo->CRC32 = v;
}

void Cart::set_mirror(int m) noexcept {
    mirror_ = m;
    if (currCartInfo) currCartInfo->mirror = m;
}

void Cart::set_mirror_as_2bits(int m) noexcept {
    mirror_as_2bits_ = m;
    if (currCartInfo) currCartInfo->mirrorAs2Bits = m;
}

void Cart::set_battery(bool b) noexcept {
    battery_ = b ? 1 : 0;
    if (currCartInfo) currCartInfo->battery = b ? 1 : 0;
}

void Cart::set_ines2(bool b) noexcept {
    ines2_ = b ? 1 : 0;
    if (currCartInfo) currCartInfo->ines2 = b ? 1 : 0;
}

void Cart::set_submapper(int s) noexcept {
    submapper_ = s;
    if (currCartInfo) currCartInfo->submapper = s;
}

void Cart::set_wram_size(int s) noexcept {
    wram_size_ = s;
    if (currCartInfo) currCartInfo->wram_size = s;
}

void Cart::set_battery_wram_size(int s) noexcept {
    battery_wram_size_ = s;
    if (currCartInfo) currCartInfo->battery_wram_size = s;
}

void Cart::set_vram_size(int s) noexcept {
    vram_size_ = s;
    if (currCartInfo) currCartInfo->vram_size = s;
}

void Cart::set_battery_vram_size(int s) noexcept {
    battery_vram_size_ = s;
    if (currCartInfo) currCartInfo->battery_vram_size = s;
}

void Cart::set_mapper_number(uint32_t n) noexcept {
    mapper_number_ = n;
}

// ---------------------------------------------------------------------------
// Battery save (Phase C1 stub)
// ---------------------------------------------------------------------------

bool Cart::has_battery() const noexcept {
    return battery_present();
}

bool Cart::save_battery(const std::filesystem::path& path) const {
    if (!has_battery() || save_games_.empty()) {
        return false;
    }

    std::vector<FceuSaveGameEntry> entries;
    entries.reserve(save_games_.size());
    for (const auto& sg : save_games_) {
        if (sg.bufptr) {
            entries.push_back({sg.bufptr, sg.buflen});
        }
    }
    const auto path_str = path.string();
    return fceux11_rust_cart_battery_save(path_str.c_str(), entries.data(),
                                          entries.size());
}

bool Cart::load_battery(const std::filesystem::path& path) const {
    if (!has_battery() || save_games_.empty()) {
        return false;
    }

    std::vector<FceuSaveGameEntry> entries;
    entries.reserve(save_games_.size());
    for (auto& sg : save_games_) {
        if (sg.bufptr) {
            entries.push_back({sg.bufptr, sg.buflen});
        }
    }
    const auto path_str = path.string();
    return fceux11_rust_cart_battery_load(path_str.c_str(), entries.data(),
                                          entries.size());
}

void Cart::clear_battery() noexcept {
    if (save_games_.empty()) {
        return;
    }

    std::vector<FceuSaveGameEntry> entries;
    entries.reserve(save_games_.size());
    for (const auto& sg : save_games_) {
        if (sg.bufptr) {
            entries.push_back({sg.bufptr, sg.buflen});
        }
    }
    fceux11_rust_cart_battery_clear(entries.data(), entries.size());

    for (auto& sg : save_games_) {
        if (sg.resetFunc) {
            sg.resetFunc();
        }
    }

    save_games_.clear();
    if (currCartInfo) {
        currCartInfo->SaveGame.clear();
    }
}

// ---------------------------------------------------------------------------
// SaveGame buffer registration (Phase C1)
// ---------------------------------------------------------------------------

void Cart::addSaveGameBuf(uint8_t* bufptrIn, uint32_t buflenIn,
                          void (*resetFuncIn)(void)) {
    SaveGame_t tmp;
    tmp.bufptr = bufptrIn;
    tmp.buflen = buflenIn;
    tmp.resetFunc = resetFuncIn;
    save_games_.push_back(tmp);

    // Dual-write to the legacy CartInfo::SaveGame vector so existing
    // FCEU_SaveGameSave / FCEU_LoadGameSave / FCEU_ClearGameSave paths
    // continue to work for both old board files and new Cart subclasses.
    if (currCartInfo) {
        CartInfo::SaveGame_t ci_tmp;
        ci_tmp.bufptr = bufptrIn;
        ci_tmp.buflen = buflenIn;
        ci_tmp.resetFunc = resetFuncIn;
        currCartInfo->SaveGame.push_back(ci_tmp);
    }
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
