// throttle.h
int SpeedThrottle(void);
void RefreshThrottleFPS(void);
int getTimingMode(void);
int setTimingMode(int mode);


struct frameTimingStat_t
{

	struct {
		double tgt;
		double cur;
		double min;
		double max;
	} frameTimeAbs;

	struct {
		double tgt;
		double cur;
		double min;
		double max;
	} frameTimeDel;

	struct {
		double tgt;
		double cur;
		double min;
		double max;
	} frameTimeWork;

	struct {
		double tgt;
		double cur;
		double min;
		double max;
	} frameTimeIdle;

	struct {
		double tgt;
		double cur;
		double min;
		double max;
	} videoTimeDel;

	unsigned int lateCount;

	bool enabled;
};

void resetFrameTiming(void);
void setFrameTimingEnable( bool enable );
int  getFrameTimingStats( struct frameTimingStat_t *stats );
void videoBufferSwapMark(void);
double getHighPrecTimeStamp(void);
double getFrameRate(void);
double getFrameRateAdjustmentRatio(void);
double getBaseFrameRate(void);

// v2.0 S2-b4 stage 3'. Pin the pacing rate to `hz` regardless of what the
// video system reports, or pass 0 to go back to following the video system.
// The caller re-tims as soon as it sets this, so there is no window where the
// old rate is still in force.
//
// This exists so that a second machine with its own frame rate can be paced
// without this file knowing that machine exists -- see RefreshThrottleFPS.
void SetThrottleBaseRateOverride(double hz);

extern bool useIntFrameRate;
