// F11QA — Famicom microphone protocol lock tests (kgmqa-108 chain, v2.0.4 V6).
//
// Drives the $4016/$4017 read handler (JPRead, src/input.cpp) through the
// real bus dispatch (ARead / BWrite aliases from bus.h) with a ROM loaded:
// PowerNES (inside fceu11::LoadGame) runs InitializeInput, which is what
// installs JPRead on 0x4016-0x4017 — without a ROM the bus still holds
// ANull and the handler is unreachable.
//
// Asserts the mic report (v2.0.4 V6 fix, replacing the 1996 per-read-toggle
// hack):
//
//   $4016 bit2 = microphone LEVEL read (pressed=1 on EVERY read,
//                released=0) — NESdev wiki "Standard controller", Famicom
//                section (r10 snapshot, Research_only/w204/): $4016 reads
//                xxxx xMES, bit2 = "Microphone status bit (Famicom, $4016
//                only)"; Famicom games track 0<->1 transitions of the
//                report, not a specific level, so a per-read toggle is a
//                protocol defect. Mesen2 NesController.h:116-118 agrees
//                (level read, no shift, $4016 only).
//   NOT part of the shift sequence (controller-I D0 stream untouched).
//   NOT present on $4017.
//   Source = P2 Start button only: replaceP2StartWithMicrophone routes the
//   P2 Start button into the mic (key mapping preserved); P1's Start never
//   produces the bit (P2-only peripheral — Mesen2 gates on port == 1).
//
// Vectors are assembled field-by-field (repo discipline 5): the joypad byte
// is built from named button bits, never hand-written literals; expected
// shift bits are derived from the assembled byte, not tabulated.
//
// Test list (red-before-fix under mutation M-1 below: T1, T2, T3, T4 —
// T2 goes red through its precondition line, which reads the pressed level
// first; T5/T6 stay green under M-1 and catch their own mutations):
//   T1  pressed -> bit2 == 1 on every $4016 read (the 1996 hack alternated)
//   T2  released -> bit2 == 0 (precondition: the pressed level was 1)
//   T3  mic state does not disturb the controller-I shift sequence, and the
//       level stays constant across a full read sequence
//   T4  $4017 never carries the mic bit (the 1996 hack toggled on ANY read)
//   T5  P1-side Start never produces the mic bit (P2-only peripheral)
//   T6  protocol gate: flag off -> no mic bit even with P2 Start pressed;
//       flag on -> P2 Start leaves the $4017 stream (existing nullify
//       mapping behaviour, unchanged) and appears as the mic bit on $4016
//
// Mutation legs (discipline 3). TRACED BY HAND against the pre-fix source
// (hunk preserved in the V6 task report) — EXECUTION PENDING the gate
// session, which owns the build; every leg below must be seen red before
// this suite is trusted:
//   M-1  revert to the 1996 toggle (static bool microphone; per-read
//        `microphone = !microphone` while joy[1]&8, bit2 only when true)  -> T1, T2, T3, T4 red
//   M-2  drop the !(A&1) gate (report on $4017 too)                       -> T4 red
//   M-3  widen the source to (joy[0]|joy[1])&8                            -> T5 red
//   M-4  drop the replaceP2StartWithMicrophone gate                       -> T6 leg A red
//   M-5  fold the mic into the shift stream (drive bit0 with the mic)     -> T3 red
//
// NOTE: the injection face (kagami_bridge_set_famicom_mic,
// f11qa_bridge.cpp) presses the same P2 Start bit through a persistent
// port-1 gamepad buffer — the bridge is not exercised here; these locks
// pin the core protocol the bridge feeds.

#include "core/test_helpers.h"

#include <cstdio>
#include <cstdint>

#include "input.h"  // joy[4] (src/input.h — core, not the Qt header)

using namespace fceu11_test;

// Core symbol (src/input.cpp; declared for the Qt driver in
// src/drivers/Qt/input.h:145 — declared here to avoid the Qt header).
extern bool replaceP2StartWithMicrophone;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

