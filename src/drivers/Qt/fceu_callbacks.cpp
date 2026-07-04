/* FCEUX11 v1.11 Bridge — fceu_callbacks.cpp
 * Qt-side FCEUD_* callback implementations + register_driver() */

#include <stdio.h>
#include <stdlib.h>
#include <cstring>
#include <QThread>

#include "driver_callbacks.h"
#include "../../fceu.h"
#include "Qt/fceuWrapper.h"
#include "Qt/sdl.h"
#include "Qt/config.h"
#include "Qt/TasEditor/TasEditorWindow.h"
#include "utils/timeStamp.h"

// Phase C/D/E/F forward declarations
// Phase C: forward declarations for renamed FCEUD_* implementations
// (defined below in this file, referenced by register_driver() in fceuWrapperInit)
const char* fceuWrapper_GetCompilerString(void);
int   fceuWrapper_ShowStatusIcon(void);
void  fceuWrapper_ToggleStatusIcon(void);
void  fceuWrapper_HideMenuToggle(void);
bool  fceuWrapper_PauseAfterPlayback(void);
void  fceuWrapper_TurboOn(void);
void  fceuWrapper_TurboOff(void);
void  fceuWrapper_TurboToggle(void);

// Phase D: forward declarations for renamed FCEUD_* implementations (Batch 2)
FILE*             fceWrapper_UTF8fopen(const char *fn, const char *mode);
EMUFILE_FILE*     fceWrapper_UTF8_fstream(const char *fn, const char *m);
ArchiveScanRecord fceWrapper_ScanArchive(std::string fname);
FCEUFILE*         fceWrapper_OpenArchive(ArchiveScanRecord& asr, std::string& fname,
                                          std::string* innerFilename, int* userCancel);
FCEUFILE*         fceWrapper_OpenArchiveIndex(ArchiveScanRecord& asr, std::string& fname,
                                               int innerIndex, int* userCancel);

// Phase D: forward declarations for renamed FCEUD_* implementations (Batch 8)
void fceWrapper_AviRecordTo(void);
void fceWrapper_AviStop(void);
void fceWrapper_SaveStateAs(void);
void fceWrapper_LoadStateFrom(void);
void fceWrapper_MovieRecordTo(void);
void fceWrapper_MovieReplayFrom(void);

// Phase E: forward declarations for renamed FCEUD_* implementations
void     fceWrapper_SetPalette(uint8 index, uint8 r, uint8 g, uint8 b);
void     fceWrapper_GetPalette(uint8 i, uint8* r, uint8* g, uint8* b);
void     fceWrapper_VideoChanged(void);
bool     fceWrapper_ShouldDrawInputAids(void);
uint64   fceWrapper_GetTime(void);
uint64   fceWrapper_GetTimeFreq(void);
void     fceWrapper_SoundToggle(void);
void     fceWrapper_SoundVolumeAdjust(int n);
void     fceWrapper_SetInput(bool fourscore, bool microphone,
                              ESI port0, ESI port1, ESIFC fcexp);
void     fceWrapper_DebugBreakpoint(int bpNum);
void     fceWrapper_TraceInstruction(uint8 *opcode, int size);
void     fceWrapper_FlushTrace(void);
void     fceWrapper_UpdateNTView(int scanline, bool drawall);
void     fceWrapper_UpdatePPUView(int scanline, int refreshchr);

// Phase E: audit §5 supplemental callbacks
void     fceWrapper_GetKeyboardState(void* out);
void     fceWrapper_TaseditorDisableRunFunction(void);
const char* fceWrapper_GetThreadName(void);

// Phase D residual: UI refresh / dialog callbacks (registered in Phase F)
void     fceWrapper_SetMainWindowText(const char* s);
void     fceWrapper_UpdateRamSearch(void);
int      fceWrapper_MessageBox(const char* title, const char* msg, int type);
void     fceWrapper_EmuCommand(int cmd);

// Phase F: Batch 9 — Netplay callbacks (renamed from QtNetplay.cpp)
extern int  qNetplay_SendData(void* data, uint32 len);
extern int  qNetplay_RecvData(void* data, uint32 len);
extern void qNetplay_NetplayText(uint8* text);
extern void qNetplay_NetworkClose(void);

// ======== Phase D: Batch 2 — File I/O Qt implementations ========
FILE *fceWrapper_UTF8fopen(const char *fn, const char *mode)
{
   FILE *fp = ::fopen(fn,mode);
	return(fp);
}

