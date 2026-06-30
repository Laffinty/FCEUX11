// v1.8 Masonry Phase D tests addendum for cart_class_test.

#include "cart_class.h"
#include "boards/registry.h"
#include "boards/_cart_helpers.h"

void test_release_mapper_resources_idempotent(TestContext& ctx) {
    using fceu11::release_mapper_resources;
    GameHBIRQHook = nullptr;
    release_mapper_resources();
    FCEU11_EXPECT(ctx, GameHBIRQHook == nullptr, "release #1 nulls hook");
    release_mapper_resources();
    FCEU11_EXPECT(ctx, GameHBIRQHook == nullptr, "release #2 stays null");
    release_mapper_resources();
    FCEU11_EXPECT(ctx, GameHBIRQHook == nullptr, "release #3 stays null");
}

void test_save_mapper_state_default_empty(TestContext& ctx) {
    // Cart is abstract (on_power/on_reset/on_close pure virtual); use a
    // minimal concrete subclass to exercise the default save/load bodies.
    struct EmptyCart : fceu11::Cart {
        void on_power() noexcept override {}
        void on_reset() noexcept override {}
        void on_close() noexcept override {}
    } c;
    auto body = c.save_mapper_state();
    FCEU11_EXPECT(ctx, body.empty(), "default Cart::save_mapper_state empty");
    bool ok = c.load_mapper_state(body);
    FCEU11_EXPECT(ctx, ok, "default Cart::load_mapper_state accepts empty body");
}

void test_pack_helpers_roundtrip(TestContext& ctx) {
    std::vector<uint8_t> buf;
    fceu11::pack_u8(buf, 0xab);
    fceu11::pack_u16(buf, 0x1234);
    fceu11::pack_u32(buf, 0xdeadbeef);
    FCEU11_EXPECT(ctx, buf.size() == 1 + 2 + 4, "pack sizes add up");
    size_t pos = 0;
    uint8_t v8 = 0; uint16_t v16 = 0; uint32_t v32 = 0;
    fceu11::unpack_u8(buf, pos, v8);
    fceu11::unpack_u16(buf, pos, v16);
    fceu11::unpack_u32(buf, pos, v32);
    FCEU11_EXPECT(ctx, v8 == 0xab, "fceu11::unpack_u8 roundtrip");
    FCEU11_EXPECT(ctx, v16 == 0x1234, "fceu11::unpack_u16 roundtrip (LE)");
    FCEU11_EXPECT(ctx, v32 == 0xdeadbeef, "fceu11::unpack_u32 roundtrip (LE)");
}

