// TaskbarProgress.h
//
// v0.3.15.x PHASE-3: thin wrapper around the Windows 7+ ITaskbarList3
// COM interface. The wrapper hides the COM boilerplate (CoCreateInstance,
// Release, the three progress states) and exposes a Qt-friendly API
// that the rest of fceux11 can call from any thread (the methods
// internally marshal to the UI thread via Qt's main thread affinity).
//
// Scope: this is the progress + overlay icon subset of ITaskbarList3.
// Snap Layouts hover prompts (SetThumbnailTooltip) and the Win11
// 22H2+ taskbar progress accent policies are deferred to v2.0 per
// v0.3.x construction plan v3 §5 v0.3.15 PHASE-3 task 3.3.

#pragma once

#ifdef _WIN32

#include <windows.h>
#include <shobjidl.h>

namespace fceu11::platform::win11 {

class TaskbarProgress {
public:
	TaskbarProgress() = default;
	~TaskbarProgress();

	TaskbarProgress(const TaskbarProgress&) = delete;
	TaskbarProgress& operator=(const TaskbarProgress&) = delete;

	/// Initialise the COM object and bind it to the given window.
	/// Must be called on the UI thread; subsequent setProgress /
	/// setOverlayIcon / setThumbnailTooltip calls are thread-safe
	/// because ITaskbarList3 itself marshals internally when
	/// invoked from a non-UI thread that has CoInitialized.
	///
	/// Returns true on success; false if CoCreateInstance failed
	/// (typically: Win10 pre-7, or CoInitialize not yet called).
	bool init(HWND hwnd);

	/// Release the COM object. Safe to call multiple times.
	void release();

	/// Set the progress bar on the taskbar icon.
	///   pct in [0.0, 1.0]  -> progress fill
	///   pct <  0.0         -> clear (TBPF_NOPROGRESS)
	/// The progress state defaults to TBPF_NORMAL (green); use
	/// setState() to switch to indeterminate / error / paused.
	void setProgress(double pct);

	/// Set the progress state (TBPF enum).
	///   0 NOPROGRESS
	///   1 INDETERMINATE
	///   2 NORMAL
	///   3 ERROR
	///   4 PAUSED
	void setState(int tbpfState);

	/// Set / clear a small overlay icon on the taskbar icon.
	/// Pass nullptr to clear.
	void setOverlayIcon(HICON icon, LPCWSTR description);

	/// Update the thumbnail tooltip (Win11 Snap Layouts uses this
	/// for the hover preview). Pass nullptr to clear.
	/// Not all Windows builds honour this — Win10 1607+ does.
	void setThumbnailTooltip(LPCWSTR text);

	/// True if init() succeeded and the interface is usable.
	bool isActive() const { return m_pTaskbarList != nullptr; }

private:
	ITaskbarList3* m_pTaskbarList = nullptr;
	HWND           m_hwnd         = nullptr;
	bool           m_initialized  = false;
};

} // namespace fceu11::platform::win11

#endif // _WIN32