EMUFILE_FILE* fceWrapper_UTF8_fstream(const char *fn, const char *m)
{
	std::ios_base::openmode mode = std::ios_base::binary;
	if(!strcmp(m,"r") || !strcmp(m,"rb"))
		mode |= std::ios_base::in;
	else if(!strcmp(m,"w") || !strcmp(m,"wb"))
		mode |= std::ios_base::out | std::ios_base::trunc;
	else if(!strcmp(m,"a") || !strcmp(m,"ab"))
		mode |= std::ios_base::out | std::ios_base::app;
	else if(!strcmp(m,"r+") || !strcmp(m,"r+b"))
		mode |= std::ios_base::in | std::ios_base::out;
	else if(!strcmp(m,"w+") || !strcmp(m,"w+b"))
		mode |= std::ios_base::in | std::ios_base::out | std::ios_base::trunc;
	else if(!strcmp(m,"a+") || !strcmp(m,"a+b"))
		mode |= std::ios_base::in | std::ios_base::out | std::ios_base::app;
    return new EMUFILE_FILE(fn, m);
	//return new std::fstream(fn,mode);
}

#if defined(MSVC)
 #ifdef _M_X64
   #define _MSVC_ARCH "x64"
 #else
   #define _MSVC_ARCH "x86"
 #endif
 #ifdef _DEBUG
  #define _MSVC_BUILD "debug"
 #else
  #define _MSVC_BUILD "release"
 #endif
 #define __COMPILER__STRING__ "msvc " _Py_STRINGIZE(_MSC_VER) " " _MSVC_ARCH " " _MSVC_BUILD
 #define _Py_STRINGIZE(X) _Py_STRINGIZE1((X))
 #define _Py_STRINGIZE1(X) _Py_STRINGIZE2 ## X
 #define _Py_STRINGIZE2(X) #X
 //re: http://72.14.203.104/search?q=cache:HG-okth5NGkJ:mail.python.org/pipermail/python-checkins/2002-November/030704.html+_msc_ver+compiler+version+string&hl=en&gl=us&ct=clnk&cd=5
#elif defined(__GNUC__)
 #define __COMPILER__STRING__ "gcc " __VERSION__
#else
 #define __COMPILER__STRING__ "unknown"
#endif

static const char *s_CompilerString = __COMPILER__STRING__;
/**
 * Returns the compiler string.
 */
const char *fceuWrapper_GetCompilerString(void)
{
	return s_CompilerString;
}

/**
 * Get the time in ticks.
 */
uint64 fceWrapper_GetTime(void)
{
	uint64 t;

	if (FCEU::timeStampModuleInitialized())
	{
		FCEU::timeStampRecord ts;

		ts.readNew();

		t = ts.toCounts();
	}
	else
	{
		t = (double)SDL_GetTicks();

		t = t * 1e-3;
	}
	return t;
}

/**
 * Get the tick frequency in Hz.
 */
uint64
fceWrapper_GetTimeFreq(void)
{
	// SDL_GetTicks() is in milliseconds
	uint64 f = 1000;

	if (FCEU::timeStampModuleInitialized())
	{
		f = FCEU::timeStampRecord::countFreq();
	}
	return f;
}

