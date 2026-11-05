// F11QA kgmqa-121 — APU frame-counter cycle-exact lock tests (A2 baseline).
//
// Characterization baseline for work item A2 (batch stepping of the APU
// frame counter inside FCEU_SoundCPUHook). The test is its own per-cycle
// driver: it does NOT run Emulate; every CPU cycle is driven explicitly
// with FCEU_SoundCPUHook(1) and the observable tuple
//   (fhcnt, fcnt, SIRQStat&0x40, _IRQlow&FCEU_IQFCOUNT)
// is compared, cycle by cycle, against a reference model written from the
// approved event table (NOT copied from sound.cpp — the model below is the
// spec; the production comparison chain in FrameCounterTick is the fire
// authority and must agree with it):
//
//   NTSC mode0 : Q 7457 / H 14913 / Q 22371 / IRQ 29828 / IRQ+H 29829 /
//                WRAP 29830 (non-inhibit: flag+line persist across wrap)
//   NTSC mode1 : Q 7457 / H 14913 / Q 22371 / H 37281 / WRAP 37282, no IRQ
//   PAL  mode0 : Q 8313 / H 16627 / Q 24939 / IRQ 33252 / IRQ+H 33253 /
//                WRAP 33254
//   PAL  mode1 : Q 8313 / H 16627 / Q 24939 / H 41565 / WRAP 41566
//
// $4017 write semantics (Write_IRQFM):
//   - reset maturation delay = 3 CPU cycles when (timestampbase+ts) is even
//     (APU-aligned), 4 when odd (clock jitter);
//   - the sequencer is FROZEN (no fhcnt advance, no event) during the
//     countdown; maturation restarts at fhcnt=0,fcnt=0 and a 5-step write
//     additionally fires an immediate quarter+half clock at that cycle;
//   - IRQFrameMode takes effect immediately at the write, so the post-
//     maturation sequence walks the NEW table;
//   - an inhibit write (raw bit 6) clears the frame-IRQ flag and the CPU
//     IRQ line immediately at the write, before any further tick; a write
//     without inhibit must NOT clear them.
//
// This file was authored against the pre-A2 per-cycle implementation and
// must stay green on it. The expected timestamps are derived at runtime
// from the fhcnt base observed after reset (discipline 5: no hand-copied
// baseline literals for positions; the only literals are the approved
// event-table constants and the approved 3/4-cycle jitter delay).

#include "test_helpers.h"
#include "cpu.h"   // g_cpu (timestamp_ref), timestampbase

#include <cstdio>
#include <cstdint>
#include <cstring>
#include <vector>
#include <algorithm>

using namespace fceu11_test;

static const char* kRom = "fixtures/nestest.nes";

// ---------------------------------------------------------------------------
// Observable accessors
// ---------------------------------------------------------------------------
static int obs_line() {
    return (g_cpu.native_layout().IRQlow & FCEU_IQFCOUNT) ? 1 : 0;
}

static int obs_flag() {
    return (SIRQStat & 0x40) ? 1 : 0;
}

// ---------------------------------------------------------------------------
// Event table, assembled per field (discipline 5).
// quarter[0..1] are the pure quarter clocks; quarter[2] coincides with the
// terminal half clock (mode 1) or the second IRQ step (mode 0). irq_a/irq_b
// are -1 for 5-step tables (no IRQ steps).
// ---------------------------------------------------------------------------
struct FcTable {
    int quarter[3];
    int half1;
    int half2;
    int irq_a;
    int irq_b;
    int wrap;
};

static const FcTable kNtscMode0 = { {7457, 22371, 29829}, 14913, 29829, 29828, 29829, 29830 };
static const FcTable kNtscMode1 = { {7457, 22371, 37281}, 14913, 37281,   -1,    -1, 37282 };
static const FcTable kPalMode0  = { {8313, 24939, 33253}, 16627, 33253, 33252, 33253, 33254 };
static const FcTable kPalMode1  = { {8313, 24939, 41565}, 16627, 41565,   -1,    -1, 41566 };

