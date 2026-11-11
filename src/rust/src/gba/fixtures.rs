//! S1-a 最小可复现夹具 —— 合成 GBA 卡带、ARM 编码器、运行器。
//!
//! # 这个文件是什么
//!
//! v2.0.3 计划 S1-a 的载体：**不产生任何二进制入库**。夹具是 Rust 函数，
//! 现场拼出一张卡带镜像交给 `gba_core::gba::Gba`，跑一段 ARM 程序，再断言读数。
//! 实施计划见 `docs/plans/FCEUX11-v2.0.3_S1a_最小可复现夹具-实施计划.md`。
//!
//! # 为什么编码器要自己写，而且必须配端到端自检
//!
//! 夹具的每一条断言都建立在「我拼出的那几个字确实是那条指令」之上。
//! **编码器一旦错，全部夹具都在测别的东西，而且照样全绿**——它是全部夹具的**共因**，
//! 比「相对比较型测试对共因变异不敏感」更隐蔽（AGENTS.md 纪律 12 的同族，但更严重）。
//!
//! ⇒ 因此每个编码器都配一条**端到端**自检：程序把一个已知值写进内存，再读回。
//! ⇒ 这些自检必须**逐条变异可红**（把编码器的一个字段改错 ⇒ 对应那条转红，
//! **其余仍然绿** ⇒ 证明它们各测各的，不是共因）。
//!
//! # 编码从哪里来
//!
//! **不是凭记忆**，而是从本仓**已被既有测试验证过**的常量反推字段布局：
//!
//! | 指令 | 依据 | 常量 |
//! |---|---|---|
//! | `MOV rd, #imm` | `gate.rs` 的 `mov_r0_imm` | `0xE3A0_0000 \| field` |
//! | `LDR rd, [rn]` | `bios.rs` 的 `LDR_R2_R2` | `0xE592_2000` |
//! | `STRH rd, [rn]` | `bios.rs` 的 `STRH_R1_R0`（其偏移字段恰好是 0） | `0xE1C0_10B0` |
//! | `SWI n` | `gate.rs` 的 `swi`（号码在 **bits 16-23**） | `0xEF00_0000 \| (n << 16)` |
//!
//! 旋转立即数的规则**不复制第二份**：直接用 `gate::immediate_field`
//! （本文件把它的可见性放宽到 `pub(crate)`），避免两份实现漂移。
//!
//! # 踩过的坑（写在这里，因为它们不会自己显形）
//!
//! 1. **vendor 的立即数/寄存器判别与真 ARM 相反**：真 ARM 里 `I=1` 是寄存器形式，
//!    而 vendor 的 `OperandKind::from(true)` 是 **Immediate**
//!    （`cpu/arm/instructions.rs:796` 用 `get_bit(25)`）。
//!    ⇒ `MOV`/`ORR`（真 ARM 的立即数形式本来就是 `I=1`）**两边一致**；
//!    ⇒ `ADD`/`SUB` 这类**真 ARM 用 `I=0`** 的指令在 vendor 上**会被读成寄存器形式**
//!    ⇒ **本文件的编码器不提供 `ADD`**。需要拼地址时用 `MOV` + `ORR`。
//! 2. **半字存取的偏移语义与字存取不同**：字存取的立即数偏移在 vendor 上
//!    **不乘 4**（真 ARM 乘 4）⇒ **本文件的存取编码器只提供零偏移**。
//! 3. **`STRB` 写 OAM 被忽略**（`bus.rs:1231-1234`）⇒ 造非法 OAM 属性必须用半字/字写。
//! 4. **旋转立即数旋不进就报错，不许掩码**：掩码后的值仍是「一个合法立即数，
//!    只是另一个值」⇒ 错误会以「某个没人选过的数字」的形式出现。
//!    （照 `gate.rs:88-95` 的先例。）
//!
//! # 变异验证（Step 1 的验收，2026-11-09）
//!
//! 每个编码器各做一处**单字段**变异，跑这 8 条自检，记下逐条红/绿。
//! 执行器 `.zcode/mutation_matrix.ps1`（不入库），逐条读数见计划
//! `docs/plans/FCEUX11-v2.0.3_S1a_最小可复现夹具-实施计划.md` §十一 r4。
//!
//! **首轮（修之前）测出 3 条盲区** —— 目标测试在自己的编码器被改坏时**照样绿**：
//!
//! | 盲区 | 现象 | 根因 | 处置 |
//! |---|---|---|---|
//! | `g3_orr_imm_…` | 「orr_imm 忘记编码旋转」⇒ **8 条全绿** | 用的是 `0x30`，一个**不需要旋转**的合法立即数 ⇒ 丢掉旋转看不出差别 | 换成 `0x30000000`（常量 `0x30` 右旋 8 位），断言改读**寄存器** |
//! | `g3_strh_…` | 「strh 存成了字」⇒ **8 条全绿** | 存的是 `0xC500`，**高半字是 0** ⇒ `STR` 与 `STRH` 写出的**字节完全相同** | 目标地址先放哨兵 `0xDEAD_BEEF` ⇒ 写完的整字必须是 `0xDEAD_C500` |
//! | `g3_ldrh_…` | 「ldrh 读成了字」⇒ **8 条全绿** | 源字 `0x0000C500` 的**高半字是 0** ⇒ `LDR` 与 `LDRH` 读到同一个值；再走内存往返，存回去的都是低半字 | 源字改 `0xC5C5C500`，断言改读**寄存器** |
//!
//! ⇒ 这三条当时是**无牙门禁**（AGENTS.md 纪律 3：「全绿不算证据」）。
//!
//! **修后的矩阵**（红 = 该测试失败；每行的**目标**就是该行对应的编码器）：
//!
//! ```text
//! 变异                            1mov_imm 2orr_imm 3str 4strh 5ldr 6ldrh 7swi 8mov_reg
//! M1 mov_imm 忘记编码旋转            RED   GREEN  RED   RED   RED   RED  GREEN  GREEN
//! M2 orr_imm 忘记编码旋转           GREEN   RED  GREEN GREEN GREEN GREEN GREEN  GREEN
//! M3 str 存成了半字                 GREEN GREEN   RED GREEN GREEN GREEN GREEN  GREEN
//! M4 strh 存成了字                  GREEN GREEN GREEN   RED GREEN GREEN GREEN  GREEN
//! M5 ldr 读成了半字                 GREEN GREEN GREEN GREEN   RED GREEN GREEN  GREEN
//! M6 ldrh 读成了字                  GREEN GREEN GREEN GREEN GREEN   RED GREEN  GREEN
//! M7 swi 号码放进低字节              RED    RED   RED   RED   RED   RED   RED    RED
//! M8 mov_reg 源寄存器放错字段         GREEN GREEN GREEN GREEN GREEN GREEN GREEN   RED
//! ```
//!
//! **8/8 的目标都红了** ⇒ 每条自检都能抓住自己编码器的缺陷。
//!
//! **两条仍不是「只红一条」**，原因是**结构性耦合**，不是自检的缺陷：
//!
//! * **M1（`mov_imm`）**：它是**唯一**能把地址放进寄存器的手段 ⇒ 凡是需要
//!   基址指针的自检（1/3/4/5/6）都用它 ⇒ 它的缺陷必然让这些自检一起红。
//!   红的集合恰好 = **用 `mov_imm` 造地址的自检集合**，一一对应、可核对。
//! * **M7（`swi`）**：`swi(HALT)` 是**每条自检的终止器** ⇒ 号码位置错了，
//!   机器永远停不下来 ⇒ 8 条一起红。这是「一个坏 SWI 编码器 = 全局失败」的
//!   **正确表现**（响亮地失败），但它让这一行无法定位。
//!
//! ⇒ 要把这两行也变成「只红一条」，得改夹具**脚手架**（基址寄存器由夹具
//! 预置、终止器改用 `gate.rs` 的参考实现）⇒ 那是**设计变更**，不在本工作项内。
//!
//! # 踩过的坑（写在这里，因为它们不会自己显形）
//!
//! 5. **变异验证的「还原腿」会假红**：还原源文件时若不更新 mtime，cargo 判定
//!    「无需重建」⇒ 跑的是**上一轮变异版**的二进制。2026-11-09 实测：还原后
//!    `g3_mov_reg` 稳定假红（读到 r0 的 `0x04000204`，正是 M8 把源寄存器挪到
//!    bits 15-12 的结果），触碰文件后即恢复全绿。⇒ **反向腿必须确认二进制是新的。**