// ========= register_driver(cb) — called by fceuWrapperInit ==========
void fceuWrapper_registerCallbacks(void)
{
	{
		extern void msgLog_Message(const char*);
		extern void msgLog_PrintError(const char*);
		extern void throttle_SetEmulationSpeed(int);

		fceu11::DriverCallbacks cb{};
		cb.message = msgLog_Message;
		cb.print_error = msgLog_PrintError;
		cb.get_compiler_string = fceuWrapper_GetCompilerString;
		cb.show_status_icon = fceuWrapper_ShowStatusIcon;
		cb.toggle_status_icon = fceuWrapper_ToggleStatusIcon;
		cb.hide_menu_toggle = fceuWrapper_HideMenuToggle;
		cb.pause_after_playback = fceuWrapper_PauseAfterPlayback;
		cb.set_emulation_speed = throttle_SetEmulationSpeed;
		cb.turbo_on = fceuWrapper_TurboOn;
		cb.turbo_off = fceuWrapper_TurboOff;
		cb.turbo_toggle = fceuWrapper_TurboToggle;

		// Phase D: Batch 2 — File I/O
		cb.utf8_fopen     = fceWrapper_UTF8fopen;
		cb.utf8_fstream   = fceWrapper_UTF8_fstream;
		cb.scan_archive   = fceWrapper_ScanArchive;
		cb.open_archive   = fceWrapper_OpenArchive;
		cb.open_archive_index = fceWrapper_OpenArchiveIndex;

		// Phase D: Batch 8 — Driver-command file dialogs
		cb.avi_record_to    = fceWrapper_AviRecordTo;
		cb.avi_stop         = fceWrapper_AviStop;
		cb.save_state_as    = fceWrapper_SaveStateAs;
		cb.load_state_from  = fceWrapper_LoadStateFrom;
		cb.movie_record_to  = fceWrapper_MovieRecordTo;
		cb.movie_replay_from = fceWrapper_MovieReplayFrom;

		// Phase E: Batch 3 — Video / Palette
		cb.set_palette      = fceWrapper_SetPalette;
		cb.get_palette      = fceWrapper_GetPalette;
		cb.video_changed    = fceWrapper_VideoChanged;
		cb.should_draw_input_aids = fceWrapper_ShouldDrawInputAids;
		cb.get_time         = fceWrapper_GetTime;
		cb.get_time_freq    = fceWrapper_GetTimeFreq;

		// Phase E: Batch 4 — Sound
		cb.sound_toggle      = fceWrapper_SoundToggle;
		cb.sound_volume_adjust = fceWrapper_SoundVolumeAdjust;

		// Phase E: Batch 5 — Input
		cb.set_input         = fceWrapper_SetInput;

		// Phase E: Batch 6 — Debug
		cb.debug_breakpoint   = fceWrapper_DebugBreakpoint;
		cb.trace_instruction  = fceWrapper_TraceInstruction;
		cb.flush_trace        = fceWrapper_FlushTrace;
		cb.update_nt_view     = fceWrapper_UpdateNTView;
		cb.update_ppu_view    = fceWrapper_UpdatePPUView;

		// Phase E: audit §5 supplemental callbacks
		cb.get_keyboard_state = fceWrapper_GetKeyboardState;
		cb.taseditor_disable_run_function = fceWrapper_TaseditorDisableRunFunction;
		cb.get_thread_name = fceWrapper_GetThreadName;

		// Phase D residual: UI refresh / dialog callbacks
		cb.set_main_window_text = fceWrapper_SetMainWindowText;
		cb.update_ram_search    = fceWrapper_UpdateRamSearch;
		cb.message_box          = fceWrapper_MessageBox;
		cb.emu_command          = fceWrapper_EmuCommand;

		// Phase F: Batch 9 — Netplay callbacks
		cb.send_data     = qNetplay_SendData;
		cb.recv_data     = qNetplay_RecvData;
		cb.netplay_text  = qNetplay_NetplayText;
		cb.network_close = qNetplay_NetworkClose;

		fceu11::register_driver(cb);
	}
}

// ========= DUMMY / stub / simple callback implementations =========
// dummy functions

#define DUMMY(__f) \
    void __f(void) {\
        printf("%s\n", #__f);\
        FCEU_DispMessage("Not implemented.",0);\
    }
DUMMY(fceuWrapper_HideMenuToggle)
DUMMY(fceWrapper_MovieReplayFrom)
//DUMMY(FCEUD_AviRecordTo)
//DUMMY(FCEUD_AviStop)
//void fceu11::AviVideoUpdate(const unsigned char* buffer) { }
//bool fceu11::AviIsRecording(void) {return false;}
void fceu11::UseInputPreset(int preset) { }
bool fceuWrapper_PauseAfterPlayback() { return pauseAfterPlayback; }

int fceuWrapper_ShowStatusIcon(void)
{
	return showStatusIconOpt;
}
void fceuWrapper_ToggleStatusIcon(void)
{
	showStatusIconOpt = !showStatusIconOpt;
}

bool fceWrapper_ShouldDrawInputAids(void)
{
	return drawInputAidsEnable;
}

void fceuWrapper_TurboOn (void) { turbo = true; };
void fceuWrapper_TurboOff   (void) { turbo = false; };
void fceuWrapper_TurboToggle(void) { turbo = !turbo; };

// Phase E: audit §5 callbacks

void fceWrapper_GetKeyboardState(void* out) {
	uint8_t* keys = static_cast<uint8_t*>(out);
	memset(keys, 0, 256);
	const uint8_t* keyBuf = QtSDL_getKeyboardState(nullptr);
	if (keyBuf) {
		for (int i = 0; i < 256 && i < SDL_NUM_SCANCODES; i++) {
			if (keyBuf[i]) keys[i] = 0x80;
		}
	}
}

void fceWrapper_TaseditorDisableRunFunction(void) {
	extern TASEDITOR_LUA *taseditor_lua;
	if (taseditor_lua) taseditor_lua->disableRunFunction();
}

const char* fceWrapper_GetThreadName(void) {
	QThread* thread = QThread::currentThread();
	if (thread) {
		static thread_local std::string s;
		s = thread->objectName().toStdString();
		return s.c_str();
	}
	return "MainThread";
}

// Phase D residual: UI refresh / dialog callback stubs
void fceWrapper_SetMainWindowText(const char* ) { }

void fceWrapper_UpdateRamSearch(void) { }

int fceWrapper_MessageBox(const char* /*title*/, const char* msg, int /*type*/) {
	FCEUD_PrintError(msg);
	return 0;
}

void fceWrapper_EmuCommand(int cmd) {
	FCEU_DispMessage("EmuCommand %d not implemented.", 0, cmd);
}

