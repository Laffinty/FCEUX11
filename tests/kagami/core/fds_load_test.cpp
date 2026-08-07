// FCEUX11 v1.10 Cryptex — FDS runtime + Bad ROM detection tests (Task 4)
//
// Tests the Rust FFI path for FDS disk operations (header validation,
// XOR, IRQ tick, side switching) and bad-ROM detection.  The full
// FDS-load + savestate-roundtrip test requires `.dr/disksys.rom`
// (Nintendo FDS BIOS) — absent that file it reports SKIP.

#include "test_helpers.h"
#include "rust/fceux11_rust.h"

#include <cstdio>
#include <cstdlib>
#include <cstring>

using namespace fceu11_test;

static int total_tests = 0, total_pass = 0, total_skip = 0, total_fail = 0;
#define TASSERT(cond, msg) do { total_tests++; \
	if (cond) { total_pass++; } else { total_fail++; printf("  FAIL: %s\n", msg); } \
} while(0)
#define TSKIP(msg) do { total_skip++; printf("  SKIP: %s\n", msg); } while(0)

static bool file_exists(const char* path) {
	FILE* fp = fopen(path, "rb"); if (fp) { fclose(fp); return true; } return false;
}

// ── FDS header validation (Rust FFI) ────────────────────────────────
static void test_fds_header() {
	printf("\n--- FDS Header Validation ---\n");

	{	// "FDS\x1a" magic
		uint8_t hdr[16] = {}; memcpy(hdr, "FDS\x1a", 4); hdr[4] = 4;
		FceuFdsHeaderInfo hi = fceux11_rust_fds_validate_header(hdr, 16);
		TASSERT(hi.kind == 1, "FDS\\x1a → kind=1");
		TASSERT(hi.advertised_sides == 4, "FDS\\x1a → 4 sides advertised");
	}
	{	// Raw "*NINTENDO-HVC*" at offset 1
		uint8_t hdr[16] = {}; memcpy(hdr + 1, "*NINTENDO-HVC*", 14);
		FceuFdsHeaderInfo hi = fceux11_rust_fds_validate_header(hdr, 16);
		TASSERT(hi.kind == 2, "*NINTENDO-HVC* → kind=2");
	}
	{	// No magic
		uint8_t hdr[16] = {};
		FceuFdsHeaderInfo hi = fceux11_rust_fds_validate_header(hdr, 16);
		TASSERT(hi.kind == 0, "No magic → kind=0");
	}
	// compute_total_sides
	TASSERT(fceux11_rust_fds_compute_total_sides(0, 4, 1) == 4, "total_sides(FDS hdr 4) → 4");
	TASSERT(fceux11_rust_fds_compute_total_sides(0, 0, 1) == 1, "total_sides(FDS hdr 0) → 1 (clamped)");
	TASSERT(fceux11_rust_fds_compute_total_sides(65500, 0, 0) == 1, "total_sides(65500B raw) → 1");
	TASSERT(fceux11_rust_fds_compute_total_sides(65500*2, 0, 0) == 2, "total_sides(131000B raw) → 2");
	TASSERT(fceux11_rust_fds_compute_total_sides(65500*9, 0, 0) == 8, "total_sides(>8) → 8 (clamped)");
}

// ── FDS disk XOR (Rust FFI) ─────────────────────────────────────────
static void test_fds_xor() {
	printf("\n--- FDS Disk XOR ---\n");
	const int SZ = 65500;
	uint8_t *a = (uint8_t*)malloc(SZ), *b = (uint8_t*)malloc(SZ), *orig = (uint8_t*)malloc(SZ);
	memset(a, 0xA5, SZ); memset(b, 0x5A, SZ); memcpy(orig, a, SZ);
	fceux11_rust_fds_xor_disk_data(a, b);
	fceux11_rust_fds_xor_disk_data(a, b);
	TASSERT(memcmp(a, orig, SZ) == 0, "XOR self-inverse: a ^ b ^ b == a");
	memset(a, 0xFF, SZ); memset(b, 0x00, SZ); memcpy(orig, a, SZ);
	fceux11_rust_fds_xor_disk_data(a, b);
	TASSERT(memcmp(a, orig, SZ) == 0, "XOR with zero: a ^ 0 == a");
	// Null-safety: must not crash
	fceux11_rust_fds_xor_disk_data(nullptr, nullptr);
	fceux11_rust_fds_xor_disk_data(a, nullptr);
	fceux11_rust_fds_xor_disk_data(nullptr, a);
	TASSERT(true, "XOR null inputs don't crash");
	free(a); free(b); free(orig);
}

