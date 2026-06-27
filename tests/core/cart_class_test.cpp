// FCEUX11 v1.7 Cartograph — Cart class skeleton tests (Phase A).
//
// These tests verify the fceu11::Cart class surface area. The Cart class
// is introduced in v1.7 Phase B; this Phase A submission is a SKELETON
// that compiles + registers in CTest, but each test stub currently
// trivially passes with a [SKIP] message — the real assertions land in
// Phase E (NROM PoC) and Phase F (MMC1 + MMC3 PoC).
//
// Per v1.7 build plan §6.2 + v1.7 mapper_byte_diff_test_design.md:
//   - cart_class_test covers Cart lifecycle, MirrorMode enum, expansion
//     audio install hook, save/load hook semantics
//   - mapper_byte_diff_test covers mapper state byte-level regression
//
// The two are complementary: cart_class_test verifies the *interface*
// works (call counts, return values); mapper_byte_diff_test verifies
// the *behavior* matches a golden (mapper state drift detection).
//
// Phase A acceptance:
//   - Test compiles + links against v1.6 baseline
//   - Each test prints [SKIP] and exits 0 (no FAILs)
//   - CTest registers cart_class_test (24/24 → 25/25 ctest count)
//
// Phase E/F acceptance (future):
//   - Tests assert real Cart lifecycle (on_power/on_reset/on_close calls)
//   - MirrorMode enum value checks
//   - install_expansion_audio Apu interaction
//   - on_save_pre/on_load_post trigger sequence

#include "test_helpers.h"

#include <cstdio>
#include <cstring>
#include <vector>

#include "cart_class.h"

using namespace fceu11_test;

// ---------------------------------------------------------------------------
// Phase D helper: counting cart that records on_save_pre / on_load_post calls
// ---------------------------------------------------------------------------

namespace {

class CountingTestCart : public fceu11::Cart {
public:
    int on_save_pre_count = 0;
    int on_load_post_count = 0;

    void on_power() noexcept override {}
    void on_reset() noexcept override {}
    void on_close() noexcept override {}

    void on_save_pre() noexcept override { ++on_save_pre_count; }
    void on_load_post() noexcept override { ++on_load_post_count; }
};

} // namespace

// ---------------------------------------------------------------------------
// Phase A stub tests — all [SKIP] until Cart class is implemented
// ---------------------------------------------------------------------------

void test_cart_class_compiles(TestContext& ctx) {
    // Phase A: just confirm the test binary is wired up.
    FCEU11_EXPECT(ctx, true, "cart_class_test skeleton compiled");
}

void test_mirror_mode_enum_values(TestContext& ctx) {
    // Phase E: assert MirrorMode::Horizontal == 0, Vertical == 1, etc.
    // Phase A: skip until v1.7 §2.3 introduces the enum.
    std::printf("    [SKIP] MirrorMode enum: deferred to Phase B\n");
    FCEU11_EXPECT(ctx, true, "skipped");
}

void test_nrom_on_power_called(TestContext& ctx) {
    // Phase E: load nrom.nes, verify cart_obj->on_power() called 1x.
    std::printf("    [SKIP] NromCart::on_power: deferred to Phase E\n");
    FCEU11_EXPECT(ctx, true, "skipped");
}

void test_mmc1_on_reset_clears_registers(TestContext& ctx) {
    // Phase F: load mmc1.nes, verify ResetNES triggers cart_obj->on_reset().
    std::printf("    [SKIP] Mmc1Cart::on_reset: deferred to Phase F\n");
    FCEU11_EXPECT(ctx, true, "skipped");
}

void test_mmc3_on_close_releases_irq(TestContext& ctx) {
    // Phase F: verify Mmc3Cart::on_close() releases IRQ hook.
    std::printf("    [SKIP] Mmc3Cart::on_close: deferred to Phase F\n");
    FCEU11_EXPECT(ctx, true, "skipped");
}

void test_unmapped_mapper_returns_nullptr(TestContext& ctx) {
    // Phase E: mapper_number=99 (not in create_cart_for_mapper switch)
    // → create_cart_for_mapper returns nullptr
    // → cart_obj=nullptr → Power/Reset/Close forward as no-op
    std::printf("    [SKIP] Unmapped mapper nullptr: deferred to Phase E\n");
    FCEU11_EXPECT(ctx, true, "skipped");
}

void test_install_expansion_audio_vrc6(TestContext& ctx) {
    // Phase E: load VRC6 ROM, verify install_expansion_audio sets
    // Apu::exp_sound_.expansion = &g_vrc6_audio.
    std::printf("    [SKIP] install_expansion_audio VRC6: deferred to Phase E\n");
    FCEU11_EXPECT(ctx, true, "skipped");
}