namespace {

// Controller byte layout (UpdateGamepad, Qt input.cpp: "a, b, select, start,
// up, down, left, right" — bit0..bit7 per controller).
constexpr uint8_t BTN_A     = 0x01;
constexpr uint8_t BTN_START = 0x08;
constexpr uint8_t BTN_LEFT  = 0x40;

constexpr uint8_t MIC_BIT = 0x04;  // $4016 D2 — microphone status
constexpr uint8_t PAD_BIT = 0x01;  // $4016/$4017 D0 — shift data line

// Persistent pad buffers for the SI_GAMEPAD attach (nothing dereferences
// them in this suite — UpdateGP only runs inside Emulate frames — but the
// attach gives the ports the real GPC driver so ReadGP/StrobeGP machinery
// is in play; core_init leaves SI_NONE / DummyJPort whose Read returns 0
// and never shifts).
uint32_t pad_state[2] = {0, 0};

void attach_gamepads()
{
	FCEUI_SetInput(0, static_cast<ESI>(SI_GAMEPAD), &pad_state[0], 0);
	FCEUI_SetInput(1, static_cast<ESI>(SI_GAMEPAD), &pad_state[1], 0);
}

// Real strobe choreography: write $4016=1 then $4016=0 — the falling edge
// runs B4016 -> StrobeGP -> joy_readbit reset. joy[] itself is pinned
// directly (white-box, mirroring how the movie load path feeds it).
void strobe_reset()
{
	BWrite[0x4016](0x4016, 1);
	BWrite[0x4016](0x4016, 0);
}

uint8_t read4016() { return ARead[0x4016](0x4016); }
uint8_t read4017() { return ARead[0x4017](0x4017); }

// Field-derived expected D0 for read index i (ReadGP, src/input.cpp):
// i < 8 -> bit i of the assembled pad byte; i >= 8 -> 1 (authentic pad
// reads D=1 past the 8 report bits; no fourscore in this suite).
uint8_t expected_shift_bit(int i, uint8_t pad)
{
	if (i >= 8) return 1;
	return (uint8_t)((pad >> i) & 1);
}

} // namespace

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

static void test_t1_pressed_level(TestContext &ctx)
{
	std::printf("-- T1 pressed -> bit2 == 1 on every $4016 read\n");
	replaceP2StartWithMicrophone = true;
	joy[0] = 0;
	joy[1] = BTN_START; // the preserved key mapping: P2 Start = mic pressed

	strobe_reset();
	for (int i = 0; i < 6; ++i) {
		char label[96];
		std::snprintf(label, sizeof(label),
		              "T1: $4016 read #%d bit2 == 1 (level read, no per-read toggle)", i);
		FCEU11_EXPECT(ctx, (read4016() & MIC_BIT) == MIC_BIT, label);
	}

	joy[1] = 0;
	replaceP2StartWithMicrophone = false;
}

static void test_t2_released_clear(TestContext &ctx)
{
	std::printf("-- T2 released -> bit2 == 0\n");
	replaceP2StartWithMicrophone = true;
	joy[0] = 0;
	joy[1] = BTN_START;

	strobe_reset();
	FCEU11_EXPECT(ctx, (read4016() & MIC_BIT) == MIC_BIT,
	              "T2: precondition — pressed read reports bit2 == 1");

	joy[1] = 0; // release
	strobe_reset();
	for (int i = 0; i < 3; ++i) {
		char label[96];
		std::snprintf(label, sizeof(label),
		              "T2: released $4016 read #%d bit2 == 0", i);
		FCEU11_EXPECT(ctx, (read4016() & MIC_BIT) == 0, label);
	}

	replaceP2StartWithMicrophone = false;
}

static void test_t3_shift_sequence_unaffected(TestContext &ctx)
{
	std::printf("-- T3 mic state does not disturb the shift sequence\n");
	replaceP2StartWithMicrophone = true;

	// Field-assembled P1 pattern (discipline 5): A + Start + Left.
	const uint8_t p1 = BTN_A | BTN_START | BTN_LEFT;
	joy[0] = p1;
	joy[1] = BTN_START; // mic pressed during the first pass

	// Pressed pass: the D0 shift stream must carry exactly the assembled
	// bits, and bit2 must sit at the constant pressed level on EVERY read
	// (the 1996 hack alternated bit2 mid-sequence without touching D0 —
	// this pin keeps the level-read fix from ever drifting into the shift).
	strobe_reset();
	for (int i = 0; i < 10; ++i) { // 8 report bits + 2 past-the-end
		const uint8_t r = read4016();
		char label[96];
		std::snprintf(label, sizeof(label),
		              "T3: pressed run read #%d D0 == expected shift bit", i);
		FCEU11_EXPECT(ctx, (r & PAD_BIT) == expected_shift_bit(i, p1), label);
		std::snprintf(label, sizeof(label),
		              "T3: pressed run read #%d bit2 == 1 (constant across the sequence)", i);
		FCEU11_EXPECT(ctx, (r & MIC_BIT) == MIC_BIT, label);
	}

	// Released pass, same P1 pattern: the shift stream must be identical —
	// the mic state never consumes a shift read.
	joy[1] = 0;
	strobe_reset();
	for (int i = 0; i < 10; ++i) {
		const uint8_t r = read4016();
		char label[96];
		std::snprintf(label, sizeof(label),
		              "T3: released run read #%d D0 == same expected shift bit", i);
		FCEU11_EXPECT(ctx, (r & PAD_BIT) == expected_shift_bit(i, p1), label);
	}

	replaceP2StartWithMicrophone = false;
}

