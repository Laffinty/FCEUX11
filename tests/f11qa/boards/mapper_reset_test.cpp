
#include <cstdio>
#include <cstdlib>
#include <cstring>

#include "types.h"
#include "fceu.h"
#include "driver.h"

struct MapperTestCase {
    const char* filename;
};

static const MapperTestCase tests[] = {
    { "fixtures/mapper_nrom.nes" },
    { "fixtures/mapper_mmc1.nes" },
    { "fixtures/mapper_uxrom.nes" },
    { "fixtures/mapper_mmc3.nes" },
    { "fixtures/mapper_mmc5.nes" },
    { "fixtures/mapper_axrom.nes" },
    { "fixtures/mapper_vrc6.nes" },
    { "fixtures/mapper_vrc7.nes" },
};

static const int NUM_TESTS = sizeof(tests) / sizeof(tests[0]);

int main() {
    printf("=== Mapper Reset Test ===\n\n");

    if (!fceu11::Initialize()) {
        printf("FAIL: fceu11::Initialize() returned false\n");
        return 1;
    }

    bool failed = false;
    int passed = 0;

    for (int i = 0; i < NUM_TESTS; ++i) {
        const char* path = tests[i].filename;

        printf("[%d/%d] Load + Reset %-30s ... ", i + 1, NUM_TESTS, path);

        FCEUGI* gi = fceu11::LoadGame(path, 1, true);
        if (!gi) {
            printf("FAIL (load returned null)\n");
            failed = true;
            continue;
        }

        // Perform reset; if it crashes we won't reach the next line
        fceu11::ResetNES();
        printf("OK\n");
        ++passed;

        fceu11::CloseGame();
    }

    fceu11::Kill();

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
