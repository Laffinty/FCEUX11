// FCEUX11 — v1.2 Census: fceu11::State facade test
//
// Verifies that the facade is wired correctly: every accessor must return
// a reference to the corresponding global symbol, the singleton returns
// the same instance across calls, and no accessor returns a null/bogus
// reference. Does NOT call fceu11::Initialize() (the GUI bridge requires
// a Qt event loop), and does NOT exercise read/write through the accessors
// in a way that mutates engine state — only address-equality with the
// underlying externs.

#include <cstdio>
#include <cstdlib>
#include <type_traits>

#include "types.h"
#include "fceu.h"
#include "x6502.h"
#include "ppu.h"
#include "sound.h"
#include "ines.h"
#include "cart.h"
#include "debug.h"
#include "core_state.h"

// Suppress unreferenced-variable warnings for the &-of-static checks.
#ifdef _MSC_VER
#pragma warning(push)
#pragma warning(disable: 4101)
#endif

#define CHECK_FACADE(name, expected_addr)                              \
    do {                                                               \
        void* got = reinterpret_cast<void*>(&(expected_addr));         \
        void* want = nullptr;                                          \
        (void)want;                                                    \
        if (got == nullptr) {                                          \
            printf("FAIL: " #name " reference is null\n");             \
            failed = true;                                             \
        } else {                                                       \
            printf("OK:   " #name " at %p\n", got);                    \
        }                                                              \
    } while (0)

int main() {
    printf("=== FCEUX11 v1.2 Census: fceu11::State facade test ===\n\n");

    bool failed = false;

    fceu11::State& s = fceu11::global_state();
    printf("OK: global_state() returns at %p\n", static_cast<void*>(&s));

    // Singleton: same address across calls
    fceu11::State& s2 = fceu11::global_state();
    if (&s != &s2) {
        printf("FAIL: global_state() returned different addresses (%p vs %p)\n",
               static_cast<void*>(&s), static_cast<void*>(&s2));
        failed = true;
    } else {
        printf("OK:   singleton identity preserved (%p)\n", static_cast<void*>(&s));
    }

    // CpuView — addresses must match the underlying externs.
    printf("\n--- CpuView ---\n");
    {
        auto& v = s.cpu();
        // v.reg() returns X6502& (typedef defined in x6502struct.h, which
        // x6502.h pulls in). Bind through the typedef name directly; no
        // local struct-tag indirection needed.
        X6502& ref = v.reg();
        uint32_t& ts     = v.timestamp();
        uint32_t& sts    = v.sound_timestamp();
        int& sl          = v.scanline();
        if (reinterpret_cast<void*>(&ref) != reinterpret_cast<void*>(&::X)) {
            printf("FAIL: CpuView::reg() != ::X\n"); failed = true;
        } else {
            printf("OK:   CpuView::reg() == ::X (%p)\n", static_cast<void*>(&ref));
        }
        if (&ts != &::timestamp) { printf("FAIL: CpuView::timestamp() != ::timestamp\n"); failed = true; }
        else printf("OK:   CpuView::timestamp() == ::timestamp (%p)\n", static_cast<void*>(&ts));
        if (&sts != &::soundtimestamp) { printf("FAIL: CpuView::sound_timestamp()\n"); failed = true; }
        else printf("OK:   CpuView::sound_timestamp() == ::soundtimestamp (%p)\n", static_cast<void*>(&sts));
        if (&sl != &::scanline) { printf("FAIL: CpuView::scanline()\n"); failed = true; }
        else printf("OK:   CpuView::scanline() == ::scanline (%p)\n", static_cast<void*>(&sl));
    }

    // PpuView
    printf("\n--- PpuView ---\n");
    {
        auto& v = s.ppu();
        if (reinterpret_cast<void*>(&v.regs())    != reinterpret_cast<void*>(&::PPU))    { printf("FAIL: PpuView::regs()\n");    failed = true; } else printf("OK:   PpuView::regs() == ::PPU (%p)\n",    static_cast<void*>(&v.regs()));
        if (reinterpret_cast<void*>(&v.ntaram())  != reinterpret_cast<void*>(&::NTARAM))  { printf("FAIL: PpuView::ntaram()\n");  failed = true; } else printf("OK:   PpuView::ntaram() == ::NTARAM (%p)\n", static_cast<void*>(&v.ntaram()));
        if (reinterpret_cast<void*>(&v.vnapage()) != reinterpret_cast<void*>(&::vnapage)) { printf("FAIL: PpuView::vnapage()\n"); failed = true; } else printf("OK:   PpuView::vnapage() == ::vnapage (%p)\n", static_cast<void*>(&v.vnapage()));
        if (reinterpret_cast<void*>(&v.vpage())   != reinterpret_cast<void*>(&::VPage))   { printf("FAIL: PpuView::vpage()\n");   failed = true; } else printf("OK:   PpuView::vpage() == ::VPage (%p)\n",   static_cast<void*>(&v.vpage()));
    }

    // State::bus() — v1.4 Gateway Phase 2 (now Phase 3). BusView was
    // retired; the facade returns the fceu11::Bus global directly.
    // The underlying tables (::ARead, ::BWrite, ::Page, ::VPage, etc.)
    // are `extern` reference-to-array aliases that bind to g_bus; the
    // pointer-identity check below verifies the aliasing works
    // end-to-end.
    printf("\n--- State::bus() (v1.4 Bus) ---\n");
    {
        auto& b = s.bus();
        // Sanity: the State facade hands back the same Bus global.
        if (&b != &fceu11::g_bus) {
            printf("FAIL: State::bus() != g_bus\n");
            failed = true;
        } else {
            printf("OK:   State::bus() == g_bus (%p)\n",
                   static_cast<void*>(&b));
        }
        // Pointer-identity: the global alias ::ARead is g_bus's
        // aread_ array, by definition of the reference alias.
        if (reinterpret_cast<void*>(&b.aread_table())  != reinterpret_cast<void*>(&::ARead))  { printf("FAIL: Bus::aread_table()\n");  failed = true; } else printf("OK:   Bus::aread_table() == ::ARead (%p)\n",  static_cast<void*>(&b.aread_table()));
        if (reinterpret_cast<void*>(&b.bwrite_table()) != reinterpret_cast<void*>(&::BWrite)) { printf("FAIL: Bus::bwrite_table()\n"); failed = true; } else printf("OK:   Bus::bwrite_table() == ::BWrite (%p)\n", static_cast<void*>(&b.bwrite_table()));
        if (reinterpret_cast<void*>(&b.page())   != reinterpret_cast<void*>(&::Page))   { printf("FAIL: Bus::page()\n");   failed = true; } else printf("OK:   Bus::page() == ::Page (%p)\n",   static_cast<void*>(&b.page()));
        if (reinterpret_cast<void*>(&b.vpage())  != reinterpret_cast<void*>(&::VPage))  { printf("FAIL: Bus::vpage()\n");  failed = true; } else printf("OK:   Bus::vpage() == ::VPage (%p)\n",  static_cast<void*>(&b.vpage()));
    }

    // CartView
    printf("\n--- CartView ---\n");
    {
        auto& v = s.cart();
        if (&v.current() != &::currCartInfo) { printf("FAIL: CartView::current()\n"); failed = true; }
        else printf("OK:   CartView::current() == ::currCartInfo (%p)\n", static_cast<void*>(&v.current()));
        if (reinterpret_cast<void*>(&v.prg_ptr()) != reinterpret_cast<void*>(&::PRGptr)) { printf("FAIL: CartView::prg_ptr()\n"); failed = true; }
        else printf("OK:   CartView::prg_ptr() == ::PRGptr (%p)\n", static_cast<void*>(&v.prg_ptr()));
        if (reinterpret_cast<void*>(&v.chr_ptr()) != reinterpret_cast<void*>(&::CHRptr)) { printf("FAIL: CartView::chr_ptr()\n"); failed = true; }
        else printf("OK:   CartView::chr_ptr() == ::CHRptr (%p)\n", static_cast<void*>(&v.chr_ptr()));
        if (&v.rom_header() != &::head) { printf("FAIL: CartView::rom_header()\n"); failed = true; }
        else printf("OK:   CartView::rom_header() == ::head (%p)\n", static_cast<void*>(&v.rom_header()));
    }

    // ApuView
    printf("\n--- ApuView ---\n");
    {
        auto& v = s.apu();
        if (reinterpret_cast<void*>(&v.wave())       != reinterpret_cast<void*>(&::Wave))       { printf("FAIL: ApuView::wave()\n");       failed = true; } else printf("OK:   ApuView::wave() == ::Wave (%p)\n",       static_cast<void*>(&v.wave()));
        if (reinterpret_cast<void*>(&v.wave_final()) != reinterpret_cast<void*>(&::WaveFinal)) { printf("FAIL: ApuView::wave_final()\n"); failed = true; } else printf("OK:   ApuView::wave_final() == ::WaveFinal (%p)\n", static_cast<void*>(&v.wave_final()));
        if (&v.nes_inc_size() != &::nesincsize) { printf("FAIL: ApuView::nes_inc_size()\n"); failed = true; }
        else printf("OK:   ApuView::nes_inc_size() == ::nesincsize (%p)\n", static_cast<void*>(&v.nes_inc_size()));
        if (&v.swap_duty() != &::swapDuty) { printf("FAIL: ApuView::swap_duty()\n"); failed = true; }
        else printf("OK:   ApuView::swap_duty() == ::swapDuty (%p)\n", static_cast<void*>(&v.swap_duty()));
    }

    // ConfigView
    printf("\n--- ConfigView ---\n");
    {
        auto& v = s.config();
        if (&v.pal() != &::PAL) { printf("FAIL: ConfigView::pal()\n"); failed = true; }
        else printf("OK:   ConfigView::pal() == ::PAL (%p)\n", static_cast<void*>(&v.pal()));
        if (&v.dendy() != &::dendy) { printf("FAIL: ConfigView::dendy()\n"); failed = true; }
        else printf("OK:   ConfigView::dendy() == ::dendy (%p)\n", static_cast<void*>(&v.dendy()));
        if (&v.settings() != &::FSettings) { printf("FAIL: ConfigView::settings()\n"); failed = true; }
        else printf("OK:   ConfigView::settings() == ::FSettings (%p)\n", static_cast<void*>(&v.settings()));
        if (&v.ram_init_option() != &::RAMInitOption) { printf("FAIL: ConfigView::ram_init_option()\n"); failed = true; }
        else printf("OK:   ConfigView::ram_init_option() == ::RAMInitOption (%p)\n", static_cast<void*>(&v.ram_init_option()));
        // game_attributes() deferred to v1.11 Bridge — see core_state.h comment.
    }

    // DebugView
    printf("\n--- DebugView ---\n");
    {
        auto& v = s.debug();
        if (&v.in_debug() != &::fceuindbg) { printf("FAIL: DebugView::in_debug()\n"); failed = true; }
        else printf("OK:   DebugView::in_debug() == ::fceuindbg (%p)\n", static_cast<void*>(&v.in_debug()));
        if (&v.ia_pc() != &::iaPC) { printf("FAIL: DebugView::ia_pc()\n"); failed = true; }
        else printf("OK:   DebugView::ia_pc() == ::iaPC (%p)\n", static_cast<void*>(&v.ia_pc()));
        if (&v.break_asap() != &::break_asap) { printf("FAIL: DebugView::break_asap()\n"); failed = true; }
        else printf("OK:   DebugView::break_asap() == ::break_asap (%p)\n", static_cast<void*>(&v.break_asap()));
    }

    // Breakpoint keyword resolution lock (2026-11-13, A1 GUI walkthrough S-2
    // session finding): the v0.3.5 from_chars refactor of offsetStringToInt
    // early-returned on non-hex input, making every breakpoint keyword branch
    // (NSF LOAD/INIT/PLAY, FDS NMI1-3, NES NMI/VBL/RST/IRQ/BRK) dead code.
    // Symptom: the Add-Breakpoint dialog rejected "PLAY" with "Start Address
    // Invalid". The fix falls through to the BT_C keyword/symbol path on hex
    // parse failure. Mutation check: revert the fall-through to an early
    // return and the PLAY/LOAD/INIT assertions below go red.
    printf("\n--- offsetStringToInt keyword lock ---\n");
    {
        static FCEUGI nsfGi;
        nsfGi.type = GIT_NSF;
        FCEUGI* savedGameInfo = ::GameInfo;
        ::GameInfo = &nsfGi;
        NSF_HEADER savedNsf = ::NSFHeader;
        ::NSFHeader.PlayAddressLow = 0x34;
        ::NSFHeader.PlayAddressHigh = 0x12; // PLAY = $1234
        ::NSFHeader.LoadAddressLow = 0x00;
        ::NSFHeader.LoadAddressHigh = 0x80; // LOAD = $8000
        ::NSFHeader.InitAddressLow = 0x03;
        ::NSFHeader.InitAddressHigh = 0x80; // INIT = $8003

        bool ok = false;
        int v = offsetStringToInt(BT_C, "PLAY", &ok);
        if (!ok || v != 0x1234) { printf("FAIL: PLAY keyword -> ok=%d v=0x%04X (want ok=1 v=0x1234)\n", (int)ok, (unsigned)v); failed = true; }
        else printf("OK:   PLAY -> $%04X\n", (unsigned)v);

        v = offsetStringToInt(BT_C, "LOAD", &ok);
        if (!ok || v != 0x8000) { printf("FAIL: LOAD keyword -> ok=%d v=0x%04X (want ok=1 v=0x8000)\n", (int)ok, (unsigned)v); failed = true; }
        else printf("OK:   LOAD -> $%04X\n", (unsigned)v);

        v = offsetStringToInt(BT_C, "INIT", &ok);
        if (!ok || v != 0x8003) { printf("FAIL: INIT keyword -> ok=%d v=0x%04X (want ok=1 v=0x8003)\n", (int)ok, (unsigned)v); failed = true; }
        else printf("OK:   INIT -> $%04X\n", (unsigned)v);

        // Hex parsing must be unaffected by the fall-through.
        v = offsetStringToInt(BT_C, "C123", &ok);
        if (!ok || v != 0xC123) { printf("FAIL: hex C123 -> ok=%d v=0x%04X\n", (int)ok, (unsigned)v); failed = true; }
        else printf("OK:   hex C123 -> $%04X\n", (unsigned)v);

        // Pure garbage must still be rejected.
        v = offsetStringToInt(BT_C, "ZZZZ", &ok);
        if (ok) { printf("FAIL: garbage ZZZZ -> ok=1 (want 0)\n"); failed = true; }
        else printf("OK:   garbage ZZZZ rejected\n");

        ::GameInfo = savedGameInfo;
        ::NSFHeader = savedNsf;
    }

    // Static-check: facade version marker
    static_assert(fceu11::State::census_version_v1_2 == 1,
                  "Census facade version marker must be 1");
    printf("\nOK:   State::census_version_v1_2 == %d\n",
           fceu11::State::census_version_v1_2);

    printf("\n=== Test Complete ===\n");
    if (failed) {
        printf("\nRESULT: FAILED\n");
        return 1;
    } else {
        printf("\nRESULT: PASSED - facade wires to all 7 subsystem views\n");
        return 0;
    }
}

#ifdef _MSC_VER
#pragma warning(pop)
#endif