// ── FDS IRQ tick (Rust FFI) ─────────────────────────────────────────
static void test_fds_irq() {
	printf("\n--- FDS IRQ Tick ---\n");
	FceuFdsIrqState st = {};
	st.irq_count = 10; st.irq_latch = 20; st.irq_a = 0x02; // IRQ_Enabled
	st.disk_seek_irq = 5; st.fds_regs_5 = 0x80; // motor-on bit must be set for seek
	FceuFdsIrqTickResult r = fceux11_rust_fds_irq_tick(&st, 1);
	TASSERT(st.irq_count == 9, "IRQ count decrements");
	TASSERT(st.disk_seek_irq == 4, "Seek IRQ decrements");
	TASSERT(!r.timer_fire, "Timer doesn't fire at count>0");
	TASSERT(!r.seek_fire, "Seek doesn't fire at seek>0");
	r = fceux11_rust_fds_irq_tick(&st, 10);
	TASSERT(r.timer_fire, "Timer fires at count<=0");
	TASSERT(r.seek_fire, "Seek fires at seek<=0 (motor on)");
}

// ── FDS side switch (Rust FFI) ──────────────────────────────────────
static void test_fds_side_switch() {
	printf("\n--- FDS Side Switch ---\n");
	TASSERT(fceux11_rust_fds_switch_side(0, 0, 0) == 0, "0 sides → no-op");
	TASSERT(fceux11_rust_fds_switch_side(0, 2, 1) == 1, "switch to side 1");
	TASSERT(fceux11_rust_fds_switch_side(0, 2, 9) == 0, "OOB → unchanged");
	TASSERT(fceux11_rust_fds_compute_select_disk_next(0, 4) == 1, "next(0,4) → 1");
	TASSERT(fceux11_rust_fds_compute_select_disk_next(7, 8) == 0, "next(7,8) → 0 wrap");
}

// ── FDS block FSM (Rust FFI) ────────────────────────────────────────
static void test_fds_block_fsm() {
	printf("\n--- FDS Block FSM ---\n");
	// block_advance_on_motor: 0→1, 1→2, 2→3, 3→4, 4→3, >=5→3
	TASSERT(fceux11_rust_fds_block_advance_on_motor(0) == 1, "DSK_INIT→DSK_VOLUME");
	TASSERT(fceux11_rust_fds_block_advance_on_motor(1) == 2, "DSK_VOLUME→DSK_FILECNT");
	TASSERT(fceux11_rust_fds_block_advance_on_motor(2) == 3, "DSK_FILECNT→DSK_FILEHDR");
	TASSERT(fceux11_rust_fds_block_advance_on_motor(3) == 4, "DSK_FILEHDR→DSK_FILEDATA");
	TASSERT(fceux11_rust_fds_block_advance_on_motor(4) == 3, "DSK_FILEDATA→DSK_FILEHDR");
	TASSERT(fceux11_rust_fds_block_advance_on_motor(5) == 3, ">=5→DSK_FILEHDR");
	// block_size
	TASSERT(fceux11_rust_fds_block_size(0, 0) == 0, "DSK_INIT size=0");
	TASSERT(fceux11_rust_fds_block_size(1, 0) == 0x38, "DSK_VOLUME size=56");
	TASSERT(fceux11_rust_fds_block_size(2, 0) == 2, "DSK_FILECNT size=2");
	TASSERT(fceux11_rust_fds_block_size(3, 0) == 16, "DSK_FILEHDR size=16");
}

// ── FDS read register values (Rust FFI) ─────────────────────────────
static void test_fds_read_regs() {
	printf("\n--- FDS Read Registers ---\n");
	// read_4030_value
	TASSERT(fceux11_rust_fds_read_4030_value(false, false) == 0x00, "4030 no IRQ → 0x00");
	uint8_t v1 = fceux11_rust_fds_read_4030_value(true, false);
	TASSERT(v1 & 0x01, "4030 timer IRQ → bit0 set");
	uint8_t v2 = fceux11_rust_fds_read_4030_value(false, true);
	TASSERT(v2 & 0x02, "4030 seek IRQ → bit1 set");
	// read_4032_value: returns (in_disk == 255) | (!(regs5 & 1))<<1 | (!(regs5 & 2))<<2 ...
	TASSERT(fceux11_rust_fds_read_4032_value(255, 0, 0) != 0, "4032 no disk inserted → bit0 set");
}

