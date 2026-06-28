// FCEUX11 — v1.7 Cartograph §1: fceu11::Cart class declaration.
//
// Goal: objectify the v1.0 CartInfo C structure into an fceu11::Cart class
// using the same pattern as Cpu / Bus / Ppu / Apu. Phase B only erects the
// class shell, MirrorMode enum, and Mapper base; the 17 CartInfo fields and
// SaveGame vector migrate in Phase C, lifecycle virtualization lands in
// Phase D, and the NROM/MMC1/MMC3 PoC subclasses land in Phase E/F.
//
// Compat: cart.h keeps struct CartInfo as the v1.0 API surface and adds a
// cart_obj back-pointer plus Power/Reset/Close forwarding functions so the
// 168 un-migrated src/boards/*.cpp files continue to compile and behave
// unchanged.

#ifndef FCEU11_CART_CLASS_H
#define FCEU11_CART_CLASS_H

#include <array>
#include <cstdint>
#include <cstddef>
#include <filesystem>
#include <memory>
#include <vector>

#include "types.h"
#include "utils/cache.h"       // FCEUX11_CACHE_ALIGN

namespace fceu11 {

// Forward declarations to keep this header free of bus.h / apu.h (which
// would pull in the full Bus / Apu surface for every translation unit).
class Bus;
class Mapper;

// ---------------------------------------------------------------------------
// Pack helpers for save_mapper_state() (v1.8 §6)
// ---------------------------------------------------------------------------
// Inline so every Mapper subclass can call them without per-TU codegen
// overhead.  Endianness: little-endian (matches the savestate .fc0 header
// and the Rust state_file crate's u32 LE encoding).

inline void pack_u8(std::vector<uint8_t>& out, uint8_t v) noexcept {
    out.push_back(v);
}

inline void pack_u8_array(std::vector<uint8_t>& out,
                          const uint8_t* src, size_t n) noexcept {
    out.insert(out.end(), src, src + n);
}

inline void pack_u16(std::vector<uint8_t>& out, uint16_t v) noexcept {
    out.push_back(static_cast<uint8_t>(v & 0xff));
    out.push_back(static_cast<uint8_t>((v >> 8) & 0xff));
}

inline void pack_u32(std::vector<uint8_t>& out, uint32_t v) noexcept {
    out.push_back(static_cast<uint8_t>(v & 0xff));
    out.push_back(static_cast<uint8_t>((v >> 8) & 0xff));
    out.push_back(static_cast<uint8_t>((v >> 16) & 0xff));
    out.push_back(static_cast<uint8_t>((v >> 24) & 0xff));
}

// Forward-declare unpack counterparts so subclasses can implement
// load_mapper_state symmetrically without exposing the vector type to
// board.cpp's C-style header.
void unpack_u8(const std::vector<uint8_t>& body, size_t& pos, uint8_t& out) noexcept;
void unpack_u16(const std::vector<uint8_t>& body, size_t& pos, uint16_t& out) noexcept;
void unpack_u32(const std::vector<uint8_t>& body, size_t& pos, uint32_t& out) noexcept;

// MirrorMode enum class (v1.7 §2.3) — replaces MI_H / MI_V / MI_0 / MI_1
// macros. The legacy macros in cart.h are kept as int aliases so existing
// board files continue to compile unchanged.
enum class MirrorMode : uint8_t {
    Horizontal = 0,    // 原 MI_H
    Vertical   = 1,    // 原 MI_V
    Mode0      = 2,    // 原 MI_0 (single-screen low)
    Mode1      = 3,    // 原 MI_1 (single-screen high)
};

#ifdef _MSC_VER
#pragma warning(push)
#pragma warning(disable: 4324)
#endif
class FCEUX11_CACHE_ALIGN Cart {
public:
    Cart() noexcept;
    virtual ~Cart() = default;

    // ---- Lifecycle (v1.0 Power / Reset / Close function pointers) ----
    // Phase B: pure virtual; concrete subclasses implement them.
    virtual void on_power() noexcept = 0;
    virtual void on_reset() noexcept = 0;
    virtual void on_close() noexcept = 0;

    // ---- Savestate hooks (v1.4 vrc7_PreSave deferred interface) ----
    // Phase B/D: default no-op; v1.14 Anvil LTO may override safely.
    virtual void on_save_pre() noexcept {}
    virtual void on_load_post() noexcept {}

    // ---- Battery save (v1.0 SaveGame vector objectization) ----
    // Phase B: scaffolding declarations; Phase C1 fills the bodies.
    bool has_battery() const noexcept;
    bool save_battery(const std::filesystem::path& path) const;
    bool load_battery(const std::filesystem::path& path) const;
    void clear_battery() noexcept;

