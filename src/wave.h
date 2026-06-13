#include "types.h"

namespace fceu11 {
    bool BeginWaveRecord(const char *fn);
    bool WaveRecordRunning();
    int  EndWaveRecord();
} // namespace fceu11

void FCEU_WriteWaveData(int32 *Buffer, int Count);
