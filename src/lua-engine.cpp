#ifdef WIN32
#include <Windows.h>
#include <direct.h>
#define SetCurrentDir _chdir
#else
#error "Platform not supported"
#endif

#include "types.h"
#include "fceu.h"
#include "file.h"
#include "video.h"
#include "debug.h"
#include "debugsymboltable.h"
#include "sound.h"
#include "state.h"
#include "movie.h"
#include "driver.h"
#include "cheat.h"
#include "x6502.h"
#include "ppu.h"
#include "utils/xstring.h"
#include "utils/memory.h"
#include "utils/crc32.h"
#include "fceulua.h"
#include "input.h"

extern char FileBase[];

#ifdef __WIN_DRIVER__
#include "drivers/win/common.h"
#include "drivers/win/main.h"
#include "drivers/win/taseditor/selection.h"
#include "drivers/win/taseditor/laglog.h"
#include "drivers/win/taseditor/markers.h"
#include "drivers/win/taseditor/snapshot.h"
#include "drivers/win/taseditor/taseditor_lua.h"
#include "drivers/win/cdlogger.h"
extern TASEDITOR_LUA taseditor_lua;
#endif

#ifdef __SDL__

#ifdef __QT_DRIVER__
#include "drivers/Qt/sdl.h"
#include "drivers/Qt/main.h"
#include "drivers/Qt/input.h"
#include "drivers/Qt/fceuWrapper.h"
#include "drivers/Qt/TasEditor/selection.h"
#include "drivers/Qt/TasEditor/laglog.h"
#include "drivers/Qt/TasEditor/markers.h"
#include "drivers/Qt/TasEditor/snapshot.h"
#include "drivers/Qt/TasEditor/taseditor_lua.h"
extern TASEDITOR_LUA *taseditor_lua;
#else
int LoadGame(const char *path, bool silent = false);
int reloadLastGame(void);
void fceuWrapperRequestAppExit(void);
#endif

#endif

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <cctype>
#include <cassert>
#include <cstdlib>
#include <cmath>
#include <zlib.h>

#include <vector>
#include <map>
#include <string>
#include <algorithm>
#include <bitset>

#include "x6502abbrev.h"

// ==========================================================================
// FCEUX11 Rust Lua Engine: only FFI bridge functions and shared state.
// The old C++ Lua engine code has been removed (Phase A).
// Backed up at docs/history/legacy_code/lua-engine-cpp-else-block.txt
// ==========================================================================

extern bool turbo;
extern int32 fps_scale;

struct LuaSaveState {
	std::string filename;
	EMUFILE_MEMORY *data;
	bool anonymous, persisted;
	LuaSaveState()
		: data(0)
		, anonymous(false)
		, persisted(false)
	{}
	~LuaSaveState() {
		if(data) delete data;
	}
	void persist() {
		persisted = true;
		FILE* outf = fopen(filename.c_str(),"wb");
		fwrite(data->buf(),1,data->size(),outf);
		fclose(outf);
	}
	void ensureLoad() {
		if(data) return;
		persisted = true;
		FILE* inf = fopen(filename.c_str(),"rb");
		fseek(inf,0,SEEK_END);
		long int len = ftell(inf);
		fseek(inf,0,SEEK_SET);
		data = new EMUFILE_MEMORY(len);
		if ( fread(data->buf(),1,len,inf) != static_cast<size_t>(len) )
		{
			FCEU_printf("Warning: LuaSaveState::ensureLoad failed to load full buffer.\n");
		}
		fclose(inf);
	}
};

static uint8 luajoypads1[4]= { 0xFF, 0xFF, 0xFF, 0xFF };
static uint8 luajoypads2[4]= { 0x00, 0x00, 0x00, 0x00 };
static int luazapperx = -1;
static int luazappery = -1;
static int luazapperfire = -1;
static enum {SPEED_NORMAL, SPEED_NOTHROTTLE, SPEED_TURBO, SPEED_MAXIMUM} speedmode = SPEED_NORMAL;
static int transparencyModifier = 255;

static std::map<int, LuaSaveState*> s_savestate_objects;
static int s_next_savestate_id = 1;