use gba_core::gba::Gba;

/// Where a cartridge's code begins.
///
/// The same offset `gate.rs` uses: `0xC0` is the *Nintendo reserved* slot on a
/// real cartridge (it holds a branch like any other code), and `0xC4` is where
/// a branch from offset 0 lands.
pub const CODE_OFFSET: usize = 0xC4;

/// Where that code runs.
pub const CODE_BASE: u32 = 0x0800_0000 + CODE_OFFSET as u32;

/// Where a cartridge image starts.
pub const ROM_BASE: u32 = 0x0800_0000;

/// The image size this builder produces.
///
/// The vendor's `CartridgeHeader::new` slices up to `0xE4`, so a shorter image
/// panics; the root crate's C ABI refuses anything under `0x200`
/// (`frame.rs:74`). `0x200` satisfies both.
pub const ROM_LEN: usize = 0x200;

/// The SWI number that stops the machine.
pub const HALT: u32 = crate::gba::swi::Swi::Halt as u32;

/// How many steps a self-check may run before it is declared stuck.
///
/// A generous budget: the stub BIOS boots in a handful of instructions and a
/// fixture's program is a few words long, so this is far above what a working
/// program needs and far below "wait and see".
pub const BUDGET: usize = 100_000;

/// Where the self-checks put their scratch bytes.
///
/// IWRAM, so the writes are visible through the bus exactly as a game's would
/// be.
pub const SCRATCH: usize = 0x0300_0000;

