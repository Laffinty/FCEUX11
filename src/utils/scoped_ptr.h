// FCEUX11 — Scoped pointer RAII helper (v0.3.7)
// Split out of src/types.h per plan v3 §5 v0.3.7. The legacy
// fceuScopedPtr<T> is now a deprecated alias for std::unique_ptr<T>;
// the fceuAllocType enum is preserved as a no-op for any out-of-tree
// call sites that still reference it. New code should use
// std::unique_ptr<T> directly (or FceuMallocPtr from utils/memory.h
// for malloc-backed buffers, which provides the matching deleter).

#ifndef __FCEU_SCOPED_PTR_H
#define __FCEU_SCOPED_PTR_H

#include <memory>

// fceuAllocType was the pre-v0.3.6 RAII helper's allocation-kind tag.
// The v0.3.6 RAII migration (commit 314f6d1) replaced fceuScopedPtr with
// std::unique_ptr<T> + custom deleters (utils/memory.h FceuMallocDeleter).
// fceuAllocType is now unused inside the tree; the enum is preserved
// here as a no-op for any out-of-tree code that still references the
// symbol via #include "types.h" transitively. The values are kept for
// ABI / source-level compatibility; the values themselves are
// meaningless (no runtime check uses them).
enum fceuAllocType
{
	FCEU_ALLOC_TYPE_NEW = 0,
	FCEU_ALLOC_TYPE_NEW_ARRAY,
	FCEU_ALLOC_TYPE_MALLOC
};

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