// Rust FFI declarations — implemented in fceux11-lua crate
extern "C" {
    int fceux11_lua_init(void);
    int fceux11_lua_load_script(const char *path, const char *arg);
    void fceux11_lua_frame_boundary(void);
    void fceux11_lua_stop(void);
    int fceux11_lua_running(void);
    void fceux11_lua_gui(uint8_t *xbuf, int width, int height);
    uint8_t fceux11_lua_read_joypad(int controller, uint8_t original);
    int fceux11_lua_call_registered(int call_id);
    void fceux11_lua_call_mem_hook(unsigned int addr, int size, unsigned int value, int hook_type);
}

// Public C++ entry points — delegate to Rust FFI
void FCEU_LuaFrameBoundary() {
    fceux11_lua_frame_boundary();
}

int FCEU_LoadLuaCode(const char *filename, const char *arg) {
    if (!fceux11_lua_running()) {
        fceux11_lua_init();
    }
    return fceux11_lua_load_script(filename, arg);
}

void FCEU_ReloadLuaCode() {
    // TODO: store last script path and reload
}

void FCEU_LuaStop() {
    fceux11_lua_stop();
}

int FCEU_LuaRunning() {
    return fceux11_lua_running();
}

void FCEU_LuaGui(uint8_t *XBuf) {
    fceux11_lua_gui(XBuf, 256, 240);
}

uint8 FCEU_LuaReadJoypad(int controller, uint8 original) {
    return fceux11_lua_read_joypad(controller, original);
}

int FCEU_LuaSpeed() {
    return speedmode != SPEED_NORMAL;
}

int FCEU_LuaFrameskip() {
    return 0;
}

int FCEU_LuaRerecordCountSkip() {
    return 0;
}

void CallRegisteredLuaFunctions(LuaCallID calltype) {
    fceux11_lua_call_registered(static_cast<int>(calltype));
}

void CallRegisteredLuaMemHook(unsigned int address, int size, unsigned int value, LuaMemHookType hookType);

void CallRegisteredLuaSaveFunctions(int savestateNumber, LuaSaveData& saveData) {
    fceux11_lua_call_registered(3); // LUACALL_BEFORESAVE = 3
}

void CallRegisteredLuaLoadFunctions(int savestateNumber, const LuaSaveData& saveData) {
    fceux11_lua_call_registered(4); // LUACALL_AFTERLOAD = 4
}

void FCEU_LuaUpdatePalette() {
    // Rust engine does not hook palette updates
}

struct lua_State* FCEU_GetLuaState() {
    return nullptr;
}

char* FCEU_GetLuaScriptName() {
    return nullptr;
}

void FCEU_LuaReadZapper(const uint32* mouse_in, uint32* mouse_out) {
    // Zapper state managed via fceux11_lua_zapper_* FFI
}

// ==========================================================================
// Phase A cut: the full C++ Lua engine (#else block, ~6,600 lines) has been
// removed. Only the FFI bridge + thin shell remain. The removed code is
// backed up at docs/history/legacy_code/lua-engine-cpp-else-block.txt
//
// Below: MUST KEEP code extracted from the #else block and adapted for
// the Rust Lua engine (no lua_State* dependency).
// ==========================================================================

// ---------------------------------------------------------------------------
// LuaSaveData: ExportRecords / ImportRecords / ClearRecords
// These don't depend on lua_State* and are needed for savestate persistence.
// ---------------------------------------------------------------------------

#if defined(i386) || defined(__i386) || defined(__i386__) || defined(M_I86) || defined(_M_IX86) || defined(WIN32)
	#define IS_LITTLE_ENDIAN
#endif

template<typename T>
void PushBinaryItem(T item, std::vector<unsigned char>& output)
{
	unsigned char* buf = (unsigned char*)&item;
#ifdef IS_LITTLE_ENDIAN
	for(int i = sizeof(T); i; i--)
		output.push_back(*buf++);
#else
	int vecsize = output.size();
	for(int i = sizeof(T); i; i--)
		output.insert(output.begin() + vecsize, *buf++);
#endif
}

static void fwriteint(unsigned int value, FILE* file)
{
	for(int i=0;i<4;i++)
	{
		int w = value & 0xFF;
		fwrite(&w, 1, 1, file);
		value >>= 8;
	}
}

static void freadint(unsigned int& value, FILE* file)
{
	int rv = 0;
	for(int i=0;i<4;i++)
	{
		int r = 0;
		if ( fread(&r, 1, 1, file) == 0)
		{
			break;
		}
		rv |= r << (i*8);
	}
	value = rv;
}

