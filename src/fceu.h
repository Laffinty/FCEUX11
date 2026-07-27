#ifndef _FCEUH
#define _FCEUH

#include "types.h"
#include "bus.h"   // inline reference-to-array aliases for ::ARead / ::BWrite /
                   // ::Page / ::VPage / ::PRGptr / ::CHRptr / etc., plus
                   // inline forwarders for setprg8/16/32, setchr1/4/8,
                   // setmirror/setmirrorw/setntamem, and SetupCart* /
                   // ResetCartMapping. v1.4 Gateway Phase 2.

extern int fceuindbg;
extern int newppu;
void ResetGameLoaded(void);

//overclocking-related
// v1.4 Post-Release Optimization Plan §2.1: `overclocking` moved
// into fceu11::Cpu as a private member with public accessors
// (Cpu::overclocking() / Cpu::set_overclocking()). The global
// `extern bool overclocking;` is gone — callers go through
// g_cpu.set_overclocking() / g_cpu.overclocking() instead.
extern bool overclock_enabled;
extern bool skip_7bit_overclocking;
extern int normalscanlines;
extern int totalscanlines;
extern int postrenderscanlines;
extern int vblankscanlines;

extern bool AutoResumePlay;
extern bool frameAdvanceLagSkip;
extern char romNameWhenClosingEmulator[];

#ifndef FCEU_DECLFW_GUARD
#define FCEU_DECLFW_GUARD
#define DECLFR(x) uint8 x (uint32 A)
#define DECLFW(x) void x (uint32 A, uint8 V)
#endif
// v0.3.6: DECLFW-decorated mapper write functions are protected by Control
// Flow Guard (CFG) — /guard:cf is set globally in CMakeLists.txt. The
// compiler emits the guard check at the indirect call site (SetWriteHandler
// dispatch), so no per-function __declspec(guard_overwrite) is required.

void FCEU_MemoryRand(uint8 *ptr, uint32 size, bool default_zero=false);
void SetReadHandler(int32 start, int32 end, readfunc func);
void SetWriteHandler(int32 start, int32 end, writefunc func);
writefunc GetWriteHandler(int32 a);
readfunc GetReadHandler(int32 a);

int AllocGenieRW(void);
void FlushGenieRW(void);

void FCEU_ResetVidSys(void);

void ResetMapping(void);
void ResetNES(void);
void PowerNES(void);

void SetAutoFireOffset(int offset);
void SetAutoFirePattern(int onframes, int offframes);
void GetAutoFirePattern( int *onframes, int *offframes);
bool GetAutoFireState(int btnIdx);
void AutoFire(void);
void FCEUI_RewindToLastAutosave(void);

//mbg 7/23/06
const char *FCEUI_GetAboutString(void);

extern uint64 timestampbase;

// MMC5 external shared buffers/vars
extern int MMC5Hack;
extern uint32 MMC5HackVROMMask;
extern uint8 *MMC5HackExNTARAMPtr;
extern uint8 *MMC5HackVROMPTR;
extern uint8 MMC5HackCHRMode;
extern uint8 MMC5HackSPMode;
extern uint8 MMC50x5130;
extern uint8 MMC5HackSPScroll;
extern uint8 MMC5HackSPPage;

extern int PEC586Hack;

// VRCV extarnal shared buffers/vars
extern int QTAIHack;
extern uint8 QTAINTRAM[2048];
extern uint8 qtaintramreg;

inline constexpr uint32_t GAME_MEM_BLOCK_SIZE = 131072;

extern  uint8  *RAM;            //shared memory modifications
extern int EmulationPaused;
extern int frameAdvance_Delay;
extern int RAMInitOption;

uint8 FCEU_ReadRomByte(uint32 i);
void FCEU_WriteRomByte(uint32 i, uint8 value);

// ARead[] / BWrite[] are now inline reference-to-array aliases
// declared in bus.h. fceu.h used to declare them as extern arrays;
// those declarations are removed because the inline alias is the
// sole definition. Files that include fceu.h (or transitively
// include bus.h via cart.h) see the same ARead / BWrite symbols.

enum GI {
	GI_RESETM2	=1,
	GI_POWER =2,
	GI_CLOSE =3,
	GI_RESETSAVE = 4
};

