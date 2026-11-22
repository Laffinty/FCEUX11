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

#include <cstdio>
#include <cstdlib>
#include <cstring>

#include "types.h"
#include "fceu.h"
#include "core_api.h"
#include "io_api.h"
#include "net_api.h"
#include "diag_api.h"

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
