<!--
source: https://www.nesdev.org/wiki/PPU_OAM
retrieved: 2026-09-13
conversion: automated from the owner-provided HTML archive; conventions in ../../README.md
-->

# PPU OAM (NESdev Wiki)

> Source: https://www.nesdev.org/wiki/PPU_OAM

The OAM (Object Attribute Memory) is internal memory inside the PPU that contains a display list of up to 64 sprites, where each sprite's information occupies 4 bytes.












## OAM (Sprite) Data

### Byte 0 - Y position

Y position of top of sprite

Sprite data is delayed by one scanline; you must subtract 1 from the sprite's Y coordinate before writing it here.
Hide a sprite by moving it down offscreen, by writing any values between #$EF-#$FF here.
Sprites are never displayed on the first line of the picture, and it is impossible to place a sprite partially off the top of the screen.

### Byte 1 - Tile/index

Tile index number

For 8x8 sprites, this is the tile number of this sprite within the pattern table selected in bit 3 of PPUCTRL ($2000).

For 8x16 sprites (bit 5 of PPUCTRL set), the PPU ignores the pattern table selection and selects a pattern table from bit 0 of this number.

76543210
||||||||
|||||||+- Bank ($0000 or $1000) of tiles
+++++++-- Tile number of top of sprite (0 to 254; bottom half gets the next tile)

Thus, the pattern table memory map for 8x16 sprites looks like this:

- $00: $0000-$001F

- $01: $1000-$101F

- $02: $0020-$003F

- $03: $1020-$103F

- $04: $0040-$005F[...]

- $FE: $0FE0-$0FFF

- $FF: $1FE0-$1FFF

### Byte 2 - Attributes

Attributes

76543210
||||||||
||||||++- Palette (4 to 7) of sprite
|||+++--- Unimplemented (read 0)
||+------ Priority (0: in front of background; 1: behind background)
|+------- Flip sprite horizontally
+-------- Flip sprite vertically

Flipping does not change the position of the sprite's bounding box, just the position of pixels within the sprite.
If, for example, a sprite covers (120, 130) through (127, 137), it'll still cover the same area when flipped.
In 8x16 mode, vertical flip flips each of the subtiles and also exchanges their position; the odd-numbered tile of a vertically flipped sprite is drawn on top.
This behavior differs from the behavior of the unofficial 16x32 and 32x64 pixel sprite sizes on the Super NES, which will only vertically flip each square sub-region.

The three unimplemented bits of each sprite's byte 2 do not exist in the PPU and always read back as 0 on PPU revisions that allow reading PPU OAM through OAMDATA ($2004). This can be emulated by ANDing byte 2 with $E3 either when writing to or when reading from OAM. Bits that have decayed also read back as 0 through OAMDATA. These are side effects of the DRAM controller precharging an internal data bus with 0 to prevent writing high-impedance values to OAM DRAM cells.[1]

### Byte 3 - X position

X position of left side of sprite.

X-scroll values of $F9-FF results in parts of the sprite to be past the right edge of the screen, thus invisible. It is not possible to have a sprite partially visible on the left edge. Instead, left-clipping through PPUMASK ($2001) can be used to simulate this effect.

## Details

### DMA

Most programs write to a copy of OAM somewhere in CPU addressable RAM (often $0200-$02FF) and then copy it to OAM each frame using the OAMDMA ($4014) register. Writing N to this register causes the DMA circuitry inside the 2A03/07 to fully initialize the OAM by writing OAMDATA 256 times using successive bytes from starting at address $100*N). The CPU is suspended while the transfer is taking place.

The address range to copy from could lie outside RAM, though this is only useful for static screens with no animation.

Not counting the OAMDMA write tick, the above procedure takes 513 CPU cycles (+1 on odd CPU cycles): first one (or two) idle cycles, and then 256 pairs of alternating read/write cycles. (For comparison, an unrolled LDA/STA loop would usually take four times as long.)

### Sprite 0 hits

Sprites are conventionally numbered 0 to 63.
Sprite 0 is the sprite controlled by OAM addresses $00-$03, sprite 1 is controlled by $04-$07, ..., and sprite 63 is controlled by $FC-$FF.

While the PPU is drawing the picture, when an opaque pixel of sprite 0 overlaps an opaque pixel of the background, this is a sprite 0 hit.
The PPU detects this condition and sets bit 6 of PPUSTATUS ($2002) to 1 starting at this pixel, letting the CPU know how far along the PPU is in drawing the picture.

Sprite 0 hit does not happen:

- If background or sprite rendering is disabled in PPUMASK ($2001)

- At x=0 to x=7 if the left-side clipping window is enabled (if bit 2 or bit 1 of PPUMASK is 0).

- At x=255, for an obscure reason related to the pixel pipeline.

- At any pixel where the background or sprite pixel is transparent (2-bit color index from the CHR pattern is %00).