    // ---- ROM metadata (v1.0 CartInfo 17 fields) ----
    // Phase B: scaffolding getters/setters; Phase C2 wires the setters to
    // iNES/UNIF parsing and keeps CartInfo fields in sync.
    const std::array<uint8_t, 16>& md5() const noexcept;
    uint32_t crc32() const noexcept { return crc32_; }
    int mirror_raw() const noexcept { return mirror_; }
    int mirror_as_2bits() const noexcept { return mirror_as_2bits_; }
    bool battery_present() const noexcept { return battery_ != 0; }
    bool ines2() const noexcept { return ines2_ != 0; }
    int submapper() const noexcept { return submapper_; }
    int wram_size() const noexcept { return wram_size_; }
    int battery_wram_size() const noexcept { return battery_wram_size_; }
    int vram_size() const noexcept { return vram_size_; }
    int battery_vram_size() const noexcept { return battery_vram_size_; }
    uint32_t mapper_number() const noexcept { return mapper_number_; }

    // Phase C2: setters are defined in cart_class.cpp so they can dual-write
    // to the legacy CartInfo fields (currCartInfo) while populating Cart.
    void set_md5(const uint8_t (&md5)[16]) noexcept;
    void set_crc32(uint32_t v) noexcept;
    void set_mirror(int m) noexcept;
    void set_mirror_as_2bits(int m) noexcept;
    void set_battery(bool b) noexcept;
    void set_ines2(bool b) noexcept;
    void set_submapper(int s) noexcept;
    void set_wram_size(int s) noexcept;
    void set_battery_wram_size(int s) noexcept;
    void set_vram_size(int s) noexcept;
    void set_battery_vram_size(int s) noexcept;
    void set_mapper_number(uint32_t n) noexcept;

    // ---- v1.0 compatible SaveGame buffer registration ----
    struct SaveGame_t {
        uint8_t* bufptr;
        uint32_t buflen;
        void (*resetFunc)(void);
    };
    void addSaveGameBuf(uint8_t* bufptrIn, uint32_t buflenIn,
                        void (*resetFuncIn)(void) = nullptr);
    const std::vector<SaveGame_t>& save_games() const noexcept { return save_games_; }

    // ---- Expansion-audio installation (v1.6 §11.1 contract) ----
    // Phase B: default no-op; Phase E Vrc6Cart overrides this to inject
    // g_vrc6_audio into g_apu via set_exp_sound().
    virtual void install_expansion_audio(class Apu& apu) noexcept;

    // ---- v1.8 Masonry §6: mapper-internal state snapshot ----
    // Default implementation returns an empty body (legacy carts work as-is).
    // Subclasses override to expose private registers / PRGptr / CHRptr /
    // IRQ counters via the pack helpers below.  The harness (mapper_byte_diff_test)
    // prepends a header [mapper_number | body_size] before persisting to disk.
    virtual std::vector<uint8_t> save_mapper_state() const noexcept;
    virtual bool load_mapper_state(const std::vector<uint8_t>& body) noexcept;

    // ---- Savestate chunk registration helper (v1.9 Savestate V2 prep) ----
    // Phase B: scaffolding; Phase D wires the real SFORMAT registration.
    void register_state_chunk(const char* name, void* data,
                              uint32_t size, uint32_t count = 1);

    // ---- Bus injection (same pattern as Bus::attach_ppu) ----
    void attach_bus(Bus& bus) noexcept { bus_ = &bus; }
    Bus* bus_ptr() const noexcept { return bus_; }

protected:
    // Subclasses operate on the Bus through this pointer.
    Bus* bus_ = nullptr;
    uint32_t mapper_number_ = 0;

private:
    // ROM metadata (Phase C2 populates these from iNES/UNIF loaders).
    std::array<uint8_t, 16> md5_{};
    uint32_t crc32_ = 0;
    int mirror_ = 0;
    int mirror_as_2bits_ = 0;
    int battery_ = 0;
    int ines2_ = 0;
    int submapper_ = 0;
    int wram_size_ = 0;
    int battery_wram_size_ = 0;
    int vram_size_ = 0;
    int battery_vram_size_ = 0;

    // Objectized SaveGame vector (Phase C1 connects to CartInfo::SaveGame).
    std::vector<SaveGame_t> save_games_;
};
#ifdef _MSC_VER
#pragma warning(pop)
#endif

// Mapper base class (v1.7 §7.2). Holds a Bus reference and will grow
// bank-switching convenience methods + IRQ hooks as v1.7/v1.8 progress.
// Phase B: thin abstract subclass of Cart.
class Mapper : public Cart {
public:
    Mapper() noexcept = default;
    explicit Mapper(Bus& bus) noexcept { attach_bus(bus); }
};

// Global cart pointer. Initialized in cart_class.cpp to a no-op placeholder
// so fceu.cpp::Initialize() can safely call attach_bus() before any ROM is
// loaded. A concrete mapper subclass replaces the pointer during iNES/UNIF
// loading (Phase E/F).
extern Cart* g_cart;

// Phase D factory: create a concrete Cart subclass for a given iNES mapper
// number. Phase D stub returns nullptr for all mappers; Phase E/F override
// for NROM (0), MMC1 (1), and MMC3 (4).
std::unique_ptr<Cart> create_cart_for_mapper(uint32_t mapper_no, Bus& bus);

// Take ownership of a newly-created cart and update the global g_cart pointer.
// Passing nullptr restores g_cart to the no-op placeholder.
void assign_cart(std::unique_ptr<Cart> cart);

} // namespace fceu11
#endif