// ---------------------------------------------------------------------------
// Reference model (the spec — see file header)
// ---------------------------------------------------------------------------
struct RefState {
    long long fhcnt;       // position since sequence start
    int fcnt;
    int flag;              // SIRQStat bit 6
    int line;              // _IRQlow FCEU_IQFCOUNT bit
    int reset_in;          // pending $4017 reset countdown
    int pending_mode;      // reduced (raw&0xC0)>>6 of the pending write
    int mode;              // current IRQFrameMode
    bool pal;
    long long t;           // absolute cycles since scenario start (labels)
};

static RefState ref_from_observed(bool pal) {
    RefState r;
    r.fhcnt = fhcnt;               // base offset read, not assumed
    r.fcnt = fcnt;
    r.flag = obs_flag();
    r.line = obs_line();
    r.reset_in = 0;                // fresh scenario precondition (FCEUSND_Reset)
    r.pending_mode = 0;
    r.mode = IRQFrameMode;
    r.pal = pal;
    r.t = 0;
    return r;
}

static void ref_write(RefState& r, uint8_t raw, int parity) {
    const int V = (raw & 0xC0) >> 6;
    r.reset_in = (parity == 0) ? 3 : 4;   // approved jitter: even->3, odd->4
    r.pending_mode = V;
    if (raw & 0x40) {                     // inhibit: immediate flag+line clear
        r.flag = 0;
        r.line = 0;
    }
    r.mode = V;                           // mode takes effect immediately
}

