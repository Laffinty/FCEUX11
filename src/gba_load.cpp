// GBA cartridge loading and session state (v2.0 S2-b4 stage 1).
//
// This file is the C++ side of GBAEUX11's only front-end touchpoint in stage 1:
// recognising a `.gba`, handing it to the Rust core, and remembering that the
// active machine is no longer a NES. It deliberately does **not** touch video
// output or the throttle — those are stage 2 and stage 3, and the reasons they
// are separate are in the v2.0 plan's r39.
//
// # One machine at a time
//
// A session is either NES or GBA, never both. That is not a simplification for
// its own sake: the whole front end has exactly one frame slot, one audio ring
// and one throttle, and running both cores would mean two producers into one
// consumer. What the user gets is a second kind of game they can open, which is
// what the plan asks for -- `.gba` files are playable.
//
// # The screen will be black
//
// Stage 1 has no 32-bit path into the video pool, so a GBA session advances the
// machine and draws nothing. That is a deliberate choice over the alternative:
// leaving the previous NES frame on screen would show a **wrong** picture rather
// than a missing one, and a black screen cannot be mistaken for a game that
// rendered badly. `GbaLoad` clears the pool for exactly this reason.

#include "gba_load.h"

#include <string>
#include <vector>

#include <filesystem>
#include <fstream>
#include <iterator>

#include "drivers/common/nes_shm.h"
#include "fceu.h"
#include "rust/fceux11_rust.h"

// The video pool both sessions share. `joy[]` is deliberately *not* declared
// here any more: it was the GBA's button source until r51, and reading it did
// nothing at all, because the NES core is the only thing that writes it.
extern nes_shm_t* nes_shm;

// ---------------------------------------------------------------------------
// Session state
// ---------------------------------------------------------------------------

namespace
{
	bool g_gbaActive = false;
	std::string g_gbaPath;

	// The GBA's own frame, RGBA. 240x160x4 is 153,600 bytes.
	constexpr uint32_t kGbaFrameWidth = 240;
	constexpr uint32_t kGbaFrameHeight = 160;
	constexpr size_t kGbaFrameBytes =
		static_cast<size_t>(kGbaFrameWidth) * kGbaFrameHeight * 4;
	std::vector<uint8_t> g_frame;
	uint32_t g_frameSerial = 0;

	// The pad, pushed in by the Qt layer each frame in `GbaPadBit` order. See
	// fceu11_gba_button_mask for why it does not come from the NES side.
	//
	// There is deliberately no translation table from NES bits here. Section
	// 7.2's mapping is applied in the Qt layer, where the user's actual key
	// bindings live: A->A, B->B, Select->Select, Start->Start, the four
	// directions to the same, and the two NES shoulder keys to L and R.
	// Translating here instead meant reading `joy[]`, which the NES core is
	// the only writer of, and a GBA session does not run it.
	uint16_t g_padState = 0;

	// Where a GBA ROM's identifying byte lives, and its value. 0xB2 is the
	// "fixed value" the GBA header must carry for the BIOS to boot the cart;
	// every retail and homebrew cartridge has it, and no NES/UNIF/FDS/NSF file
	// has that byte in that place.
	constexpr long kHeaderFixedValueOffset = 0xB2;
	constexpr uint8_t kHeaderFixedValue = 0x96;

	// Enough of the header to read the byte above. The core wants 0xE4 and
	// checks the length itself; this is only so a truncated file is not read
	// past its end here.
	constexpr long kHeaderProbeBytes = 0xC0;
}  // namespace

bool fceu11_gba_active(void)
{
	return g_gbaActive;
}

