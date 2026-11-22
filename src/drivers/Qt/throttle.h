/* FCE Ultra - NES/Famicom Emulator
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program; if not, write to the Free Software
 * Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA  02110-1301  USA
 */
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
