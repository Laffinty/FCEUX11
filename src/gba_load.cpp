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

#include "drivers/common/nes_shm.h"
#include "fceu.h"
#include "rust/fceux11_rust.h"

// The NES pad state, and the video pool both sessions share.
extern uint8 joy[4];
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

	// GBA button bits, from the core's `GbaButton` (keypad.rs). A set bit means
	// held.
	enum GbaButtonBit : uint16_t
	{
		kGbaA = 1u << 0,
		kGbaB = 1u << 1,
		kGbaSelect = 1u << 2,
		kGbaStart = 1u << 3,
		kGbaRight = 1u << 4,
		kGbaLeft = 1u << 5,
		kGbaUp = 1u << 6,
		kGbaDown = 1u << 7,
		kGbaR = 1u << 8,
		kGbaL = 1u << 9,
	};

	// The NES pad's own bit order, read out of the Qt input backend rather than
	// assumed: `UpdateGamepad` sets bit 4 on "up", rejects 5 as its opposite and
	// 7 as the opposite of 6 ("left"), and treats `JS == 15` as A+B+Start+Select
	// (`drivers/Qt/input.cpp:1514-1531,1538`). So: 0 A, 1 B, 2 Select, 3 Start,
	// 4 Up, 5 Down, 6 Left, 7 Right.
	enum NesButtonBit : uint8_t
	{
		kNesA = 1u << 0,
		kNesB = 1u << 1,
		kNesSelect = 1u << 2,
		kNesStart = 1u << 3,
		kNesUp = 1u << 4,
		kNesDown = 1u << 5,
		kNesLeft = 1u << 6,
		kNesRight = 1u << 7,
	};
}  // namespace

bool fceu11_gba_active(void)
{
	return g_gbaActive;
}

void fceu11_gba_deactivate(void)
{
	if (!g_gbaActive) return;
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
	// Player 1. The NES side has no second player's worth of bindings to offer
	// a GBA, and a game that needs two GBA pads is not what section 7.2 promises.
	const uint8_t pad = joy[0];

	uint16_t mask = 0;
	if (pad & kNesA) mask |= kGbaA;
	if (pad & kNesB) mask |= kGbaB;
	if (pad & kNesSelect) mask |= kGbaSelect;
	if (pad & kNesStart) mask |= kGbaStart;
	if (pad & kNesUp) mask |= kGbaUp;
	if (pad & kNesDown) mask |= kGbaDown;
	if (pad & kNesLeft) mask |= kGbaLeft;
	if (pad & kNesRight) mask |= kGbaRight;

	// L and R are deliberately left at zero. Section 7.2 wants them on Z and X,
	// but on a NES pad those keys are *already* bound to something -- very often
	// to A and B -- so "L is the Z key" would quietly make L a second A. Which
	// key each shoulder gets is the binding work section S3 does, and until
	// then an unbound shoulder is honest where a guessed one is not.
	(void)kGbaL;
	(void)kGbaR;
	return mask;
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

	FCEU_printf("GBA: loaded %s\n", name);
	return LOADER_OK;
}