/// Build a `std::filesystem::path` from UTF-8 bytes.
///
/// The paths reaching this file are UTF-8: Qt's `toStdString()` produces UTF-8,
/// and `gba_load_rom` reads the same bytes back through `from_utf8`. The
/// `path(const char*)` constructor decodes them as the **ANSI code page**
/// instead, and that is where a non-ASCII cartridge name used to die: the
/// mis-decoded path could not be mapped back onto the code page, so
/// `path::string()` threw "No mapping for the Unicode character exists in the
/// target multi-byte code page", and that exception escaped through the Qt
/// event loop into `std::terminate` -- the whole emulator went down while
/// merely opening a ROM, with nothing on stderr to say so.
///
/// C++20's `path(char8_t-sequence)` is the constructor that means "these bytes
/// are UTF-8", and unlike `string()` it cannot fail. `std::filesystem::u8path`
/// says the same thing but is deprecated in C++20.
static std::filesystem::path path_from_utf8(const std::string& utf8)
{
    return std::filesystem::path(std::u8string(utf8.begin(), utf8.end()));
}

/// The inverse, for messages. `u8string()` cannot fail either, which is rather
/// the point: printing a path must never be able to throw.
static std::string to_utf8(const std::filesystem::path& path)
{
    const std::u8string utf8 = path.u8string();
    return std::string(reinterpret_cast<const char*>(utf8.c_str()));
}

static bool read_file(const std::filesystem::path& path, std::vector<uint8_t>& out)
{
    // The path overload, not the `const char*` one: on Windows the latter
    // re-encodes through the code page and mangles anything non-ASCII.
    std::ifstream file(path, std::ios::binary);
    if (!file.is_open()) return false;
    out.assign(std::istreambuf_iterator<char>(file), std::istreambuf_iterator<char>());
    return true;
}
static bool write_file(const std::filesystem::path& path, const std::vector<uint8_t>& data)
{
    std::ofstream file(path, std::ios::binary | std::ios::trunc);
    if (!file.is_open()) return false;
    if (!data.empty())
    {
        file.write(reinterpret_cast<const char*>(data.data()),
                   static_cast<std::streamsize>(data.size()));
    }
    return file.good();
}

namespace
{
    /// The `.srm` that belongs to a cartridge: same directory, same stem.
    ///
    /// Empty when the ROM path has no parent to strip, which happens only for a
    /// bare filename -- and a bare filename cannot be opened either, so there is
    /// nothing to be relative to.
    ///
    /// A `std::filesystem::path` rather than a `std::string`, and never
    /// `string()` on it: this value has to survive a cartridge named in any
    /// script, and `string()` is the call that cannot promise that.
    std::filesystem::path battery_path_for(const std::string& rom)
    {
        const std::filesystem::path rom_path = path_from_utf8(rom);
        const std::filesystem::path parent = rom_path.parent_path();
        if (parent.empty())
        {
            return std::filesystem::path();
        }
        std::u8string stem = rom_path.stem().u8string();
        stem += u8".srm";
        return parent / stem;
    }
}  // namespace

void fceu11_gba_load_battery()
{
	if (!g_gbaActive) return;
	// A cartridge with no save hardware has nothing to restore, and asking for
	// one would be refused anyway -- so ask first and say nothing if not.
	if (gba_battery_save_type() == 0) return;

	const std::filesystem::path path = battery_path_for(g_gbaPath);
	if (path.empty()) return;

	std::error_code ec;
	if (!std::filesystem::exists(path, ec) || ec)
	{
		// No save file yet. A first run is not a problem worth a message.
		return;
	}

	std::vector<uint8_t> data;
	if (!read_file(path, data)) return;
	if (gba_battery_write(data.data(), static_cast<uint32_t>(data.size())) != GBA_OK)
	{
		FCEU_PrintError("The GBA save file could not be restored.");
		return;
	}
	const std::string shown = to_utf8(path);
	FCEU_printf("GBA: restored save from %s (%u bytes)\n", shown.c_str(), static_cast<unsigned>(data.size()));
}

