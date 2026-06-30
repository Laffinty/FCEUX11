#ifndef _DEBUG_H_
#define _DEBUG_H_

#include <array>

#include "conddebug.h"
#include "git.h"
#include "nsf.h"

//watchpoint stuffs
#define WP_E       0x01  //watchpoint, enable
#define WP_W       0x02  //watchpoint, write
#define WP_R       0x04  //watchpoint, read
#define WP_X       0x08  //watchpoint, execute
#define WP_F       0x10  //watchpoint, forbid

#define BT_C       0x00  //break type, cpu mem
#define BT_P       0x20  //break type, ppu mem
#define BT_S       0x40  //break type, sprite mem
#define BT_R       0x80  //break type, rom mem

#define BREAK_TYPE_STEP -1
#define BREAK_TYPE_BADOP -2
#define BREAK_TYPE_CYCLES_EXCEED -3
#define BREAK_TYPE_INSTRUCTIONS_EXCEED -4
#define BREAK_TYPE_LUA -5
#define BREAK_TYPE_UNLOGGED_CODE -6
#define BREAK_TYPE_UNLOGGED_DATA -7

//opbrktype is used to grab the breakpoint type that each instruction will cause.
//WP_X is not used because ALL opcodes will have the execute bit set.
static const uint8 opbrktype[256] = {
	      /*0,    1, 2, 3,    4,    5,         6, 7, 8,    9, A, B,    C,    D,         E, F*/
/*0x00*/	0, WP_R, 0, 0,    0, WP_R, WP_R|WP_W, 0, 0,    0, 0, 0,    0, WP_R, WP_R|WP_W, 0,
/*0x10*/	0, WP_R, 0, 0,    0, WP_R, WP_R|WP_W, 0, 0, WP_R, 0, 0,    0, WP_R, WP_R|WP_W, 0,
/*0x20*/	0, WP_R, 0, 0, WP_R, WP_R, WP_R|WP_W, 0, 0,    0, 0, 0, WP_R, WP_R, WP_R|WP_W, 0,
/*0x30*/	0, WP_R, 0, 0,    0, WP_R, WP_R|WP_W, 0, 0, WP_R, 0, 0,    0, WP_R, WP_R|WP_W, 0,
/*0x40*/	0, WP_R, 0, 0,    0, WP_R, WP_R|WP_W, 0, 0,    0, 0, 0,    0, WP_R, WP_R|WP_W, 0,
/*0x50*/	0, WP_R, 0, 0,    0, WP_R, WP_R|WP_W, 0, 0, WP_R, 0, 0,    0, WP_R, WP_R|WP_W, 0,
/*0x60*/	0, WP_R, 0, 0,    0, WP_R, WP_R|WP_W, 0, 0,    0, 0, 0, WP_R, WP_R, WP_R|WP_W, 0,
/*0x70*/	0, WP_R, 0, 0,    0, WP_R, WP_R|WP_W, 0, 0, WP_R, 0, 0,    0, WP_R, WP_R|WP_W, 0,
/*0x80*/	0, WP_W, 0, 0, WP_W, WP_W,      WP_W, 0, 0,    0, 0, 0, WP_W, WP_W,      WP_W, 0,
/*0x90*/	0, WP_W, 0, 0, WP_W, WP_W,      WP_W, 0, 0, WP_W, 0, 0,    0, WP_W,         0, 0,
/*0xA0*/	0, WP_R, 0, 0, WP_R, WP_R,      WP_R, 0, 0,    0, 0, 0, WP_R, WP_R,      WP_R, 0,
/*0xB0*/	0, WP_R, 0, 0, WP_R, WP_R,      WP_R, 0, 0, WP_R, 0, 0, WP_R, WP_R,      WP_R, 0,
/*0xC0*/	0, WP_R, 0, 0, WP_R, WP_R, WP_R|WP_W, 0, 0,    0, 0, 0, WP_R, WP_R, WP_R|WP_W, 0,
/*0xD0*/	0, WP_R, 0, 0,    0, WP_R, WP_R|WP_W, 0, 0, WP_R, 0, 0,    0, WP_R, WP_R|WP_W, 0,
/*0xE0*/	0, WP_R, 0, 0, WP_R, WP_R, WP_R|WP_W, 0, 0,    0, 0, 0, WP_R, WP_R, WP_R|WP_W, 0,
/*0xF0*/	0, WP_R, 0, 0,    0, WP_R, WP_R|WP_W, 0, 0, WP_R, 0, 0,    0, WP_R, WP_R|WP_W, 0
};


typedef struct {
	uint32 address;
	uint32 endaddress;
	uint16 flags;

	Condition* cond;
	char* condText;
	char* desc;

} watchpointinfo;

//mbg merge 7/18/06 had to make this extern
extern watchpointinfo watchpoint[65]; //64 watchpoints, + 1 reserved for step over

extern unsigned int debuggerPageSize;
int getBank(int offs);
int GetNesFileAddress(int A);
int GetPRGAddress(int A);
int GetRomAddress(int A);
//int GetEditHex(HWND hwndDlg, int id);
uint8 *GetNesPRGPointer(int A);
uint8 *GetNesCHRPointer(int A);
void KillDebugger();
uint8 GetMem(uint16 A);
uint8 GetPPUMem(uint8 A);

//---------CDLogger
void LogCDVectors(int which);
void LogCDData(uint8 *opcode, uint16 A, int size);
extern volatile int codecount, datacount, undefinedcount;
extern unsigned char *cdloggerdata;
extern unsigned int cdloggerdataSize;

