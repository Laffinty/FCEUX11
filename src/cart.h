#ifndef CART_H
#define CART_H

#include <vector>

// Pull in Bus — provides the inline global aliases (ARead, BWrite,
// Page, VPage, PRGptr, CHRptr, PRGsize, CHRmask*, etc.) and inline
// forwarders for setprg8/16/32, setchr1/4/8, setmirror/setmirrorw/
// setntamem, SetupCart*Mapping, SetupCartMirroring, ResetCartMapping.
// v1.4 Gateway Phase 2: the bus now owns the data; cart.h keeps
// only the non-Bus surface (CartInfo, the `r` variants, Genie).
#include "bus.h"

struct CartInfo
{
	// Set by mapper/board code:
	void (*Power)(void);
	void (*Reset)(void);
	void (*Close)(void);

	struct SaveGame_t
	{
		uint8  *bufptr;	// Pointer to memory to save/load.
		uint32  buflen;	// How much memory to save/load.
		void (*resetFunc)(void); // Callback to reset save game memory

		SaveGame_t(void)
			: bufptr(nullptr), buflen(0), resetFunc(nullptr)
		{
		}
	};
	std::vector <SaveGame_t> SaveGame;

	void addSaveGameBuf( uint8* bufptrIn, uint32 buflenIn, void (*resetFuncIn)(void) = nullptr )
	{
		SaveGame_t tmp;

		tmp.bufptr = bufptrIn;
		tmp.buflen = buflenIn;
		tmp.resetFunc = resetFuncIn;

		SaveGame.push_back( tmp );
	}

	// Set by iNES/UNIF loading code.
	int mirror;		// As set in the header or chunk.
				// iNES/UNIF specific.  Intended
				// to help support games like "Karnov"
				// that are not really MMC3 but are
				// set to mapper 4.
	int mirrorAs2Bits;
	int battery;	// Presence of an actual battery.
	int ines2;
	int submapper;	// Submappers as defined by NES 2.0
	int wram_size;
	int battery_wram_size;
	int vram_size;
	int battery_vram_size;
	uint8 MD5[16];
	uint32 CRC32;	// Should be set by the iNES/UNIF loading
					// code, used by mapper/board code, maybe
					// other code in the future.

	CartInfo(void)
	{
		clear();
	}

	void clear(void)
	{
		Power = nullptr;
		Reset = nullptr;
		Close = nullptr;

		SaveGame.clear();

		mirror = 0;
		mirrorAs2Bits = 0;
		battery = 0;
		ines2 = 0;
		submapper = 0;
		wram_size = 0;
		battery_wram_size = 0;
		vram_size = 0;
		battery_vram_size = 0;
		memset( MD5, 0, sizeof(MD5));
		CRC32 = 0;
	};
};

extern CartInfo *currCartInfo;

void FCEU_SaveGameSave(CartInfo *LocalHWInfo);
void FCEU_LoadGameSave(CartInfo *LocalHWInfo);
void FCEU_ClearGameSave(CartInfo *LocalHWInfo);

// Page-table read/write handlers (read from / write to the bus's
// Page[] / PRGIsRAM[] — both now Bus-owned via inline aliases).
DECLFR(CartBROB);
DECLFR(CartBR);
DECLFW(CartBW);

// `r` variants of the bank-switching free functions. The plain
// (non-r) setprg8/16/32, setchr1/4/8, setmirror/setmirrorw/setntamem
// are inline forwarders in bus.h (Bus methods). The r-variants stay
// as free functions in cart.cpp because the v1.4 Bus class spec
// doesn't list them and no board file in the v1.4 migration batches
// uses them.
void setprg2  (uint32 A, uint32 V);
void setprg4  (uint32 A, uint32 V);
void setprg2r (int r, unsigned int A, unsigned int V);
void setprg4r (int r, unsigned int A, unsigned int V);
void setprg8r (int r, uint32 A, uint32 V);
void setprg16r(int r, uint32 A, uint32 V);
void setprg32r(int r, uint32 A, uint32 V);

void setchr1r (int r, unsigned int A, unsigned int V);
void setchr2  (uint32 A, uint32 V);
void setchr2r (int r, unsigned int A, unsigned int V);
void setchr4r (int r, unsigned int A, unsigned int V);
void setchr8r (int r, uint32 V);

#define MI_H 0
#define MI_V 1
#define MI_0 2
#define MI_1 3

extern int geniestage;

void FCEU_GeniePower(void);

bool FCEU_OpenGenie(void);
void FCEU_CloseGenie(void);
void FCEU_KillGenie(void);

#endif