// ---------------------------------------------------------------------------
// The eight S1-a fixtures, one module per defect domain
// ---------------------------------------------------------------------------
//
// One file per domain, so that the domains can be written, reviewed and
// mutation-tested independently -- and so that two people never edit the same
// file. The shared toolkit (the cartridge builder, the runner, the ARM
// encoders) stays here; each module below owns exactly one defect domain.

pub mod backup;
pub mod guards;
pub mod lcd;
pub mod oam;
pub mod rom;
pub mod swi;

// ---------------------------------------------------------------------------
// ARM instruction encoders
// ---------------------------------------------------------------------------

/// `MOV rd, #imm` — ARM data processing with a rotated 8-bit immediate.
///
/// The immediate is an 8-bit constant plus a count of **even right rotations**.
/// A value that cannot be expressed that way is an **error**, not something to
/// mask: a masked value is still a legal immediate, just a different one, so
/// the mistake would show up as a test failing on a number nobody chose.
pub const fn mov_imm(rd: u32, value: u32) -> u32 {
    match crate::gba::gate::immediate_field(value) {
        Some(field) => 0xE3A0_0000 | (rd << 12) | field,
        None => panic!("not an ARM rotated immediate"),
    }
}

/// `ORR rd, rn, #imm` — the same rotated-immediate rule, OR-ed into `rn`.
///
/// This is how a 32-bit address is put together when it is not a single
/// rotated immediate. `ORR`'s immediate form uses `I=1` on real ARM too, so
/// this one agrees with the vendor (see the module docs).
pub const fn orr_imm(rd: u32, rn: u32, value: u32) -> u32 {
    match crate::gba::gate::immediate_field(value) {
        Some(field) => 0xE380_0000 | (rn << 16) | (rd << 12) | field,
        None => panic!("not an ARM rotated immediate"),
    }
}

