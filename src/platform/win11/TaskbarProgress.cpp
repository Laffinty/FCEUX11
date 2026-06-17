// TaskbarProgress.cpp
//
// v0.3.15.x PHASE-3: ITaskbarList3 wrapper implementation.
// COM is initialised on the calling thread (typically the Qt UI
// thread, which is also where consoleWin_t lives). The destructor
// is idempotent: calling release() twice is safe.

#include "TaskbarProgress.h"

#ifdef _WIN32

#include <objbase.h>

namespace fceu11::platform::win11 {

TaskbarProgress::~TaskbarProgress()
{
	release();
}

bool TaskbarProgress::init(HWND hwnd)
{
	if (m_initialized) {
		// Re-init: release the old binding first so we do not leak
		// a reference if the caller passes a different hwnd.
		release();
	}

	m_hwnd = hwnd;

	// ITaskbarList3 is part of the Windows shell. CLSID_TaskbarList
	// and IID_ITaskbarList3 are declared in <shobjidl.h>.
	HRESULT hr = CoCreateInstance(CLSID_TaskbarList, nullptr,
	                              CLSCTX_INPROC_SERVER,
	                              IID_PPV_ARGS(&m_pTaskbarList));
	if (FAILED(hr) || m_pTaskbarList == nullptr) {
		m_pTaskbarList = nullptr;
		return false;
	}

	// HrInit is required before any other ITaskbarList method
	// (per MSDN, "ITaskbarList3::HrInit"). It is safe to call
	// multiple times; subsequent calls return S_FALSE.
	if (FAILED(m_pTaskbarList->HrInit())) {
		m_pTaskbarList->Release();
		m_pTaskbarList = nullptr;
		return false;
	}

	m_initialized = true;
	return true;
}

void TaskbarProgress::release()
{
	if (m_pTaskbarList != nullptr) {
		// Clear any visible progress / overlay before releasing so
		// the taskbar icon does not retain a stale state across
		// the next init() call (e.g. after a window recreate).
		if (m_hwnd != nullptr) {
			m_pTaskbarList->SetProgressState(m_hwnd, TBPF_NOPROGRESS);
			m_pTaskbarList->SetOverlayIcon(m_hwnd, nullptr, L"");
		}
		m_pTaskbarList->Release();
		m_pTaskbarList = nullptr;
	}
	m_hwnd = nullptr;
	m_initialized = false;
}

void TaskbarProgress::setProgress(double pct)
{
	if (!isActive() || m_hwnd == nullptr) {
		return;
	}
	if (pct < 0.0) {
		m_pTaskbarList->SetProgressState(m_hwnd, TBPF_NOPROGRESS);
		return;
	}
	if (pct > 1.0) pct = 1.0;
	// SetProgressValue expects ULONGLONG with (completed, total).
	// We map pct in [0, 1] to (pct * 1000, 1000) — 0.1% granularity
	// is more than enough for a savestate write or TAS record
	// progress bar.
	const ULONGLONG total = 1000ULL;
	const ULONGLONG completed = static_cast<ULONGLONG>(pct * total);
	m_pTaskbarList->SetProgressState(m_hwnd, TBPF_NORMAL);
	m_pTaskbarList->SetProgressValue(m_hwnd, completed, total);
}

void TaskbarProgress::setState(int tbpfState)
{
	if (!isActive() || m_hwnd == nullptr) {
		return;
	}
	// Clamp to the valid TBPF enum range; the call is a no-op for
	// out-of-range values per the ITaskbarList3 contract.
	if (tbpfState < TBPF_NOPROGRESS || tbpfState > TBPF_PAUSED) {
		return;
	}
	m_pTaskbarList->SetProgressState(m_hwnd,
		static_cast<TBPFLAG>(tbpfState));
}

void TaskbarProgress::setOverlayIcon(HICON icon, LPCWSTR description)
{
	if (!isActive() || m_hwnd == nullptr) {
		return;
	}
	m_pTaskbarList->SetOverlayIcon(m_hwnd, icon,
	                               description ? description : L"");
}

void TaskbarProgress::setThumbnailTooltip(LPCWSTR text)
{
	if (!isActive() || m_hwnd == nullptr) {
		return;
	}
	// SetThumbnailTooltip was added in Win10 1607. The interface
	// method exists on every ITaskbarList3, so we just call it;
	// older shells silently ignore it.
	m_pTaskbarList->SetThumbnailTooltip(m_hwnd, text ? text : L"");
}

} // namespace fceu11::platform::win11

#endif // _WIN32
