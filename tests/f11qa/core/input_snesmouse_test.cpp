// F11QA — SNES mouse protocol lock tests (kgmqa-107 chain, D9).
//
// Drives FCEU_InitSNESMouse() directly through the INPUTC interface
// (input.h: _Write / _Read / _Update) and asserts the byte stream the
// emulated SNES mouse hands to the game on $4016:
//
//   byte0  signature (alternates 0xFF/0x00 per report — new-report marker)
//   byte1  0x1 low nibble | sensitivity<<4 | buttons<<6
//   byte2  dy: sign<<7 | overflow<<6 | magnitude (6 bit)
//   byte3  dx: sign<<7 | overflow<<6 | magnitude (6 bit)
//
// Vectors are assembled field-by-field (never hand-written literals) per
// repo discipline 5. The read path is: strobe falling edge latches a
// report; 32 reads at $4016 shift out byte0..byte3 MSB-first.
//
// Test list (red-before-fix expected: T1, T3, T4):
//   T1  signature alternation (A1)
//   T2  field assembly: buttons / sensitivity nibble / sign+magnitude (A-fix shape)
//   T3  6-bit magnitude + overflow bit (A4)
//   T4  sensitivity scaling x1/x2/x4, dy symmetric with dx (A2/A3)
//   T5  empty report + reads past 32 bits (read-out behaviour)
//   T6  sensitivity cycling while strobe high (existing behaviour lock)
//
// NOTE: a T7 "savestate roundtrip of the SNESMouse state (A5)" was planned
// but WITHDRAWN with A5 itself (2026-11-06 review): registering the driver
// state in FCEUCTRL_STATEINFO drifted the frozen savestate baselines
// (golden_savestate_test, golden_savestate_hashes.json), and the baselines
// are not to be regenerated for a peripheral this late. SNESMouse stays
// outside savestate serialization.

#include "test_helpers.h"

#include <cstdio>
#include <cstdint>
#include <cstring>
#include <array>

#include "input.h"

// snesmouse.cpp — core export; the test pokes the driver directly.
INPUTC *FCEU_InitSNESMouse(int w);

using namespace fceu11_test;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

namespace {

// Wrap the driver pointer with the exact strobe/read choreography a game
// performs: strobe high -> strobe low (latch build on falling edge) ->
// 32 reads on $4016, MSB-first -> byte0..byte3.
std::array<uint8_t, 4> read_report_bytes(INPUTC *d)
{
	std::array<uint8_t, 4> out{};
	uint32_t bits = 0;

	d->Write(1); // strobe high (no-op if already high)
	d->Write(0); // falling edge: latch the next report
	for (int i = 0; i < 32; ++i) {
		bits = (bits << 1) | (d->Read(0) & 1);
	}
	out[0] = (uint8_t)((bits >> 24) & 0xFF);
	out[1] = (uint8_t)((bits >> 16) & 0xFF);
	out[2] = (uint8_t)((bits >> 8) & 0xFF);
	out[3] = (uint8_t)(bits & 0xFF);
	return out;
}

// Feed pending relative movement + buttons via Update (the per-frame
// refresh path; data is int32[3] = {dx, dy, buttons}).
void feed(INPUTC *d, int32_t dx, int32_t dy, int32_t buttons)
{
	int32_t data[3] = {dx, dy, buttons};
	d->Update(0, data, 0);
}

// Advance the sensitivity setting while the strobe is held high: one $4016
// read per step, cycling 0 -> 1 -> 2 -> 0. Leaves the strobe HIGH, so the
// caller's next read_report_bytes() performs the single falling edge that
// latches the report.
void set_sensitivity(INPUTC *d, int steps)
{
	d->Write(1);
	for (int i = 0; i < steps; ++i) {
		d->Read(0);
	}
}

// Field-assembled expected bytes for one report (discipline 5: vectors are
// built from the protocol fields, not hand-written literals).
struct Fields {
	int sensitivity;   // 0..2
	int buttons;       // bit0 = left, bit1 = right
	bool dx_sign;
	bool dx_overflow;
	int dx_mag;
	bool dy_sign;
	bool dy_overflow;
	int dy_mag;
};

uint8_t expected_byte0(int report_index /*0-based*/ )
{
	// A1: first report carries 0xFF, then alternates per report.
	return (report_index % 2 == 0) ? 0xFF : 0x00;
}

uint8_t expected_byte1(const Fields &f)
{
	return (uint8_t)(0x1 | ((f.sensitivity & 3) << 4) | ((f.buttons & 3) << 6));
}

uint8_t expected_dbyte(bool sign, bool overflow, int mag)
{
	return (uint8_t)((sign ? 0x80 : 0x00) | (overflow ? 0x40 : 0x00) | (mag & 0x3F));
}

} // namespace

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

