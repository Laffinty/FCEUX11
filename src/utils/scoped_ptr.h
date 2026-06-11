// FCEUX11 — Scoped pointer RAII helper (v0.3.7 / v0.3.8)
// Split out of src/types.h per plan v3 §5 v0.3.7. The legacy
// fceuScopedPtr<T> is now a deprecated alias for std::unique_ptr<T>;
// the fceuAllocType enum (v0.3.7 vintage) was rewritten as
// fceu11::AllocKind in v0.3.8, preserved as a no-op for any out-of-tree
// call sites that still reference it. New code should use
// std::unique_ptr<T> directly (or FceuMallocPtr from utils/memory.h
// for malloc-backed buffers, which provides the matching deleter).

#ifndef __FCEU_SCOPED_PTR_H
#define __FCEU_SCOPED_PTR_H

#include <cstdint>
#include <memory>

// v0.3.8: fceuAllocType was a pre-v0.3.6 RAII helper allocation-kind
// tag. The v0.3.6 RAII migration (commit 314f6d1) replaced fceuScopedPtr
// with std::unique_ptr<T> + custom deleters (utils/memory.h
// FceuMallocDeleter), leaving the enum tree-internally unused. Per plan
// v3 §5 v0.3.8 task 2 ("FCEU_ALLOC_TYPE → enum class AllocKind : u8"),
// the canonical type is now fceu11::AllocKind. The legacy unscoped
// enum names (fceuAllocType + FCEU_ALLOC_TYPE_*) remain as global
// `using` / `inline constexpr` aliases per plan §6.1 phase 1 ("only NEW
// symbols enter fceu11::; OLD symbols stay global"). Removal of the
// aliases is planned for v0.4.0.
//
// Plan §5 v0.3.8 specifies `: u8` for AllocKind; uint8_t is the
// canonical spelling. No -1 sentinel exists in the value list, so the
// unsigned underlying type is unambiguous.
namespace fceu11 {
    enum class AllocKind : uint8_t
    {
        New      = 0,
        NewArray = 1,
        Malloc   = 2,
    };
} // namespace fceu11

// Legacy global aliases — preserve pre-v0.3.8 token stream so any
// out-of-tree consumer that transitively includes "types.h" continues
// to compile. fceuAllocType is a type alias; the FCEU_ALLOC_TYPE_*
// constants are constexpr enumerator values (NOT preprocessor macros)
// to keep `static_cast<fceuAllocType>(int)` round-trips well-defined.
using fceuAllocType = fceu11::AllocKind;
inline constexpr fceuAllocType FCEU_ALLOC_TYPE_NEW       = fceu11::AllocKind::New;
inline constexpr fceuAllocType FCEU_ALLOC_TYPE_NEW_ARRAY = fceu11::AllocKind::NewArray;
inline constexpr fceuAllocType FCEU_ALLOC_TYPE_MALLOC    = fceu11::AllocKind::Malloc;

// v0.3.6 + v0.3.7: fceuScopedPtr<T> is a deprecated alias for
// std::unique_ptr<T>. The [[deprecated]] attribute is gated by
// FCEUX11_NO_DEPRECATION_WARNINGS (provided until v0.4.0, per plan v3
// §6.3) so the one remaining in-tree call site (src/state.cpp:FCEUSS_Load)
// and any external code keep building during the v0.3.x transition
// window. Migration path:
//   fceuScopedPtr<T> x = new T;       → std::unique_ptr<T> x(new T);
//   fceuScopedPtr<T> x;                → std::unique_ptr<T> x;
//   fceuScopedPtr<T> x(raw_p);         → std::unique_ptr<T> x(raw_p);
#if !defined(FCEUX11_NO_DEPRECATION_WARNINGS)
template <typename T>
using fceuScopedPtr [[deprecated("fceuScopedPtr is deprecated since v0.3.6; use std::unique_ptr<T> (or FceuMallocPtr from utils/memory.h for malloc-backed buffers)")]] = std::unique_ptr<T>;
#else
template <typename T>
using fceuScopedPtr = std::unique_ptr<T>;
#endif

#endif // __FCEU_SCOPED_PTR_H
