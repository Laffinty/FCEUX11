#ifndef _VALUEARRAY_H_
#define _VALUEARRAY_H_

// R2.1 (docs/internal/refactor_plan_R1_R5_archive.md §Phase R2): const-correctness overhaul.
// The original had three issues:
//   1. `operator==` was missing `const` — could not be called on
//      `const ValueArray` (e.g., `if (guid1 == guid2)` in a const
//      method).
//   2. `operator!=` was defined as `return !operator==(other);` —
//      this is undefined behaviour: `operator==` is non-const, so
//      `*this` is passed as a non-const lvalue to a method that
//      may mutate. The original code "happened to work" because
//      operator== is logically const, but the signature lied.
//   3. `operator[]` had no const overload — `data[i]` from a
//      `const ValueArray` was a compile error.
//
// Layout: `T data[N]` is preserved (FCEU_Guid and MD5DATA inherit
// from ValueArray<uint8, 16> and rely on the 16-byte layout for
// SFORMAT serialization). Only method signatures and const-ness
// change. No new members, no reordering.
template<typename T, int N>
struct ValueArray
{
	T data[N];
	static const int size = N;

	[[nodiscard]] T&       operator[](int index)       noexcept { return data[index]; }
	[[nodiscard]] const T& operator[](int index) const noexcept { return data[index]; }

	[[nodiscard]] bool operator==(const ValueArray& other) const noexcept {
		for (int i = 0; i < size; ++i) {
			if (data[i] != other[i]) return false;
		}
		return true;
	}
	[[nodiscard]] bool operator!=(const ValueArray& other) const noexcept {
		return !(*this == other);
	}
};

#endif