static void test_t4_not_on_4017(TestContext &ctx)
{
	std::printf("-- T4 $4017 never carries the mic bit\n");
	replaceP2StartWithMicrophone = true;
	joy[0] = 0;
	joy[1] = BTN_START;

	strobe_reset();
	// The 1996 hack toggled on ANY read (both registers): the very first
	// $4017 read with the mic held raised bit2 there too.
	FCEU11_EXPECT(ctx, (read4017() & MIC_BIT) == 0,
	              "T4: first $4017 read bit2 == 0 (mic is $4016 only)");
	FCEU11_EXPECT(ctx, (read4017() & MIC_BIT) == 0,
	              "T4: second $4017 read bit2 == 0");

	// Precondition sanity: the same state DOES report on $4016.
	FCEU11_EXPECT(ctx, (read4016() & MIC_BIT) == MIC_BIT,
	              "T4: precondition — same state reports bit2 on $4016");

	joy[1] = 0;
	replaceP2StartWithMicrophone = false;
}

static void test_t5_p2_only(TestContext &ctx)
{
	std::printf("-- T5 P1-side Start never produces the mic bit\n");
	replaceP2StartWithMicrophone = true;
	joy[0] = BTN_START; // P1 Start pressed
	joy[1] = 0;         // P2 Start clear -> mic released

	strobe_reset();
	for (int i = 0; i < 4; ++i) {
		char label[96];
		std::snprintf(label, sizeof(label),
		              "T5: P1-only Start $4016 read #%d bit2 == 0", i);
		FCEU11_EXPECT(ctx, (read4016() & MIC_BIT) == 0, label);
	}

	// Sanity (guards a false green): P1 Start really entered the shift
	// stream — read index 3 carries bit3 of the assembled byte.
	strobe_reset();
	uint8_t r3 = 0;
	for (int i = 0; i < 4; ++i) r3 = read4016();
	FCEU11_EXPECT(ctx, (r3 & PAD_BIT) == expected_shift_bit(3, BTN_START),
	              "T5: sanity — P1 Start appears on the controller-I shift (read #3)");

	replaceP2StartWithMicrophone = false;
}

static void test_t6_protocol_gate_and_mapping(TestContext &ctx)
{
	std::printf("-- T6 protocol gate + P2 Start mapping lock\n");

	// Leg A: flag off (NES mode) -> no mic bit even with P2 Start pressed.
	replaceP2StartWithMicrophone = false;
	joy[0] = 0;
	joy[1] = BTN_START;
	strobe_reset();
	for (int i = 0; i < 3; ++i) {
		char label[96];
		std::snprintf(label, sizeof(label),
		              "T6: flag off $4016 read #%d bit2 == 0 despite P2 Start", i);
		FCEU11_EXPECT(ctx, (read4016() & MIC_BIT) == 0, label);
	}

	// Leg B: flag on -> P2 Start leaves the $4017 stream (existing
	// nullify-on-read mapping behaviour, deliberately unchanged by V6) and
	// the same press reports as the mic bit on $4016.
	replaceP2StartWithMicrophone = true;
	strobe_reset();
	uint8_t r_start = 0;
	for (int i = 0; i < 4; ++i) r_start = read4017();
	FCEU11_EXPECT(ctx, (r_start & PAD_BIT) == 0,
	              "T6: P2 Start nullified from the $4017 shift (mapping, unchanged)");
	FCEU11_EXPECT(ctx, (read4016() & MIC_BIT) == MIC_BIT,
	              "T6: same press reports as the mic bit on $4016");

	joy[1] = 0;
	replaceP2StartWithMicrophone = false;
}

int main()
{
	std::setvbuf(stdout, nullptr, _IONBF, 0);
	std::printf("=== FCEUX11 F11QA Famicom microphone protocol test suite ===\n\n");

	TestContext ctx;

	if (!core_init()) {
		return 1;
	}

	// PowerNES (inside load_rom) installs JPRead on 0x4016-0x4017 via
	// InitializeInput (fceu.cpp:1076) — without a loaded ROM the bus holds
	// ANull and the handler is unreachable.
	FCEUGI *gi = load_rom("fixtures/nestest.nes");
	if (!gi) {
		core_shutdown();
		return 1;
	}

	attach_gamepads();

	test_t1_pressed_level(ctx);
	test_t2_released_clear(ctx);
	test_t3_shift_sequence_unaffected(ctx);
	test_t4_not_on_4017(ctx);
	test_t5_p2_only(ctx);
	test_t6_protocol_gate_and_mapping(ctx);

	core_shutdown();

	return report_and_exit(ctx, "Famicom microphone protocol test suite");
}