/// `STR rd, [rn]` — a **word** store, zero offset.
pub const fn str(rd: u32, rn: u32) -> u32 {
    0xE580_0000 | (rn << 16) | (rd << 12)
}

/// `LDR rd, [rn]` — a **word** load, zero offset.
pub const fn ldr(rd: u32, rn: u32) -> u32 {
    0xE590_0000 | (rn << 16) | (rd << 12)
}

/// `STRH rd, [rn]` — a **halfword** store, zero offset.
///
/// The low byte of the encoding is **not** zero: a halfword transfer is
/// recognised by the pattern `1 sh(2) 1` in bits 7-4, and `sh = 01` is what
/// says "unsigned halfword". The vendor reads `sh` from bits 5-6
/// (`cpu/arm/instructions.rs:643`), so bit 5 must be set and bit 6 clear —
/// hence `0xB0` in the low byte. Zeroing it makes the instruction decode as
/// something else entirely, which is how the first draft of this file wrote
/// to memory a zero instead of the value.
pub const fn strh(rd: u32, rn: u32) -> u32 {
    0xE1C0_00B0 | (rn << 16) | (rd << 12)
}

/// `LDRH rd, [rn]` — a **halfword** load, zero offset.
///
/// Same `1 sh(2) 1` pattern as [`strh`]; the load/store bit is bit 20.
pub const fn ldrh(rd: u32, rn: u32) -> u32 {
    0xE1D0_00B0 | (rn << 16) | (rd << 12)
}

/// `SWI n` — the number lives in **bits 16-23** of the ARM encoding.
///
/// Bits 0-7 are a comment field real hardware ignores, and it is bits 16-23
/// that the BIOS reads. An earlier version of this repository's own encoder put
/// the number in the low byte, where it happened to agree with the core's
/// misreading — and every program written against it validated the wrong
/// encoding. See the v2.0.1 plan's change record, r1.
pub const fn swi(number: u32) -> u32 {
    assert!(
        number < 0x100,
        "the SWI number is 8 bits and lives at bits 16-23"
    );
    0xEF00_0000 | (number << 16)
}

/// `MOV rd, rm` — a register-to-register move.
///
/// **The source register lives in bits 3-0**, not in bits 15-12. That is what
/// the core executes: the destination is `rn` (bits 19-16) and the source is
/// the operand's register field (bits 3-0) — which is also real ARM's layout
/// for the register form of `MOV`.
///
/// The core's *disassembler* disagrees: it prints the destination from bits
/// 15-12, which for `MOV` is not where ARM puts it, so a real `MOV r5, r3`
/// is labelled `MOV r0, r3`. That is a **disassembler** defect, not an
/// execution one — the register really does land in the right place. The
/// fixtures read the registers, never the disassembly text, so they are not
/// affected; but anything that shows the GBA disassembly to a human is.
///
/// The spin idiom that leaves the register alone: `MOV r0, r0` spins forever
/// without disturbing `r0` (unlike a zero word, which is `ANDEQ r0, r0, r0`
/// and would clear the very register a test reads).
pub const fn mov_reg(rd: u32, rm: u32) -> u32 {
    0xE1A0_0000 | (rd << 16) | (rm & 0x0F)
}

// ---------------------------------------------------------------------------
// The cartridge
// ---------------------------------------------------------------------------

/// A synthetic GBA cartridge image.
///
/// A real cartridge opens with a branch to its code, and the stub BIOS hands
/// control to offset 0 — so the branch is a **real instruction the machine
/// runs**, not decoration.
pub struct Cartridge {
    rom: Vec<u8>,
    body: Vec<u32>,
    thumb_body: Vec<u16>,
}

