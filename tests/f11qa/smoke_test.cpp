
#include <cstdio>
#include <cstdlib>
#include <cstring>

#include "types.h"
#include "fceu.h"
#include "driver.h"

#define CHECK_SYMBOL(x) \
    do { \
        void* addr = reinterpret_cast<void*>(&x); \
        if (addr == nullptr) { \
            printf("FAIL: Symbol " #x " is null\n"); \
            failed = true; \
        } else { \
            printf("OK: Symbol " #x " is valid\n"); \
        } \
    } while(0)

int main() {
    printf("=== FCEUX11 Smoke Test ===\n\n");

    bool failed = false;

    printf("--- Checking Core Symbols ---\n");
    CHECK_SYMBOL(FCEUI_Initialize);
    CHECK_SYMBOL(PowerNES);
    CHECK_SYMBOL(ResetNES);
    CHECK_SYMBOL(FCEUI_LoadGame);
    CHECK_SYMBOL(FCEUI_CloseGame);
    CHECK_SYMBOL(FCEUI_Kill);
    CHECK_SYMBOL(FCEUI_Emulate);
    CHECK_SYMBOL(FCEUI_SetInput);
    CHECK_SYMBOL(FCEUI_SetInputFC);
    CHECK_SYMBOL(FCEUI_Sound);
    CHECK_SYMBOL(FCEUD_SetPalette);
    CHECK_SYMBOL(FCEUD_GetPalette);
    CHECK_SYMBOL(FCEUI_ResetNES);
    CHECK_SYMBOL(FCEUI_PowerNES);
    CHECK_SYMBOL(FCEUI_NTSCSELHUE);
    CHECK_SYMBOL(FCEUI_NTSCSELTINT);
    CHECK_SYMBOL(FCEUI_GetNTSCTH);
    CHECK_SYMBOL(FCEUI_SetNTSCTH);
    CHECK_SYMBOL(FCEUI_GetUserPaletteAvail);
    CHECK_SYMBOL(FCEUI_SetUserPalette);

    printf("\n--- Checking Board Symbols ---\n");
    CHECK_SYMBOL(ResetGameLoaded);
    CHECK_SYMBOL(FCEU_MemoryRand);
    CHECK_SYMBOL(SetReadHandler);
    CHECK_SYMBOL(SetWriteHandler);

    printf("\n--- Checking Utility Symbols ---\n");
    CHECK_SYMBOL(FCEU_PrintError);
    CHECK_SYMBOL(FCEU_printf);
    CHECK_SYMBOL(FCEU_DispMessage);
    CHECK_SYMBOL(FCEUI_CRC32);

    printf("\n--- Running Minimal Initialization ---\n");
    if (!fceu11::Initialize()) {
        printf("FAIL: fceu11::Initialize() returned false\n");
        failed = true;
    } else {
        printf("OK: fceu11::Initialize() succeeded\n");
        fceu11::Kill();
        printf("OK: fceu11::Kill() completed\n");
    }

    printf("\n=== Test Complete ===\n");
    if (failed) {
        printf("\nRESULT: FAILED\n");
        return 1;
    } else {
        printf("\nRESULT: PASSED - All symbols are valid and initialization succeeded\n");
        return 0;
    }
}
