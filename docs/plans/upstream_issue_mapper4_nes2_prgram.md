# Upstream issue draft — mapper 4 (MMC3) NES 2.0 PRG-RAM size forwarded verbatim

> **状态：草稿，未发布。** 待业主审阅后决定是否向上游（FCEUX）提交。
> 上游仓库：`https://github.com/TASEmulators/fceux`（2.8.0 开发版）。
> 按业主偏好：**介绍性段落英中对照，技术正文纯英文**；发布前请删除所有中文对照段。

---

## Title

`Mapper 4 (MMC3): forwarding the NES 2.0 PRG-RAM size verbatim removes $6000-$7FFF from mapper 4 carts`

---

## Body

> We are working on a fork of FCEUX (NES emulator) and ran into a behaviour change
> in the 2.8.0 development branch that makes a set of real mapper 4 cartridges lose
> their work RAM. We would like to check our reading of the hardware before
> deciding how to handle it on our side, and we would appreciate any correction
> or confirmation from someone familiar with the MMC3.
>
> 我们在做 FCEUX（NES 模拟器）的分支，在 2.8.0 开发版上遇到一个行为变更：一批真实的
> mapper 4 卡带会失去工作 RAM。在决定我们这边怎么处理之前，想先就硬件行为向熟悉
> MMC3 的人核对一下；若我们的理解有误，也请指出。

`Mapper4_Init` currently derives the MMC3 WRAM size directly from the NES 2.0
header's PRG-RAM field:

```c
void Mapper4_Init(CartInfo *info) {
    int ws = 8;

    if (info->ines2) {
        ws = (info->wram_size + info->battery_wram_size) / 1024;
    } else if ((info->CRC32 == 0x93991433 || info->CRC32 == 0xaf65aa84)) {
        ...
        ws = 0;
    }
    GenMMC3_Init(info, 512, 256, ws, info->battery);
```

For a mapper 4 ROM whose header declares `byte 10 = 0x00`, this yields `ws = 0`.
`GenMMC3_Init` then skips the WRAM allocation entirely (`if (wram)`), so
`$6000-$7FFF` is left unmapped for that cartridge. On a real MMC3 the PRG RAM is
an 8 KiB chip at `$6000-$7FFF` and the byte 10 field cannot describe any other
size, so the only two meanings a declaration can carry are "8 KiB present" and
"absent" — yet the formula forwards the raw declared size, which turns a
`0x00` declaration into "no WRAM at all".

The case that made us notice this is `tepples/mmc3bigchrram/mmc3bigchrram.nes`
from the `christopherpow/nes-test-roms` collection: it is a mapper 4 cartridge
with 32 KiB CHR RAM, its header declares `byte 10 = 0x00` and `byte 11 = 0x09`
(32 KiB CHR RAM), and it reports its test result through the `$6000` protocol.
With `ws = 8` the ROM passes; with `ws = 0` the `$6000` channel is gone and the
test fails. The same applies to every mapper 4 dump whose header leaves byte 10
at zero, which is common because the PRG RAM chip is optional on TxROM boards
while the dumpers do not distinguish "no chip" from "8 KiB chip".

The NESdev wiki pages we consulted seem to support reading the field as
mapper-specific rather than as a literal size:

- [MMC3](https://www.nesdev.org/wiki/MMC3) lists `PRG RAM: Up to 8 KB` with
  `CPU $6000-$7FFF: 8 KB PRG RAM bank`.
- [NES 2.0](https://www.nesdev.org/wiki/NES_2.0) states that the memory
  location and valid sizes of PRG RAM depend on the specific mapper, that the
  vast majority may place up to 8 KiB in `$6000-7FFF`, and that specifying 0 for
  the PRG RAM size would usually leave that region as open bus.
- [INES Mapper 004](https://www.nesdev.org/wiki/INES_Mapper_004) notes that
  NES 2.0 can be used to specify a PRG-RAM size of 0, which is how the
  Low-G-Man case (an MMC3 with no PRG RAM at all) is meant to be expressed.

Our reading is therefore that a mapper 4 PRG-RAM declaration can only mean
"present" (8 KiB) or "absent" (0), and that forwarding the declared size
verbatim cannot be right for MMC3 in any case, because MMC3 has no way to
present more than 8 KiB at `$6000-$7FFF`. We have locally reverted the
`info->ines2` branch for mapper 4 and kept the historical 8 KiB default, with
the existing CRC32 Low-G-Man workaround still handling the "absent" case for
iNES 1.0 dumps.

We would appreciate it if someone could confirm two points: whether the
NES 2.0 byte 10 declaration for mapper 4 should be interpreted as
present/absent rather than as a literal size, and whether the upstream branch
is intended to keep the work RAM for mapper 4 cartridges whose header declares
zero. If the current behaviour is intentional, we would also like to understand
which mapper 4 dumps are expected to lose `$6000-$7FFF`, so that we can match
it deliberately rather than by accident.

> To summarise what we are asking: we are not asking for any feature to be
> removed — the change only concerns how the declared PRG-RAM size of a mapper 4
> cartridge is interpreted. If our reading of the MMC3 hardware is wrong, a
> pointer to the correct behaviour would be very welcome, and we will adjust
> our fork accordingly.
>
> 小结一下我们的诉求：我们不是要求删掉任何功能 —— 这只涉及 mapper 4 卡带声明的
> PRG-RAM 大小该怎么解读。如果我们对 MMC3 硬件的理解有误，请指出正确行为，我们
> 会相应调整分支。