void fceu11_gba_flush_battery()
{
	if (!g_gbaActive) return;
	// The flag is *taken* by the core, so asking is how this function learns
	// there is anything to do. A false here means already flushed.
	if (gba_battery_take_dirty() == 0) return;
	if (gba_battery_save_type() == 0) return;  // nothing to write, and must not guess

	uint32_t size = 0;
	if (gba_battery_size(&size) != GBA_OK || size == 0) return;
	std::vector<uint8_t> data(size);
	if (gba_battery_read(data.data(), size) != GBA_OK) return;

	const std::filesystem::path path = battery_path_for(g_gbaPath);
	if (path.empty()) return;
	if (write_file(path, data))
	{
		const std::string shown = to_utf8(path);
		FCEU_printf("GBA: wrote save to %s (%u bytes)\n", shown.c_str(), static_cast<unsigned>(size));
	}
	else
	{
		FCEU_PrintError("The GBA save could not be written.");
	}
}

bool fceu11_gba_savestate_save(const char* path)
{
	if (!g_gbaActive || !path || !*path) return false;

	// Two encodes, not one: `gba_savestate_size` serialises a state to measure
	// it, then `gba_savestate_save` serialises it again into our buffer. That is
	// the ABI's documented contract rather than an oversight -- the payload is
	// JSON, so its length cannot be computed without doing the work -- and at
	// the measured 2.4 ms a release encode it is not worth a second buffer or a
	// hand-rolled size estimate. A wrong estimate would fail as a capacity
	// error the caller cannot tell from a corrupt state.
	uint32_t size = 0;
	if (gba_savestate_size(&size) != GBA_OK || size == 0) return false;

	std::vector<uint8_t> data(size);
	uint32_t written = 0;
	if (gba_savestate_save(data.data(), size, &written) != GBA_OK) return false;
	data.resize(written);

	// UTF-8, like every other path this file handles: the same conversion the
	// battery save went through, and for the same reason -- a state file named
	// after a non-ASCII cartridge would otherwise die the same way.
	const std::filesystem::path save_path = path_from_utf8(path);

	if (!write_file(save_path, data))
	{
		FCEU_PrintError("The GBA state could not be written.");
		return false;
	}
	FCEU_printf("GBA: wrote state to %s (%u bytes)\n", path, static_cast<unsigned>(written));
	return true;
}

bool fceu11_gba_savestate_load(const char* path)
{
	if (!g_gbaActive || !path || !*path) return false;

	std::vector<uint8_t> data;
	if (!read_file(path_from_utf8(path), data)) return false;
	if (data.empty()) return false;

	// The core compares the ROM fingerprint before it touches the machine, so a
	// state belonging to another cartridge is a refusal and the session carries
	// on untouched. That is also what makes it safe for a GBA state and a NES
	// state to sit in the same slot directory: neither can be read as the other
	// by accident, because one of the two formats is not ours and the other
	// names a different cartridge.
	if (gba_savestate_load(data.data(), static_cast<uint32_t>(data.size())) != GBA_OK)
	{
		FCEU_PrintError("That GBA state was refused: wrong cartridge, or not a GBA state.");
		return false;
	}
	FCEU_printf("GBA: loaded state from %s (%u bytes)\n", path,
	            static_cast<unsigned>(data.size()));
	return true;
}

void fceu11_gba_deactivate(void)
{
	if (!g_gbaActive) return;
	// Flush before the machine goes, or the last few seconds of play are lost.
	// This is the one place a save must not be skipped.
	fceu11_gba_flush_battery();
	gba_unload_rom();
	g_gbaActive = false;
	g_gbaPath.clear();
}

const std::string& fceu11_gba_path(void)
{
	return g_gbaPath;
}

uint16_t fceu11_gba_button_mask(void)
{
	// Whatever the Qt layer pushed in this frame, in the core's own bit order.
	//
	// The translation from keys to GBA bits happens up there, where the user's
	// actual bindings live. This module has no way to ask: `joy[]` is written
	// by `UpdateGP` copying out of the NES controller port register
	// (`input.cpp:231`), and a GBA session does not run the NES core, so
	// nothing ever writes that register and `joy[]` stays zero. Reading it
	// here is what left all eight face and d-pad buttons dead while L and R
	// worked.
	return g_padState;
}

