<!--
source: https://www.nesdev.org/wiki/PPU_registers
retrieved: 2026-09-13
conversion: automated from the owner-provided HTML archive; conventions in ../../README.md
-->

# PPU registers (NESdev Wiki)

> Source: https://www.nesdev.org/wiki/PPU_registers

The PPU exposes eight memory-mapped registers to the CPU. These nominally sit at $2000 through $2007 in the CPU's address space, but because their addresses are incompletely decoded, they're mirrored in every 8 bytes from $2008 through $3FFF. For example, a write to $3456 is the same as a write to $2006.

The PPU starts rendering immediately after power-on or reset, but ignores writes to most registers (specifically $2000, $2001, $2005 and $2006) until reaching the pre-render scanline of the next frame; more specifically, for around 29658 NTSC CPU cycles or 33132 PAL CPU cycles, assuming the CPU and PPU are reset at the same time. See PPU power up state and Init code for details.

























## Summary

| Common Name | Address | Bits | Type | Notes |
|---|---|---|---|---|
| PPUCTRL | $2000 | VPHB SINN | W | NMI enable (V), PPU master/slave (P), sprite height (H), background tile select (B), sprite tile select (S), increment mode (I), nametable select / X and Y scroll bit 8 (NN) |
| PPUMASK | $2001 | BGRs bMmG | W | color emphasis (BGR), sprite enable (s), background enable (b), sprite left column enable (M), background left column enable (m), greyscale (G) |
| PPUSTATUS | $2002 | VSO- ---- | R | vblank (V), sprite 0 hit (S), sprite overflow (O); read resets write pair for $2005/$2006 |
| OAMADDR | $2003 | AAAA AAAA | W | OAM read/write address |
| OAMDATA | $2004 | DDDD DDDD | RW | OAM data read/write |
| PPUSCROLL | $2005 | XXXX XXXX YYYY YYYY | Wx2 | X and Y scroll bits 7-0 (two writes: X scroll, then Y scroll) |
| PPUADDR | $2006 | ..AA AAAA AAAA AAAA | Wx2 | VRAM address (two writes: most significant byte, then least significant byte) |
| PPUDATA | $2007 | DDDD DDDD | RW | VRAM data read/write |
| OAMDMA | $4014 | AAAA AAAA | W | OAM DMA high address |

Register types:

- R - Readable

- W - Writeable

- x2 - Internal 2-byte state accessed by two 1-byte accesses

## MMIO registers

The PPU has an internal data bus that it uses for communication with the CPU.
This bus, called _io_db in Visual 2C02 and PPUGenLatch in FCEUX,[1] behaves as an 8-bit dynamic latch due to capacitance of very long traces that run to various parts of the PPU.
Writing any value to any PPU port, even to the nominally read-only PPUSTATUS, will fill this latch.
Reading any readable port (PPUSTATUS, OAMDATA, or PPUDATA) also fills the latch with the bits read.
Reading a nominally "write-only" register returns the latch's current value, as do the unused bits of PPUSTATUS.
At least one bit in this value decays after 3ms to 30ms, faster when the PPU is warm. [2]

### PPUCTRL - Miscellaneous settings ($2000 write)

