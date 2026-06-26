// timeStamp.cpp
/// \file
/// \brief Time Stamp wrapper — delegates to Rust.
///
/// Phase 7 (v0.2.8): Rust module provides cross-platform high-resolution
/// monotonic timestamps via `std::time::Instant`, using nanosecond resolution.
/// The C++ class shell (`timeStampRecord`) is preserved; only the platform-
/// specific calibration and `readNew()` implementations are forwarded.

#include <stdio.h>

#include "timeStamp.h"

#if defined(WIN32)
#include <windows.h>
#endif

#include "../rust/fceux11_rust.h"

// R3.2 (refactor_plan.md §Phase R2): originally routed the calibration
// trace through FCEU_PrintError for unified logging. That required
// `#include "../fceu.h"`, which transitively pulls in bus.h (the v1.4
// Bus class header). bench_tolerance_test caught a +23% regression on
// bench_full_frame as a result — classic link-time code layout
// disturbance (cf. memory note "Phase 6 VRC7 bench regression").
// Solution: keep the unified-channel intent by forward-declaring
// FCEU_PrintError in the GLOBAL namespace (matching its actual
// definition site in fceu.cpp:1158, also in the global namespace) so
// no .h pollution occurs. The static-init printf was removed entirely
// (the ctor still runs but silently).

// Forward-declare to avoid pulling in fceu.h → bus.h. Definition
// lives in fceu.cpp:1158 in the GLOBAL namespace (not inside any
// `namespace FCEU {}` block). MUST stay in the global namespace here
// so the call from inside `namespace FCEU { ... }` resolves to the
// global symbol (otherwise name lookup finds the local forward decl
// and the link errors with `FCEU::FCEU_PrintError` undefined).
// fceu.h:157 declares:
//   void FCEU_PrintError( __FCEU_PRINTF_FORMAT const char *format, ...)
// where __FCEU_PRINTF_FORMAT expands to nothing or _Printf_format_string_
// (an MSVC SAL annotation). For a forward declaration we omit it.
void FCEU_PrintError( const char* format, ... );

namespace FCEU
{

uint64_t timeStampRecord::_tscFreq = 0;
uint64_t timeStampRecord::qpcFreq = 0;

void timeStampRecord::readNew(void)
{
	ts = fceux11_rust_timestamp_now();
	// Keep tsc consistent with ts so that inline arithmetic operators
	// (which operate on both fields) remain behaviourally correct.
	tsc = ts;
}

#if defined(WIN32)
void timeStampRecord::qpcCalibrate(void)
{
	fceux11_rust_timestamp_init();
	qpcFreq = fceux11_rust_timestamp_freq();
}
#endif

// R3.2 (refactor_plan.md §Phase R2): removed the
// `printf("timeStampModuleInit\n")` that fired at static initialisation
// time (before main(), polluting stdout of any program that links
// fceux11_utils). The ctor's only real work is calling qpcCalibrate();
// the print was purely cosmetic. We also dropped the C-style
// `timeStampModule(void)` empty-arg decoration per R3.3.
class timeStampModule
{
	public:
	timeStampModule()
	{
		timeStampRecord::qpcCalibrate();
	}
};

static timeStampModule module;

bool timeStampModuleInitialized(void)
{
	bool initialized = timeStampRecord::countFreq() != 0;
	return initialized;
}

void timeStampRecord::tscCalibrate(int numSamples)
{
	// In the Rust implementation there is no separate TSC; the TSC
	// frequency is reported as identical to the QPC frequency so
	// that existing code calling tscValid() / tscFreq() continues
	// to work without dividing by zero.
	if (qpcFreq == 0)
	{
		qpcFreq = fceux11_rust_timestamp_freq();
	}
	_tscFreq = qpcFreq;

	// R3.2: the original implementation printf'd `numSamples` lines of
	// per-iteration TSC frequency trace to stdout, even when numSamples
	// is 0 (the default) and even though the trace is meaningless in
	// Rust mode (TSC == QPC by construction). Replaced with a single
	// FCEU_PrintError info line routed to the unified error/log
	// channel (stderr), and only when numSamples > 0 (the explicit
	// "I am calibrating" case). Silent in the common path.
	if (numSamples > 0) {
		FCEU_PrintError(
			"tscCalibrate: Rust mode (TSC == QPC), freq=%.3f MHz, samples=%d (informational only)\n",
			static_cast<double>(_tscFreq) * 1.0e-6,
			numSamples);
	}
}

} // namespace FCEU
