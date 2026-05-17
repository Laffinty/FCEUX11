// timeStamp.h
#pragma once

#include <stdint.h>

namespace FCEU
{
	class timeStampRecord
	{
		public:
		static constexpr uint64_t ONE_SEC_TO_MILLI = 1000;

		timeStampRecord(void)
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

		timeStampRecord& operator += (const timeStampRecord& op)
		{
			ts  += op.ts;
			tsc += op.tsc;
			return *this;
		}

		timeStampRecord operator + (const timeStampRecord& op)
		{
			timeStampRecord res;

			res.ts  = ts  + op.ts;
			res.tsc = tsc + op.tsc;
			return res;
		}

		timeStampRecord operator - (const timeStampRecord& op)
		{
			timeStampRecord res;

			res.ts  = ts  - op.ts;
			res.tsc = tsc - op.tsc;

			return res;
		}

		timeStampRecord operator * (const unsigned int multiplier)
		{
			timeStampRecord res;

			res.ts  = ts  * multiplier;
			res.tsc = tsc * multiplier;

			return res;
		}

		timeStampRecord operator / (const unsigned int divisor)
		{
			timeStampRecord res;

			res.ts  = ts  / divisor;
			res.tsc = tsc / divisor;

			return res;
		}

		bool operator > (const timeStampRecord& op)
		{
			return ts > op.ts;
		}
		bool operator >= (const timeStampRecord& op)
		{
			return ts >= op.ts;
		}

		bool operator < (const timeStampRecord& op)
		{
			return ts < op.ts;
		}
		bool operator <= (const timeStampRecord& op)
		{
			return ts <= op.ts;
		}

		void zero(void)
		{
			ts = 0;
			tsc = 0;
		}

		bool isZero(void)
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

		double toSeconds(void)
		{
			double sec = static_cast<double>(ts) / static_cast<double>(qpcFreq);
			return sec;
		}

		void fromMilliSeconds(uint64_t ms)
		{
			ts = (ms * qpcFreq) / ONE_SEC_TO_MILLI;
		}

		uint64_t toMilliSeconds(void)
		{
			uint64_t ms = (ts * ONE_SEC_TO_MILLI) / qpcFreq;
			return ms;
		}

		uint64_t toCounts(void)
		{
			return ts;
		}

		static uint64_t countFreq(void)
		{
			return qpcFreq;
		}

		static void qpcCalibrate(void);

		uint64_t getTSC(void){ return tsc; };

		static uint64_t tscFreq(void)
		{
			return _tscFreq;
		}
		static bool tscValid(void){ return _tscFreq != 0; };

		// Call this function to calibrate the estimated TSC frequency
		static void tscCalibrate(int numSamples = 0);

		void readNew(void);

		private:
		uint64_t ts;
		static uint64_t qpcFreq;
		uint64_t tsc;
		static uint64_t _tscFreq;
	};

	bool timeStampModuleInitialized(void);

} // namespace FCEU

