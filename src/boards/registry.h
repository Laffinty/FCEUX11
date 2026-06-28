// registry.h - v1.8 Masonry §2 (Roadmap §8.4) MapperEntry static registry.
//
// Phase C: replaces v1.7's `create_cart_for_mapper` switch in cart_class.cpp
// with a per-board-file `kXxxRegister` static instance that calls
// MapperEntryRegister's constructor. The constructor stores the MapperEntry
// in a Meyers-singleton array indexed by mapper number.
//
// Lookup:
//   const MapperEntry* find_mapper(uint32_t number) noexcept;
//   const MapperEntry* find_mapper_by_name(const char* name) noexcept;
//
// Each board file (.cpp) that wants a Cart subclass dispatches via:
//
//   namespace {
//   static ::fceu11::MapperEntryRegister kNromRegister{
//       ::fceu11::MapperEntry{
//           /*mapper_number=*/0,
//           /*name=*/"NROM",
//           /*legacy_init=*/&NROM_Init,
//           /*factory=*/[](::fceu11::Bus& bus) {
//               return std::make_unique<NromCart>(bus);
//           }
//       }
//   };
//   }
//
// The factory lambda captures anything it needs (e.g. VRC6 captures the
// mapper number 24 or 26 so Vrc6Cart can dispatch to Mapper24_Init or
// Mapper26_Init inside its on_power()).

#ifndef FCEU11_BOARDS_REGISTRY_H
#define FCEU11_BOARDS_REGISTRY_H

#include <cstdint>
#include <cstddef>
#include <functional>
#include <memory>

#include "../cart_class.h"   // fceu11::Mapper, fceu11::Bus (forward decls)

// CartInfo lives in the global namespace (declared in src/cart.h); forward-
// declare it here so the MapperEntry struct can reference the same type the
// legacy Init functions use (e.g. NROM_Init, Mapper1_Init).
struct CartInfo;

namespace fceu11 {

struct MapperEntry {
    uint32_t mapper_number;                                   // iNES mapper id
    const char* name;                                         // static str literal
    void (*legacy_init)(CartInfo*);                           // v1.0 init (compat)
    std::function<std::unique_ptr<Mapper>(Bus&)> factory;     // v1.8+ factory
};

// Look up by mapper number.  Returns nullptr if no entry registered for `number`
// or if `number >= 256`.  Always safe to call from any TU at any time.
const MapperEntry* find_mapper(uint32_t number) noexcept;

// Look up by name (UNIF loader uses this).  Linear scan over the 256-entry
// registry; UNIF board count is small so the cost is negligible.
const MapperEntry* find_mapper_by_name(const char* name) noexcept;

// Constructor helper: each board file declares a static instance of this
// type.  The constructor appends the entry to the Meyers-singleton registry.
// Defined in registry.cpp.
struct MapperEntryRegister {
    explicit MapperEntryRegister(const MapperEntry& entry) noexcept;
};

}  // namespace fceu11

#endif  // FCEU11_BOARDS_REGISTRY_H