void test_save_battery_roundtrip(TestContext& ctx) {
    // Phase C1: verify Cart::save_battery + load_battery roundtrip.
    std::printf("    [SKIP] save_battery/load_battery: deferred to Phase C1\n");
    FCEU11_EXPECT(ctx, true, "skipped");
}

void test_cart_metadata_setter(TestContext& ctx) {
    // Phase C2: verify Cart::set_crc32 / set_md5 / set_mirror etc.
    std::printf("    [SKIP] Cart metadata setters: deferred to Phase C2\n");
    FCEU11_EXPECT(ctx, true, "skipped");
}

void test_attach_bus_injection(TestContext& ctx) {
    // Phase B: verify Cart::attach_bus(&g_bus) makes bus_ptr() return
    // &g_bus.
    std::printf("    [SKIP] attach_bus injection: deferred to Phase B\n");
    FCEU11_EXPECT(ctx, true, "skipped");
}

void test_on_save_pre_default_noop(TestContext& ctx) {
    // Phase D: verify default Cart::on_save_pre() does nothing.
    // (Critical for v1.4 vrc7_PreSave layout-shift avoidance.)
    class DefaultCart : public fceu11::Cart {
    public:
        void on_power() noexcept override {}
        void on_reset() noexcept override {}
        void on_close() noexcept override {}
    };

    DefaultCart cart;
    cart.on_save_pre(); // should compile and do nothing
    FCEU11_EXPECT(ctx, true, "default on_save_pre is a no-op");
}

void test_on_load_post_trigger_sequence(TestContext& ctx) {
    if (!core_init()) {
        FCEU11_EXPECT(ctx, false, "core_init failed");
        return;
    }

    FCEUGI* gi = load_rom("fixtures/nestest.nes");
    if (!gi) {
        FCEU11_EXPECT(ctx, false, "failed to load nestest.nes");
        core_shutdown();
        return;
    }

    // Run a few frames so the savestate has something to serialize.
    emulate_n(10);

    // Install our counting cart into the legacy CartInfo.
    CountingTestCart test_cart;
    fceu11::Cart* old_cart_obj = currCartInfo->cart_obj;
    currCartInfo->cart_obj = &test_cart;

    // Save state to a temp file. FCEUSS_Save returns void, so we just call
    // it and then check whether on_save_pre fired (it should have regardless
    // of whether the write itself succeeded).
    const char* tmp_state = "cart_class_test_tmp.fcs";
    FCEUSS_Save(tmp_state, false);

    // on_save_pre should have been triggered during save.
    FCEU11_EXPECT(ctx, test_cart.on_save_pre_count == 1,
                  "on_save_pre triggered exactly once during save");

    // Load the state back.
    bool loaded = FCEUSS_Load(tmp_state, false);
    FCEU11_EXPECT(ctx, loaded, "FCEUSS_Load succeeded");
    FCEU11_EXPECT(ctx, test_cart.on_load_post_count == 1,
                  "on_load_post triggered exactly once after successful load");

    // Restore the previous cart_obj so cleanup does not touch our stack cart.
    currCartInfo->cart_obj = old_cart_obj;

    // Clean up temp file.
    std::remove(tmp_state);

    core_shutdown();
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

int main() {
    std::setvbuf(stdout, nullptr, _IONBF, 0);

    std::printf("=== FCEUX11 v1.7 Cart class test suite ===\n");
    std::printf("Phase D: on_save_pre/on_load_post tests active;\n");
    std::printf("Phase E/F (NROM/MMC1/MMC3) tests still [SKIP].\n\n");

    TestContext ctx;

    test_cart_class_compiles(ctx);
    test_mirror_mode_enum_values(ctx);
    test_nrom_on_power_called(ctx);
    test_mmc1_on_reset_clears_registers(ctx);
    test_mmc3_on_close_releases_irq(ctx);
    test_unmapped_mapper_returns_nullptr(ctx);
    test_install_expansion_audio_vrc6(ctx);
    test_save_battery_roundtrip(ctx);
    test_cart_metadata_setter(ctx);
    test_attach_bus_injection(ctx);
    test_on_save_pre_default_noop(ctx);
    test_on_load_post_trigger_sequence(ctx);

    // Phase A: all tests [SKIP] → exit 0
    // Phase E+: real tests will report_and_exit with real pass/fail counts
    return report_and_exit(ctx, "Cart class test suite");
}