void LuaSaveData::ExportRecords(void* fileV) const
{
	FILE* file = (FILE*)fileV;
	if(!file)
		return;

	Record* cur = recordList;
	while(cur)
	{
		fwriteint(cur->key, file);
		fwriteint(cur->size, file);
		fwrite(cur->data, cur->size, 1, file);
		cur = cur->next;
	}
}

void LuaSaveData::ImportRecords(void* fileV)
{
	FILE* file = (FILE*)fileV;
	if(!file)
		return;

	ClearRecords();

	Record rec;
	Record* cur = &rec;
	Record* last = NULL;
	while(1)
	{
		freadint(cur->key, file);
		freadint(cur->size, file);

		if(feof(file) || ferror(file))
			break;

		cur->data = new unsigned char [cur->size];
		if ( fread(cur->data, cur->size, 1, file) == 0 )
		{
			memset( cur->data, 0, cur->size );
		}

		Record* next = new Record();
		memcpy(next, cur, sizeof(Record));
		next->next = NULL;

		if(last)
			last->next = next;
		else
			recordList = next;
		last = next;
	}
}

void LuaSaveData::ClearRecords()
{
	Record* cur = recordList;
	while(cur)
	{
		Record* del = cur;
		cur = cur->next;

		delete[] del->data;
		delete del;
	}

	recordList = NULL;
}

// ---------------------------------------------------------------------------
// TieredRegion: fast-reject for memory hooks (performance critical)
// Adapted: dispatch matches to Rust FFI instead of lua_pcall
// ---------------------------------------------------------------------------

struct TieredRegion
{
	template<unsigned int maxGap>
	struct Region
	{
		struct Island
		{
			unsigned int start;
			unsigned int end;
			bool Contains(unsigned int address, int size) const { return address < end && address+size > start; }
		};
		std::vector<Island> islands;

		void Calculate(const std::vector<unsigned int>& bytes)
		{
			islands.clear();

			unsigned int lastEnd = ~0;

			std::vector<unsigned int>::const_iterator iter = bytes.begin();
			std::vector<unsigned int>::const_iterator end = bytes.end();
			for(; iter != end; ++iter)
			{
				unsigned int addr = *iter;
				if(addr < lastEnd || addr > lastEnd + (long long)maxGap)
				{
					islands.push_back(Island());
					islands.back().start = addr;
				}
				islands.back().end = addr+1;
				lastEnd = addr+1;
			}
		}
		bool Contains(unsigned int address, int size) const
		{
			for (size_t i = 0; i != islands.size(); ++i)
			{
				if (islands[i].Contains(address, size))
					return true;
			}
			return false;
		}
	};

	Region<0xFFFFFFFF> broad;
	Region<0x1000> mid;
	Region<0> narrow;

	void Calculate(std::vector<unsigned int>& bytes)
	{
		std::sort(bytes.begin(), bytes.end());

		broad.Calculate(bytes);
		mid.Calculate(bytes);
		narrow.Calculate(bytes);
	}

	TieredRegion()
	{
		std::vector <unsigned int> temp;
		Calculate(temp);
	}

	int NotEmpty()
	{
		return broad.islands.size() != 0;
	}

	bool Contains(unsigned int address, int size)
	{
		return broad.islands[0].Contains(address,size) &&
		       mid.Contains(address,size) &&
			   narrow.Contains(address,size);
	}
};

static TieredRegion hookedRegions [LUAMEMHOOK_COUNT];

// FFI declarations for hook management in Rust
extern "C" {
    int fceux11_lua_get_mem_hook_count(int hook_type);
    unsigned int fceux11_lua_get_mem_hook_address(int hook_type, int index);
}

static void CalculateMemHookRegions(LuaMemHookType hookType)
{
	std::vector<unsigned int> hookedBytes;
	int count = fceux11_lua_get_mem_hook_count(static_cast<int>(hookType));
	for (int i = 0; i < count; i++)
	{
		unsigned int addr = fceux11_lua_get_mem_hook_address(static_cast<int>(hookType), i);
		hookedBytes.push_back(addr);
	}
	hookedRegions[hookType].Calculate(hookedBytes);
}

