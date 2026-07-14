// iNES save functions — extracted from ines.cpp for v1.10 Cryptex Phase A.3.

#include "types.h"
#include "utils/safe_string.h"
#include "fceu.h"
#include "cart.h"
#include "ines.h"
#include "file.h"

#include <cstdio>
#include <cstring>

extern uint8 *trainerpoo;
extern uint8 *ROM;
extern uint8 *VROM;
extern uint32 ROM_size;
extern iNES_HEADER head;
extern void iNESGI(GI h);

// bbit edited: the whole function below was added
int iNesSave(void) {
	char name[2048];

	FCEU_strlcpy(name, sizeof(name), LoadedRomFName);
	if (strcmp(name + strlen(name) - 4, ".nes") != 0) { //para edit
		safe_strcat(name, sizeof(name), ".nes");
	}

	return iNesSaveAs(name);
}

int iNesSaveAs(const char* name)
{
	FILE *fp;

	if ((GameInfo->type != GIT_CART) && (GameInfo->type != GIT_VSUNI)) return 0;
	if (GameInterface != iNESGI) return 0;

	fp = fopen(name, "wb");
	if (!fp)
		return 0;

	// hotfix1 P2-12 (H-12, H-30): every fwrite below used to discard its
	// return value, so a short write, a full disk, or an early EOF would
	// still report "save succeeded" via the final `return 1`. That left
	// the caller with a partially written .nes and no error indication.
	// Treat any fwrite that returns less than requested as a failure so
	// the GUI can surface the problem instead of silently producing a
	// corrupt ROM on disk.
	if (fwrite(&head, 1, 16, fp) != 16)
	{
		fclose(fp);
		return 0;
	}

	if (head.ROM_type & 4)
	{
		/* Trainer */
		if (fwrite(trainerpoo, 512, 1, fp) != 1) {
			fclose(fp);
			return 0;
		}
	}

	if (fwrite(ROM, 0x4000, ROM_size, fp) != ROM_size)
	{
		fclose(fp);
		return 0;
	}

	if (head.VROM_size) {
		if (fwrite(VROM, 0x2000, head.VROM_size, fp) != head.VROM_size) {
			fclose(fp);
			return 0;
		}
	}

	fclose(fp);
	return 1;
}

//para edit: added function below
char *iNesShortFName(void) {
	char *ret;

	if (!(ret = strrchr(LoadedRomFName, '\\')))
	{
		if (!(ret = strrchr(LoadedRomFName, '/')))
			return 0;
	}
	return ret + 1;
}