impl Cartridge {
    /// A fresh image: `ROM_LEN` zero bytes with the GBA main unit code set.
    #[must_use]
    pub fn new() -> Self {
        let mut rom = vec![0u8; ROM_LEN];
        // Main unit code 0x00 marks a GBA cartridge.
        rom[0xB3] = 0x00;
        Self { rom, body: Vec::new(), thumb_body: Vec::new() }
    }

    /// Append ARM words to the program body.
    pub fn code(&mut self, words: &[u32]) -> &mut Self {
        self.body.extend_from_slice(words);
        self
    }

    /// Append Thumb halfwords to the program body, after any ARM words.
    ///
    /// Thumb code never starts on its own: the machine boots in ARM state and
    /// the BIOS hands control to an ARM branch, so the body's ARM words must
    /// switch state first -- an `ARM BX` to [`Cartridge::thumb_base`] with bit
    /// 0 set, which is what makes the target address decode as Thumb.
    pub fn thumb_code(&mut self, halfwords: &[u16]) -> &mut Self {
        self.thumb_body.extend_from_slice(halfwords);
        self
    }

    /// Where the Thumb halfwords run (they follow the ARM words).
    ///
    /// Bit 0 is **not** set here: the caller ORs it in on the `BX` value --
    /// the address the code sits at and the value that selects Thumb state
    /// are two different things, and keeping them apart is what keeps the
    /// halfwords word-aligned.
    #[must_use]
    pub fn thumb_base(&self) -> u32 {
        CODE_BASE + (self.body.len() * 4) as u32
    }

    /// Put ASCII at `offset` — used for the cartridge-type markers
    /// (`EEPROM_V124`, `FLASH_V124`, ...) that the core's `BackupType::detect`
    /// scans the whole image for.
    ///
    /// The offset must be **below** [`CODE_OFFSET`]: the header occupies
    /// `0x00..=0xC3` and the program body starts at `0xC4`, and the two must
    /// not overlap.
    pub fn ascii(&mut self, offset: usize, text: &[u8]) -> &mut Self {
        self.bytes(offset, text)
    }

    /// Put raw bytes at `offset`. Same placement rule as [`Cartridge::ascii`].
    pub fn bytes(&mut self, offset: usize, data: &[u8]) -> &mut Self {
        assert!(
            offset + data.len() <= CODE_OFFSET,
            "data goes below the code region (0x{:X}..0x{:X})",
            CODE_OFFSET,
            ROM_LEN
        );
        self.rom[offset..offset + data.len()].copy_from_slice(data);
        self
    }

    /// The finished image.
    #[must_use]
    pub fn build(&self) -> Vec<u8> {
        let mut rom = self.rom.clone();

        // The entry branch: `b` to the body. The target is `PC + 8 + (offset << 2)`
        // and the branch sits at offset 0, so the word offset is
        // `(CODE_BASE - (ROM_BASE + 8)) >> 2`.
        let word_offset = (CODE_BASE - (ROM_BASE + 8)) >> 2;
        rom[0..4].copy_from_slice(&(0xEA00_0000 | word_offset).to_le_bytes());

        let end = CODE_OFFSET + self.body.len() * 4 + self.thumb_body.len() * 2;
        assert!(end <= rom.len(), "the program does not fit in the image");
        for (i, word) in self.body.iter().enumerate() {
            let at = CODE_OFFSET + i * 4;
            rom[at..at + 4].copy_from_slice(&word.to_le_bytes());
        }
        let thumb_at = CODE_OFFSET + self.body.len() * 4;
        for (i, halfword) in self.thumb_body.iter().enumerate() {
            let at = thumb_at + i * 2;
            rom[at..at + 2].copy_from_slice(&halfword.to_le_bytes());
        }
        rom
    }
}

