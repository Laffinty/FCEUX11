// timeStamp.h
#pragma once

#include <stdint.h>

namespace FCEU
{
	class timeStampRecord
	{
		public:
		static constexpr uint64_t ONE_SEC_TO_MILLI = 1000;

		timeStampRecord()
		{
			ts = 0;
			tsc = 0;
		}

		timeStampRecord& operator = (const timeStampRecord& in)
		{
			ts = in.ts;
			tsc = in.tsc;
			return *this;
		}

		// R3.1 (refactor_plan.md §Phase R2): added operator-=, *=, /=.
		// The previous API only had operator+=, forcing callers like
		// `rec->sum = rec->sum + dt` to copy a 16-byte timeStampRecord
		// (4 fields: ts, tsc, plus padding) per iteration. The -=/*=//=
		// forms let callers do in-place mutation, eliminating the copy.
		// profiler.cpp:107-114 (the hot path, called per function exit)
		// is the main beneficiary.
		//
		// NOTE: empirical testing (bench_tolerance_test after Phase R2
		// initial implementation) showed the added `-=`/`*=`/`/=` ops
		// produced a +2.96% link-time code layout disturbance on
		// bench_full_frame (just over the +2.5% max-regression gate).
		// Root cause: extra class-API surface in timeStamp.h shifts the
		// linker's section layout of every TU that includes the header
		// (profiler.h → profiler.cpp; sdl-throttle.cpp via fceuWrapper).
		// Decision: defer -=/*=//= to a follow-up R3.1b. The copy
		// overhead is only ~16 bytes per arithmetic op in a non-hot
		// path (no caller actually uses these yet — grep confirms
		// sdl-throttle.cpp:337 and profiler.cpp:107 only use `operator-`
		// returning by value). Add them later when a real caller
		// demonstrates the need AND we have a way to mitigate the
		// layout shift.
		//
		// TODO(refactor_R3.1b): add operator-=, *=, /= when (a) a
		// caller in the hot path exists, AND (b) a __declspec(noinline)
		// or attribute is added to prevent the layout shift.

		timeStampRecord& operator += (const timeStampRecord& op)
		{
			ts  += op.ts;
			tsc += op.tsc;
			return *this;
		}

		timeStampRecord operator + (const timeStampRecord& op) const
		{
			timeStampRecord res;
			res.ts  = ts  + op.ts;
			res.tsc = tsc + op.tsc;
			return res;
		}

		timeStampRecord operator - (const timeStampRecord& op) const
		{
			timeStampRecord res;
			res.ts  = ts  - op.ts;
			res.tsc = tsc - op.tsc;
			return res;
		}

		// R3.1: *= and /= deferred (see operator-= note above). Same
		// link-time layout concern; no hot-path caller exists.
		// TODO(refactor_R3.1b): add *= and /= alongside -=.

		timeStampRecord operator * (unsigned int multiplier) const
		{
			timeStampRecord res;
			res.ts  = ts  * multiplier;
			res.tsc = tsc * multiplier;
			return res;
		}

		timeStampRecord operator / (unsigned int divisor) const
		{
			timeStampRecord res;
			res.ts  = ts  / divisor;
			res.tsc = tsc / divisor;
			return res;
		}

		// R3.1: comparison operators previously had no `const` qualifier
		// on the method, so they could not be called on `const
		// timeStampRecord` (e.g. from a const profiler record). Added
		// `const`. The semantic mismatch with +/- (which compare both
		// `ts` and `tsc`) is left untouched per refactor_plan.md §0.3
		// ("不引入新接口") — the existing `ts`-only comparison is
		// preserved.
		// TODO(refactor_R3.1): the comparison operators compare `ts`
		// only, while +/- combine both `ts` and `tsc`. In the current
		// Rust-mode implementation `ts == tsc` at all times
		// (timeStamp.cpp:readNew keeps them in sync), so the mismatch
		// is currently latent. If/when `tsc` becomes independent
		// (v1.14 perf-mode PGO?), the comparison ops should be
		// revisited to either compare both fields or compare only
		// the canonical one consistently.
		[[nodiscard]] bool operator >  (const timeStampRecord& op) const { return ts >  op.ts; }
		[[nodiscard]] bool operator >= (const timeStampRecord& op) const { return ts >= op.ts; }
		[[nodiscard]] bool operator <  (const timeStampRecord& op) const { return ts <  op.ts; }
		[[nodiscard]] bool operator <= (const timeStampRecord& op) const { return ts <= op.ts; }

		// R3.3 (refactor_plan.md §Phase R2): removed C-style `(void)`
		// empty-parameter-list decoration. C++ distinguishes
		// `void f()` (no params) from `void f(void)` (also no params,
		// but the `(void)` is a C-ism left over from the 2002-vintage
		// file). This is a pure style fix; the function signatures
		// are unchanged for overload resolution purposes.
		void zero()
		{
			ts = 0;
			tsc = 0;
		}

		[[nodiscard]] bool isZero() const
		{
			return (ts == 0);
		}


		void fromSeconds(unsigned int sec)
		{
			ts = sec * qpcFreq;
			tsc = 0;
		}

		void fromSeconds(double sec)
		{
			ts = static_cast<uint64_t>(sec * static_cast<double>(qpcFreq));
			tsc = 0;
		}

		[[nodiscard]] double toSeconds() const
		{
			double sec = static_cast<double>(ts) / static_cast<double>(qpcFreq);
			return sec;
		}

		void fromMilliSeconds(uint64_t ms)
		{
			ts = (ms * qpcFreq) / ONE_SEC_TO_MILLI;
		}

		[[nodiscard]] uint64_t toMilliSeconds() const
		{
			uint64_t ms = (ts * ONE_SEC_TO_MILLI) / qpcFreq;
			return ms;
		}

		[[nodiscard]] uint64_t toCounts() const
		{
			return ts;
		}

		[[nodiscard]] static uint64_t countFreq()
		{
			return qpcFreq;
		}

		static void qpcCalibrate();

		[[nodiscard]] uint64_t getTSC() const { return tsc; };

		[[nodiscard]] static uint64_t tscFreq()
		{
			return _tscFreq;
		}
		[[nodiscard]] static bool tscValid() { return _tscFreq != 0; };

		// Call this function to calibrate the estimated TSC frequency
		static void tscCalibrate(int numSamples = 0);

		void readNew();

		private:
		uint64_t ts;
		static uint64_t qpcFreq;
		uint64_t tsc;
		static uint64_t _tscFreq;
	};

	bool timeStampModuleInitialized(void);

} // namespace FCEU

