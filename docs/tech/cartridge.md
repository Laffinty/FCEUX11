# 卡带格式与 Mapper — 原理、FCEUX11 设计与经验知识

> **STATUS: ACTIVE**（知识库参考文档；最后核对 2026-09-19）
> 素材来源：NESdev Wiki（CC BY-SA，`nesdev.org/wiki/INES`、`NES_2.0`、`UNIF`、
> `MMC1`、`MMC3`、`UxROM`、`PPU_rendering`）+ FCEUX11 源码。

---

## 一、硬件原理（Wiki 综合）

### 1.1 卡带的本质

NES 主板上没有 ROM——所有程序与图形数据都在卡带上，卡带同时决定**地址映射**
（PRG/CHR bank）、**镜像模式**（nametable 布线）、**扩展 RAM/音源**。CPU 总线上
$6000-$FFFF、PPU 总线上 $0000-$1FFF 的行为完全由 mapper 芯片决定。

- **PRG**：程序 ROM，8/16/32KB 换页；最后 16KB 通常固定在 $C000。
- **CHR**：图案 ROM/RAM，4/8KB 为基本页粒度。
- **WRAM**：$6000-$7FFF，常带电池。
- **Nametable mirroring**：水平/垂直/单屏/四屏，由卡带布线或 mapper 寄存器动态控制
  （MMC1 是后者的代表）。

### 1.2 MMC3 的 IRQ（精度要点）

- 计数器由 **PPU A12 上升沿**驱动（CHR pattern 取址 $0000/$1000 空间切换的边沿），
  不是「每 scanline」——同一行内 sprite fetch 即可触发多次。
- $C000 latch / $C001 reload / $E000 disable / $E001 enable；count 减到 0 或 reload
  时重装并在下一次 A12 边沿触发 IRQ。
- 常见游戏兼容 hack：在 A12 边沿前需要「fetch 一轮背景 pattern」做滤波（硬件在
  A12 拉高前有取址序列）；不同游戏对触发行的选择敏感。

### 1.3 MMC1 的串行写入

- 5 次写移位组装一个 5bit 命令；写 bit7=1 复位整个移位链到「接收第 0 位」状态。
- 寄存器 0：镜像 + CHR 模式 + PRG 模式低 2 位；寄存器 3：PRG bank，$C000 固定
  最后 bank 的模式下 $8000 可换页。

### 1.4 iNES / NES 2.0 / UNIF（Wiki `INES`、`NES_2.0`、`UNIF`）

- iNES 头 16 字节：magic `NES\x1A`、PRG/CHR 大小、flags 6-15。flags6: mirroring
  （bit0，1=垂直）、battery、trainer、四屏（bit3，压倒 bit0）、VS/PlayChoice 位；
  flags7: NES 2.0 识别（bit2-3 = 0x08）。
- **NES 2.0** 扩展：mapper 高位 + submapper（byte 7/8 高 4 位）、PRG/CHR RAM shift
  计数字节、TV system（byte 12）、VS 参数（byte 13）、扩展设备（byte 15）。
- **UNIF**：块结构替代 iNES，板名直接对应布线（`NES-xxx`/`UNL-xxx` 前缀）。
- 现实中的 ROM 头普遍不可靠：垃圾头清洗 + 按 MD5/CRC 的数据库修正（如
  `ines-correct.h`）是模拟器必备工序。

---

## 二、FCEUX11 具体设计

### 2.1 加载流程

**iNES**：`iNESLoad`（`src/ines.cpp:54-91`，薄封装）→ Rust `fceux11-formats`
（`src/rust/crates/fceux11-formats/src/ines.rs`：magic 校验、垃圾头清洗、NES 2.0
识别 :548-562）→ `iNESLoadCore`（`src/ines_load.cpp:142-271`：PRG/CHR 分配 0xFF
填充、trainer 512B、NES 2.0 WRAM/VRAM/submapper、MD5 数据库修正、默认手柄按 CRC
查询、四屏 ExtraNTARAM、`SetupCartMirroring`）→ `iNES_Init`（`src/ines_init.cpp`，
查 `ines_bmap.h` 的 `bmap[]`）→ `create_cart_for_mapper`（`src/cart_class.cpp:47-59`）。

**UNIF**：`UNIFLoadCore`（`src/unif_load.cpp:103-185`），板名剥前缀后查
`unif_bmap.h`，PRG/CHR 向上取 2 的幂（最小 2KB/8KB）。

**NES 2.0 支持度**（`ines.rs:548-648`）：submapper、WRAM/VRAM/battery shift 换算、
TV system（NTSC/PAL）、VS PPU 类型、扩展设备→默认手柄。未使用 misc ROMs 等字段。