extern "C" void fceux11_lua_recalculate_mem_hook_regions(int hook_type)
{
	if (hook_type >= 0 && hook_type < LUAMEMHOOK_COUNT)
		CalculateMemHookRegions(static_cast<LuaMemHookType>(hook_type));
}

// Override the simple CallRegisteredLuaMemHook from the thin shell
// with a TieredRegion fast-reject version
#undef CallRegisteredLuaMemHook

void CallRegisteredLuaMemHook(unsigned int address, int size, unsigned int value, LuaMemHookType hookType)
{
	if(hookedRegions[hookType].NotEmpty())
	{
		if(hookedRegions[hookType].Contains(address, size))
			fceux11_lua_call_mem_hook(address, size, value, static_cast<int>(hookType));
	}
}

// ---------------------------------------------------------------------------
// Taseditor stubs (full implementations require C++ taseditor API)
// ---------------------------------------------------------------------------
#ifdef __WIN_DRIVER__
void TaseditorDisableManualFunctionIfNeeded() {
	taseditor_lua.disableRunFunction();
}
#elif defined(__QT_DRIVER__)
void TaseditorDisableManualFunctionIfNeeded() {
	if (taseditor_lua) taseditor_lua->disableRunFunction();
}
#endif

// ---------------------------------------------------------------------------
// v0.2.22.2: Rust Lua FFI bridge functions
// These are called from fceux11-lua (Rust) via FFI
// ---------------------------------------------------------------------------

// C++ forward declarations needed by FFI bridge functions
// (must be outside extern "C" to get C++ name mangling)
extern ENVUNIT EnvUnits[3];
extern int CheckFreq(uint32 cf, uint8 sr);
extern int32 curfreq[2];
extern uint8 PSG[0x10];
extern int32 lengthcount[4];
extern uint8 TriCount;
extern char DMCHaveSample;
extern int32 DMCPeriod;
extern uint8 DMCFormat;
extern uint8 DMCAddressLatch;
extern uint8 DMCSizeLatch;
extern uint8 InitialRawDALatch;
extern const uint32 NoiseFreqTableNTSC[0x10];
extern const uint32 NoiseFreqTablePAL[0x10];
extern uint64 timestampbase;
extern uint64 total_cycles_base;
extern uint32 timestamp;
extern uint64 total_instructions;
extern bool break_asap;
extern void ResetDebugStatisticsCounters();
extern void ResetCyclesCounter();
extern void ResetInstructionsCounter();

void TaseditorAutoFunction() {}
void TaseditorManualFunction() {}
void ForceExecuteLuaFrameFunctions() {}

// v0.2.23 fix: s_get_mouse_data_fn and fceux11_lua_SetMouseDataCallback must
// use C++ linkage (C++ name mangling), because the only caller is
// `src/drivers/Qt/fceuWrapper.cpp` which declares the function with C++
// linkage. Previously these were inside the `extern "C"` block below, which
// made the linker look for an unmangled symbol that did not exist from the
// C++ caller's perspective. `fceux11_lua_GetMouseState` (which IS called
// from Rust) stays inside the `extern "C"` block; it can still access the
// file-scope static because `extern "C"` only affects name mangling, not
// accessibility of file-scope symbols.
static void (*s_get_mouse_data_fn)(uint32_t *md) = nullptr;

void fceux11_lua_SetMouseDataCallback(void (*fn)(uint32_t *md)) {
	s_get_mouse_data_fn = fn;
}

