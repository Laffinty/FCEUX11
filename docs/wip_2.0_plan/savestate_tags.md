# savestate_tags.md · SFORMAT 契约

> **来源**：`src/state.cpp`、`src/ppu_state.cpp`、`src/sound.cpp`、`src/input.cpp`、
> `src/movie_playback.cpp`、`src/vsuni.cpp`（2026-08-10 实测）
> **目的**：vNESU11 savestate 序列化的"tag 契约"——Rust 端按 tag 输出与 C++ 等价字节
> **审计依据**：`AUDIT_20260810.md` S2——savestate 是 V2 chunked 格式，**不是**64 字节整块

---

## 1. 文件格式总览

savestate 文件 = **`[V2 header][若干 chunk]`，每 chunk = `[type:u8][size:u32 LE][data]`**。
chunk 的 `data` 是 SFORMAT 数组的递归序列化结果：每个 SFORMAT entry = `[desc:4 bytes][size:u32][value]`。
- `desc` 的 4 字节是**固定 tag 字符串**（如 `"PC\0"`、`"JAMM"`），必须逐字节一致
- `value` 按 `s` 的低 31 位大小 + flags（`FCEUSTATE_RLSB` / `FCEUSTATE_INDIRECT`）处理
- `FCEUSTATE_RLSB`（bit 31）：多字节值按 little-endian 存储
- `FCEUSTATE_INDIRECT`（bit 30）：`v` 是 `void**`，要先解引用再读

`size` 字段本身是 **native u32 LE**（不带 RLSB flag，因为它是元数据）。

---

## 2. Chunk Type 清单（state.cpp:333-371 实测）

| Type | 名字 | 来源 TU | 内容 | SFORMAT 数组 |
|------|------|--------|------|-------------|
| `1`  | CPU 主块 | state.cpp | CPU 主要寄存器 + WRAM | `SFCPU` |
| `2`  | CPU 扩展块 | state.cpp | jammed / IRQlow / tcount / count / timestampbase / mooPI | `SFCPUC` |
| `3`  | **PPU 旧**（newppu=0） | ppu_state.cpp | NTARAM / PALRAM / SPRAM / PPU 寄存器等 | `FCEUPPU_STATEINFO` |
| `31` | **PPU 新**（newppu=1） | ppu_state.cpp | spr_read / ppur / ppur.status 等 | `FCEU_NEWPPU_STATEINFO` |
| `4`  | Controller | input.cpp | joy_readbit / joy / LastStrobe / lagFlag / lagCounter / currFrameCounter | `FCEUCTRL_STATEINFO` |
| `5`  | Sound (APU) | sound.cpp | fhcnt / fcnt / PSG / envelope / length / sweep / DMC | `FCEUSND_STATEINFO` |
| `6`  | Movie | movie_playback.cpp | currFrameCounter | `FCEUMOV_STATEINFO` |
| `0x10` | mapper 通用（动态注册） | 动态 | `register_state_chunk` 调用的所有 chunk（mapper 各自） | `SFMDATA` |

> **关键**：chunk `1` 含 `RAM`（0x800 字节 WRAM），通过 `FCEUSTATE_INDIRECT` 解引用。
> chunk `5` 的 size 包含 `fceu11::g_apu` 通过 getter 暴露的状态字段。

---

## 3. 完整 Tag 清单（逐 tag 实测）

### 3.1 Chunk 1: `SFCPU`（state.cpp:122-132）

| Tag | 字段 | Type | Size | Flags | 来源 |
|-----|------|------|------|-------|------|
| `"PC\0"` | g_cpu.native_layout().PC | u16 | 2 | RLSB | `x6502struct.h` |
| `"A\0\0"` | g_cpu.native_layout().A | u8 | 1 | — | `x6502struct.h` |
| `"X\0\0"` | g_cpu.native_layout().X | u8 | 1 | — | `x6502struct.h` |
| `"Y\0\0"` | g_cpu.native_layout().Y | u8 | 1 | — | `x6502struct.h` |
| `"S\0\0"` | g_cpu.native_layout().S | u8 | 1 | — | `x6502struct.h` |
| `"P\0\0"` | g_cpu.native_layout().P | u8 | 1 | — | `x6502struct.h` |
| `"DB\0"` | g_cpu.native_layout().DB | u8 | 1 | — | `x6502struct.h` |
| `"RAM"` | RAM（0x800 字节 WRAM） | u8 | 0x800 | INDIRECT | `bus.cpp` |

### 3.2 Chunk 2: `SFCPUC`（state.cpp:134-142）

| Tag | 字段 | Type | Size | Flags |
|-----|------|------|------|-------|
| `"JAMM"` | X.jammed | u8 | 1 | — |
| `"IQLB"` | X.IRQlow | u32 | 4 | RLSB |
| `"ICoa"` | X.tcount | i32 | 4 | RLSB |
| `"ICou"` | X.count | i32 | 4 | RLSB |
| `"TSBS"` | timestampbase | u64 | 8 | RLSB |
| `"MooP"` | X.mooPI | u8 | 1 | — |

