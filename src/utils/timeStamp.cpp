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

class timeStampModule
{
	public:
	timeStampModule(void)
	{
		printf("timeStampModuleInit\n");
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
	printf("Running TSC Calibration: %i sec... (Rust mode: TSC == QPC)\n", numSamples);
	for (int i = 0; i < numSamples; i++)
	{
		printf("%i Calibration: TSC Freq: %f MHz (nanosecond resolution)\n", i,
		       static_cast<double>(_tscFreq) * 1.0e-6);
	}
}

} // namespace FCEU
