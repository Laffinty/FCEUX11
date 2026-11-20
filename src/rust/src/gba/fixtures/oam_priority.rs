//! W4（MKSC 取证计划 `docs/plans/FCEUX11-GBA兼容性方法论与MKSC取证-构建计划-2026-11-21.md`
//! §三）：OBJ 同优先级并列时按 **OAM 序号**裁决——与仿射位无关。
//!
//! GBATEK 依据（本地快照 `Research_only/w205/lt16_gbatek_full.txt`）：
//! * `:1345`「OBJ0 has highest priority, OBJ127 lowest」——同一 OBJ 优先级内的
//!   并列按 OAM 序号，序号小者在上；`:1451-1455` 的警示确认 OBJ 序号优先级
//!   独立于 BG 优先级字段。
//! * `:1371-1374`：OBJ0 属性在 07000000，OBJ1 在 07000008（每条目 8 字节）。
//! * `:1420-1422`：attr0 bit8 = 仿射开关；`:1462-1469`：第 k 组
//!   PA=07000006+k*20h、PB/PC/PD 依次 +8 半字；恒等变换 = PA=PD=0100h。
//! * `:1479-1482`：attr1 bits 9-13 = 参数组选择。
//!
//! 锁什么（GBADotnet #64 的场景）：仿射精灵与非仿射精灵 **同 OBJ 优先级**、
//! 坐标重叠时，把「仿射」当成独立排序维度的实现会把裁决判错。两条腿互为
//! 镜像：腿 A = OBJ0 普通 / OBJ1 仿射，腿 B = OBJ0 仿射 / OBJ1 普通——
//! 重叠列都必须显示 **OBJ0** 的颜色；两条腿各自再断言两个**独占列**，
//! 证明两条渲染管线（普通 + 仿射）都真的画了，重叠断言不是空对空。
//!
//! 渲染布置：mode 3（位图）+ 仅 OBJ 使能，无 BG——精灵永远盖过 backdrop，
//! 断言不被 BG 优先级语义污染。8x8 精灵两枚，X 相差 4 像素：
//! OBJ0 独占 96..99 / 重叠 100..103 / OBJ1 独占 104..107。

use super::{swi, Cartridge, Fixture, BUDGET, HALT};

const OAM_BASE: usize = 0x0700_0000;
const DISPCNT: usize = 0x0400_0000;
const OBJ_PALETTE: usize = 0x0500_0200;
/// mode 3-5 的 OBJ 字符区基址（GBATEK `:1361`，16 KB 窗）。
const OBJ_TILE_BASE: usize = 0x0601_4000;
const VCOUNT: usize = 0x0400_0006;

/// 行 60 起渲染约需 74K 周期（1232 周期/行 × 60）；HALT 之后每步恰好
/// 一个周期（`arm7tdmi.rs` 的 halted 路径只跑 `bus.step()`），所以 10 万步
/// 覆盖到行 81 左右。
const RENDER_STEPS: usize = 100_000;

const OBJ0_X: u16 = 96;
const OBJ1_X: u16 = 100;
const SPRITE_Y: u16 = 60;
/// attr2 bits 10-11 = OBJ 相对 BG 的优先级，两枚精灵都取 1（并列）。
const OBJ_PRIO_1: u16 = 1 << 10;
/// mode 3-5 下 OBJ 字符区从 VRAM 06014000 开始，对应 2D 编址的 **tile 512**
/// （GBATEK `:3156-3157`；vendor `layer_obj.rs` 跳过位图模式的低 512 号）。
/// 砖数据放在 06014000/06014020 = tile 512/513。
const TILE_OBJ0: u16 = 512;
const TILE_OBJ1: u16 = 513;

const ROW: usize = 60;
/// 独占/重叠采样列（都落在两枚精灵的行范围内）。
const COLUMN_OBJ0_ONLY: usize = 98;
const COLUMN_OVERLAP: usize = 101;
const COLUMN_OBJ1_ONLY: usize = 105;
const COLUMN_BACKDROP: usize = 50;

const COLOR_A: u16 = 0x001F; // 红——OBJ0 的砖（tile 0，全 index 1）
const COLOR_B: u16 = 0x7C00; // 蓝——OBJ1 的砖（tile 1，全 index 2）

