<!--
source: https://www.nesdev.org/wiki/PPU_power_up_state
retrieved: 2026-09-13
conversion: automated from the owner-provided HTML archive; conventions in ../../README.md
-->

# PPU power up state (NESdev Wiki)

> Source: https://www.nesdev.org/wiki/PPU_power_up_state

In March 2008, Blargg reverse-engineered the power-up/reset state and behavior of the NES PPU, NTSC version.

| Register | At Power | After Reset |
|---|---|---|
| PPUCTRL ($2000) | 0000 0000 | 0000 0000 |
| PPUMASK ($2001) | 0000 0000 | 0000 0000 |
| PPUSTATUS ($2002) | +0+x xxxx | U??x xxxx |
| OAMADDR ($2003) | $00 | unchanged1 |
| $2005 / $2006 latch | cleared | cleared |
| PPUSCROLL ($2005) | $0000 | $0000 |
| PPUADDR ($2006) | $0000 | unchanged |
| PPUDATA ($2007) read buffer | $00 | $00 |
| odd frame | no | no |
| OAM | unspecified | unspecified |
| Palette | unspecified | unchanged |
| NT RAM (external, in Control Deck) | unspecified | unchanged |
| CHR RAM (external, in Game Pak) | unspecified | unchanged |

? = unknown,
x = irrelevant,
+ = often set,
U = unchanged

- The PPU comes out of power and reset at the top of the picture. See: PPU rendering.

- Writes to the following registers are ignored if earlier than ~29658 CPU clocks after reset: PPUCTRL, PPUMASK, PPUSCROLL, PPUADDR. This also means that the PPUSCROLL/PPUADDR latch will not toggle. The other registers work immediately: PPUSTATUS, OAMADDR, OAMDATA ($2004), PPUDATA, and OAMDMA ($4014). There is an internal reset signal that clears PPUCTRL, PPUMASK, PPUSCROLL, PPUADDR, the PPUSCROLL/PPUADDR latch, and the PPUDATA read buffer. (Clearing PPUSCROLL and PPUADDR corresponds to clearing the VRAM address latch (T) and the fine X scroll. Note that the VRAM address itself (V) is not cleared.) This reset signal is set on reset and cleared at the end of VBlank, by the same signal that clears the VBlank, sprite 0, and overflow flags. Attempting to write to a register while it is being cleared has no effect, which explains why writes are "ignored" after reset.

- If the NES is powered on after having been off for less than 20 seconds, register writes are ignored as if it were a reset, and register starting values differ: PPUSTATUS = $80 (VBlank flag set), OAMADDR = $2F or $01, and PPUADDR = $0001.

- The VBL flag (PPUSTATUS bit 7) is random at power, and unchanged by reset. It is next set around 27384, then around 57165.

- Preliminary testing on a PAL NES shows that writes are ignored until ~33132 CPU clocks after power and reset, 9 clocks less than 311 scanlines. It is conjectured that the first VBL flag setting will be close to 241 * 341/3.2 cycles (241 PAL scanlines); further testing by nocash has confirmed this.

- It is known that after power and reset, it is as if the APU's $4017 were written 10 clocks before the first code starts executing. This delay is probably the same source of the 9 clock difference in the times for PPU writes being ignored. The cause is likely the reset sequence of the 2A03, when it reads the reset vector.

- 1: Although OAMADDR is unchanged by reset, it is changed during rendering and cleared at the end of normal rendering, so you should assume its contents will be random.

- On front-loading consoles (NES-001), the Reset button on the Control Deck resets both the CPU and PPU. On top-loaders (Famicom, NES-101), only the CPU is reset.

Some of the initial state has unspecified values. Different lots of chips have different initial values due to the relative strengths of pull-down and pull-up elements in each bit cell, and the exact values of some bits may vary from one power-on to the next with ambient temperature or electromagnetic noise.

- The contents of OAM are unspecified both at power on and at reset due to DRAM decay.

- The contents of the palette are unspecified at power on and unchanged at reset. During the warmup state, the PPU outputs a solid color screen based on the value at $3F00.

- The contents of nametable RAM (in the Control Deck) and CHR RAM (in the Game Pak) are unspecified at power on and unchanged at reset.

- In almost all mappers, CHR bank values are unspecified at power on and unchanged at reset. The few exceptions, if any, are described on each mapper's page.






## Best practice

The easiest way to make sure that 29658 cycles have passed, and the way used by commercial NES games, involves a pair of loops like this in your init code:

bit PPUSTATUS ; clear the VBL flag if it was set at reset time
vwait1:
bit PPUSTATUS
bpl vwait1 ; at this point, about 27384 cycles have passed
vwait2:
bit PPUSTATUS
bpl vwait2 ; at this point, about 57165 cycles have passed

Due to the $2002 race condition, alignment between the CPU and PPU clocks at reset may cause the NES to miss an occasional VBL flag setting, but the only consequence of this is that your program will take one frame longer to start up.
You might want to do various other initialization, such as getting the mapper and RAM into a known state, between the two loops.

## Famicom

On the NTSC NES, the PPU and CPU are reset at the exact same time. On the Famicom, the PPU does not respond to the reset button, only the CPU is reset.

At power-on, on the Famicom, the PPU initialization begins approximately one frame before the CPU reset, because PPU /reset is tied to 5V, and CPU /reset is connected to a 0.47µF capacitor. The exact timing has not been measured, and may vary.

In particular, the Famicom game Magic John only waits 9217 CPU cycles before trying to enable NMI. This is before the required 29658 cycles required by the NTSC NES, so the game will not boot on that system, but using a Game Genie will allow the game to boot. This was corrected for the international release of the game, retitled as Totally Rad. This also affected The Lord of King, internationally known as Astyanax[1]. Both of these games were developed by Aicom for Jaleco.

## Dendy

Reading $2002 at the exact start of vblank clears the flag to 0 without reading back a 1.
On most consoles and with most wait loops, an alignment is eventually reached such that the flag is read other than on at the exact start of vblank.
However, Dendy-style PAL famiclones have a frame of exactly 113.667 by 312 = 35464 cycles, and 35464 is a multiple of 8.
A bit/bpl loop that crosses a page boundary, such as that found in the game Eliminator Boat Duel, lasts 8 cycles.
On some alignments, it hits the start of vblank every time and thus always fails to advance.

So for the $2002 wait loop, do not make a wait loop whose length in cycles evenly divides the frame length.

## See also

- CPU power up state

## References

- Confirmation by nocash

- Notes on reset color

- PPU warmup testing by Blargg

- Famicom CPU/PPU reset capacitor notes by lidnariq

- ↑ Forum thread: Re: PPU wait for ready in Donkey Kong - Test reports of Magic John and The Lord of King on different consoles.