- If sprite 0 hit has already occurred this frame. Bit 6 of PPUSTATUS ($2002) is cleared to 0 at dot 1 of the pre-render line. This means only the first sprite 0 hit in a frame can be detected.

Sprite 0 hit happens regardless of the following:

- Sprite priority. Sprite 0 can still hit the background from behind.

- The pixel colors. Only the CHR pattern bits are relevant, not the actual rendered colors, and any CHR color index except %00 is considered opaque.

- The palette. The contents of the palette are irrelevant to sprite 0 hits. For example: a black ($0F) sprite pixel can hit a black ($0F) background as long as neither is the transparent color index %00.

- The PAL PPU blanking on the left and right edges at x=0, x=1, and x=254 (see Overscan).

### Sprite overlapping

Priority between sprites is determined by their address inside OAM.
So to have a sprite displayed in front of another sprite in a scanline, the sprite data that occurs first will overlap any other sprites after it.
For example, when sprites at OAM $0C and $28 overlap, the sprite at $0C will appear in front.

### Internal operation

In addition to the primary OAM memory, the PPU contains 32 bytes (enough for 8 sprites) of secondary OAM memory that is not directly accessible by the program. During each visible scanline this secondary OAM is first cleared (set to all $FF), and then a linear search of the entire primary OAM is carried out to find sprites that are within y range for the next scanline (the sprite evaluation phase). The OAM data for each sprite found to be within range is copied into the secondary OAM, which is then used to initialize eight internal sprite output units.

See PPU rendering for information on precise timing.

The reason sprites at lower addresses in OAM overlap sprites at higher addresses is that sprites at lower addresses also get assigned a lower address in the secondary OAM, and hence get assigned a lower-numbered sprite output unit during the loading phase. Output from lower-numbered sprite output units is wired inside the PPU to take priority over output from higher-numbered sprite output units.

Sprite 0 hit detection relies on the fact that sprite 0, when it is within y range for the next scanline, always gets assigned the first sprite output unit. The hit condition is basically sprite 0 is in range AND the first sprite output unit is outputting a non-zero pixel AND the background drawing unit is outputting a non-zero pixel. (Internally the PPU actually uses two flags: one to keep track of whether sprite 0 occurs on the next scanline, and another one—initialized from the first—to keep track of whether sprite 0 occurs on the current scanline. This is to avoid sprite evaluation, which takes place concurrently with potential sprite 0 hits, trampling on the second flag.)

### Dynamic RAM decay

Because OAM is implemented with dynamic RAM instead of static RAM, the data stored in OAM memory will quickly begin to decay into random bits if it is not being refreshed. The OAM memory is refreshed once per scanline while rendering is enabled (if either the sprite or background bit is enabled via the register at $2001), but on an NTSC PPU this refresh is prevented whenever rendering is disabled.

When rendering is turned off, or during vertical blanking between frames, the OAM memory will hold stable values for a short period before it begins to decay. It will last at least as long as an NTSC vertical blank interval (~1.3ms), but not much longer than this.[2] Because of this, it is not normally useful to write to OAM outside of vertical blank, where rendering is expected to start refreshing its data soon after the write. Writes to $4014 or $2004 should usually be done in an NMI routine, or otherwise within vertical blanking.

If using an advanced technique like forced blanking to manually extend the vertical blank time, it may be necessary to do the OAM DMA last, before enabling rendering mid-frame, to avoid decay.

Because OAM decay is more or less random, and with timing that is sensitive to temperature or other environmental factors, it is not something a game could normally rely on. Most emulators do not simulate the decay, and suffer no compatibility problems as a result. Software developers targeting the NES hardware should be careful not to rely on this.

Because PAL machines have a much longer vertical blanking interval, the 2C07 (PAL PPU) forcibly refreshes OAM during scanlines 265 through 310 (starting 24 scanlines after the start of NMI[3][4][5]), incrementing the OAM address once every 2 pixels (except at pixel 0). This prevents the values in DRAM from decaying in the remaining 46 scanlines before the picture starts and is long enough to allow unmodified NTSC vblank code to run correctly on PAL. As a result of this, software taking advantage of PAL's longer vblank must do OAM DMA early in vblank. In exchange, OAM decay does not occur at all on the PAL NES provided that rendering remains enabled - if rendering is turned off, then OAM can still decay during scanlines 311 and 0-264 (and mid-frame DMA can still be performed if necessary).

## See also

- PPU sprite evaluation

- PPU sprite priority

- Sprite overflow games

## References

- ↑ "OAM" on Breaking NES Wiki. Accessed 2022-04-19.

- ↑ Forum post: Re: Just how cranky is the PPU OAM?

- ↑ Forum post: OAM reading on PAL NES

- ↑ Forum post: PAL NES, sprite evaluation and $2004 reads/writes

- ↑ BreakingNESWiki: PAL circuit analysis


