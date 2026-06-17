// DirectStorageProbe.h
//
// v0.3.15.x PHASE-3: DirectStorage 1.2 NVMe probe scaffold.
// This file ONLY probes the host for DirectStorage support; it does NOT
// take over any I/O. The actual IDStorageFactory / IDStorageQueue
// takeover of .fc0/.fcs writes is deferred to v0.4.x (see plan v3
// §5 v0.3.15 "Win11 platform features" subset + docs/v0.3.15_Build_Plan.md
// PHASE-3 task 3.2).
//
// Probe results are cached in a global static so the cost is paid at
// most once per process. The state.cpp TODO comment references the
// cached `g_directStorageCaps` symbol; callers should treat
// `isSupported == false` as "fall back to std::fstream" without any
// further checks.
//
// Note: this header deliberately avoids Qt (uses std::string) so the
// probe library stays UI-framework-free and can be reused by any
// future headless tooling (regression tests, CI probes, etc.).

#pragma once

#ifdef _WIN32

#include <windows.h>
#include <string>

namespace fceu11::platform::win11 {

/// Result of the DirectStorage 1.2 NVMe probe.
/// All fields are populated in one shot by probeDirectStorage() and
/// cached for the lifetime of the process.
struct DirectStorageCaps {
	/// True if the host reports NvmeSdsSupported() == TRUE AND the
	/// DirectStorage runtime DLLs are loadable.
	bool isSupported = false;

	/// Total size of the system drive in bytes (best-effort; 0 if
	/// GetDiskFreeSpaceExW fails or returns a non-numeric value).
	DWORDLONG totalBytes = 0;

	/// Free bytes on the system drive.
	DWORDLONG freeBytes = 0;

	/// Human-readable reason for !isSupported. Empty when supported.
	/// Examples:
	///   "NvmeSdsSupported() returned FALSE"
	///   "DirectStorage runtime not present (dstorage.dll missing)"
	///   "Win10 1903 or later required"
	std::string errorReason;
};

/// Probe the host for DirectStorage 1.2 NVMe support.
/// Returns a cached value on subsequent calls; the first call invokes
/// NvmeSdsSupported() and GetDiskFreeSpaceExW(). Safe to call from any
/// thread (the cache is a function-local static).
DirectStorageCaps probeDirectStorage();

/// Convenience: true iff the system drive is an NVMe SSD. Used by the
/// v0.3.15.x savestate path as a coarse "should we even consider the
/// DirectStorage fast path" check before doing the full probe.
bool isNvmeStorageForSavestate();

/// Global cached caps, populated by fceuWrapperInit() at startup so
/// state.cpp (and any other savestate I/O code) can read the result
/// without a thread-local lookup. See the comment block at the top of
/// FCEUSS_Save() in state.cpp.
extern DirectStorageCaps g_directStorageCaps;

} // namespace fceu11::platform::win11

#endif // _WIN32