impl Default for Cartridge {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// The machine
// ---------------------------------------------------------------------------

/// A machine running a synthetic cartridge.
pub struct Fixture {
    gba: Gba,
    /// The core's own disassembler channel.
    ///
    /// `Gba::new` already opens it (`gba.rs:95`), so the core can be asked
    /// **what it thinks it executed** — which is a far harder check than
    /// watching a register change: it catches an encoding that happens to
    /// produce the right value by the wrong route.
    disasm: rtrb::Consumer<gba_core::cpu::DisasmEntry>,
}

impl Fixture {
    /// A machine with the stub BIOS and our SWI hook installed — the same
    /// pairing production uses (`frame.rs:100-101`).
    #[must_use]
    pub fn new(rom: &[u8]) -> Self {
        let mut gba = Gba::new(crate::gba::bios::stub(), rom);
        crate::gba::install_swi_hook(&mut gba);
        // The channel exists from `Gba::new`, but the CPU only fills it when
        // this flag is set (`arm7tdmi.rs:161`).
        gba.cpu.disasm_enabled = true;
        let disasm = gba
            .disasm_rx
            .take()
            .expect("the core opens its disassembler channel in `Gba::new`");
        Self { gba, disasm }
    }

    /// The instructions the core has executed, as the core disassembled them.
    ///
    /// The entries come out in execution order. The BIOS boot sequence is at
    /// the front, so a test that cares about one instruction should look for
    /// it rather than index from zero.
    pub fn take_disassembly(&mut self) -> Vec<String> {
        let mut out = Vec::new();
        // `rtrb`'s `pop` reports "empty" as an error, so the loop ends when
        // it stops yielding entries.
        while let Ok(entry) = self.disasm.pop() {
            out.push(entry.format());
        }
        out
    }

    /// Step until the machine halts, or `budget` steps run out.
    ///
    /// Returns whether it halted. The budget is a **step** count, never a
    /// program-counter test: the ARM7TDMI fetch pointer runs speculatively
    /// past a not-yet-taken branch, so "the PC reached address X" fires on the
    /// first pass through a one-instruction loop.
    pub fn run_to_halt(&mut self, budget: usize) -> bool {
        for _ in 0..budget {
            if self.gba.cpu.halted {
                return true;
            }
            self.gba.step();
        }
        self.gba.cpu.halted
    }

    /// Step a fixed number of times.
    pub fn run_steps(&mut self, steps: usize) {
        for _ in 0..steps {
            self.gba.step();
        }
    }

    /// Register `n`.
    #[must_use]
    pub fn reg(&self, n: usize) -> u32 {
        self.gba.cpu.registers.register_at(n)
    }

    /// The program counter.
    #[must_use]
    pub fn pc(&self) -> usize {
        self.gba.cpu.registers.program_counter()
    }

    /// Whether the machine has stopped.
    #[must_use]
    pub fn halted(&self) -> bool {
        self.gba.cpu.halted
    }

    /// Read a word through the bus — the same path a game's `LDR` takes.
    pub fn read_word(&mut self, address: usize) -> u32 {
        self.gba.cpu.bus.read_word(address)
    }

    /// Read a halfword through the bus.
    pub fn read_half_word(&mut self, address: usize) -> u16 {
        self.gba.cpu.bus.read_half_word(address)
    }

    /// Write a word through the bus.
    pub fn write_word(&mut self, address: usize, value: u32) {
        self.gba.cpu.bus.write_word(address, value);
    }
}

// ---------------------------------------------------------------------------
// Tests: the encoder self-checks
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::{
        ldr, ldrh, mov_imm, mov_reg, orr_imm, str, strh, swi, Cartridge, Fixture, BUDGET,
        HALT, SCRATCH,
    };

    /// Build the machine for `body` **without running it**.
    ///
    /// The split exists so a test can plant a value in memory *before* the
    /// program runs. The stub BIOS does not clear IWRAM -- its boot code only
    /// programs `WAITCNT` and branches to the cartridge (`bios.rs:151-160`) --
    /// so a planted word survives the boot.
    fn fixture_for(body: &[u32]) -> Fixture {
        let mut cartridge = Cartridge::new();
        cartridge.code(body);
        let rom = cartridge.build();
        Fixture::new(&rom)
    }