extern "C" {

uint8_t fceux11_lua_GetMem(uint32_t addr) {
	return GetMem(static_cast<uint16_t>(addr & 0xFFFF));
}

void fceux11_lua_BWrite(uint32_t addr, uint8_t val) {
	uint16_t a = static_cast<uint16_t>(addr & 0xFFFF);
	if (a < 0x8000) {
		// RAM/writable memory — call through BWrite handler
		writefunc wf = BWrite[a];
		if (wf) wf(a, val);
	}
}

uint16_t fceux11_lua_GetRegister(int reg_id) {
	// reg_id: 0=PC,1=A,2=X,3=Y,4=S,5=P
	switch (reg_id) {
		case 0: return X.PC;
		case 1: return X.A;
		case 2: return X.X;
		case 3: return X.Y;
		case 4: return X.S;
		case 5: return X.P;
		default: return 0;
	}
}

uint32_t fceux11_lua_GetJoypadState(int port) {
	// Returns current joypad state for port (0-3) from the joy[] array
	if (port < 0 || port > 3) return 0;
	return static_cast<uint32_t>(joy[port]);
}

void fceux11_lua_SetJoypadOverride(int port, uint32_t mask1, uint32_t mask2) {
	// Sets the Lua joypad override masks (pass-through & force bits)
	// luajoypads1 = pass-through mask, luajoypads2 = force-on mask
	if (port < 0 || port > 3) return;
	luajoypads1[port] = static_cast<uint8_t>(mask1 & 0xFF);
	luajoypads2[port] = static_cast<uint8_t>(mask2 & 0xFF);
}

uint8_t fceux11_lua_ReadRomByte(uint32_t addr) {
	return FCEU_ReadRomByte(addr);
}

void fceux11_lua_WriteRomByte(uint32_t addr, uint8_t val) {
	FCEU_WriteRomByte(addr, val);
}

int32_t fceux11_lua_GetRomMD5(uint8_t* buf) {
	if (!GameInfo) return -1;
	const uint8_t* md5 = GameInfo->MD5.data;
	if (!buf) return -1;
	memcpy(buf, md5, 16);
	return 0;
}

int32_t fceux11_lua_GetKeyboardState(uint8_t* keys) {
	if (!keys) return -1;
#ifdef __WIN_DRIVER__
	extern int EnableBackgroundInput;
	if (!EnableBackgroundInput) {
		if (!GetKeyboardState(keys)) return -1;
	} else {
		memset(keys, 0, 256);
		for (int i = 1; i < 255; i++) {
			int active;
			if (i == VK_CAPITAL || i == VK_NUMLOCK || i == VK_SCROLL)
				active = GetKeyState(i) & 0x01;
			else
				active = GetAsyncKeyState(i) & 0x8000;
			if (active) keys[i] = 0x80;
		}
	}
	return 0;
#elif defined(__QT_DRIVER__)
	memset(keys, 0, 256);
	const uint8_t *keyBuf = QtSDL_getKeyboardState(nullptr);
	if (keyBuf) {
		for (int i = 0; i < 256 && i < SDL_NUM_SCANCODES; i++) {
			if (keyBuf[i]) keys[i] = 0x80;
		}
	}
	return 0;
#else
	memset(keys, 0, 256);
	return -1;
#endif
}

void fceux11_lua_GetMouseState(int32_t* x, int32_t* y, int32_t* click) {
	uint32_t MouseData[3] = {0, 0, 0};
	if (s_get_mouse_data_fn) {
		s_get_mouse_data_fn(MouseData);
	}
	if (x) *x = MouseData[0];
	if (y) *y = MouseData[1];
	if (click) *click = MouseData[2];
}

// v0.2.22.3: Additional FFI bridge for new Rust Lua bindings
// ---------------------------------------------------------------------------

uint8_t fceux11_lua_PPURead(uint32_t addr) {
	return FFCEUX_PPURead(addr);
}

int32_t fceux11_lua_movie_get_mode() {
	if (FCEUMOV_IsRecording()) return 1;   // record
	if (FCEUMOV_IsPlaying()) return 2;      // playback
	if (FCEUMOV_Mode(MOVIEMODE_TASEDITOR)) return 3; // finished
	return 0; // none
}

int64_t fceux11_lua_movie_get_rerecordcount() {
	return FCEUI_GetMovieRerecordCount();
}

int64_t fceux11_lua_movie_get_length() {
	return FCEUI_GetMovieLength();
}

void fceux11_lua_movie_stop() {
	fceu11::StopMovie();
}

int32_t fceux11_lua_movie_get_readonly() {
	return fceu11::GetMovieToggleReadOnly() ? 1 : 0;
}

void fceux11_lua_movie_set_readonly(int32_t val) {
	fceu11::SetMovieToggleReadOnly(val != 0);
}

int32_t fceux11_lua_movie_is_poweron() {
	return FCEUMOV_FromPoweron() ? 1 : 0;
}

int32_t fceux11_lua_movie_is_from_savestate() {
	return FCEUMOV_FromPoweron() ? 0 : 1;
}

const char* fceux11_lua_movie_get_name() {
	// Returns internal movie name (from header or filename)
	static std::string name;
	name = fceu11::GetMovieName();
	return name.c_str();
}

const char* fceux11_lua_movie_get_filename() {
	// Returns filename stripped of path
	static std::string name;
	name = fceu11::GetMovieName();
	int x = name.find_last_of("/\\") + 1;
	if (x)
		name = name.substr(x, name.length() - x);
	return name.c_str();
}

// v0.2.22.4: savestate FFI
// ---------------------------------------------------------------------------

int32_t fceux11_lua_savestate_save_slot(int slot) {
	// slot is 0-9 (C++ numbering)
	FCEUSS_Save(FCEU_MakeFName(FCEUMKF_STATE, slot, 0).c_str(), false);
	return 0;
}

int32_t fceux11_lua_savestate_load_slot(int slot) {
	// slot is 0-9 (C++ numbering)
	return FCEUSS_Load(FCEU_MakeFName(FCEUMKF_STATE, slot, 0).c_str(), false) ? 0 : -1;
}

// v0.2.22.4: savestate object lifecycle
// LuaSaveState objects are managed by C++ but tracked via a simple ID map from Rust

int32_t fceux11_lua_savestate_create_object(const char* path, int which, int anonymous) {
	// which: 1-10 slot number, 0 = use path
	// anonymous: 1 = temp file, 0 = persistent
	int id = s_next_savestate_id++;
	LuaSaveState* ss = new LuaSaveState();
	if (which >= 1 && which <= 10) {
		// Original numbering: 1-10 maps to slots 0-9
		ss->filename = FCEU_MakeFName(FCEUMKF_STATE, which - 1, 0);
	} else if (path && path[0]) {
		ss->filename = path;
	}
	ss->anonymous = (anonymous != 0);
	ss->persisted = false;
	if (!ss->filename.empty() && !ss->anonymous) {
		ss->ensureLoad();
	}
	s_savestate_objects[id] = ss;
	return id;
}

void fceux11_lua_savestate_delete_object(int obj_id) {
	auto it = s_savestate_objects.find(obj_id);
	if (it != s_savestate_objects.end()) {
		delete it->second;
		s_savestate_objects.erase(it);
	}
}

int32_t fceux11_lua_savestate_object_save(int obj_id) {
	auto it = s_savestate_objects.find(obj_id);
	if (it == s_savestate_objects.end()) return -1;
	LuaSaveState* ss = it->second;
	if (ss->filename.empty()) return -1;
	FCEUSS_Save(ss->filename.c_str(), false);
	if (!ss->anonymous) ss->ensureLoad();
	return 0;
}

int32_t fceux11_lua_savestate_object_load(int obj_id) {
	auto it = s_savestate_objects.find(obj_id);
	if (it == s_savestate_objects.end()) return -1;
	LuaSaveState* ss = it->second;
	if (ss->filename.empty()) return -1;
	return FCEUSS_Load(ss->filename.c_str(), false) ? 0 : -1;
}

int32_t fceux11_lua_savestate_object_persist(int obj_id) {
	auto it = s_savestate_objects.find(obj_id);
	if (it == s_savestate_objects.end()) return -1;
	it->second->persist();
	return 0;
}

// v0.2.22.4: emu FFI (framecount/lagcount/paused/speedmode)
// ---------------------------------------------------------------------------

int64_t fceux11_lua_emu_get_framecount() {
	return FCEUMOV_GetFrame();
}

int64_t fceux11_lua_emu_get_lagcount() {
	return FCEUI_GetLagCount();
}

int32_t fceux11_lua_emu_is_paused() {
	return fceu11::IsEmulationPaused() ? 1 : 0;
}

void fceux11_lua_emu_set_speedmode(int mode) {
	speedmode = static_cast<decltype(speedmode)>(mode);
	if (mode == 0) {
		FCEUD_SetEmulationSpeed(EMUSPEED_NORMAL);
		FCEUD_TurboOff();
	} else if (mode == 2) {
		FCEUD_TurboOn();
	} else {
		FCEUD_SetEmulationSpeed(EMUSPEED_FASTEST);
	}
}

void fceux11_lua_emu_poweron() {
	if (GameInfo)
		fceu11::PowerNES();
}

void fceux11_lua_emu_softreset() {
	if (GameInfo)
		fceu11::ResetNES();
}

void fceux11_lua_emu_message(const char* msg) {
	FCEU_DispMessage("%s", 0, msg);
}

void fceux11_lua_emu_pause() {
	if (!fceu11::IsEmulationPaused())
		fceu11::ToggleEmulationPause();
}

void fceux11_lua_emu_unpause() {
	if (fceu11::IsEmulationPaused())
		fceu11::ToggleEmulationPause();
}

void fceux11_lua_gui_popup(const char* msg) {
	FCEUD_Message(msg);
}

void fceux11_lua_gui_savescreenshot(const char* filename) {
	if (filename && filename[0]) {
		FCEUI_SetSnapshotAsName(filename);
	}
	fceu11::SaveSnapshotAs();
}

// v0.2.22.6: P3 sound/zapper/debugger FFI
// ---------------------------------------------------------------------------

// Sound: square1
double fceux11_lua_sound_get_square1_volume() {
    if (curfreq[0] < 8 || curfreq[0] > 0x7ff ||
        CheckFreq(curfreq[0], PSG[1]) == 0 ||
        lengthcount[0] == 0) return 0.0;
    int mode = EnvUnits[0].Mode & 1;
    double vol = mode ? EnvUnits[0].Speed : EnvUnits[0].decvolume;
    return vol / 15.0;
}

double fceux11_lua_sound_get_square1_frequency() {
    return ((PAL?PAL_CPU:NTSC_CPU)/16.0) / (curfreq[0] + 1);
}

double fceux11_lua_sound_get_square1_midikey() {
    double freq = ((PAL?PAL_CPU:NTSC_CPU)/16.0) / (curfreq[0] + 1);
    return (log(freq / 440.0) * 12 / log(2.0)) + 69;
}

int fceux11_lua_sound_get_square1_duty() {
    return (PSG[0] & 0xC0) >> 6;
}

int fceux11_lua_sound_get_square1_regs() {
    return curfreq[0];
}

// Sound: square2
double fceux11_lua_sound_get_square2_volume() {
    if (curfreq[1] < 8 || curfreq[1] > 0x7ff ||
        CheckFreq(curfreq[1], PSG[5]) == 0 ||
        lengthcount[1] == 0) return 0.0;
    int mode = EnvUnits[1].Mode & 1;
    double vol = mode ? EnvUnits[1].Speed : EnvUnits[1].decvolume;
    return vol / 15.0;
}

double fceux11_lua_sound_get_square2_frequency() {
    return ((PAL?PAL_CPU:NTSC_CPU)/16.0) / (curfreq[1] + 1);
}

double fceux11_lua_sound_get_square2_midikey() {
    double freq = ((PAL?PAL_CPU:NTSC_CPU)/16.0) / (curfreq[1] + 1);
    return (log(freq / 440.0) * 12 / log(2.0)) + 69;
}

int fceux11_lua_sound_get_square2_duty() {
    return (PSG[4] & 0xC0) >> 6;
}

int fceux11_lua_sound_get_square2_regs() {
    return curfreq[1];
}

// Sound: triangle
double fceux11_lua_sound_get_triangle_volume() {
    if (lengthcount[2] == 0 || TriCount == 0) return 0.0;
    return 1.0;
}

int fceux11_lua_sound_get_triangle_linear() {
    return PSG[0xa] | ((PSG[0xb] & 7) << 8);
}

// Sound: noise
double fceux11_lua_sound_get_noise_volume() {
    if (lengthcount[3] == 0) return 0.0;
    int mode = EnvUnits[2].Mode & 1;
    double vol = mode ? EnvUnits[2].Speed : EnvUnits[2].decvolume;
    return vol / 15.0;
}

int fceux11_lua_sound_get_noise_mode() {
    return (PSG[0xE] & 0x80) != 0 ? 1 : 0;
}

int fceux11_lua_sound_get_noise_regs() {
    return PSG[0xE] & 0xF;
}

// Sound: DMC
double fceux11_lua_sound_get_dmc_volume() {
    return DMCHaveSample ? 1.0 : 0.0;
}

int fceux11_lua_sound_get_dmc_rate() {
    return DMCPeriod;
}

int fceux11_lua_sound_get_dmc_regs() {
    return DMCFormat & 0xF;
}

// Sound: frame sequencer
int fceux11_lua_sound_get_frame_sequencer() {
    return 0;
}

// Sound: triangle frequency/midikey
double fceux11_lua_sound_get_triangle_frequency() {
    int freqReg = PSG[0xa] | ((PSG[0xb] & 7) << 8);
    return ((PAL?PAL_CPU:NTSC_CPU)/32.0) / (freqReg + 1);
}

double fceux11_lua_sound_get_triangle_midikey() {
    int freqReg = PSG[0xa] | ((PSG[0xb] & 7) << 8);
    double freq = ((PAL?PAL_CPU:NTSC_CPU)/32.0) / (freqReg + 1);
    return (log(freq / 440.0) * 12 / log(2.0)) + 69;
}

// Sound: noise frequency/midikey
double fceux11_lua_sound_get_noise_frequency() {
    int freqReg = PSG[0xE] & 0xF;
    bool shortMode = ((PSG[0xE] & 0x80) != 0);
    double freq = PAL ? PAL_CPU/NoiseFreqTablePAL[freqReg] : NTSC_CPU/NoiseFreqTableNTSC[freqReg];
    if (shortMode) freq /= 93.0;
    return freq;
}

double fceux11_lua_sound_get_noise_midikey() {
    int freqReg = PSG[0xE] & 0xF;
    bool shortMode = ((PSG[0xE] & 0x80) != 0);
    double freq = PAL ? PAL_CPU/NoiseFreqTablePAL[freqReg] : NTSC_CPU/NoiseFreqTableNTSC[freqReg];
    if (shortMode) freq /= 93.0;
    return (log(freq / 440.0) * 12 / log(2.0)) + 69;
}

// Sound: DMC frequency/midikey/address/size/loop/seed
double fceux11_lua_sound_get_dmc_frequency() {
    return (PAL?PAL_CPU:NTSC_CPU) / (double)DMCPeriod;
}

double fceux11_lua_sound_get_dmc_midikey() {
    double freq = (PAL?PAL_CPU:NTSC_CPU) / (double)DMCPeriod;
    return (log(freq / 440.0) * 12 / log(2.0)) + 69;
}

int fceux11_lua_sound_get_dmc_address() {
    return 0xC000 + (DMCAddressLatch << 6);
}

int fceux11_lua_sound_get_dmc_size() {
    return (DMCSizeLatch << 4) + 1;
}

int fceux11_lua_sound_get_dmc_loop() {
    return (DMCFormat & 0x40) != 0 ? 1 : 0;
}

int fceux11_lua_sound_get_dmc_seed() {
    return InitialRawDALatch;
}

// Sound: sample rate and length
int fceux11_lua_sound_get_sample_rate() {
    return (int)((PAL?PAL_CPU:NTSC_CPU) / (curfreq[0] + 1));
}

int fceux11_lua_sound_get_length_count() {
    return lengthcount[0];
}

// Zapper — uses luazapperx/y/fire from lua-engine.cpp globals
int fceux11_lua_zapper_get_x() {
    if (luazapperx < 0) return 0;
    return luazapperx;
}

int fceux11_lua_zapper_get_y() {
    if (luazapperx < 0) return 0;
    return luazappery;
}

int fceux11_lua_zapper_get_click() {
    if (luazapperx < 0) return 0;
    return luazapperfire;
}

void fceux11_lua_zapper_set(int x, int y, int fire) {
    luazapperx = x;
    luazappery = y;
    luazapperfire = fire;
}

// Debugger
void fceux11_lua_debugger_hitbreakpoint() {
    break_asap = true;
}

uint64 fceux11_lua_debugger_get_cycles_count() {
    int64 counter_value = timestampbase + (uint64)timestamp - total_cycles_base;
    if (counter_value < 0) {
        ResetDebugStatisticsCounters();
        counter_value = 0;
    }
    return (uint64)counter_value;
}

uint64 fceux11_lua_debugger_get_instructions_count() {
    return total_instructions;
}

void fceux11_lua_debugger_reset_cycles_count() {
    ResetCyclesCounter();
}

void fceux11_lua_debugger_reset_instructions_count() {
    ResetInstructionsCounter();
}

int64 fceux11_lua_debugger_get_symbol_offset(const char* name) {
    if (!name || !name[0]) return -1;
    extern debugSymbolTable_t debugSymbolTable;
    debugSymbol_t* sym = debugSymbolTable.getSymbolAtAnyBank(name);
    return sym ? (int64)sym->offset() : -1;
}

} // extern "C"
