
#include <cstdio>
#include <cstdlib>
#include <cstring>

#include "types.h"
#include "fceu.h"
#include "driver.h"
#include "ines.h"

struct MapperTestCase {
    const char* filename;
    int expectedMapper;
};

static const MapperTestCase tests[] = {
    { "fixtures/mapper_nrom.nes",   0 },
    { "fixtures/mapper_mmc1.nes",   1 },
    { "fixtures/mapper_uxrom.nes",  2 },
    { "fixtures/mapper_mmc3.nes",   4 },
    { "fixtures/mapper_mmc5.nes",   5 },
    { "fixtures/mapper_axrom.nes",  7 },
    { "fixtures/mapper_vrc6.nes",  24 },
    { "fixtures/mapper_vrc7.nes",  85 },
};

static const int NUM_TESTS = sizeof(tests) / sizeof(tests[0]);

int main() {
    setvbuf(stdout, NULL, _IONBF, 0);
    printf("=== Mapper Load Test ===\n\n");
    fflush(stdout);

    // Initialize emulator core (required before loading games)
    printf("Calling FCEUI_Initialize()...\n");
    fflush(stdout);
    if (!FCEUI_Initialize()) {
        printf("FAIL: FCEUI_Initialize() returned false\n");
        return 1;
    }
    printf("FCEUI_Initialize() OK\n");
    fflush(stdout);

    bool failed = false;
    int passed = 0;

    for (int i = 0; i < NUM_TESTS; ++i) {
        const char* path = tests[i].filename;
        int expected = tests[i].expectedMapper;

        printf("[%d/%d] Loading %-30s (expected mapper %3d) ... ",
               i + 1, NUM_TESTS, path, expected);

        FCEUGI* gi = FCEUI_LoadGame(path, 1, true);
        if (!gi) {
            printf("FAIL (load returned null)\n");
            failed = true;
            continue;
        }

        int actual = gi->mappernum;
        if (actual != expected) {
            printf("FAIL (mapper %d)\n", actual);
            failed = true;
        } else {
            printf("OK (mapper %d)\n", actual);
            ++passed;
        }

        FCEUI_CloseGame();
    }

    FCEUI_Kill();

    printf("\n=== Results ===\n");
    printf("Passed: %d / %d\n", passed, NUM_TESTS);

    if (failed) {
        printf("RESULT: FAILED\n");
        return 1;
    } else {
        printf("RESULT: PASSED\n");
        return 0;
    }
}