7 bit 0
---- ----
VPHB SINN
|||| ||||
|||| ||++- Base nametable address
|||| || (0 = $2000; 1 = $2400; 2 = $2800; 3 = $2C00)
|||| |+--- VRAM address increment per CPU read/write of PPUDATA
|||| | (0: add 1, going across; 1: add 32, going down)
|||| +---- Sprite pattern table address for 8x8 sprites
|||| (0: $0000; 1: $1000; ignored in 8x16 mode)
|||+------ Background pattern table address (0: $0000; 1: $1000)
||+------- Sprite size (0: 8x8 pixels; 1: 8x16 pixels – see PPU OAM#Byte 1)
|+-------- PPU master/slave select
| (0: read backdrop from EXT pins; 1: output color on EXT pins)
+--------- Vblank NMI enable (0: off, 1: on)

PPUCTRL (the "control" or "controller" register) contains a mix of settings related to rendering, scroll position, vblank NMI, and dual-PPU configurations. After power/reset, writes to this register are ignored until the first pre-render scanline.

#### Vblank NMI

Enabling NMI in PPUCTRL causes the NMI handler to be called at the start of vblank (scanline 241, dot 1). This provides a reliable time source for software so it can run at the display's frame rate, and it signals vblank to the software. Vblank is the only time with rendering enabled that the software can send data to VRAM and OAM, and this NMI is the only reliable way to detect vblank; polling the vblank flag in PPUSTATUS can miss vblank entirely.

Changing NMI enable from 0 to 1 while the vblank flag in PPUSTATUS is 1 will immediately trigger an NMI. This happens during vblank if the PPUSTATUS register has not yet been read. It can result in graphical glitches by making the NMI routine execute too late in vblank to finish on time, or cause the game to handle more frames than have actually occurred. To avoid this problem, it is prudent to read PPUSTATUS first to clear the vblank flag before enabling NMI in PPUCTRL.

#### Scrolling

The current nametable bits in PPUCTRL bits 0 and 1 can equivalently be considered the most significant bit of the scroll coordinates, which are 9 bits wide (see Nametables and PPUSCROLL):

7 bit 0
---- ----
.... ..YX
||
|+- X scroll position bit 8 (i.e. add 256 to X)
+-- Y scroll position bit 8 (i.e. add 240 to Y)

These two bits go to the same internal t register as the values written to PPUSCROLL, and they must be written alongside PPUSCROLL in order to fully specify the scroll position.

#### Master/slave mode and the EXT pins

Bit 6 of PPUCTRL should never be set on stock consoles because it may damage the PPU.

When this bit is clear (the usual case), the PPU gets the palette index for the backdrop color from the EXT pins. The stock NES grounds these pins, making palette index 0 the backdrop color as expected. A secondary picture generator connected to the EXT pins would be able to replace the backdrop with a different image using colors from the background palette, which could be used for features such as parallax scrolling.

Setting bit 6 causes the PPU to output the lower four bits of the palette memory index on the EXT pins for each pixel. Since only four bits are output, background and sprite pixels can't normally be distinguished this way. Setting this bit does not affect the image in the PPU's composite video output. As the EXT pins are grounded on an unmodified NES, setting bit 6 is discouraged as it could potentially damage the chip whenever it outputs a non-zero pixel value (due to it effectively shorting Vcc and GND together). Note that EXT output for transparent pixels is not a backdrop color as normal, but rather entry 0 of that background sliver's palette. When rendering is disabled, EXT output is always index 0 regardless of backdrop override.

#### Bit 0 race condition

Be careful when writing to this register outside vblank if using a horizontal nametable arrangement (a.k.a. vertical mirroring) or 4-screen VRAM.
For specific CPU-PPU alignments, a write that starts on dot 257 will cause only the next scanline to be erroneously drawn from the left nametable.
This can cause a visible glitch, and it can also interfere with sprite 0 hit for that scanline (by being drawn with the wrong background).

The glitch has no effect in horizontal or one-screen mirroring because the left and right nametables are identical.
Only writes that start on dot 257 and continue through dot 258 can cause this glitch: any other horizontal timing is safe.
The glitch specifically writes the value of open bus to the register, which will almost always be the upper byte of the address. Writing to this register or the mirror of this register at $2100 according to the desired nametable appears to be a functional workaround.

This produces an occasionally visible glitch in Super Mario Bros. when the program writes to PPUCTRL at the end of game logic.
It appears to be turning NMI off during game logic and then turning NMI back on once the game logic has finished in order to prevent the NMI handler from being called again before the game logic finishes.
Another workaround is to use a software flag to prevent NMI reentry, instead of using the PPU's NMI enable.

### PPUMASK - Rendering settings ($2001 write)

7 bit 0
---- ----
BGRs bMmG
|||| ||||
|||| |||+- Greyscale (0: normal color, 1: greyscale)
|||| ||+-- 1: Show background in leftmost 8 pixels of screen, 0: Hide
|||| |+--- 1: Show sprites in leftmost 8 pixels of screen, 0: Hide
|||| +---- 1: Enable background rendering
|||+------ 1: Enable sprite rendering
||+------- Emphasize red (green on PAL/Dendy)
|+-------- Emphasize green (red on PAL/Dendy)
+--------- Emphasize blue

PPUMASK (the "mask" register) controls the rendering of sprites and backgrounds, as well as color effects. After power/reset, writes to this register are ignored until the first pre-render scanline.

Most commonly, PPUMASK is set to $00 outside of gameplay to allow transferring a large amount of data to VRAM, and $1E during gameplay to enable all rendering with no color effects.

#### Rendering control

Rendering is the PPU's process of actively fetching memory and drawing an image to the screen. Rendering as a whole is enabled as long as one or both of sprite and background rendering is enabled in PPUMASK. If one component is enabled and the other is not, the disabled component is simply treated as transparent; the rendering process is otherwise unaffected. When both components are disabled via bits 3 and 4, the rendering process stops and the PPU displays the backdrop color.

During rendering, the PPU is actively using VRAM and OAM. This prevents the CPU from being able to access VRAM via PPUDATA or OAM via OAMDATA, so these accesses must be done outside of rendering: either during vblank (for data transfers during gameplay) or with rendering turned off (for large data transfers, such as when loading a level). To avoid numerous hardware bugs and limitations, it is generally recommended that rendering be turned on or off only during vblank. This can be done by writing the desired PPUMASK value to a variable rather than the register itself and then only copying that variable to PPUMASK during vblank in the NMI handler.

The PPU can optionally hide sprites and backgrounds in just the leftmost 8 pixels of the screen, making them transparent and thus drawing the backdrop color there. For sprites, this can be useful to avoid sprite pop-in, a limitation where sprites cannot partially hang off the left edge of the screen like they can off the right edge. For backgrounds, this can eliminate tile artifacts and reduce attribute artifacts when scrolling horizontally with either a vertical or one-screen nametable arrangement, as these arrangements do not allow hiding the scroll seam off-screen. Note that the backdrop color may not match the color used by the art for the background, so disabling the left column may be more distracting than minor artifacts.

Notes:

- Writing to PPUDATA during rendering can corrupt VRAM, so writes must be done in vblank or with rendering disabled in PPUMASK bits 3 and 4.

- Sprite 0 hit does not trigger in any area where the background or sprites are disabled.

- Toggling rendering takes effect approximately 3-4 dots after the write. This delay is required by Battletoads to avoid a crash.

- Toggling rendering mid-screen often corrupts 1 row of OAM and draws incorrect sprites for the current and next scanline. (See: Errata)

- Turning rendering off mid-screen can corrupt palette RAM if the low 14 bits of the internal v register have a value between $3C00-$3FFF.

- Turning rendering on late causes the dot at the end of pre-render to never be skipped, which can cause dot crawl on stationary screens.

- Turning rendering on late causes the PPU to have an incorrect scroll value unless it is set manually with a complicated series of writes.

#### Color control

Greyscale mode forces all colors to be a shade of grey or white. This is done by bitwise ANDing the color with $30, causing all colors to come from the grey column ($00, $10, $20, $30), which notably lacks a black color. Note that this AND behavior means that RGB PPUs with scrambled colors (the 2C04 series) do not actually get shades of grey, but rather whatever colors are in the $x0 column. When reading from palette RAM, the returned value reflects this AND behavior, but the underlying data is preserved. Palette writes function normally regardless of greyscale mode.

Color emphasis causes a color tint effect that works by darkening the other two color components, making the selected component comparatively brighter and thus emphasized. Emphasizing all 3 components simply dims all colors. This works independently of greyscale, allowing greys to be tinted. Note that PAL and Dendy PPUs have a different emphasis bit order, so ports and dual-region games should reorder the bits. Furthermore, emphasis on RGB PPUs is completely different, instead maximizing the brightness of the emphasized component and producing a completely white screen when all components are emphasized. RGB emphasis is far less useful and generally best avoided.

### PPUSTATUS - Rendering events ($2002 read)

7 bit 0
---- ----
VSOx xxxx
|||| ||||
|||+-++++- (PPU open bus or 2C05 PPU identifier)
||+------- Sprite overflow flag
|+-------- Sprite 0 hit flag
+--------- Vblank flag, cleared on read. Unreliable; see below.

PPUSTATUS (the "status" register) reflects the state of rendering-related events and is primarily used for timing. The three flags in this register are automatically cleared on dot 1 of the prerender scanline; see PPU rendering for more information on the set and clear timing.

Reading this register has the side effect of clearing the PPU's internal w register. It is commonly read before writes to PPUSCROLL and PPUADDR to ensure the writes occur in the correct order.

#### Vblank flag

The vblank flag is set at the start of vblank (scanline 241, dot 1). Reading PPUSTATUS will return the current state of this flag and then clear it. If the vblank flag is not cleared by reading, it will be cleared automatically on dot 1 of the prerender scanline.

Reading the vblank flag is not a reliable way to detect vblank. NMI should be used, instead. Reading the flag on the dot before it is set (scanling 241, dot 0) causes it to read as 0 and be cleared, so polling PPUSTATUS for the vblank flag can miss vblank and cause games to stutter. NMI is also suppressed when this occurs, and may even be suppressed by reads landing on the following dot or two. On NTSC and PAL, it is guaranteed that the flag cannot be dropped two frames in a row, but on Dendy, it is possible for it to happen every frame, crashing the game. Using NMI ensures that software correctly detects vblank every frame. It is also required by PlayChoice-10, which will reject the game if NMI is disabled for too long. Polling the vblank flag is still required while booting up the console, but timing at this point is not critical (see Init code for more information on booting safely).

The vblank flag is used in the generation of NMI, and enabling NMI while this flag is 1 will cause an immediate NMI (see PPUCTRL).

#### Sprite 0 hit flag

Sprite 0 hit is a hardware collision detection feature that detects pixel-perfect collision between the first sprite in OAM (sprite 0) and the background. The sprite 0 hit flag is immediately set when any opaque pixel of sprite 0 overlaps any opaque pixel of background, regardless of sprite priority. 'Opaque' means that the pixel is not 'transparent' — that is, its two pattern bits are not %00. The flag stays set until dot 1 of the prerender scanline; thus, it can only detect one collision per frame.

Although this flag detects collision, it is primarily used for timing. Many games place sprite 0 at a fixed location on the screen and poll this flag until it becomes set. This allows the CPU to know its approximate location on the screen so it can time mid-screen writes to hardware registers. Commonly, this is used to change the scroll position mid-screen to allow for a background-based HUD, like in Super Mario Bros. However, some modern homebrew games use this for actual collision, such as Lunar Limit and Irritating Ship.

Sprite 0 hit cannot detect collision at X=255, nor anywhere where either sprites or backgrounds are disabled via PPUMASK. This includes X=0..7 when the leftmost 8 pixels are hidden. However, it is not affected by the cropping on the left and right edges on PAL.

There are some important considerations when using this flag for timing:

- Because sprite 0 hit is not cleared until the prerender scanline, software can potentially mistake the previous frame's hit as being from the current frame. Therefore, it may be necessary to poll the flag until it becomes clear before then polling for it to be set again.

- If a game expects sprite 0 hit to occur and it does not, this often results in a crash. If there is any risk that the hit may not occur (perhaps because an overlap may not happen when scrolling or because it relies on precise mid-screen timings that may vary across power cycles, consoles, or emulators), it can be critical to have another way to exit the poll loop. For example, this may be done by also polling the vblank flag or having the NMI handler check if the game is still polling for sprite 0 hit.

- Games often don't handle sprite 0 hit on lag frames, preventing the mid-screen event from occurring. A common result of this is HUD flickering during lag. Handling sprite 0 hit in the NMI handler, at least on lag frames, can work around this.

#### Sprite overflow flag

The sprite overflow flag was intended to be set any time there are more than 8 sprites on a scanline. Unfortunately, the logic for detecting this does not work correctly, resulting in the PPU checking incorrect indices in OAM when searching for a 9th sprite. This produces both false positives and false negatives. See PPU sprite evalution for details on its incorrect behavior. In practice, sprite overflow is usually used for timing like sprite 0 hit, but because of its buggy behavior and its cost of 9 sprite tiles, it is generally only used when more than one timing source is required. Like sprite 0 hit, this flag is cleared at the start of the prerender scanline and can only be set once per frame.

Using sprite overflow is often a last resort. When mapper IRQs are not available, the DMC IRQ can be an effective alternative for timing, albeit complicated to use.

#### 2C05 identifier

The 2C05 series of arcade PPUs returns an identifier in bits 4-0 instead of PPU open bus. This value is checked by games as a form of copy protection. Note that this does not apply to the consumer 2C05-99, which returns open bus as usual. While we haven't yet collected data directly from the PPUs, 2C05 games expect the following values:

| PPU | Mask | Value |
|---|---|---|
| 2C05-02 | $3F | $3D |
| 2C05-03 | $1F | $1C |
| 2C05-04 | $1F | $1B |

### OAMADDR - Sprite RAM address ($2003 write)

7 bit 0
---- ----
AAAA AAAA
|||| ||||
++++-++++- OAM address

Write the address of OAM you want to access here. Most games just write $00 here and then use OAMDMA. (DMA is implemented in the 2A03/7 chip and works by repeatedly writing to OAMDATA)

#### Values during rendering

OAMADDR is set to 0 during each of ticks 257–320 (the sprite tile loading interval) of the pre-render and visible scanlines. This also means that at the end of a normal complete rendered frame, OAMADDR will always have returned to 0.

If rendering is enabled mid-scanline[3], there are further consequences of an OAMADDR that was not set to 0 before OAM sprite evaluation begins at tick 65 of the visible scanline. The value of OAMADDR at this tick determines the starting address for sprite evaluation for this scanline, which can cause the sprite at OAMADDR to be treated as it was sprite 0, both for sprite-0 hit and priority. If OAMADDR is unaligned and does not point to the Y position (first byte) of an OAM entry, then whatever it points to (tile index, attribute, or X coordinate) will be reinterpreted as a Y position, and the following bytes will be similarly reinterpreted. No more sprites will be found once the end of OAM is reached, effectively hiding any sprites before the starting OAMADDR.

#### OAMADDR precautions

On the 2C02G, writes to OAMADDR corrupt OAM. The exact corruption isn't fully described, but this usually seems to copy sprites 8 and 9 (address $20) over the 8-byte row at the target address. The source address for this copy seems to come from the previous value on the CPU BUS (most often $20 from the $2003 operand).[3][4] There may be other possible behaviors as well. This can then be worked around by writing all 256 bytes of OAM, though due to the limited time before OAM decay will begin this should normally be done through OAMDMA.

It is also the case that if OAMADDR is not less than eight when rendering starts, the eight bytes starting at OAMADDR & 0xF8 are copied to the first eight bytes of OAM; it seems likely that this is related. On the Dendy, the latter bug is required for 2C02 compatibility.

It is known that in the 2C03, 2C04, 2C05[5], and 2C07, OAMADDR works as intended. It is not known whether this bug is present in all revisions of the 2C02.

### OAMDATA - Sprite RAM data ($2004 read/write)

7 bit 0
---- ----
DDDD DDDD
|||| ||||
++++-++++- OAM data

Write OAM data here. Writes will increment OAMADDR after the write; reads do not. Reads during vertical or forced blanking return the value from OAM at that address.

Do not write directly to this register in most cases. Because changes to OAM should normally be made only during vblank, writing through OAMDATA is only effective for partial updates, as it is too slow to update all of OAM within one vblank interval, and as described above, partial writes cause corruption. Most games use the DMA feature through OAMDMA instead.

- Reading OAMDATA while the PPU is rendering will expose internal OAM accesses during sprite evaluation and loading; Micro Machines does this.

- Writes to OAMDATA during rendering (on the pre-render line and the visible lines 0–239, provided either sprite or background rendering is enabled) do not modify values in OAM, but do perform a glitchy increment of OAMADDR, bumping only the high 6 bits (i.e., it bumps the [n] value in PPU sprite evaluation – it's plausible that it could bump the low bits instead depending on the current status of sprite evaluation). This extends to DMA transfers via OAMDMA, since that uses writes to $2004. For emulation purposes, it is probably best to completely ignore writes during rendering.

- It used to be thought that reading from this register wasn't reliable[6], however more recent evidence seems to suggest that this is solely due to corruption by OAMADDR writes.

- In the oldest instantiations of the PPU, as found on earlier Famicoms and NESes, this register is not readable[7]. The readability was added on the RP2C02G, found on most NESes and later Famicoms.[8]

- In the 2C07, sprite evaluation can never be fully disabled, and will always start 24 scanlines after the start of vblank[9] (same as when the prerender scanline would have been on the 2C02). As such, any updates to OAM should be done within the first 24 scanlines after the 2C07 signals vertical blanking.

### PPUSCROLL - X and Y scroll ($2005 write)

1st write
7 bit 0
---- ----
XXXX XXXX
|||| ||||
++++-++++- X scroll bits 7-0 (bit 8 in PPUCTRL bit 0)

2nd write
7 bit 0
---- ----
YYYY YYYY
|||| ||||
++++-++++- Y scroll bits 7-0 (bit 8 in PPUCTRL bit 1)

This register is used to change the scroll position, telling the PPU which pixel of the nametable selected through PPUCTRL should be at the top left corner of the rendered screen. PPUSCROLL takes two writes: the first is the X scroll and the second is the Y scroll. Whether this is the first or second write is tracked internally by the w register, which is shared with PPUADDR. Typically, this register is written to during vertical blanking to make the next frame start rendering from the desired location, but it can also be modified during rendering in order to split the screen. Changes made to the vertical scroll during rendering will only take effect on the next frame. Together with the nametable bits in PPUCTRL, the scroll can be thought of as 9 bits per component, and PPUCTRL must be updated along with PPUSCROLL to fully specify the scroll position.

| | The PPU scroll registers share internal state with the PPU address registers. Because of this, PPUSCROLL and the nametable bits in PPUCTRL should be written after any writes to PPUADDR. |
|---|---|

After reading PPUSTATUS to clear w (the write latch), write the horizontal and vertical scroll offsets to PPUSCROLL just before turning on the screen:

; Set the high bit of X and Y scroll.
lda ppuctrl_value
ora current_nametable
sta PPUCTRL

; Set the low 8 bits of X and Y scroll.
bit PPUSTATUS
lda cam_position_x
sta PPUSCROLL
lda cam_position_y
sta PPUSCROLL

Horizontal offsets range from 0 to 255. "Normal" vertical offsets range from 0 to 239, while values of 240 to 255 cause the attributes data at the end of the current nametable to be used incorrectly as tile data. The PPU normally skips from 239 to 0 of the next nametable automatically, so these "invalid" scroll positions only occur if explicitly written.

By changing the scroll values here across several frames and writing tiles to newly revealed areas of the nametables, one can achieve the effect of a camera panning over a large background.

### PPUADDR - VRAM address ($2006 write)

1st write 2nd write
15 bit 8 7 bit 0
---- ---- ---- ----
..AA AAAA AAAA AAAA
|| |||| |||| ||||
++-++++--++++-++++- VRAM address

Because the CPU and the PPU are on separate buses, neither has direct access to the other's memory. The CPU writes to VRAM through a pair of registers on the PPU by first loading an address into PPUADDR and then writing data repeatedly to PPUDATA. The VRAM address only needs to be set once for every series of data writes because each PPUDATA access automatically increments the address by 1 or 32, as configured in PPUCTRL.

The 16-bit address is written to PPUADDR one byte at a time, high byte first. Whether this is the first or second write is tracked by the PPU's internal w register, which is shared with PPUSCROLL. If w is not 0 or its state is not known, it must be cleared by reading PPUSTATUS before writing the address. For example, to set the VRAM address to $2108 after w is known to be 0:

lda #$21
sta PPUADDR
lda #$08
sta PPUADDR

The PPU address space is 14-bit, spanning $0000–$3FFF. Bits 14 and 15 of the value written to this register are ignored. However, bit 14 of the internal t register that holds the data written to PPUADDR is forced to 0 when writing the PPUADDR high byte. This detail doesn't matter when using PPUADDR to set a VRAM address, but is an important limitation when using it to control mid-screen scrolling (see PPU scrolling for more information).

#### Note

Access to PPUSCROLL and PPUADDR during screen refresh produces interesting raster effects; the starting position of each scanline can be set to any pixel position in nametable memory. For more information, see PPU scrolling.

#### Palette corruption

In specific circumstances, entries of the PPU's palette can be corrupted. It's unclear exactly how or why this happens, but all revisions of the NTSC PPU seem to be at least somewhat susceptible.[10]

When done writing to palette memory, the workaround is to always

- Update the address, if necessary, so that it's pointing at $3F00, $3F10, $3F20, or any other mirror.

- Only then change the address to point outside of palette memory.

A code fragment to implement this workaround is present in vast numbers of games:[11]

lda #$3F
sta PPUADDR
lda #0
sta PPUADDR
sta PPUADDR
sta PPUADDR

#### Bus conflict

During raster effects, if the second write to PPUADDR happens at specific times, at most one axis of scrolling will be set to the bitwise AND of the written value and the current value. The only safe time to finish the second write is during blanking; see PPU scrolling for more specific timing. [1]

### PPUDATA - VRAM data ($2007 read/write)

7 bit 0
---- ----
DDDD DDDD
|||| ||||
++++-++++- VRAM data

VRAM read/write data register. After access, the video memory address will increment by an amount determined by bit 2 of $2000.

When the screen is turned off by disabling the background/sprite rendering flag with the PPUMASK or during vertical blank, data can be read from or written to VRAM through this port. Since accessing this register increments the VRAM address, it should not be accessed outside vertical or forced blanking because it will cause graphical glitches, and if writing, write to an unpredictable address in VRAM. However, a handful of games are known to read from PPUDATA during rendering, causing scroll position changes. See PPU scrolling and Tricky-to-emulate games.

VRAM reading and writing shares the same internal address register that rendering uses. Therefore, after loading data into video memory, the program should reload the scroll position afterwards with PPUSCROLL and PPUCTRL (bits 1-0) writes in order to avoid wrong scrolling.

#### The PPUDATA read buffer

Reading from PPUDATA does not directly return the value at the current VRAM address, but instead returns the contents of an internal read buffer. This read buffer is updated on every PPUDATA read, but only after the previous contents have been returned to the CPU, effectively delaying PPUDATA reads by one. This is because PPU bus reads are too slow and cannot complete in time to service the CPU read. Because of this read buffer, after the VRAM address has been set through PPUADDR, one should first read PPUDATA to prime the read buffer (ignoring the result) before then reading the desired data from it.

Note that the read buffer is updated only on PPUDATA reads. It is not affected by writes or other PPU processes such as rendering, and it maintains its value indefinitely until the next read.

#### Reading palette RAM

Later PPUs added an unreliable feature for reading palette data from $3F00-$3FFF. These reads work differently than standard VRAM reads, as palette RAM is a separate memory space internal to the PPU that is overlaid onto the PPU address space. The referenced 6-bit palette data is returned immediately instead of going to the internal read buffer, and hence no priming read is required. Simultaneously, the PPU also performs a normal read from PPU memory at the specified address, "underneath" the palette data, and the result of this read goes into the read buffer as normal. The old contents of the read buffer are discarded when reading palettes, but by changing the address to point outside palette RAM and performing one read, the contents of this shadowed memory (usually mirrored nametables) can be accessed. On PPUs that do not support reading palette RAM, this memory range behaves the same as the rest of PPU memory.

This feature is supported by the 2C02G, 2C02H, and PAL PPUs. The byte returned when reading palettes contains PPU open bus in the top 2 bits, and the value is returned after it is modified by greyscale mode, which clears the bottom 4 bits if enabled. Unfortunately, on some consoles, palette reads can be corrupted on one of the 4 CPU/PPU alignments relative to the master clock. This corruption depends on when the PPU /CS signal that indicates register access is deasserted, which varies by console. Combined with this feature not being present in all PPUs, developers should not rely on reading from palette RAM.

#### Read conflict with DPCM samples

If currently playing DPCM samples, there is a chance that an interruption from the APU's sample fetch will cause an extra read cycle if it happened at the same time as an instruction that reads $2007. This will cause an extra increment and a byte to be skipped over, resulting in the wrong data being read. See: APU DMC

### OAMDMA - Sprite DMA ($4014 write)

7 bit 0
---- ----
AAAA AAAA
|||| ||||
++++-++++- Source page (high byte of source address)

OAMDMA is a CPU register that suspends the CPU so it can quickly copy a page of CPU memory to PPU OAM using DMA. It always copies 256 bytes and the source address always starts page-aligned (ending in $00). The value written to this register is the high byte of the source address, and the copy begins on the cycle immediately after the write. The copy takes 513 or 514 cycles and is implemented as 256 pairs of a read from CPU memory and a write to OAMDATA. Because vblank is so short and because changing OAMADDR often corrupts OAM, OAM DMA is normally the only realistic option for updating sprites each frame. 0 should be written to OAMADDR before initiating DMA to ensure the data is properly aligned and to avoid corruption.[4] While OAM DMA is possible to do mid-frame while rendering is disabled, it is normally only done in vblank.

OAM consists of dynamic RAM (DRAM) which decays if not refreshed often enough, and this requires different considerations on NTSC and PAL. Refresh happens automatically any time a row of DRAM is read or written, so it is refreshed every scanline during rendering by the sprite evaluation process. On NTSC, vblank is short enough that OAM will not decay before rendering begins again, so OAM DMA can be done anytime in vblank. On PAL, vblank is much longer, so to avoid decay during that time, the PPU automatically performs a forced refresh starting 24 scanlines after NMI, during which OAM cannot be written. This means that OAM DMA is limited to the start of vblank on PAL. Note that NTSC vblank is shorter than 24 PAL scanlines, so NTSC-compatible NMI handlers will finish before the forced refresh and therefore should work on PAL regardless of their OAM DMA timing. In either case, OAM does not decay if it is not updated during vblank, and in fact it should generally not be updated on lag frames (frames where the CPU did not finish its work before vblank) to avoid copying incomplete sprite data to the PPU.

## Internal registers

The PPU also has 4 internal registers, described in detail on PPU scrolling:

- v: During rendering, used for the scroll position. Outside of rendering, used as the current VRAM address.

- t: During rendering, specifies the starting coarse-x scroll for the next scanline and the starting y scroll for the screen. Outside of rendering, holds the scroll or VRAM address before transferring it to v.

- x: The fine-x position of the current scroll, used during rendering alongside v.

- w: Toggles on each write to either PPUSCROLL or PPUADDR, indicating whether this is the first or second write. Clears on reads of PPUSTATUS. Sometimes called the 'write latch' or 'write toggle'.

## References

- ↑ ppu.cpp by Bero and Xodnizel

- ↑ replies to lidnariq's PPU decay test ROM

- ↑ Jump up to: 3.0 3.1 OAMDATA $2003 corruption clarification? - forum thread

- ↑ Jump up to: 4.0 4.1 Manual OAM write glitchyness thread by blargg

- ↑ Writes to $2003 appear to not cause OAM corruption post by lidnariq

- ↑ $2004 reading reliable? thread by blargg

- ↑ $2004 not readable on early revisions reply by jsr

- ↑ hardware revisions and $2004 reads reply by Great Hierophant

- ↑ 2C07 PPU sprite evaluation notes thread by thefox

- ↑ Problem with palette discoloration when PPU is turned off during rendering thread by N·K

- ↑ Weird PPU writes thread by Fiskbit