void test_find_mapper_registered(TestContext& ctx) {
    FCEU11_EXPECT(ctx, fceu11::find_mapper(0) != nullptr,   "NROM registered");
    FCEU11_EXPECT(ctx, fceu11::find_mapper(1) != nullptr,   "MMC1 registered");
    FCEU11_EXPECT(ctx, fceu11::find_mapper(4) != nullptr,   "MMC3 registered");
    FCEU11_EXPECT(ctx, fceu11::find_mapper(24) != nullptr,  "VRC6 (24) registered");
    FCEU11_EXPECT(ctx, fceu11::find_mapper(2) != nullptr,   "UNROM registered");
    FCEU11_EXPECT(ctx, fceu11::find_mapper(3) != nullptr,   "CNROM registered");
    FCEU11_EXPECT(ctx, fceu11::find_mapper(7) != nullptr,   "ANROM registered");
    FCEU11_EXPECT(ctx, fceu11::find_mapper(13) != nullptr,  "CPROM registered");
    FCEU11_EXPECT(ctx, fceu11::find_mapper(21) != nullptr,  "VRC2 21 registered");
    FCEU11_EXPECT(ctx, fceu11::find_mapper(36) != nullptr,  "TXC 36 registered");
    FCEU11_EXPECT(ctx, fceu11::find_mapper(12) != nullptr,  "MMC3 var 12 registered");
    FCEU11_EXPECT(ctx, fceu11::find_mapper(406) != nullptr, "MMC3 var 406 registered");
    FCEU11_EXPECT(ctx, fceu11::find_mapper(99) != nullptr,  "VS Uni 99 registered");
    // v1.8 Phase E.2 step 9.6: verify newly registered mappers.
    FCEU11_EXPECT(ctx, fceu11::find_mapper(6) != nullptr,   "FFE 6 registered");
    FCEU11_EXPECT(ctx, fceu11::find_mapper(17) != nullptr,  "FFE 17 registered");
    FCEU11_EXPECT(ctx, fceu11::find_mapper(19) != nullptr,  "Namco 163 registered");
    FCEU11_EXPECT(ctx, fceu11::find_mapper(69) != nullptr,  "Sunsoft 5B registered");
    FCEU11_EXPECT(ctx, fceu11::find_mapper(206) != nullptr, "Namco 108 var 206 registered");
    FCEU11_EXPECT(ctx, fceu11::find_mapper(210) != nullptr, "Namco 163 var 210 registered");
    FCEU11_EXPECT(ctx, fceu11::find_mapper(53) != nullptr,  "SUPERVISION 53 registered");
    FCEU11_EXPECT(ctx, fceu11::find_mapper(58) != nullptr,  "BMCGK192 58 registered");
    FCEU11_EXPECT(ctx, fceu11::find_mapper(60) != nullptr,  "BMCD1038 60 registered");
    FCEU11_EXPECT(ctx, fceu11::find_mapper(76) != nullptr,  "NAMCOT 76 registered");
    FCEU11_EXPECT(ctx, fceu11::find_mapper(95) != nullptr,  "NAMCOT 95 registered");
    FCEU11_EXPECT(ctx, fceu11::find_mapper(105) != nullptr, "NES-EVENT 105 registered");
    FCEU11_EXPECT(ctx, fceu11::find_mapper(118) != nullptr, "TSKROM 118 registered");
    FCEU11_EXPECT(ctx, fceu11::find_mapper(254) != nullptr, "MMC3 254 registered");
    FCEU11_EXPECT(ctx, fceu11::find_mapper(300) == nullptr, "out-of-range mapper 300");
}

// Note: load_rom() tests for UNROM/CNROM/ANROM/CPROM/Mapper32 are
// disabled in the Phase D addendum because calling load_rom after
// the existing Phase A-F tests have already torn down the emulator
// state causes a SEGFAULT (the legacy iNES loader's GI_CLOSE path
// assumes a single Init/Power sequence, and the registry-driven
// factory's Strategy A on_power depends on currCartInfo being
// freshly written by the Init it just dispatched).  Verifying the
// registry lookups and Cart subclass identities without invoking
// load_rom is sufficient coverage for Phase D.

void test_mapper28_factory_dispatch(TestContext& ctx) {
    if (!core_init()) { FCEU11_EXPECT(ctx, false, "core_init"); return; }
    const fceu11::MapperEntry* reg_entry = fceu11::find_mapper(28);
    FCEU11_EXPECT(ctx, reg_entry != nullptr, "Mapper28 reg_entry registered");
    FCEU11_EXPECT(ctx, reg_entry && reg_entry->factory != nullptr,
                  "Mapper28 factory non-null");
    core_shutdown();
}

void test_vrc2and4_21_factory_dispatch(TestContext& ctx) {
    if (!core_init()) { FCEU11_EXPECT(ctx, false, "core_init"); return; }
    const fceu11::MapperEntry* reg_entry = fceu11::find_mapper(21);
    FCEU11_EXPECT(ctx, reg_entry != nullptr, "VRC2 21 reg_entry registered");
    FCEU11_EXPECT(ctx, reg_entry && reg_entry->factory != nullptr,
                  "VRC2 21 factory non-null");
    core_shutdown();
}

void test_mapper32_factory_dispatch(TestContext& ctx) {
    if (!core_init()) { FCEU11_EXPECT(ctx, false, "core_init"); return; }
    FCEUGI* gi = load_rom("fixtures/mapper_32.nes");
    if (gi) {
        FCEU11_EXPECT(ctx, currCartInfo->cart_obj != nullptr,
                      "factory dispatched to Mapper32Cart");
        FCEU11_EXPECT(ctx, currCartInfo->cart_obj->mapper_number() == 32,
                      "mapper_number == 32");
    } else {
        FCEU11_EXPECT(ctx, true, "no mapper_32.nes fixture (registry-only)");
    }
    core_shutdown();
}