### 3.3 Chunk 3: `FCEUPPU_STATEINFO`（ppu_state.cpp:51-65，新 PPU 路径不写此 chunk）

| Tag | 字段 | Size |
|-----|------|------|
| `"NTAR"` | NTARAM（0x800 nametable RAM） | 0x800 |
| `"PRAM"` | PALRAM.data()（0x20 调色板） | 0x20 |
| `"SPRA"` | SPRAM（0x100 OAM） | 0x100 |
| `"PPUR"` | PPU[4]（$2000-$2007 寄存器） | 0x4 |
| `"KOOK"` | kook | 1 |
| `"DEAD"` | ppudead | 1 |
| `"PSPL"` | PPUSPL | 1 |
| `"XOFF"` | XOffset | 1 |
| `"VTGL"` | vtoggle | 1 |
| `"RADD"` | RefreshAddrT | 2 | RLSB |
| `"TADD"` | TempAddrT | 2 | RLSB |
| `"VBUF"` | VRAMBuffer | 1 |
| `"PGEN"` | PPUGenLatch | 1 |

> **注意**：chunk `3` 只在 `newppu=0` 时被 `FCEUPPU_Loop` 序列化路径写入；
> chunk `31` 在 `newppu=1` 时被替代。两者不会同时存在。

### 3.4 Chunk 31: `FCEU_NEWPPU_STATEINFO`（ppu_state.cpp:68-104）

| Tag | 字段 | Type | Size | Flags |
|-----|------|------|------|-------|
| `"IDLS"` | idleSynch | u8 | 1 | — |
| `"SR_0"` | spr_read.num | i32 | 4 | RLSB |
| `"SR_1"` | spr_read.count | i32 | 4 | RLSB |
| `"SR_2"` | spr_read.fetch | i32 | 4 | RLSB |
| `"SR_3"` | spr_read.found | i32 | 4 | RLSB |
| `"SRx0"~"SRx7"` | spr_read.found_pos[0..7] | i32 | 4 | RLSB |
| `"SR_4"` | spr_read.ret | i32 | 4 | RLSB |
| `"SR_5"` | spr_read.last | i32 | 4 | RLSB |
| `"SR_6"` | spr_read.mode | i32 | 4 | RLSB |
| `"PFVx"` | ppur.fv | u32 | 4 | RLSB |
| `"PVxx"` | ppur.v | u32 | 4 | RLSB |
| `"PHxx"` | ppur.h | u32 | 4 | RLSB |
| `"PVTx"` | ppur.vt | u32 | 4 | RLSB |
| `"PHTx"` | ppur.ht | u32 | 4 | RLSB |
| `"P_FV"` | ppur._fv | u32 | 4 | RLSB |
| `"P_Vx"` | ppur._v | u32 | 4 | RLSB |
| `"P_Hx"` | ppur._h | u32 | 4 | RLSB |
| `"P_VT"` | ppur._vt | u32 | 4 | RLSB |
| `"P_HT"` | ppur._ht | u32 | 4 | RLSB |
| `"PFHx"` | ppur.fh | u32 | 4 | RLSB |
| `"PSxx"` | ppur.s | u32 | 4 | RLSB |
| `"PST0"` | ppur.status.sl | u32 | 4 | RLSB |
| `"PST1"` | ppur.status.cycle | u32 | 4 | RLSB |
| `"PST2"` | ppur.status.end_cycle | u32 | 4 | RLSB |

### 3.5 Chunk 4: `FCEUCTRL_STATEINFO`（input.cpp:656-665）

| Tag | 字段 | Size |
|-----|------|------|
| `"JYRB"` | joy_readbit | 2 |
| `"JOYS"` | joy | 4 |
| `"LSTS"` | LastStrobe | 1 |
| `"ZBG0"` | ZD[0].bogo | 1 |
| `"ZBG1"` | ZD[1].bogo | 1 |
| `"LAGF"` | lagFlag | 1 |
| `"LAGC"` | lagCounter | 4 |
| `"FRAM"` | currFrameCounter | 4 |

### 3.6 Chunk 5: `FCEUSND_STATEINFO`（sound.cpp:1633-1688）

