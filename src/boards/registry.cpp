// registry.cpp - v1.8 Masonry §2 MapperEntry registry implementation.
//
// The Meyers-singleton storage pattern (function-local `static MapperEntry[]`)
// makes the array's initialization order well-defined: it is constructed on
// the first call to `registry_storage()`, which `MapperEntryRegister`'s
// constructor invokes *before* assigning its entry.  Every per-board-file
// `kXxxRegister` static instance then sees an initialized array, regardless
// of the link-time TU order.
//
// Total storage cost: 256 * sizeof(MapperEntry).  At ~80 bytes per entry
// (function pointer + name pointer + std::function storage) the .bss footprint
// is ~20 KiB.  Acceptable per plan §10.

#include "registry.h"

#include <cstring>

// v1.8 Masonry Phase E.1 followup: include the full header for
// Mapper406Cart to avoid std::unique_ptr conversion errors.  The
// forward declaration is insufficient because std::unique_ptr<Mapper>
// cannot implicitly convert from std::unique_ptr<Mapper406Cart> without
// the complete type at the lambda's call site.
#include "boards/mmc3_variants_carts.h"  // Mapper406Cart full type
struct CartInfo;
void Mapper406_Init(CartInfo* info);  // defined in mmc3.cpp (global namespace)

namespace fceu11 {

namespace {

// v1.8 Masonry §2: registry size.  Originally 256 (matching the v1.0
// 8-bit iNES mapper number), but v1.8 P0 mappers include 406 (MMC3
// BMC Pirate 406) and the build plan §14 covers mappers 0-65535
// (iNES 2.0 extended the mapper number to 16 bits).  The D.10 commit
// added 406 to test_mmc3_variants_factory_dispatch without realizing
// the registry silently dropped it.  Expanded to 512 to cover all
// currently-supported mappers; v1.9 Chronicle §9.x will use a flat
// hash or B-tree for the full 65536-entry range if needed.
constexpr size_t kRegistrySize = 512;

// v1.8 Masonry Phase E.1 followup: keepalive reference array.  This
// `volatile` pointer array is defined here and its address is taken by
// find_mapper (forcing this TU to be retained).  Each MapperEntryRegister
// constructor writes its `this` to a slot in this array, ensuring the
// static instance is constructed at static-init time.  The array is
// sized to match kRegistrySize (512) so any mapper number that the
// registry accepts (0..511) can register without bounds issues.
//
// Why this works when the previous `volatile g_registration_count`
// did not: the compiler treats a write to a `volatile` global as
// "the write could be observed elsewhere, must keep the writer alive".
// A discarded read of a volatile global (the prior attempt) can still
// be optimized away at link time if the writer is in a discarded TU.
// Storing `this` into a volatile array forces the writer (i.e. the
// static instance) to be retained, since the array's address is
// referenced by find_mapper.
//
// hotfix1 P2-6 (H-19): the array used to be hard-coded to 256 entries
// while kRegistrySize was 512. The constructor and find_mapper both
// guard with `mapper_number < kRegistrySize`, so the actual code
// path was safe from a buffer overflow — but a mapper numbered 256..511
// would silently fail to be kept-alive (its static instance could be
// DCE-stripped by the linker) and disappear at runtime. Grow the
// keepalive to kRegistrySize so the guard's promise and the array
// size agree.
volatile const MapperEntryRegister* g_keepalive[kRegistrySize] = {};

// Meyers singleton: returns a pointer to the 256-entry MapperEntry array.
// First-call initialization is guaranteed thread-safe by C++11 magic statics.
MapperEntry* registry_storage() noexcept {
    // Default-constructed MapperEntry has name == nullptr and an empty
    // std::function — used as the "not registered" sentinel.
    static MapperEntry registry[kRegistrySize];
    return registry;
}

}  // namespace

MapperEntryRegister::MapperEntryRegister(const MapperEntry& entry) noexcept {
    if (entry.mapper_number < kRegistrySize) {
        registry_storage()[entry.mapper_number] = entry;
        // v1.8 Masonry Phase E.1 followup: store `this` in the volatile
        // keepalive array so the linker must retain this static
        // instance.  The volatile write is observable (find_mapper
        // reads the array below), so the static cannot be DCE-stripped.
        g_keepalive[entry.mapper_number] = this;
    }
    // mapper_number >= kRegistrySize (512) is a static-init bug; silently drop.
}

const MapperEntry* find_mapper(uint32_t number) noexcept {
    if (number >= kRegistrySize) {
        return nullptr;
    }
    // v1.8 Masonry Phase E.1 followup: read the volatile keepalive
    // array so the linker retains every MapperEntryRegister static
    // instance (the constructor writes to it).  The read of
    // g_keepalive[number] is then discarded — only the side effect
    // of forcing the link matters.
    (void)g_keepalive[number];
    const MapperEntry& entry = registry_storage()[number];
    // v1.8 Masonry Phase E.1 final fallback: MSVC's COMDAT folding
    // has been observed to DCE-strip the kMapper406Register static
    // initializer in src/boards/mmc3_406_register.cpp despite
    // multiple keepalive strategies.  As a last-resort fallback,
    // register 406 directly here if the registry slot is empty.
    // This is a one-time check on first access; subsequent calls
    // are no-ops via the static `done` guard.
    if (number == 406 && entry.name == nullptr) {
        static bool done = false;
        if (!done) {
            done = true;
            std::function<std::unique_ptr<Mapper>(Bus&)> factory_fn =
                [](Bus& bus) -> std::unique_ptr<Mapper> {
                    return std::make_unique<Mapper406Cart>(bus);
                };
            MapperEntry fallback_entry;
            fallback_entry.mapper_number = 406;
            fallback_entry.name = "MMC3 BMC Pirate 406";
            fallback_entry.legacy_init = &Mapper406_Init;
            fallback_entry.factory = factory_fn;
            static MapperEntryRegister* const kFallback406 =
                new MapperEntryRegister(fallback_entry);
            (void)kFallback406;
            return registry_storage()[406].name
                       ? &registry_storage()[406] : nullptr;
        }
    }
    return entry.name ? &entry : nullptr;
}

const MapperEntry* find_mapper_by_name(const char* name) noexcept {
    if (!name) {
        return nullptr;
    }
    MapperEntry* reg = registry_storage();
    for (size_t i = 0; i < kRegistrySize; ++i) {
        if (reg[i].name && std::strcmp(reg[i].name, name) == 0) {
            return &reg[i];
        }
    }
    return nullptr;
}

}  // namespace fceu11