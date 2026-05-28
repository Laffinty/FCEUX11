// timeStamp.cpp
/// \file
/// \brief Time Stamp wrapper — delegates to Rust when FCEUX11_RUST_ENABLED,
/// otherwise falls back to the original Windows QPC/TSC implementation.
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

#ifdef FCEUX11_RUST_ENABLED
#include "../rust/fceux11_rust.h"
#endif

//-------------------------------------------------------------------------
//---- Time Stamp Record
//-------------------------------------------------------------------------
#if !defined(FCEUX11_RUST_ENABLED)
#include <intrin.h>
#pragma intrinsic(__rdtsc)

static uint64_t rdtsc()
{
	return __rdtsc();
}
#endif

namespace FCEU
{

uint64_t timeStampRecord::_tscFreq = 0;
uint64_t timeStampRecord::qpcFreq = 0;

void timeStampRecord::readNew(void)
{
#ifdef FCEUX11_RUST_ENABLED
	ts = fceux11_rust_timestamp_now();
	// Keep tsc consistent with ts so that inline arithmetic operators
	// (which operate on both fields) remain behaviourally correct.
	tsc = ts;
#else
	QueryPerformanceCounter((LARGE_INTEGER*)&ts);
	tsc = rdtsc();
#endif
}

#if defined(WIN32)
void timeStampRecord::qpcCalibrate(void)
{
#ifdef FCEUX11_RUST_ENABLED
	fceux11_rust_timestamp_init();
	qpcFreq = fceux11_rust_timestamp_freq();
#else
	if (QueryPerformanceFrequency((LARGE_INTEGER*)&timeStampRecord::qpcFreq) == 0)
	{
		printf("QueryPerformanceFrequency FAILED!\n");
	}
#endif
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
#ifdef FCEUX11_RUST_ENABLED
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
#else
	timeStampRecord t1, t2, td;
	uint64_t td_sum = 0;
	double td_avg;

	if (QueryPerformanceFrequency((LARGE_INTEGER*)&timeStampRecord::qpcFreq) == 0)
	{
		printf("QueryPerformanceFrequency FAILED!\n");
	}
	printf("Running TSC Calibration: %i sec...\n", numSamples);

	for (int i=0; i<numSamples; i++)
	{
		t1.readNew();
		Sleep(1000);
		t2.readNew();

		td += t2 - t1;

		td_sum = td.tsc;

		td_avg = static_cast<double>(td_sum);

		timeStampRecord::_tscFreq = static_cast<uint64_t>( td_avg / td.toSeconds() );

		printf("%i Calibration: %f sec   TSC:%llu   TSC Freq: %f MHz\n", i, td.toSeconds(), 
			static_cast<unsigned long long>(td.tsc), static_cast<double>(timeStampRecord::_tscFreq) * 1.0e-6 );
	}
#endif
}

} // namespace FCEU