| Tag | 字段 | Size |
|-----|------|------|
| `"FHCN"` | fceu11::g_apu.fhcnt() | 4 | RLSB |
| `"FCNT"` | fceu11::g_apu.fcnt() | 1 |
| `"PSG"` | fceu11::g_apu.psg()（0x10 字节） | 0x10 |
| `"ENCH"` | fceu11::g_apu.enabled_channels() | 1 |
| `"IQFM"` | fceu11::g_apu.irq_frame_mode() | 1 |
| `"NREG"` | fceu11::g_apu.nreg() | 2 | RLSB |
| `"TRIM"` | fceu11::g_apu.tri_mode() | 1 |
| `"TRIC"` | fceu11::g_apu.tri_count() | 1 |
| `"E0SP"`~`"E2SP"` | env_units[0..2].Speed | 1 |
| `"E0MO"`~`"E2MO"` | env_units[0..2].Mode | 1 |
| `"E0D1"`~`"E2D1"` | env_units[0..2].DecCountTo1 | 1 |
| `"E0DV"`~`"E2DV"` | env_units[0..2].decvolume | 1 |
| `"LEN0"`~`"LEN3"` | lengthcount[0..3] | 4 | RLSB |
| `"SWEE"` | sweepon | 2 |
| `"CRF1"` | curfreq[0] | 4 | RLSB |
| `"CRF2"` | curfreq[1] | 4 | RLSB |
| `"SWCT"` | sweep_count | 2 |
| `"SIRQ"` | sirq_stat | 1 |
| `"5ACC"` | dmc_acc | 4 | RLSB |
| `"5BIT"` | dmc_bit_count | 1 |
| `"5ADD"` | dmc_address | 4 | RLSB |
| `"5SIZ"` | dmc_size | 4 | RLSB |
| `"5SHF"` | dmc_shift | 1 |
| `"5HVDM"` | dmc_have_dma | 1 |
| `"5HVSP"` | dmc_have_sample | 1 |
| `"5SZL"` | dmc_size_latch | 1 |
| `"5ADL"` | dmc_address_latch | 1 |
| `"5FMT"` | dmc_format | 1 |
| `"RWDA"` | raw_da_latch | 1 |

### 3.7 Chunk 6: `FCEUMOV_STATEINFO`（movie_playback.cpp:63-65）

| Tag | 字段 | Size |
|-----|------|------|
| `"FCNT"` | currFrameCounter（**注意与 chunk 4 FRAM 是同一字段**） | 4 | RLSB |

### 3.8 Chunk 0x10: `SFMDATA`（动态）

通过 `cart_class.h:160-163` 的 `register_state_chunk(name, data, size, count)` 动态追加。
每个 mapper / 子系统（如 VRC7、Konami QTaI Hack）自行调用注册。**tag 集合由 mapper 决定**，
无固定清单。Rust 端处理策略：
- 读：`UnknownChunk` 保留未知 chunk（`state.cpp:104` `g_unknownChunks`），原样透传
- 写：把当前所有已注册 chunk 按顺序序列化

### 3.9 其他 chunk（vsuni.cpp:120-122）

`FCEUVSUNI_STATEINFO` 是 VS UniSystem 内部 state，通过 mapper 路径注册到 `SFMDATA`，
**不是独立 chunk type**。tag 集合：`"vsdp"`/`"vscn"`/`"vsc2"`/`"vssv"`/`"vsin"`。

---

## 4. Rust 端实现映射

```rust
// crates/vnesu11/src/snapshot/mod.rs
// 内部布局自由；序列化时按 tag 输出相同字节

pub fn save_cpu_main(sink: &mut Sink) -> Result<()> {
    sink.write_tag(b"PC\0")?;
    sink.write_u16_le(self.cpu.pc);          // RLSB
    sink.write_tag(b"A\0\0")?;
    sink.write_u8(self.cpu.a);
    // ... 逐 tag 复刻 SFCPU
    sink.write_tag(b"RAM")?;
    sink.write_bytes(&self.wram);            // 0x800
    Ok(())
}

pub fn save_cpu_ext(sink: &mut Sink) -> Result<()> {
    sink.write_tag(b"JAMM")?;
    sink.write_u8(self.cpu.jammed);
    sink.write_tag(b"IQLB")?;
    sink.write_u32_le(self.cpu.irq_low);     // RLSB
    // ... 逐 tag 复刻 SFCPUC
    Ok(())
}
```

**Sink** 是 `&mut dyn FnMut(&[u8])` 或 `&mut impl Write`：
- `write_tag(4 bytes)`：原样写
- `write_u8/u16/u32/u64` 按 RLSB 标志决定 LE
- `write_bytes(size)`：直写
- chunk wrapper：`write_chunk_header(type: u8, size: u32 LE)` + payload + size 后写

---

## 5. 不在 Rust 端实现的 chunk

| Chunk | Rust 端处理 |
|-------|-----------|
| `3`（旧 PPU）| vNESU11 newppu=1 时**不写**（保留 C++ 旧 PPU 路径生成） |
| `4`（Controller）| vNESU11 接管，**Rust 实现**（joypad.rs） |
| `5`（APU）| vNESU11 接管，**Rust 实现**（apu/） |
| `6`（Movie）| vNESU11 不直接管；movie 兼容回退到 C++ 路径 |
| `0x10`（mapper 注册）| vNESU11 **不接管**——mapper C++ 端保留注册路径，由 mapper MetaVtable 触发 |

---

## 6. 验证方法（Phase 0 DoD）

```bash
# Round-trip 测试：v1.17 golden → Rust 加载 → 跑 N 帧 → 保存 → 对比
cargo test -p vnesu11 savestate_roundtrip -- --nocapture

# 失败时定位：逐 tag 对比（不是整块 MD5）
# 测试结构：每个 chunk 一个 test fn（SFCPU/SFCPUC/FCEUPPU/FCEU_NEWPPU/FCEUCTRL/FCEUSND）
# 每个 test fn：加载 golden，序列化输出，对比字节级
```

golden 来源：`build-release/tests/golden/v1.17_savestate_roundtrip/`（v1.17 build 时生成，
覆盖各 mapper 类型至少一个）。