static void ref_tick(RefState& r) {
    r.t++;
    if (r.reset_in > 0) {
        r.reset_in--;
        if (r.reset_in == 0) {
            // Maturation: new sequence starts at cycle 0; a 5-step write
            // fires an immediate quarter+half clock on the same cycle.
            r.fhcnt = 0;
            r.fcnt = 0;
            if (r.pending_mode & 0x2) r.fcnt = (r.fcnt + 1) & 3;
        }
        return;                           // frozen: no fhcnt advance, no events
    }
    r.fhcnt++;
    const long long p = r.fhcnt;
    if (r.mode & 0x2) {
        // 5-step: no frame IRQ
        if (!r.pal) {
            if (p == 7457)  r.fcnt = (r.fcnt + 1) & 3;
            else if (p == 14913) r.fcnt = (r.fcnt + 1) & 3;
            else if (p == 22371) r.fcnt = (r.fcnt + 1) & 3;
            else if (p == 37281) r.fcnt = (r.fcnt + 1) & 3;
            else if (p == 37282) { r.fhcnt = 0; r.fcnt = 0; }
        } else {
            if (p == 8313)  r.fcnt = (r.fcnt + 1) & 3;
            else if (p == 16627) r.fcnt = (r.fcnt + 1) & 3;
            else if (p == 24939) r.fcnt = (r.fcnt + 1) & 3;
            else if (p == 41565) r.fcnt = (r.fcnt + 1) & 3;
            else if (p == 41566) { r.fhcnt = 0; r.fcnt = 0; }
        }
    } else {
        // 4-step (with or without inhibit: the inhibit bit only gates the
        // IRQ line, never the step positions)
        const int inhibit = (r.mode & 0x1) ? 1 : 0;
        if (!r.pal) {
            if (p == 7457)  r.fcnt = (r.fcnt + 1) & 3;
            else if (p == 14913) r.fcnt = (r.fcnt + 1) & 3;
            else if (p == 22371) r.fcnt = (r.fcnt + 1) & 3;
            else if (p == 29828) { r.flag = 1; if (!inhibit) r.line = 1; }
            else if (p == 29829) { r.flag = 1; if (!inhibit) r.line = 1;
                                   r.fcnt = (r.fcnt + 1) & 3; }
            else if (p == 29830) { // FrameIRQEnd + wrap
                if (inhibit) { r.flag = 0; r.line = 0; }
                else         { r.flag = 1; r.line = 1; }
                r.fhcnt = 0; r.fcnt = 0;
            }
        } else {
            if (p == 8313)  r.fcnt = (r.fcnt + 1) & 3;
            else if (p == 16627) r.fcnt = (r.fcnt + 1) & 3;
            else if (p == 24939) r.fcnt = (r.fcnt + 1) & 3;
            else if (p == 33252) { r.flag = 1; if (!inhibit) r.line = 1; }
            else if (p == 33253) { r.flag = 1; if (!inhibit) r.line = 1;
                                   r.fcnt = (r.fcnt + 1) & 3; }
            else if (p == 33254) {
                if (inhibit) { r.flag = 0; r.line = 0; }
                else         { r.flag = 1; r.line = 1; }
                r.fhcnt = 0; r.fcnt = 0;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Driver: one FCEU_SoundCPUHook(1) per cycle, per-cycle equality check
// against the model, and change-edge logging.
// ---------------------------------------------------------------------------
enum EdgeMask : int {
    EDG_NONE = 0,
    EDG_FCNT = 1,   // fcnt changed
    EDG_FLAG = 2,   // SIRQStat bit6 changed
    EDG_LINE = 4,   // CPU frame-IRQ line changed
    EDG_WRAP = 8,   // fhcnt decreased (wrap or reset maturation)
};

struct Edge {
    long long t;
    int mask;
    int fhcnt;   // observed fhcnt at the edge (diagnostics)
};

struct DriveResult {
    int mismatches = 0;
    std::vector<Edge> edges;
};

static DriveResult drive(RefState& ref, long long cycles, const char* label,
                         TestContext& ctx) {
    DriveResult res;
    int prev_fcnt = fcnt;
    int prev_flag = obs_flag();
    int prev_line = obs_line();
    int prev_fhcnt = fhcnt;
    for (long long i = 0; i < cycles; ++i) {
        FCEU_SoundCPUHook(1);
        ref_tick(ref);
        const int o_fcnt = fcnt;
        const int o_flag = obs_flag();
        const int o_line = obs_line();
        const int o_fhcnt = fhcnt;
        int mask = EDG_NONE;
        if (o_fcnt != prev_fcnt) mask |= EDG_FCNT;
        if (o_flag != prev_flag) mask |= EDG_FLAG;
        if (o_line != prev_line) mask |= EDG_LINE;
        if (o_fhcnt < prev_fhcnt) mask |= EDG_WRAP;
        if (mask != EDG_NONE) {
            res.edges.push_back({ ref.t, mask, o_fhcnt });
        }
        if (o_fhcnt != ref.fhcnt || o_fcnt != ref.fcnt ||
            o_flag != ref.flag || o_line != ref.line) {
            res.mismatches++;
            if (res.mismatches <= 5) {
                std::printf("  [%s] cycle %lld MISMATCH: "
                            "fhcnt obs=%d ref=%lld fcnt obs=%d ref=%d "
                            "flag obs=%d ref=%d line obs=%d ref=%d\n",
                            label, ref.t, o_fhcnt, ref.fhcnt,
                            o_fcnt, ref.fcnt, o_flag, ref.flag,
                            o_line, ref.line);
            }
        }
        prev_fcnt = o_fcnt;
        prev_flag = o_flag;
        prev_line = o_line;
        prev_fhcnt = o_fhcnt;
    }
    FCEU11_EXPECT(ctx, res.mismatches == 0, label);
    return res;
}

// Expected change-edges of a free-running sequence, derived from the field
// table. base = fhcnt value at sequence start (observed, not assumed).
// fcnt0 = fcnt at sequence start (drives whether the wrap edge also moves
// fcnt: the wrap sets fcnt=0, so an off-phase entry yields FCNT|WRAP).
// flag_rises: whether the FLAG|LINE edge at irq_a actually occurs (only
// when the flag starts the sequence cleared).
static std::vector<Edge> expected_seq_edges(const FcTable& tb, long long base,
                                            long long t_first,
                                            long long total,
                                            bool flag_rises,
                                            int fcnt0) {
    std::vector<Edge> out;
    int fcnt_ref = fcnt0;
    long long seq = 0;
    for (;; ++seq) {
        const long long off = t_first + seq * (long long)tb.wrap - base;
        if (off + tb.wrap >= t_first + total) break;
        out.push_back({ off + tb.quarter[0], EDG_FCNT, 0 });
        fcnt_ref = (fcnt_ref + 1) & 3;
        out.push_back({ off + tb.half1,      EDG_FCNT, 0 });
        fcnt_ref = (fcnt_ref + 1) & 3;
        out.push_back({ off + tb.quarter[1], EDG_FCNT, 0 });
        fcnt_ref = (fcnt_ref + 1) & 3;
        if (tb.irq_a > 0) {
            if (seq == 0 && flag_rises)
                out.push_back({ off + tb.irq_a, EDG_FLAG | EDG_LINE, 0 });
            // The half clock coincides with the second IRQ step.
            out.push_back({ off + tb.irq_b, EDG_FCNT, 0 });
            fcnt_ref = (fcnt_ref + 1) & 3;
        } else {
            out.push_back({ off + tb.half2, EDG_FCNT, 0 });
            fcnt_ref = (fcnt_ref + 1) & 3;
        }
        // Wrap: fhcnt restarts (WRAP); fcnt is assigned 0, which is only a
        // visible change when the entry phase left it non-zero.
        out.push_back({ off + tb.wrap,
                        EDG_WRAP | (fcnt_ref != 0 ? EDG_FCNT : 0), 0 });
        fcnt_ref = 0;
    }
    std::sort(out.begin(), out.end(),
              [](const Edge& a, const Edge& b) { return a.t < b.t; });
    return out;
}

static bool edges_match(const std::vector<Edge>& obs,
                        const std::vector<Edge>& exp) {
    if (obs.size() != exp.size()) return false;
    for (size_t i = 0; i < obs.size(); ++i) {
        if (obs[i].t != exp[i].t || obs[i].mask != exp[i].mask) return false;
    }
    return true;
}

static void print_edges(const char* tag, const std::vector<Edge>& v) {
    std::printf("  %s (%zu):\n", tag, v.size());
    for (const Edge& e : v) {
        std::printf("    t=%lld mask=%d fhcnt=%d\n", e.t, e.mask, e.fhcnt);
    }
}

// ---------------------------------------------------------------------------
// Scenario helpers
// ---------------------------------------------------------------------------

// Fresh power-phase APU state: mode 0, fhcnt=4 (read at runtime), flag 0.
// Also clears the frame-IRQ line a previous scenario may have left set.
static void reset_apu() {
    FCEUSND_Reset(true);
    X6502_IRQEnd(FCEU_IQFCOUNT);
}

// timestamp_ref does not advance while no CPU emulation runs, so the
// absolute parity of a $4017 write is fully controlled here.
static void set_write_parity(int parity) {
    const uint64_t base = timestampbase;
    g_cpu.timestamp_ref() =
        (uint32_t)(((base & 1) == (uint64_t)(unsigned)parity) ? 0 : 1);
}

static void do_write(RefState& ref, uint8_t raw, int parity) {
    set_write_parity(parity);
    BWrite[0x4017](0x4017, raw);   // dispatch-table write (mapper_test precedent)
    ref_write(ref, raw, parity);
}

// ---------------------------------------------------------------------------
// Scenarios
// ---------------------------------------------------------------------------

// S1a: NTSC mode0 free run, 2 full wraps. Locks Q/H/IRQ arrival times,
// the 29830 wrap and the non-inhibit flag-persistence topology.
static void scenario_s1_ntsc(TestContext& ctx) {
    reset_apu();
    RefState ref = ref_from_observed(false);   // PAL = 0
    const long long base = ref.fhcnt;
    const int fcnt0 = ref.fcnt;
    const long long total = 2 * (long long)kNtscMode0.wrap + 400;
    DriveResult res = drive(ref, total, "S1a NTSC mode0 free run: per-cycle equality", ctx);

    // Non-inhibit topology: flag and line rise at 29828 and persist
    // across both wraps (never cleared by FrameIRQEnd).
    FCEU11_EXPECT(ctx, ref.flag == 1, "S1a frame IRQ flag set");
    FCEU11_EXPECT(ctx, ref.line == 1, "S1a frame IRQ line asserted");

    std::vector<Edge> exp = expected_seq_edges(kNtscMode0, base, 0, total, true,
                                               fcnt0);
    FCEU11_EXPECT(ctx, edges_match(res.edges, exp),
                  "S1a edge log matches NTSC mode0 table (t = pos - fhcnt base)");
    if (!edges_match(res.edges, exp)) {
        print_edges("S1a observed", res.edges);
        print_edges("S1a expected", exp);
    }
}

// S1b: PAL mode0 free run, 2 full wraps.
static void scenario_s1_pal(TestContext& ctx) {
    PAL = 1;
    reset_apu();
    RefState ref = ref_from_observed(true);
    const long long base = ref.fhcnt;
    const int fcnt0 = ref.fcnt;
    const long long total = 2 * (long long)kPalMode0.wrap + 400;
    DriveResult res = drive(ref, total, "S1b PAL mode0 free run: per-cycle equality", ctx);

    FCEU11_EXPECT(ctx, ref.flag == 1, "S1b frame IRQ flag set");
    FCEU11_EXPECT(ctx, ref.line == 1, "S1b frame IRQ line asserted");

    std::vector<Edge> exp = expected_seq_edges(kPalMode0, base, 0, total, true, fcnt0);
    FCEU11_EXPECT(ctx, edges_match(res.edges, exp),
                  "S1b edge log matches PAL mode0 table (t = pos - fhcnt base)");
    if (!edges_match(res.edges, exp)) {
        print_edges("S1b observed", res.edges);
        print_edges("S1b expected", exp);
    }
    PAL = 0;
}

// S2: mode1 + inhibit (raw $C0). No frame IRQ ever; the 5-step maturation
// fires an immediate quarter+half clock (fcnt 0->1); sequence is
// 37281/37282 with the wrap edge.
static void scenario_s2(TestContext& ctx) {
    reset_apu();
    RefState ref = ref_from_observed(false);
    do_write(ref, 0xC0, 0);   // even parity -> matures after 3 cycles

    // Countdown freeze: fhcnt must stay frozen for the 2 pre-maturation
    // cycles and hit 0 on the 3rd.
    const int frozen = fhcnt;
    drive(ref, 2, "S2 countdown freeze (2 cycles)", ctx);
    FCEU11_EXPECT(ctx, fhcnt == frozen, "S2 fhcnt frozen during countdown");
    drive(ref, 1, "S2 maturation cycle", ctx);
    FCEU11_EXPECT(ctx, fhcnt == 0, "S2 maturation restarts fhcnt at 0");
    FCEU11_EXPECT(ctx, fcnt == 1, "S2 5-step immediate quarter+half clocked");

    const long long total = 2 * (long long)kNtscMode1.wrap + 400;
    const int fcnt0 = ref.fcnt;   // sequence start after maturation (=1)
    DriveResult res = drive(ref, total, "S2 mode1+inhibit free run: per-cycle equality", ctx);
    FCEU11_EXPECT(ctx, ref.flag == 0, "S2 no frame IRQ flag in inhibit mode");
    FCEU11_EXPECT(ctx, ref.line == 0, "S2 no frame IRQ line in inhibit mode");

    std::vector<Edge> exp = expected_seq_edges(kNtscMode1, 0, ref.t - total, total,
                                               false, fcnt0);
    FCEU11_EXPECT(ctx, edges_match(res.edges, exp),
                  "S2 edge log matches NTSC mode1 table (37281/37282)");
    if (!edges_match(res.edges, exp)) {
        print_edges("S2 observed", res.edges);
        print_edges("S2 expected", exp);
    }
}

// S3: reset maturation parity. Even CPU cycle -> 3-cycle delay, odd -> 4
// (the clock jitter); freeze visible in both; 5-step immediate clock on
// the maturation cycle.
static void scenario_s3(TestContext& ctx) {
    // S3a: even parity -> delay 3
    reset_apu();
    RefState ref = ref_from_observed(false);
    const int base_fcnt = fcnt;
    do_write(ref, 0x00, 0);
    const int frozen = fhcnt;
    drive(ref, 2, "S3a even-parity countdown freeze", ctx);
    FCEU11_EXPECT(ctx, fhcnt == frozen, "S3a fhcnt frozen during countdown");
    drive(ref, 1, "S3a maturation at write+3", ctx);
    FCEU11_EXPECT(ctx, fhcnt == 0, "S3a fhcnt restarted at cycle 3");
    FCEU11_EXPECT(ctx, fcnt == base_fcnt, "S3a mode0 write: no immediate clock");
    const long long t_matured = ref.t;
    DriveResult r3a = drive(ref, 7460, "S3a first quarter after maturation", ctx);
    // First fcnt change must be exactly 7457 cycles after maturation.
    FCEU11_EXPECT(ctx, r3a.edges.size() == 1 && r3a.edges[0].t == t_matured + 7457 &&
                  r3a.edges[0].mask == EDG_FCNT,
                  "S3a first quarter exactly at maturation+7457");

    // S3b: odd parity -> delay 4
    reset_apu();
    ref = ref_from_observed(false);
    do_write(ref, 0x00, 1);
    const int frozen_b = fhcnt;
    drive(ref, 3, "S3b odd-parity countdown freeze", ctx);
    FCEU11_EXPECT(ctx, fhcnt == frozen_b, "S3b fhcnt frozen through cycle 3");
    drive(ref, 1, "S3b maturation at write+4", ctx);
    FCEU11_EXPECT(ctx, fhcnt == 0, "S3b fhcnt restarted at cycle 4");

    // S3c: 5-step write fires the immediate quarter+half ON the maturation
    // cycle (not earlier, not later).
    reset_apu();
    ref = ref_from_observed(false);
    do_write(ref, 0xC0, 0);
    DriveResult r3c = drive(ref, 3, "S3c 5-step maturation window", ctx);
    FCEU11_EXPECT(ctx, fcnt == 1, "S3c 5-step immediate clock at maturation");
    FCEU11_EXPECT(ctx, r3c.edges.size() == 1 && r3c.edges[0].t == 3 &&
                  r3c.edges[0].mask == (EDG_FCNT | EDG_WRAP),
                  "S3c immediate clock edge exactly at write+3");
}

// S4: mode switch inside the write->maturation window. Mode0 runs to a
// set flag; an inhibit 5-step write clears flag+line immediately, the
// sequencer freezes, matures into the NEW table (37282 wrap) with the
// immediate quarter+half clock.
static void scenario_s4(TestContext& ctx) {
    reset_apu();
    RefState ref = ref_from_observed(false);
    drive(ref, (long long)kNtscMode0.wrap + 40,
          "S4 pre-phase mode0 free run", ctx);
    FCEU11_EXPECT(ctx, obs_flag() == 1 && obs_line() == 1,
                  "S4 pre-phase flag+line set");

    do_write(ref, 0xC0, 0);
    // Inhibit write clears flag+line immediately, before any further tick.
    FCEU11_EXPECT(ctx, obs_flag() == 0, "S4 inhibit write clears flag immediately");
    FCEU11_EXPECT(ctx, obs_line() == 0, "S4 inhibit write deasserts line immediately");

    const int frozen = fhcnt;
    drive(ref, 2, "S4 countdown freeze in new mode", ctx);
    FCEU11_EXPECT(ctx, fhcnt == frozen, "S4 fhcnt frozen during countdown");
    drive(ref, 1, "S4 maturation into new mode", ctx);
    FCEU11_EXPECT(ctx, fhcnt == 0, "S4 maturation restarts fhcnt at 0");
    FCEU11_EXPECT(ctx, fcnt == 1, "S4 5-step immediate clock at maturation");

    const long long total = 2 * (long long)kNtscMode1.wrap + 400;
    const int fcnt0 = ref.fcnt;   // sequence start after maturation (=1)
    DriveResult res = drive(ref, total, "S4 new-table free run: per-cycle equality", ctx);
    FCEU11_EXPECT(ctx, ref.flag == 0 && ref.line == 0,
                  "S4 no IRQ in post-switch inhibit 5-step");
    std::vector<Edge> exp = expected_seq_edges(kNtscMode1, 0, ref.t - total, total,
                                               false, fcnt0);
    FCEU11_EXPECT(ctx, edges_match(res.edges, exp),
                  "S4 edge log matches NEW mode1 table after switch");
    if (!edges_match(res.edges, exp)) {
        print_edges("S4 observed", res.edges);
        print_edges("S4 expected", exp);
    }
}

// S5: inhibit clear timing.
//  A: a mode0 write WITHOUT inhibit must not touch flag/line.
//  B: an inhibit write (raw $40, 4-step) clears them immediately; the
//     subsequent 4-step sequence with inhibit sets the flag at 29828/29829
//     without asserting the line, and FrameIRQEnd clears flag+line at wrap.
static void scenario_s5(TestContext& ctx) {
    // S5a
    reset_apu();
    RefState ref = ref_from_observed(false);
    drive(ref, (long long)kNtscMode0.wrap + 40, "S5a pre-phase mode0 free run", ctx);
    FCEU11_EXPECT(ctx, obs_flag() == 1 && obs_line() == 1, "S5a pre-phase flag+line set");
    do_write(ref, 0x00, 0);   // no inhibit bit
    FCEU11_EXPECT(ctx, obs_flag() == 1, "S5a write without inhibit keeps flag");
    FCEU11_EXPECT(ctx, obs_line() == 1, "S5a write without inhibit keeps line");
    drive(ref, 2 * (long long)kNtscMode0.wrap,
          "S5a post-write free run: per-cycle equality", ctx);
    FCEU11_EXPECT(ctx, ref.flag == 1 && ref.line == 1,
                  "S5a flag+line persist after non-inhibit write");

    // S5b: inhibit-only write, odd parity (delay 4 on this path too)
    reset_apu();
    ref = ref_from_observed(false);
    drive(ref, (long long)kNtscMode0.wrap + 40, "S5b pre-phase mode0 free run", ctx);
    do_write(ref, 0x40, 1);   // inhibit, stays 4-step
    FCEU11_EXPECT(ctx, obs_flag() == 0, "S5b inhibit write clears flag immediately");
    FCEU11_EXPECT(ctx, obs_line() == 0, "S5b inhibit write deasserts line immediately");
    const int frozen = fhcnt;
    drive(ref, 3, "S5b odd-parity countdown freeze", ctx);
    FCEU11_EXPECT(ctx, fhcnt == frozen, "S5b fhcnt frozen during countdown");
    drive(ref, 1, "S5b maturation at write+4", ctx);
    FCEU11_EXPECT(ctx, fhcnt == 0, "S5b fhcnt restarted");
    const long long total = 2 * (long long)kNtscMode0.wrap;
    DriveResult res = drive(ref, total, "S5b inhibit 4-step free run: per-cycle equality", ctx);
    // FLAG edges across 2 sequences: up at 29828 (seq 0 and 1), down at
    // 29830 (seq 0 and 1) — the inhibit branch of FrameIRQEnd at wrap.
    int flag_edges = 0;
    for (const Edge& e : res.edges) {
        if (e.mask & EDG_FLAG) flag_edges++;
    }
    FCEU11_EXPECT(ctx, flag_edges == 4,
                  "S5b flag toggles twice per wrap cycle (set+End) under inhibit");
    FCEU11_EXPECT(ctx, ref.flag == 0,
                  "S5b flag cleared by FrameIRQEnd at wrap (inhibit branch)");
}

int main() {
    std::setvbuf(stdout, nullptr, _IONBF, 0);

    std::printf("=== kgmqa-121 APU frame-counter timing lock tests ===\n");
    std::printf("ROM: %s\n\n", kRom);

    TestContext ctx;

    if (!core_init()) { return 1; }
    FCEUGI* gi = load_rom(kRom);
    if (!gi) { core_shutdown(); return 1; }

    scenario_s1_ntsc(ctx);
    scenario_s1_pal(ctx);
    scenario_s2(ctx);
    scenario_s3(ctx);
    scenario_s4(ctx);
    scenario_s5(ctx);

    fceu11::CloseGame();
    core_shutdown();

    return report_and_exit(ctx, "APU frame timing lock suite (kgmqa-121)");
}