    /// Build a cartridge whose body is `body`, run it to its halt, and hand
    /// the machine back for reading.
    fn run_to_halt(body: &[u32]) -> Fixture {
        let mut fixture = fixture_for(body);
        assert!(
            fixture.run_to_halt(BUDGET),
            "the program never reached its Halt"
        );
        fixture
    }

    /// `MOV rd, #imm` puts the value in the register.
    #[test]
    fn g3_mov_imm_puts_the_value_in_the_register() {
        let body = [
            mov_imm(0, SCRATCH as u32),
            mov_imm(1, 0x5A),
            str(1, 0),
            swi(HALT),
        ];
        let mut fixture = run_to_halt(&body);
        assert_eq!(
            fixture.read_word(SCRATCH),
            0x5A,
            "MOV r1, #0x5A then STR r1, [r0] must land 0x5A in memory"
        );
    }

    /// `ORR rd, rn, #imm` ORs into the source register.
    ///
    /// The immediate **needs the rotation**: `0x30000000` is the 8-bit
    /// constant `0x30` rotated right by 8. An encoder that dropped the
    /// rotation would put `0x30` there instead, and the result would be
    /// wrong.
    ///
    /// The first draft used `0x30`, which is a legal immediate *without* any
    /// rotation -- so "forgot to encode the rotation" changed nothing here,
    /// and the whole suite stayed green over a broken encoder. That is the
    /// blind spot the mutation matrix found.
    ///
    /// The assertion reads the **register**, not memory. A memory round trip
    /// would make this test depend on `str` as well, and a defect in `str`
    /// would then turn this test red and blur the matrix.
    #[test]
    fn g3_orr_imm_ors_into_the_source_register() {
        let body = [
            mov_imm(1, 0x0F),
            orr_imm(2, 1, 0x3000_0000),
            swi(HALT),
        ];
        let fixture = run_to_halt(&body);
        assert_eq!(
            fixture.reg(2),
            0x3000_000F,
            "ORR r2, r1, #0x30000000 must OR into r1's value"
        );
    }

    /// `STR rd, [rn]` writes **four** bytes.
    #[test]
    fn g3_str_writes_a_word() {
        let body = [
            mov_imm(0, SCRATCH as u32),
            mov_imm(1, 0x0400_0000),
            str(1, 0),
            swi(HALT),
        ];
        let mut fixture = run_to_halt(&body);
        assert_eq!(fixture.read_word(SCRATCH), 0x0400_0000);
        assert_eq!(
            fixture.read_half_word(SCRATCH + 2),
            0x0400,
            "the upper halfword must be written too: a word store, not a halfword one"
        );
    }

    /// `STRH rd, [rn]` writes **two** bytes and leaves the next halfword alone.
    ///
    /// The word at the target starts as `0xDEAD_BEEF`, so the difference
    /// between a halfword store and a word store is **observable**: a word
    /// store would also write the value's zero upper halfword, and the
    /// halfword above would stop being `0xDEAD`.
    ///
    /// The first draft stored `0xC500` into zeroed memory and asserted that
    /// the halfword above was still zero. A word store leaves **exactly the
    /// same bytes** there -- the value's upper halfword is zero either way --
    /// so that draft could not tell `STRH` from `STR` at all, and the
    /// mutation that turned `strh` into `str` left the whole suite green.
    #[test]
    fn g3_strh_writes_a_halfword() {
        let body = [
            mov_imm(0, SCRATCH as u32),
            mov_imm(1, 0x0000_C500),
            strh(1, 0),
            swi(HALT),
        ];
        let mut fixture = fixture_for(&body);
        fixture.write_word(SCRATCH, 0xDEAD_BEEF);
        assert!(
            fixture.run_to_halt(BUDGET),
            "the program never reached its Halt"
        );
        assert_eq!(
            fixture.read_half_word(SCRATCH),
            0xC500,
            "the halfword that was written must hold the value"
        );
        assert_eq!(
            fixture.read_word(SCRATCH),
            0xDEAD_C500,
            "a halfword store must not reach the halfword above it"
        );
    }