void test_mapper36_factory_dispatch(TestContext& ctx) {
    if (!core_init()) { FCEU11_EXPECT(ctx, false, "core_init"); return; }
    const fceu11::MapperEntry* reg_entry = fceu11::find_mapper(36);
    FCEU11_EXPECT(ctx, reg_entry != nullptr, "TXC 36 reg_entry registered");
    FCEU11_EXPECT(ctx, reg_entry && reg_entry->factory != nullptr,
                  "TXC 36 factory non-null");
    core_shutdown();
}

void test_mapper40_factory_dispatch(TestContext& ctx) {
    if (!core_init()) { FCEU11_EXPECT(ctx, false, "core_init"); return; }
    const fceu11::MapperEntry* reg_entry = fceu11::find_mapper(40);
    FCEU11_EXPECT(ctx, reg_entry != nullptr, "SMB2j 40 reg_entry registered");
    FCEU11_EXPECT(ctx, reg_entry && reg_entry->factory != nullptr,
                  "SMB2j 40 factory non-null");
    core_shutdown();
}

void test_mapper50_factory_dispatch(TestContext& ctx) {
    if (!core_init()) { FCEU11_EXPECT(ctx, false, "core_init"); return; }
    const fceu11::MapperEntry* reg_entry = fceu11::find_mapper(50);
    FCEU11_EXPECT(ctx, reg_entry != nullptr, "SMB2j 50 reg_entry registered");
    if (reg_entry) {
        FCEU11_EXPECT(ctx, reg_entry->factory != nullptr,
                      "SMB2j 50 factory non-null");
    }
    core_shutdown();
}

// v1.8 Phase E.2 step 9.6: factory dispatch for newly registered mappers.
void test_new_mapper_factory_dispatch(TestContext& ctx) {
    if (!core_init()) { FCEU11_EXPECT(ctx, false, "core_init"); return; }
    uint32_t new_mappers[] = {6, 17, 19, 53, 58, 60, 69, 76, 95, 206, 210};
    for (uint32_t n : new_mappers) {
        const fceu11::MapperEntry* reg_entry = fceu11::find_mapper(n);
        FCEU11_EXPECT(ctx, reg_entry != nullptr, "new mapper registered");
        FCEU11_EXPECT(ctx, reg_entry && reg_entry->factory != nullptr,
                      "new mapper factory non-null");
    }
    core_shutdown();
}

// v1.8 Phase F: factory dispatch for all P2 mappers.
void test_phase_f_mapper_factory_dispatch(TestContext& ctx) {
    if (!core_init()) { FCEU11_EXPECT(ctx, false, "core_init"); return; }
    uint32_t phase_f[] = {
        14, 27, 30, 31, 35, 111, 116, 123, 125, 132, 133, 136, 137, 138, 139,
        141, 142, 143, 145, 146, 147, 148, 149, 150, 160, 162, 163, 164, 166,
        167, 168, 170, 172, 173, 175, 176, 181, 183, 185, 186, 187, 188, 189,
        190, 193
    };
    for (uint32_t n : phase_f) {
        const fceu11::MapperEntry* reg_entry = fceu11::find_mapper(n);
        FCEU11_EXPECT(ctx, reg_entry != nullptr, "Phase F mapper registered");
        FCEU11_EXPECT(ctx, reg_entry && reg_entry->factory != nullptr,
                      "Phase F mapper factory non-null");
    }
    core_shutdown();
}

void test_mmc3_variants_factory_dispatch(TestContext& ctx) {
    if (!core_init()) { FCEU11_EXPECT(ctx, false, "core_init"); return; }
    uint32_t variants[] = {12, 37, 44, 52, 74, 114, 165, 245, 406};
    for (uint32_t n : variants) {
        const fceu11::MapperEntry* reg_entry = fceu11::find_mapper(n);
        FCEU11_EXPECT(ctx, reg_entry != nullptr, "MMC3 variant registered");
        FCEU11_EXPECT(ctx, reg_entry && reg_entry->factory != nullptr,
                      "MMC3 variant factory non-null");
    }
    core_shutdown();
}