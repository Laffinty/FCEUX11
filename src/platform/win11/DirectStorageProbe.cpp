// DirectStorageProbe.cpp
//
// v0.3.15.x PHASE-3: probe DirectStorage 1.2 NVMe support at startup.
//
// The probe is intentionally lightweight: one LoadLibrary on
// nvme.dll (kernel driver userspace shim) + one LoadLibrary on
// dstorage.dll (DirectStorage runtime). If either fails, the host
// is reported as unsupported with a clear reason string.
//
// The probe does NOT take over any I/O; that integration is deferred
// to v2.0 per v0.3.x construction plan v3 §5 v0.3.15 PHASE-3 task 3.2.

#include "DirectStorageProbe.h"

#ifdef _WIN32

// Some Windows SDK builds (notably older MSVC v14.2x without the
// 1903 refresh) do not ship nvme.h. To keep this TU compilable on
// every supported toolchain we forward-declare NvmeSdsSupported and
// resolve it at runtime via GetProcAddress. This avoids a hard
// link-time dependency on nvme.lib while preserving the same
// observable behaviour.
typedef BOOLEAN (WINAPI *PFN_NvmeSdsSupported)(VOID);

namespace fceu11::platform::win11 {

// Definition of the global cached caps; populated by probeDirectStorage()
// on first call. Initialised to all-zero (unsupported) so any
// out-of-order call from state.cpp before fceuWrapperInit() runs is
// safe (it will just see !isSupported and fall through to std::fstream).
DirectStorageCaps g_directStorageCaps;

static DirectStorageCaps doProbe()
{
	DirectStorageCaps caps;

	// 1. NvmeSdsSupported: ask the nvme driver whether any NVMe
	//    device on the system supports the Storage Device Service
	//    (SDS) — the prerequisite for DirectStorage 1.2 NVMe bypass.
	//
	//    The function lives in nvme.dll on Windows 10 1903+; on
	//    older versions the library is not present and we report
	//    "unsupported" with a reason.
	HMODULE hNvme = LoadLibraryW(L"nvme.dll");
	if (hNvme == nullptr) {
		caps.errorReason =
			"nvme.dll not present (requires Windows 10 1903 or later)";
		return caps;
	}

	auto pfnSds = reinterpret_cast<PFN_NvmeSdsSupported>(
		GetProcAddress(hNvme, "NvmeSdsSupported"));
	if (pfnSds == nullptr) {
		FreeLibrary(hNvme);
		caps.errorReason =
			"NvmeSdsSupported export not found in nvme.dll";
		return caps;
	}

	const BOOLEAN sdsSupported = pfnSds();
	FreeLibrary(hNvme);

	if (!sdsSupported) {
		caps.errorReason =
			"NvmeSdsSupported() returned FALSE (no NVMe SDS device)";
		return caps;
	}

	// 2. DirectStorage 1.2 runtime: dstorage.dll / dstoragecore.dll.
	//    The probe only verifies the runtime is loadable; we do not
	//    create a factory or queue. v0.4.x will own the actual
	//    IDStorageFactory creation.
	HMODULE hDs = LoadLibraryW(L"dstorage.dll");
	if (hDs == nullptr) {
		caps.errorReason =
			"DirectStorage runtime not present (dstorage.dll missing)";
		return caps;
	}
	FreeLibrary(hDs);

	// 3. Disk space: best-effort totalBytes / freeBytes for the
	//    system drive. Failures are not fatal for the supported
	//    flag — callers only need a rough order-of-magnitude.
	ULARGE_INTEGER freeBytesAvail, totalBytes, freeBytesTotal;
	if (GetDiskFreeSpaceExW(nullptr, &freeBytesAvail, &totalBytes,
	                        &freeBytesTotal)) {
		caps.totalBytes = totalBytes.QuadPart;
		caps.freeBytes  = freeBytesAvail.QuadPart;
	}

	caps.isSupported = true;
	return caps;
}

DirectStorageCaps probeDirectStorage()
{
	// Function-local static is initialised on the first call from
	// whichever thread reaches it first (C++11 magic statics are
	// thread-safe). Subsequent calls just return the cached value.
	static const DirectStorageCaps cached = doProbe();
	return cached;
}

bool isNvmeStorageForSavestate()
{
	// Coarse pre-check used by FCEUSS_Save before doing the full
	// probe. We treat "DirectStorage supported" as a sufficient
	// proxy for "NVMe SSD is the system drive" because that is
	// the only configuration where NvmeSdsSupported() can return
	// TRUE on consumer hardware.
	return probeDirectStorage().isSupported;
}

} // namespace fceu11::platform::win11

#endif // _WIN32