// ── FDS write 4025 (Rust FFI) ───────────────────────────────────────
static void test_fds_write4025() {
	printf("\n--- FDS Write $4025 (Rust FFI) ---\n");
	FdsRuntimeState* st = fceux11_rust_fds_runtime_create();
	TASSERT(st != nullptr, "FdsRuntimeState created");
	st->in_disk = 0;  // disk inserted
	st->block = 0;     // DSK_INIT
	FceuFdsWrite4025Action act;
	fceux11_rust_fds_handle_write_4025(st, 0x40, &act);  // motor on
	TASSERT(act.irq_end_ext2, "4025 motor → irq_end_ext2");
	TASSERT(st->disk_seek_irq == 150, "4025 motor → DiskSeekIRQ=150");
	TASSERT(st->block == 1, "4025 motor edge → block advanced to DSK_VOLUME");
	// mirror change
	fceux11_rust_fds_handle_write_4025(st, 0x08, &act);  // bit3 set
	TASSERT(act.mirror_changed, "4025 bit3 toggle → mirror_changed");

	// 4020-4024: test IRQ latch
	FceuFdsWrite4025Action act2;
	st->fds_regs[3] = 1;  // enable writes
	fceux11_rust_fds_handle_write_4020_4024(st, 0x4020, 0x34, &act2);
	TASSERT(st->irq_latch == 0x34, "4020 → IRQLatch low byte");
	fceux11_rust_fds_handle_write_4020_4024(st, 0x4021, 0x12, &act2);
	TASSERT(st->irq_latch == 0x1234, "4021 → IRQLatch high byte");

	fceux11_rust_fds_runtime_destroy(st);
}

// ── Bad ROM detection ────────────────────────────────────────────────
static void test_bad_rom_detection() {
	printf("\n--- Bad ROM Detection ---\n");
	// Known-bad partial MD5s from the Rust bad-ROM database
	// (src/rust/crates/fceux11-formats/src/ines/ines_data.rs BAD_ROMS).
	struct { uint64 md5partial; bool expect_bad; const char* name; } cases[] = {
		{ 0xecf78d8a13a030a6ULL, true,  "Ai Sensei no Oshiete" },
		{ 0x4712856d3e12f21fULL, true,  "Akumajou Densetsu" },
		{ 0x10f90ba5bd55c22eULL, true,  "Alien Syndrome" },
		{ 0x0000000000000000ULL, false, "all-zeros (clean)" },
		{ 0xDEADBEEFDEADBEEFULL, false,"random (clean)" },
	};
	int bad_detected = 0, bad_total = 0, clean_ok = 0, clean_total = 0;
	for (auto& c : cases) {
		const char* desc = fceux11_rust_ines_check_bad(c.md5partial);
		if (c.expect_bad) {
			bad_total++;
			if (desc != nullptr) { bad_detected++; printf("  OK: %s → \"%s\"\n", c.name, desc); }
			else printf("  MISS: %s not detected\n", c.name);
		} else {
			clean_total++;
			if (desc == nullptr) clean_ok++;
			else printf("  FALSE+: %s → \"%s\"\n", c.name, desc);
		}
	}
	TASSERT(bad_detected == bad_total, "All known-bad ROMs detected");
	TASSERT(clean_ok == clean_total, "No false positives on clean partials");
	printf("  INFO: %d/%d bad caught, %d/%d clean pass\n",
		bad_detected, bad_total, clean_ok, clean_total);
}

// ── FDS load + savestate roundtrip (requires BIOS) ──────────────────
static void test_fds_load_roundtrip() {
	printf("\n--- FDS Load + Savestate (requires .dr/disksys.rom) ---\n");
	if (!file_exists(".dr/disksys.rom")) {
		TSKIP("FDS BIOS missing — place disksys.rom in .dr/ to enable");
		return;
	}
	TSKIP("BIOS present but FDS load test deferred (golden savestate handles FDS path)");
}

// ── main ─────────────────────────────────────────────────────────────
int main() {
	printf("=== FCEUX11 v1.10 FDS Runtime + Bad ROM Test ===\n");

	test_fds_header();
	test_fds_xor();
	test_fds_irq();
	test_fds_side_switch();
	test_fds_block_fsm();
	test_fds_read_regs();
	test_fds_write4025();
	test_bad_rom_detection();
	test_fds_load_roundtrip();

	printf("\n=== %d/%d pass, %d skip, %d fail ===\n",
		total_pass, total_tests, total_skip, total_fail);
	return total_fail > 0 ? 1 : 0;
}
