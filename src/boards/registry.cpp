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

namespace fceu11 {

namespace {

constexpr size_t kRegistrySize = 256;

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
    }
    // mapper_number >= 256 is a static-init bug; silently drop in release.
}

const MapperEntry* find_mapper(uint32_t number) noexcept {
    if (number >= kRegistrySize) {
        return nullptr;
    }
    const MapperEntry& entry = registry_storage()[number];
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