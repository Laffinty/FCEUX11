#ifndef CART_H
#define CART_H

#include <vector>

#include "cart_class.h"   // v1.7 Cartograph: fceu11::Cart / Mapper / MirrorMode

// v1.7 Cartograph §1.3: forward declarations for CartInfo lifecycle
// forwarding functions. Defined in cart.cpp; declared here before struct
// CartInfo so CartInfo::clear() can install them as defaults.
extern CartInfo *currCartInfo;
void CartInfo_PowerForward(void);
void CartInfo_ResetForward(void);
void CartInfo_CloseForward(void);

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

	// v1.7 Cartograph: back-pointer to the objectized cart. Board init
	// functions continue to use the Power/Reset/Close fields; iNES/UNIF
	// loading overwrites those fields with CartInfo_*Forward and sets this
	// pointer to the concrete mapper subclass.
	fceu11::Cart* cart_obj = nullptr;

	CartInfo(void)
	{
		clear();
	}

	void clear(void)
	{
		// v1.7: default to forwarding functions so cart_obj->on_*() is
		// used whenever a concrete Cart subclass is installed. Board files
		// that assign their own Power/Reset/Close function pointers still
		// work because they overwrite these defaults during MapperNN_Init.
		Power = CartInfo_PowerForward;
		Reset = CartInfo_ResetForward;
		Close = CartInfo_CloseForward;

		SaveGame.clear();

		cart_obj = nullptr;

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

// v1.7 Cartograph §2.3: legacy MI_* macros as MirrorMode enum aliases.
// Existing board files use these as int constants, so the macros expand to
// static_cast<int>(enum_value) and keep source compatibility.
#define MI_H static_cast<int>(fceu11::MirrorMode::Horizontal)
#define MI_V static_cast<int>(fceu11::MirrorMode::Vertical)
#define MI_0 static_cast<int>(fceu11::MirrorMode::Mode0)
#define MI_1 static_cast<int>(fceu11::MirrorMode::Mode1)

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

extern int geniestage;

void FCEU_GeniePower(void);

bool FCEU_OpenGenie(void);
void FCEU_CloseGenie(void);
void FCEU_KillGenie(void);

#endif