extern void (*GameInterface)(GI h);
extern void (*GameStateRestore)(int version);


#include "git.h"
extern FCEUGI *GameInfo;
extern int GameAttributes;

extern uint8 PAL;
extern int dendy;
extern int pal_emulation;
extern bool movieSubtitles;


typedef struct fceu_settings_struct {
	int PAL;
	int NetworkPlay;
	int SoundVolume;		//Master volume
	int TriangleVolume;
	int Square1Volume;
	int Square2Volume;
	int NoiseVolume;
	int PCMVolume;
	bool GameGenie;

	//the currently selected first and last rendered scanlines.
	int FirstSLine;
	int LastSLine;

	//the number of scanlines in the currently selected configuration
	int TotalScanlines() { return LastSLine - FirstSLine + 1; }

	//Driver-supplied user-selected first and last rendered scanlines.
	//Usr*SLine[0] is for NTSC, Usr*SLine[1] is for PAL.
	int UsrFirstSLine[2];
	int UsrLastSLine[2];

	//this variable isn't used at all, snap is always name-based
	//bool SnapName;
	uint32 SndRate;
	int soundq;
	int lowpass;
} FCEUS;

int FCEU_TextScanlineOffset(int y);
int FCEU_TextScanlineOffsetFromBottom(int y);

extern FCEUS FSettings;

bool CheckFileExists(const char* filename);	//Receives a filename (fullpath) and checks to see if that file exists

void FCEU_PrintError( __FCEU_PRINTF_FORMAT const char *format, ...)  __FCEU_PRINTF_ATTRIBUTE( 1, 2 );
void FCEU_printf( __FCEU_PRINTF_FORMAT const char *format, ...)  __FCEU_PRINTF_ATTRIBUTE( 1, 2 );
void FCEU_DispMessage( __FCEU_PRINTF_FORMAT const char *format, int disppos, ...)  __FCEU_PRINTF_ATTRIBUTE( 1, 3 );
void FCEU_DispMessageOnMovie( __FCEU_PRINTF_FORMAT const char *format, ...)  __FCEU_PRINTF_ATTRIBUTE( 1, 2 );
void FCEU_TogglePPU();

void SetNESDeemph_OldHacky(uint8 d, int force);
void DrawTextTrans(uint8 *dest, uint32 width, uint8 *textmsg, uint8 fgcolor);
void FCEU_PutImage(void);
#ifdef FRAMESKIP
void FCEU_PutImageDummy(void);
#endif

#ifdef WIN32
extern void UpdateCheckedMenuItems();
extern void PushCurrentVideoSettings();
#endif

extern uint8 Exit;
extern int default_palette_selection;
extern uint8 vsdip;

//#define FCEUDEF_DEBUGGER //mbg merge 7/17/06 - cleaning out conditional compiles

// v1.13 Purify H: #define → constexpr
inline constexpr uint8_t JOY_A      = 0x01;
inline constexpr uint8_t JOY_B      = 0x02;
inline constexpr uint8_t JOY_SELECT = 0x04;
inline constexpr uint8_t JOY_START  = 0x08;
inline constexpr uint8_t JOY_UP     = 0x10;
inline constexpr uint8_t JOY_DOWN   = 0x20;
inline constexpr uint8_t JOY_LEFT   = 0x40;
inline constexpr uint8_t JOY_RIGHT  = 0x80;

// v1.13 Purify H: #define → constexpr
inline constexpr int LOADER_INVALID_FORMAT  = 0;
inline constexpr int LOADER_OK              = 1;
inline constexpr int LOADER_HANDLED_ERROR   = 2;
inline constexpr int LOADER_UNHANDLED_ERROR = 3;

// v1.13 Purify H: #define → constexpr
inline constexpr int EMULATIONPAUSED_PAUSED = 0x01;
inline constexpr int EMULATIONPAUSED_TIMER  = 0x02;
inline constexpr int EMULATIONPAUSED_FA     = 0x04;

inline constexpr int FRAMEADVANCE_DELAY_DEFAULT = 10;
inline constexpr int NES_HEADER_SIZE = 16;

#endif

#define ARRAY_SIZE(a) (sizeof(a)/sizeof(a[0]))  // needs type deduction — keep as macro