/// 布置并跑两枚精灵。`obj0_affine` 决定仿射位落在哪一枚：
/// 腿 A（false）= OBJ0 普通 / OBJ1 仿射（参数组 1）；腿 B（true）=
/// OBJ0 仿射（参数组 0）/ OBJ1 普通。两组参数恒等变换都预先写入。
fn run_two_sprites(obj0_affine: bool) -> Fixture {
    let mut cartridge = Cartridge::new();
    cartridge.code(&[swi(HALT)]);
    let mut fixture = Fixture::new(&cartridge.build());

    // DISPCNT：mode 3 + OBJ 使能，无 BG——精灵只跟 backdrop 比（backdrop
    // 永远垫底），OBJ/BG 优先级交叉语义不进断言。字写，与 lcd.rs 一致。
    fixture.gba.cpu.bus.write_word(DISPCNT, 0x1003);
    // OBJ 调色板：entry 1 = A（OBJ0 砖）、entry 2 = B（OBJ1 砖）；entry 0
    // 保持 0——OBJ 像素 index 0 是透明的，别让砖里出现它。
    fixture
        .gba
        .cpu
        .bus
        .write_half_word(OBJ_PALETTE + 2, COLOR_A);
    fixture
        .gba
        .cpu
        .bus
        .write_half_word(OBJ_PALETTE + 4, COLOR_B);
    // 两块 4bpp 全满砖（32 字节 = 8 字）：tile 0 全 index 1，tile 1 全 index 2。
    for i in 0..8usize {
        fixture
            .gba
            .cpu
            .bus
            .write_word(OBJ_TILE_BASE + i * 4, 0x1111_1111);
        fixture
            .gba
            .cpu
            .bus
            .write_word(OBJ_TILE_BASE + 32 + i * 4, 0x2222_2222);
    }
    // 恒等变换两组都写（GBATEK `:1465-1469` 的组地址），谁被选中都能用。
    for group in 0..2usize {
        let pa = OAM_BASE + 6 + group * 0x20;
        fixture.gba.cpu.bus.write_half_word(pa, 0x0100);
        fixture.gba.cpu.bus.write_half_word(pa + 8, 0);
        fixture.gba.cpu.bus.write_half_word(pa + 16, 0);
        fixture.gba.cpu.bus.write_half_word(pa + 24, 0x0100);
    }
    // 属性（GBATEK `:1371-1374` 的条目地址；`:1420-1422`/`:1479-1482` 的位段）。
    // attr0 = Y | (仿射 ? 0100h : 0)——普通精灵 bit9 必须保持 0（bit9 在
    // 非仿射时是 OBJ Disable）；attr1 = X | (仿射 ? 组号<<9 : 0)；
    // attr2 = tile | OBJ 优先级 1，两枚并列。
    let (attr0_0, attr1_0) = if obj0_affine {
        (SPRITE_Y | 0x0100, OBJ0_X)
    } else {
        (SPRITE_Y, OBJ0_X)
    };
    let (attr0_1, attr1_1) = if obj0_affine {
        (SPRITE_Y, OBJ1_X)
    } else {
        (SPRITE_Y | 0x0100, OBJ1_X | (1 << 9))
    };
    fixture.gba.cpu.bus.write_half_word(OAM_BASE, attr0_0);
    fixture.gba.cpu.bus.write_half_word(OAM_BASE + 2, attr1_0);
    fixture
        .gba
        .cpu
        .bus
        .write_half_word(OAM_BASE + 4, OBJ_PRIO_1 | TILE_OBJ0);
    fixture.gba.cpu.bus.write_half_word(OAM_BASE + 8, attr0_1);
    fixture.gba.cpu.bus.write_half_word(OAM_BASE + 10, attr1_1);
    fixture
        .gba
        .cpu
        .bus
        .write_half_word(OAM_BASE + 12, OBJ_PRIO_1 | TILE_OBJ1);

    assert!(
        fixture.run_to_halt(BUDGET),
        "the program never reached its Halt"
    );
    fixture.run_steps(RENDER_STEPS);
    fixture
}

fn row(fixture: &Fixture, y: usize) -> Vec<u16> {
    (0..240usize)
        .map(|x| fixture.gba.cpu.bus.lcd.buffer[y][x].0)
        .collect()
}

fn vcount(fixture: &mut Fixture) -> u16 {
    fixture.gba.cpu.bus.read_half_word(VCOUNT)
}

/// 腿 A：OBJ0 普通、OBJ1 仿射——重叠列显示 OBJ0（普通）的颜色。
#[test]
fn oam_priority_normal_obj0_beats_affine_obj1_at_the_same_obj_priority() {
    let mut fixture = run_two_sprites(false);
    assert!(
        vcount(&mut fixture) >= 68,
        "the LCD never rendered the compared rows"
    );
    let r = row(&fixture, ROW);
    assert_eq!(
        r[COLUMN_BACKDROP], 0,
        "backdrop sanity: palette 0 is black and no sprite covers this column"
    );
    assert_eq!(
        r[COLUMN_OBJ0_ONLY], COLOR_A,
        "OBJ0 (normal) must render its exclusive columns"
    );
    assert_eq!(
        r[COLUMN_OBJ1_ONLY], COLOR_B,
        "OBJ1 (affine, identity params) must render its exclusive columns -- \
         this leg proves the affine pipeline actually drew"
    );
    assert_eq!(
        r[COLUMN_OVERLAP], COLOR_A,
        "overlap: OBJ0 wins by OAM index; sorting affine sprites into a \
         separate layer would let OBJ1's colour through here"
    );
}

/// 腿 B：OBJ0 仿射、OBJ1 普通——重叠列仍显示 OBJ0（仿射）的颜色。
/// 这是 GBADotnet #64 的牙齿：把仿射精灵挪到排序队首/队尾的实现会在这里翻红。
#[test]
fn oam_priority_affine_obj0_beats_normal_obj1_at_the_same_obj_priority() {
    let mut fixture = run_two_sprites(true);
    assert!(
        vcount(&mut fixture) >= 68,
        "the LCD never rendered the compared rows"
    );
    let r = row(&fixture, ROW);
    assert_eq!(
        r[COLUMN_BACKDROP], 0,
        "backdrop sanity: palette 0 is black and no sprite covers this column"
    );
    assert_eq!(
        r[COLUMN_OBJ0_ONLY], COLOR_A,
        "OBJ0 (affine, identity params) must render its exclusive columns -- \
         this leg proves the affine pipeline actually drew"
    );
    assert_eq!(
        r[COLUMN_OBJ1_ONLY], COLOR_B,
        "OBJ1 (normal) must render its exclusive columns"
    );
    assert_eq!(
        r[COLUMN_OVERLAP], COLOR_A,
        "overlap: the affine OBJ0 still wins by OAM index; a sort that treats \
         affine sprites specially would show OBJ1's colour here"
    );
}