void fceu11_gba_set_pad_state(uint16_t mask)
{
	g_padState = mask;
}

GbaDrawSize fceu11_gba_draw_size(int view_w, int view_h)
{
	// Native size. A GBA frame is 240x160 and no video mode makes it anything
	// else, so this is a constant rather than a query.
	constexpr int native_w = 240;
	constexpr int native_h = 160;

	GbaDrawSize size{0, 0, true};

	// The largest whole multiple that fits on both axes. Integer division
	// floors, which is what "largest whole multiple" means.
	const int by_width = view_w / native_w;
	const int by_height = view_h / native_h;
	int factor = by_width < by_height ? by_width : by_height;

	if (factor >= 1)
	{
		size.width = native_w * factor;
		size.height = native_h * factor;
		return size;
	}

	// The viewport cannot hold the picture at 1:1. Fractional it is, so a
	// window narrower than 240 shows a small correct picture rather than a
	// clipped one. 240x160 is not the invariant here; the aspect ratio is.
	size.integral = false;
	const float sx = static_cast<float>(view_w) / static_cast<float>(native_w);
	const float sy = static_cast<float>(view_h) / static_cast<float>(native_h);
	const float scale = sx < sy ? sx : sy;
	size.width = static_cast<int>(static_cast<float>(native_w) * scale);
	size.height = static_cast<int>(static_cast<float>(native_h) * scale);
	if (size.width < 1) size.width = 1;
	if (size.height < 1) size.height = 1;
	return size;
}

void fceu11_gba_step_frame(void)
{
	if (!g_gbaActive) return;

	// Input first, so a button held during this frame is visible to the
	// instructions this frame runs. Full state, not a delta: the core releases
	// every button the mask does not name.
	gba_set_buttons(fceu11_gba_button_mask());

	// One frame of emulation, then one frame of pixels.
	//
	// Audio is deliberately **not** drained here even though the samples are
	// already available and `WriteSound` is the same call the NES side makes.
	// The sound device lives in the Qt driver library, and this file is in the
	// core library: calling across that edge is a link-time dependency the core
	// must not have, and the four F11QA test executables prove it -- they link
	// `fceux11_core` without the driver, so the unresolved `WriteSound` breaks
	// every one of them. Audio therefore belongs to stage 3', in the Qt layer,
	// where the device is; the samples stay in the core's ring until then.
	if (gba_step_frame() != GBA_OK) return;

	// Pull the frame out of the core into our own buffer. `gba_frame_buffer`
	// is the only way to get at it, and it applies the `BETA` watermark on the
	// way out, so the picture the viewer receives is the picture a player sees.
	if (g_frame.size() != kGbaFrameBytes) g_frame.assign(kGbaFrameBytes, 0);
	uint32_t size = 0;
	if (gba_frame_buffer_size(&size) != GBA_OK || size != kGbaFrameBytes) return;
	if (gba_frame_buffer(g_frame.data(), size) != GBA_OK) return;

	++g_frameSerial;

	// The one shared signal: "a new frame is ready". This is what makes the
	// GUI repaint -- `transferVideoBuffer` gates BOTH the copy and the redraw on
	// it, and the 120 Hz timer goes through the same gate, so without this
	// nothing would ever be drawn. Note what is *not* touched: `pixBufPool` and
	// `pixBufIdx` stay entirely the NES blitter's, which is what lets the NES
	// path remain byte-for-byte unchanged.
	if (nes_shm) nes_shm->blitUpdated.store(1, std::memory_order_release);

	// And if the game wrote its save memory, put it on disk. One flag test per
	// frame; the write only happens when something actually changed.
	fceu11_gba_flush_battery();
}

const uint8_t* fceu11_gba_frame()
{
	return g_gbaActive ? g_frame.data() : nullptr;
}

uint32_t fceu11_gba_frame_width()
{
	return kGbaFrameWidth;
}

uint32_t fceu11_gba_frame_height()
{
	return kGbaFrameHeight;
}

uint32_t fceu11_gba_frame_serial()
{
	return g_frameSerial;
}