    /// `LDR rd, [rn]` reads a word back.
    ///
    /// The word at the source is `0x04000000`: its low halfword is zero, so a
    /// **halfword** load would give `0x0000` instead -- the difference
    /// between `LDR` and `LDRH` is observable.
    ///
    /// The assertion reads the **register**, not memory. The first draft
    /// stored the value with `str` and read it back, but that made this test
    /// depend on `str` too: a word store turned into a halfword store turned
    /// *this* test red, and the matrix could no longer say which encoder was
    /// broken.
    #[test]
    fn g3_ldr_reads_back_what_was_stored() {
        let body = [mov_imm(0, SCRATCH as u32), ldr(2, 0), swi(HALT)];
        let mut fixture = fixture_for(&body);
        fixture.write_word(SCRATCH, 0x0400_0000);
        assert!(
            fixture.run_to_halt(BUDGET),
            "the program never reached its Halt"
        );
        assert_eq!(
            fixture.reg(2),
            0x0400_0000,
            "LDR must load the whole word, not just its low halfword"
        );
    }

    /// `LDRH rd, [rn]` reads two bytes back.
    ///
    /// The word at the source is `0xC5C5_C500`: its upper halfword is **not**
    /// zero, so a word load and a halfword load give different answers.
    ///
    /// The assertion reads the **register**, not memory. A memory round trip
    /// hides the difference -- the store that follows keeps only the low
    /// halfword either way -- so the first draft could not tell `LDRH` from
    /// `LDR`, and the mutation that turned `ldrh` into `ldr` left the whole
    /// suite green.
    #[test]
    fn g3_ldrh_reads_back_a_halfword() {
        let body = [mov_imm(0, SCRATCH as u32), ldrh(2, 0), swi(HALT)];
        let mut fixture = fixture_for(&body);
        fixture.write_word(SCRATCH, 0xC5C5_C500);
        assert!(
            fixture.run_to_halt(BUDGET),
            "the program never reached its Halt"
        );
        assert_eq!(
            fixture.reg(2),
            0x0000_C500,
            "LDRH must load the halfword and leave the upper half of the register zero"
        );
    }

    /// `SWI n` carries its number in bits 16-23.
    ///
    /// `0x0D` is `GetBiosChecksum`, whose answer is a published constant — so
    /// if the number arrived in the wrong byte the dispatch would decline it,
    /// the machine would park on the BIOS SWI vector instead of halting, and
    /// this test would fail rather than pass on the wrong encoding.
    ///
    /// The trailing `Halt` matters: the SWI returns into `r0` and the next
    /// instruction would otherwise be the image's zero words, and a zero word
    /// is `ANDEQ r0, r0, r0` — it would clear the very register this test
    /// reads.
    #[test]
    fn g3_swi_carries_its_number_in_bits_16_to_23() {
        let body = [swi(0x0D), swi(HALT)];
        let fixture = run_to_halt(&body);
        assert_eq!(
            fixture.reg(0),
            0xBA_AE_187F,
            "SWI 0x0D must reach GetBiosChecksum"
        );
    }

    /// `MOV rd, rm` copies a register and leaves it alone.
    ///
    /// The value is read from `r5` and put back into `r5`, and the machine
    /// reaches its `Halt` — so the copy neither clobbered the register nor
    /// stopped the program.
    #[test]
    fn g3_mov_reg_copies_without_clobbering() {
        let body = [mov_imm(5, 0x34), mov_reg(5, 5), swi(HALT)];
        let fixture = run_to_halt(&body);
        assert_eq!(
            fixture.reg(5),
            0x34,
            "MOV r5, r5 must leave r5 alone"
        );
    }
}