static void test_t1_signature_alternation(TestContext &ctx)
{
	std::printf("-- T1 signature alternation (A1)\n");
	INPUTC *d = FCEU_InitSNESMouse(0);

	// Three consecutive empty reports; the signature byte must alternate
	// 0xFF / 0x00 so the game can recognise the start of a new report.
	auto r1 = read_report_bytes(d);
	auto r2 = read_report_bytes(d);
	auto r3 = read_report_bytes(d);

	FCEU11_EXPECT(ctx, r1[0] == expected_byte0(0), "T1: report#1 byte0 == 0xFF");
	FCEU11_EXPECT(ctx, r2[0] == expected_byte0(1), "T1: report#2 byte0 == 0x00");
	FCEU11_EXPECT(ctx, r3[0] == expected_byte0(2), "T1: report#3 byte0 == 0xFF (alternation continues)");
}

static void test_t2_field_assembly(TestContext &ctx)
{
	std::printf("-- T2 field assembly (buttons / sensitivity / sign+magnitude)\n");
	INPUTC *d = FCEU_InitSNESMouse(0);

	const int32_t dx = 10;
	const int32_t dy = -5;
	const int32_t buttons = 3; // left + right
	feed(d, dx, dy, buttons);

	Fields f{};
	f.sensitivity = 0;
	f.buttons = (int)(buttons & 3);
	f.dx_sign = (dx < 0);
	f.dx_overflow = false;
	f.dx_mag = dx < 0 ? -(int)dx : (int)dx;
	f.dy_sign = (dy < 0);
	f.dy_overflow = false;
	f.dy_mag = dy < 0 ? -(int)dy : (int)dy;

	auto b = read_report_bytes(d);
	FCEU11_EXPECT(ctx, b[0] == expected_byte0(0), "T2: byte0 signature 0xFF");
	FCEU11_EXPECT(ctx, b[1] == expected_byte1(f), "T2: byte1 = 0x1 | sens<<4 | buttons<<6");
	FCEU11_EXPECT(ctx, b[2] == expected_dbyte(f.dy_sign, f.dy_overflow, f.dy_mag),
	              "T2: byte2 = dy sign<<7 | mag (6 bit)");
	FCEU11_EXPECT(ctx, b[3] == expected_dbyte(f.dx_sign, f.dx_overflow, f.dx_mag),
	              "T2: byte3 = dx sign<<7 | mag (6 bit)");

	// Negative dx / positive dy: signs must be independent per axis.
	feed(d, -7, 9, 0);
	auto b2 = read_report_bytes(d);
	FCEU11_EXPECT(ctx, b2[3] == expected_dbyte(true, false, 7), "T2: dx=-7 -> sign bit set, mag 7");
	FCEU11_EXPECT(ctx, b2[2] == expected_dbyte(false, false, 9), "T2: dy=+9 -> sign bit clear, mag 9");
}

static void test_t3_overflow(TestContext &ctx)
{
	std::printf("-- T3 6-bit magnitude + overflow bit (A4)\n");
	INPUTC *d = FCEU_InitSNESMouse(0);

	// |d| > 63: magnitude saturates at 63 and the overflow bit (bit6) sets.
	feed(d, 100, -100, 0);
	auto b = read_report_bytes(d);
	FCEU11_EXPECT(ctx, b[3] == expected_dbyte(false, true, 63), "T3: dx=+100 -> overflow, mag 63 (0x7F)");
	FCEU11_EXPECT(ctx, b[2] == expected_dbyte(true, true, 63), "T3: dy=-100 -> sign|overflow|mag 63 (0xFF)");

	// Boundary: 63 exactly -> no overflow; 64 -> overflow.
	feed(d, 63, -64, 0);
	auto b2 = read_report_bytes(d);
	FCEU11_EXPECT(ctx, b2[3] == expected_dbyte(false, false, 63), "T3: dx=+63 -> no overflow bit");
	FCEU11_EXPECT(ctx, b2[2] == expected_dbyte(true, true, 63), "T3: dy=-64 -> overflow bit set");
}

