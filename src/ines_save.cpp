// iNES save functions — extracted from ines.cpp for v1.10 Cryptex Phase A.3.

#include "types.h"
#include "utils/safe_string.h"
#include "fceu.h"
#include "cart.h"
#include "ines.h"
#include "driver.h"
#include "file.h"

#include <cstdio>
#include <cstring>

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

	if (fwrite(&head, 1, 16, fp) != 16)
	{
		fclose(fp);
		return 0;
	}

	if (head.ROM_type & 4)
	{
		/* Trainer */
		fwrite(trainerpoo, 512, 1, fp);
	}

	fwrite(ROM, 0x4000, ROM_size, fp);

	if (head.VROM_size)
		fwrite(VROM, 0x2000, head.VROM_size, fp);

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
