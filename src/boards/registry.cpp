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

// v1.8 Masonry Phase E.1 followup: keepalive reference array.  This
// `volatile` pointer array is defined here and its address is taken by
// find_mapper (forcing this TU to be retained).  Each MapperEntryRegister
// constructor writes its `this` to a slot in this array, ensuring the
// static instance is constructed at static-init time.  The array is
// sized to hold all 256 possible mapper numbers, so any number can
// register without bounds issues.
//
// Why this works when the previous `volatile g_registration_count`
// did not: the compiler treats a write to a `volatile` global as
// "the write could be observed elsewhere, must keep the writer alive".
// A discarded read of a volatile global (the prior attempt) can still
// be optimized away at link time if the writer is in a discarded TU.
// Storing `this` into a volatile array forces the writer (i.e. the
// static instance) to be retained, since the array's address is
// referenced by find_mapper.
volatile const MapperEntryRegister* g_keepalive[256] = {};

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
    // mapper_number >= 256 is a static-init bug; silently drop in release.
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