extern int debug_loggingCD;
static INLINE void FCEUI_SetLoggingCD(int val) { debug_loggingCD = val; }
static INLINE int FCEUI_GetLoggingCD() { return debug_loggingCD; }
//-------

//-------tracing
//we're letting the win32 driver handle this ittself for now
//extern int debug_tracing;
//static INLINE void FCEUI_SetTracing(int val) { debug_tracing = val; }
//static INLINE int FCEUI_GetTracing() { return debug_tracing; }
//---------

//--------debugger
extern int iaPC;
extern uint32 iapoffset; //mbg merge 7/18/06 changed from int
void DebugCycle();
bool CondForbidTest(int bp_num);
void BreakHit(int bp_num);

extern bool break_asap;
extern bool break_on_unlogged_code;
extern bool break_on_unlogged_data;
extern uint64 total_cycles_base;
extern uint64 delta_cycles_base;
extern bool break_on_cycles;
extern uint64 break_cycles_limit;
extern uint64 total_instructions;
extern uint64 delta_instructions;
extern bool break_on_instructions;
extern uint64 break_instructions_limit;
extern void ResetDebugStatisticsCounters();
extern void ResetCyclesCounter();
extern void ResetInstructionsCounter();
extern void ResetDebugStatisticsDeltaCounters();
extern void IncrementInstructionsCounters();
//-------------

//internal variables that debuggers will want access to
// VPage moved to fceu11::Bus in v1.4 Gateway Phase 2. The legacy
// global `VPage` is now an `extern` reference-to-array alias in
// bus.h that binds to g_bus.vpage(). Declarations in
// debug.h (and anywhere else that included the old `extern
// uint8* VPage[8]`) are removed.
// v1.5 Prism §1.1: vnapage / PPU[4] / SPRAM / VRAMBuffer /
// PPUGenLatch / XOffset moved similarly. vnapage and PPU[4]
// are now `extern` reference-to-array / reference-to-array
// aliases in ppu_class.h (which ppu.h transitively includes);
// SPRAM / VRAMBuffer / PPUGenLatch / XOffset stay as v1.0
// globals in ppu.cpp and are forward-declared in ppu.h. The
// duplicate declarations below were removed when the
// fceu11::Ppu class landed.
extern std::array<uint8_t, 0x20> PALRAM;
extern std::array<uint8_t, 3> UPALRAM;
extern uint32 FCEUPPU_PeekAddress();
extern uint8 READPAL_MOTHEROFALL(uint32 A);
extern int numWPs;

///encapsulates the operational state of the debugger core.
///
/// v0.2.25: All six fields now live in Rust (`fceux11-debug::debug` as
/// lock-free atomics). To preserve the existing `FCEUI_Debugger().step = v` /
/// `if (FCEUI_Debugger().step)` API across 30+ GUI call sites, each field is
/// exposed as a proxy that overloads `operator T()` (read) and
/// `operator=(T)` (write).
struct _DbgStateBoolProxy {
	bool (*getter)();
	void (*setter)(bool);
	operator bool() const { return getter(); }
	_DbgStateBoolProxy& operator=(bool v) { setter(v); return *this; }
};

struct _DbgStateU64Proxy {
	uint64 (*getter)();
	void   (*setter)(uint64);
	operator uint64() const { return getter(); }
	_DbgStateU64Proxy& operator=(uint64 v) { setter(v); return *this; }
};

struct _DbgStateI32Proxy {
	int (*getter)();
	void (*setter)(int);
	operator int() const { return getter(); }
	_DbgStateI32Proxy& operator=(int v) { setter(v); return *this; }
	_DbgStateI32Proxy& operator++();      // ++jsrcount  → Rust inc
	int operator++(int);                  // jsrcount++ — returns old, then inc
	_DbgStateI32Proxy& operator--();      // --jsrcount  → Rust dec
	int operator--(int);                  // jsrcount-- — returns old, then dec
};

class DebuggerState {
public:
	_DbgStateBoolProxy step;
	_DbgStateBoolProxy stepout;
	_DbgStateBoolProxy runline;
	_DbgStateU64Proxy  runline_end_time;
	_DbgStateBoolProxy badopbreak;
	_DbgStateI32Proxy  jsrcount;

	///resets the debugger state to an empty, non-debugging state
	void reset();
};

extern NSF_HEADER NSFHeader;

extern uint8_t   (& PSG            )[0x10];
extern uint8_t&    DMCFormat;
extern uint8_t&    RawDALatch;
extern uint8_t&    DMCAddressLatch;
extern uint8_t&    DMCSizeLatch;
extern uint8_t&    EnabledChannels;
extern uint8_t     SpriteDMA;
extern uint8_t     RawReg4016;
extern uint8_t&    IRQFrameMode;

///retrieves the core's DebuggerState
DebuggerState &FCEUI_Debugger();

//#define CPU_BREAKPOINT 1
//#define PPU_BREAKPOINT 2
//#define SPRITE_BREAKPOINT 4
//#define READ_BREAKPOINT 8
//#define WRITE_BREAKPOINT 16
//#define EXECUTE_BREAKPOINT 32

int offsetStringToInt(unsigned int type, const char* offsetBuffer, bool *conversionOk = nullptr);
unsigned int NewBreak(const char* name, int start, int end, unsigned int type, const char* condition, unsigned int num, bool enable);

#endif