### 2.2 Mapper 注册（双轨制）

1. **传统表**：`src/ines_bmap.h` 的 `BMAPPINGLocal bmap[]`，269 项，覆盖 mapper 0-255
   及若干 >255 号；mapper 20（FDS）被注释掉，走 `fds.cpp`。非 2 的幂 ROM 白名单
   `{53,198,228,547}`（`ines_bmap.h:16-19`）。
2. **C++ 静态注册表**：`src/boards/registry.h:48-78` 的 `MapperEntry{mapper_number,
   name, legacy_init, factory}` + `MapperEntryRegister` 常量构造（如 vrc6.cpp 注册
   24/26、mmc1.cpp 注册 1/105、mmc3.cpp 注册 4/12/37/44…）。未注册 mapper 返回
   nullptr 走旧函数指针路径。**注意 MSVC /OPT:REF 死代码消除陷阱**（registry.h:67-76）。

新 mapper 的接线模式：Init/Power 里 `SetWriteHandler/SetReadHandler` 挂地址钩子 +
`setprg8/16/32`、`setchr1/2/4/8`（`src/cart.h:28,156`；带 RAM 区号的
`setprg8r/setchr1r` 在 `cart.cpp:178-227`）+ `setmirror/SetupCartMirroring`。
写 WRAM 由 `prg_is_ram[]` 门控（`CartBW`，`cart.cpp:96-126`）。

### 2.3 代表 mapper 实现

- **MMC3（mapper 4，`src/boards/mmc3.cpp`）**：$8000/$8001 命令+数据（R 槽缓冲
  DRegBuf 延迟应用）；IRQ 计数器由 **PPU 渲染循环的 `GameHBIRQHook`** 时钟——
  新 PPU 在 sprite fetch `s==2` 处（`ppu_rendering.cpp:2060-2074`，条件用 pattern
  使能位而非 sprite 使能位：Dragon's Lair），旧 PPU 在 DoLine 中段（:772-792）。
  变体 hack：KickMaster（scanline 238 双时钟）、PAL Star Wars（240）。
- **MMC1（mapper 1，`src/boards/mmc1.cpp:128-160`）**：5 次写移位，bit7 复位；
  DRegs[0-3] = mirror/CHR/CHR+PRG/PRG；mode3 下 $8000 换页、$C000 固定 $0F。
- **UxROM（mapper 2，`src/boards/datalatch.cpp`）**：`setprg16(0xC000, ~0)` 固定
  末 bank，$8000-$FFFF 任意写锁存 $8000 bank；同文件容纳 CNROM/AOROM 变体。
  总线冲突不模拟。

### 2.4 扩展音频

VRC6/VRC7/FDS/N163/5B 等 mapper 音源经 `ExpansionAudio` 基类（`src/expansion_audio.h`）
挂入 APU 的 `FlushEmulateSound`，见 [apu.md](apu.md) §2.4。

---

## 三、经验与魔数（开发者备忘）

| 事实/魔数 | 出处 |
|---|---|
| `bmap[]` 269 项是 mapper 号→init 的唯一权威表；注册表只覆盖 C++ 化的子集 | ines_bmap.h / boards/registry.h |
| mapper 号 >255 的（256/258/259/268/406/547…）是 FCEUX 自有扩展号，非 iNES 标准 | ines_bmap.h |
| MMC3 A12 hook 的判定条件 `s==2`（sprite fetch 的第 2 次取址）是经验值 | ppu_rendering.cpp:2060-2074 |
| 非 2 的幂 ROM 白名单 mapper {53,198,228,547} | ines_bmap.h:16-19 |
| 0xFF 填充 PRG/CHR 再 memcpy | 头尺寸与实际 ROM 不符时的容错（ines_load.cpp:182-202） |
| 垃圾头清洗在 Rust 侧做，C++ 侧不做二次清洗 | ines.rs:502 起 |
| `PPU 侧 mirroring 修改前必须 notify_line_update()` | ppu_class.cpp:157-163 |
| savestate：mapper 状态与 PRG/CHR 页表都要进 chunk；新 mapper 忘记导出状态是常见 bug | 见各 board 的 MapperStateBlock |

## 四、相关文档

- MMC3 IRQ 与 PPU 的耦合细节：[ppu.md](ppu.md) §2.4
- mapper 号清单与板文件对照：`src/ines_bmap.h`（权威）
- 测试治理（MMC3 18 条 ROM）：[KagamiQA.md](KagamiQA.md)