static void test_t4_sensitivity_scaling(TestContext &ctx)
{
	std::printf("-- T4 sensitivity x1/x2/x4, dy symmetric with dx (A2/A3)\n");
	INPUTC *d = FCEU_InitSNESMouse(0);

	// Same movement magnitude on both axes must come out identical after
	// the sensitivity scaling — the historical bug was dx scaled twice and
	// dy never scaled.
	const int32_t m = 10;
	for (int sens = 0; sens <= 2; ++sens) {
		if (sens > 0) {
			set_sensitivity(d, 1); // one read while strobe high: next step in cycle
		}
		feed(d, m, m, 0);
		auto b = read_report_bytes(d);

		const int want = m << sens; // hardware ratios x1/x2/x4
		Fields f{};
		f.sensitivity = sens;
		f.buttons = 0;

		char label_dx[64];
		char label_dy[64];
		char label_b1[64];
		std::snprintf(label_dx, sizeof(label_dx), "T4: sens=%d dx scaled to %d", sens, want);
		std::snprintf(label_dy, sizeof(label_dy), "T4: sens=%d dy scaled to %d (symmetric)", sens, want);
		std::snprintf(label_b1, sizeof(label_b1), "T4: sens=%d byte1 sensitivity nibble", sens);
		FCEU11_EXPECT(ctx, b[3] == expected_dbyte(false, false, want), label_dx);
		FCEU11_EXPECT(ctx, b[2] == expected_dbyte(false, false, want), label_dy);
		FCEU11_EXPECT(ctx, b[1] == expected_byte1(f), label_b1);
	}
}

static void test_t5_empty_report(TestContext &ctx)
{
	std::printf("-- T5 empty report + reads past 32 bits\n");
	INPUTC *d = FCEU_InitSNESMouse(0);

	feed(d, 0, 0, 0);
	auto b = read_report_bytes(d);
	FCEU11_EXPECT(ctx, b[2] == 0x00, "T5: idle report byte2 (dy) == 0");
	FCEU11_EXPECT(ctx, b[3] == 0x00, "T5: idle report byte3 (dx) == 0");

	// Reads past the 32-bit report shift the latch out to zero — no wrap,
	// no garbage (read-out behaviour; kept per B-level 'keep as-is').
	uint32_t extra = 0;
	for (int i = 0; i < 32; ++i) {
		extra |= (uint32_t)(d->Read(0) & 1) << i;
	}
	FCEU11_EXPECT(ctx, extra == 0, "T5: 32 reads past the report all return 0");
}

static void test_t6_sensitivity_cycle(TestContext &ctx)
{
	std::printf("-- T6 sensitivity cycling while strobe high\n");
	INPUTC *d = FCEU_InitSNESMouse(0);

	// Holding the strobe high and reading cycles sensitivity 0->1->2->0;
	// the cycle shows up in byte1's sensitivity nibble.
	for (int round = 0; round < 3; ++round) {
		set_sensitivity(d, 1); // one step: sens = (round+1) % 3
		feed(d, 0, 0, 0);
		auto b = read_report_bytes(d);
		const int want_sens = (round + 1) % 3;
		Fields f{};
		f.sensitivity = want_sens;
		f.buttons = 0;
		char label[80];
		std::snprintf(label, sizeof(label), "T6: after %d step(s) byte1 sensitivity nibble == %d",
		              round + 1, want_sens);
		FCEU11_EXPECT(ctx, b[1] == expected_byte1(f), label);
	}
}

int main()
{
	std::setvbuf(stdout, nullptr, _IONBF, 0);
	std::printf("=== FCEUX11 F11QA SNES mouse protocol test suite ===\n\n");

	TestContext ctx;

	if (!core_init()) {
		return 1;
	}

	test_t1_signature_alternation(ctx);
	test_t2_field_assembly(ctx);
	test_t3_overflow(ctx);
	test_t4_sensitivity_scaling(ctx);
	test_t5_empty_report(ctx);
	test_t6_sensitivity_cycle(ctx);

	core_shutdown();

	return report_and_exit(ctx, "SNES mouse protocol test suite");
}
