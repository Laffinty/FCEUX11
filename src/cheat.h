#ifndef CHEAT_H
#define CHEAT_H
void FCEU_CheatResetRAM(void);
void FCEU_CheatAddRAM(int s, uint32 A, uint8 *p);

void FCEU_LoadGameCheats(FILE *override, int override_existing = 1);
void FCEU_FlushGameCheats(FILE *override, int nosave);
void FCEU_SaveGameCheats(FILE *fp, int release = 0);
void FCEU_ApplyPeriodicCheats(void);
void FCEU_PowerCheats(void);
int FCEU_CalcCheatAffectedBytes(uint32 address, uint32 size);

// Trying to find a more efficient way for determining if an address has a cheat
// each bit of 1 byte represents to 8 bytes in NES
typedef unsigned char _8BYTECHEATMAP;
#define CHEATMAP_SIZE 0x10000 / 8

namespace fceu11 {
    int FindCheatMapByte(uint16 address);
    void SetCheatMapByte(uint16 address, bool cheat);
    void CreateCheatMap();
    void RefreshCheatMap();
    void ReleaseCheatMap();
} // namespace fceu11

inline int FCEUI_FindCheatMapByte(uint16 address) { return fceu11::FindCheatMapByte(address); }
inline void FCEUI_SetCheatMapByte(uint16 address, bool cheat) { fceu11::SetCheatMapByte(address, cheat); }
inline void FCEUI_CreateCheatMap() { fceu11::CreateCheatMap(); }
inline void FCEUI_RefreshCheatMap() { fceu11::RefreshCheatMap(); }
inline void FCEUI_ReleaseCheatMap() { fceu11::ReleaseCheatMap(); }
extern unsigned int FrozenAddressCount;

int FCEU_CheatGetByte(uint32 A);
void FCEU_CheatSetByte(uint32 A, uint8 V);

extern int savecheats;
extern int globalCheatDisabled;
extern int disableAutoLSCheats;

int FCEU_DisableAllCheats(void);
int FCEU_DeleteAllCheats(void);

struct CHEATF_SUBFAST
{
	uint16 addr;
	uint8 val;
	int compare;
	readfunc PrevRead;

	CHEATF_SUBFAST(void)
	{
		addr = 0; val = 0; compare = 0; PrevRead = nullptr;
	}
};

struct CHEATF {
	struct CHEATF *next;
	std::string name;
	uint16 addr;
	uint8 val;
	int compare;	/* -1 for no compare. */
	int type;	/* 0 for replace, 1 for substitute(GG). */
	int status;
};

struct SEARCHPOSSIBLE {
	uint16 addr;
	uint8 previous;
	uint8 current;
	bool update;
};

#define FCEU_SEARCH_SPECIFIC_CHANGE         0
#define FCEU_SEARCH_RELATIVE_CHANGE         1
#define FCEU_SEARCH_PUERLY_RELATIVE_CHANGE  2
#define FCEU_SEARCH_ANY_CHANGE              3
#define FCEU_SEARCH_NEWVAL_KNOWN            4
#define FCEU_SEARCH_NEWVAL_GT               5
#define FCEU_SEARCH_NEWVAL_LT               6
#define FCEU_SEARCH_NEWVAL_GT_KNOWN         7
#define FCEU_SEARCH_NEWVAL_LT_KNOWN         8

#endif