double fceu11_gba_base_rate()
{
	// CPU clock / cycles per video frame. See gba_load.h for why this is
	// written here rather than exported.
	return 16777216.0 / (228.0 * 1232.0);
}

void fceu11_gba_configure_audio(uint32_t device_rate, uint32_t volume)
{
	if (!g_gbaActive) return;
	if (device_rate > 0) gba_set_output_rate(device_rate);
	gba_set_volume(volume);
}

uint32_t fceu11_gba_audio(int32_t* dst, uint32_t cap)
{
	if (!g_gbaActive || (dst == NULL) || (cap == 0)) return 0;
	uint32_t produced = 0;
	if (gba_render_audio(dst, cap, &produced) != GBA_OK) return 0;
	// Never report more than was asked for: the caller sized a buffer, and a
	// larger number here would be an out-of-bounds read on its side.
	return (produced > cap) ? cap : produced;
}

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

namespace
{
	/// Whether the open file looks like a GBA cartridge.
	///
	/// One byte, and that is enough: 0x96 at 0xB2 is the header's own "fixed
	/// value", which the BIOS checks before it will boot a cart. The other
	/// candidates for the slot (the 156-byte logo, the complement check) are
	/// either identical across all retail carts or cover only part of the
	/// header, so neither adds discrimination — and the one byte is the one a
	/// homebrew cart is most likely to get right, since the BIOS will not boot
	/// it otherwise.
	///
	/// Reads through the `FCEUFILE*` the chain already opened, rewinding first:
	/// the loaders before this one have moved it, and a seek is the only way to
	/// be sure. Going through `FCEUFILE*` rather than `fopen` also means a
	/// cartridge inside an archive is sniffed the same way a loose one is.
	bool looks_like_gba(FCEUFILE* fp)
	{
		if (!fp) return false;
		if (FCEU_fseek(fp, 0, SEEK_SET) != 0) return false;

		uint8_t header[kHeaderProbeBytes];
		const uint64_t read = FCEU_fread(header, 1, sizeof(header), fp);
		if (read < static_cast<uint64_t>(kHeaderProbeBytes)) return false;

		// Leave the stream where the next loader expects to find it. Every
		// loader in the chain seeks for itself, but not rewinding is a free way
		// to make the next one depend on this one's habits.
		FCEU_fseek(fp, 0, SEEK_SET);
		return header[kHeaderFixedValueOffset] == kHeaderFixedValue;
	}
}  // namespace

int GbaLoad(const char* name, FCEUFILE* fp)
{
	// Not ours. Returning this is what lets the chain go on to the next loader,
	// so a false negative costs a file and a false positive costs a game: the
	// check is the single fixed-value byte, nothing more ambitious.
	if (!looks_like_gba(fp)) return LOADER_INVALID_FORMAT;

	// A GBA session replaces whatever was running; a NES ROM loaded afterwards
	// replaces this one. Whichever way, only one of them owns the frame slot.
	fceu11_gba_deactivate();

	if (gba_load_rom(name) != GBA_OK)
	{
		FCEU_PrintError("The GBA cartridge was recognised but could not be loaded.");
		return LOADER_HANDLED_ERROR;
	}

	// The output rate is left at the core's default for this stage. Pushing the
	// device's real rate means asking the Qt sound layer for it, which is the
	// same core-to-driver edge as draining audio would be; stage 3 sets it from
	// the Qt side, where the device lives. Nothing reads it before then, because
	// nothing consumes the samples yet.
	//
	// Black, not stale. The alternative is leaving the last NES frame on screen
	// under a GBA session, which is a wrong picture rather than a missing one.
	if (nes_shm) nes_shm->clear_pixbuf();

	g_gbaActive = true;
	g_gbaPath = name;

	// The save file, if there is one. Loaded *after* the machine is built so it
	// lands in a buffer already sized for the detected type.
	fceu11_gba_load_battery();

	FCEU_printf("GBA: loaded %s\n", name);
	return LOADER_OK;
}
