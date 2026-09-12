<!--
source: https://www.nesdev.org/wiki/PPU_palettes
retrieved: 2026-09-13
conversion: automated from the owner-provided HTML archive; conventions in ../../README.md
-->

# PPU palettes (NESdev Wiki)

> Source: https://www.nesdev.org/wiki/PPU_palettes

The NES has a limited selection of color outputs. A 6-bit value in the palette memory area corresponds to one of 64 outputs. The emphasis bits of the PPUMASK register ($2001) provide an additional color modifier.

For more information on how the colors are generated on an NTSC NES, see: NTSC video. For additional information on how the colors are generated on a PAL NES, see: PAL video.



















## Palette RAM

| | Palette 0 | Palette 1 | Palette 2 | Palette 3 |
|---|---|---|---|---|
| $x0 | $x1 | $x2 | $x3 | $x4 | $x5 | $x6 | $x7 | $x8 | $x9 | $xA | $xB | $xC | $xD | $xE | $xF |
| Background | $3F0x | 0* | 1 | 2 | 3 | 0 | 1 | 2 | 3 | 0 | 1 | 2 | 3 | 0 | 1 | 2 | 3 |
| Sprite | $3F1x | 1 | 2 | 3 | 1 | 2 | 3 | 1 | 2 | 3 | 1 | 2 | 3 |

* Note: Entry 0 of palette 0 is used as the backdrop color.

Backgrounds and sprites each have 4 palettes of 4 colors, located at $3F00-$3F1F in VRAM. Each byte in this palette RAM contains a 6-bit color value referencing one of the PPU's 64 colors. Entry 0 of each palette is unique in that it is transparent, so its color value is normally unused. However, because of the PPU's EXT functionality, entry 0 of palette 0 has the unique behavior of being the backdrop color. The backdrop color is the single color shown behind both the background and sprites, wherever both layers are transparent. Artistically, the backdrop color is usually considered a fourth color of the background and is sometimes called the universal background color.

A single element on the screen can only use a single palette. For backgrounds, this is usually a 16x16 pixel region, but may be as small as an 8x1 pixel sliver with assistance from a cartridge mapper. For sprites, this is a single sprite object, which is 8x8 or 8x16 pixels, depending on the current sprite size. The palette is selected with a 2-bit value referred to as attributes.

Ultimately, the color of a pixel is determined by background vs sprite (which selects the set of 4 palettes), the 2 bits of attributes (which select 1 of those 4 palettes), and the 2 bits of graphics or tile pattern data (which select the color from that palette). These create a 5-bit index into palette RAM:

4bit0
-----
SAAPP
|||||
|||++- Pixel value from tile pattern data
|++--- Palette number from attributes
+----- Background/Sprite select

Note that entry 0 of each palette is also unique in that its color value is shared between the background and sprite palettes, so writing to either one updates the same internal storage. This means that the backdrop color can be written through both $3F00 and $3F10. Palette RAM as a whole is also mirrored through the entire $3F00-$3FFF region.

## Color Value Significance (Hue / Value)

As in some second-generation game consoles, values in the NES palette are based on hue and brightness:

5 bit 0
-------
VV HHHH
|| ||||
|| ++++- Hue (phase, determines NTSC/PAL chroma)
++------ Value (voltage, determines NTSC/PAL luma)

Hue $0 is light gray, $1-$C are blue to red to green to cyan, $D is dark gray, and $E-$F are mirrors of $1D (black).

It works this way because of the way colors are represented in a composite NTSC or PAL signal, with the phase of a color subcarrier controlling the hue. For details regarding signal generation and color decoding, see NTSC video.

The canonical code for "black" is $0F.

The 2C03 RGB PPU used in the PlayChoice-10 and 2C05-99 in the Famicom Titler renders hue $D as black, not dark gray.
The 2C04 PPUs used in many Vs. System arcade games have completely different palettes as a copy protection measure.

## Palettes

The 2C02 (NTSC) and 2C07 (PAL) PPU is used to generate an analog composite video signal. These were used in most home consoles.

The 2C03, 2C04, and 2C05, on the other hand, all output analog red, green, blue, and sync (RGBS) signals. The sync signal contains horizontal and vertical sync pulses in the same format as an all-black composite signal.
Each of the three video channels uses a 3-bit DAC driven by a look-up table in a 64x9-bit ROM inside the PPU.
The look-up tables (one digit for each of red, green, and blue, in order) are given below.

RGB PPUs were used mostly in arcade machines (e.g. Vs. System, Playchoice 10), as well as the Sharp Famicom Titler.

### 2C02

The RF Famicom, AV Famicom, NES (both front- and top-loading), and the North American version of the Sharp Nintendo TV use the 2C02 PPU.
Unlike some other consoles' video circuits, the 2C02 does not generate RGB video and then encode that to composite.
Instead it generates NTSC video directly in the composite domain, decoded by the television receiver into RGB to drive its picture tube.

| | Do not use color $0D. It results in a "blacker than black" signal that may cause problems for some TVs. Further details on $0D and its effects can be found here. |
|---|---|

Most emulators can use a predefined palette, such as one commonly stored in common .pal format, in which each triplet represents the 8bpc sRGB color that results from decoding a large flat area with a given palette value.

The palette seen on real hardware connected to a real television set has at least four sources of variation:

- variation in impedance mismatch between console and television set; this variance leads to different signal reflections with different differential phase distortion, which manifests itself in brighter colors losing saturation and shifting in hue.

- variation in user-adjustable settings of the television set (in particular, hue and saturation);

- variation in how television sets decode composite video into their native RGB display colorspace;

- variation in the native RGB colorspaces of television sets (red/green/blue phosphor chromaticities, color temperature).

The combined effect is that no single composite palette can match the intended display of every possible game title. Nintendo never specified a reference monitor to licensed developers, either. Emulators reproducing a palette or a signal decoding chain must decide on parameters for differential phase distortion, user-adjustable hue and saturation, composite decoding and assumed monitor colorimetry. For composite decoding and assumed monitor colorimetry, the coefficients from published standards such as SMPTE 170M can serve as reasonable defaults.

See this page for more details on a general algorithm to decode a PPU composite signal to color RGB.

Some tools for generating a palette include one by Bisqwit and one by Drag.
These simulate generating a large area of one flat color and then decoding that with the adjustment knobs set to various settings.

The following palette was generated using Pally v0.23.0 with the following arguments:

pally.py --skip-plot -cld -phd 4 -blp 7.5 -e -o docs/NESDev/2C02G_U_wiki.pal

Download:
File:2C02G U wiki.pal

| $00 | $01 | $02 | $03 | $04 | $05 | $06 | $07 | $08 | $09 | $0A | $0B | $0C | $0D | $0E | $0F |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| $10 | $11 | $12 | $13 | $14 | $15 | $16 | $17 | $18 | $19 | $1A | $1B | $1C | $1D | $1E | $1F |
| $20 | $21 | $22 | $23 | $24 | $25 | $26 | $27 | $28 | $29 | $2A | $2B | $2C | $2D | $2E | $2F |
| $30 | $31 | $32 | $33 | $34 | $35 | $36 | $37 | $38 | $39 | $3A | $3B | $3C | $3D | $3E | $3F |

The 2C02 composite output does not output a 7.5 IRE black setup, instead having 0 IRE for black ($0F).

Some TVs or NTSC regions (such as NTSC in Japan) also do not use the 7.5 IRE setup black level, instead directly using the blank level. This results in colors that may appear brighter near the dark row colors.

The following palette was generated using Pally v0.23.0 with the following arguments:

pally.py --skip-plot -cld -phd 4 -e -o docs/NESDev/2C02G_J_wiki.pal

Download:
File:2C02G J wiki.pal

| $00 | $01 | $02 | $03 | $04 | $05 | $06 | $07 | $08 | $09 | $0A | $0B | $0C | $0D | $0E | $0F |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| $10 | $11 | $12 | $13 | $14 | $15 | $16 | $17 | $18 | $19 | $1A | $1B | $1C | $1D | $1E | $1F |
| $20 | $21 | $22 | $23 | $24 | $25 | $26 | $27 | $28 | $29 | $2A | $2B | $2C | $2D | $2E | $2F |
| $30 | $31 | $32 | $33 | $34 | $35 | $36 | $37 | $38 | $39 | $3A | $3B | $3C | $3D | $3E | $3F |

### 2C07

The PAL PPU (2C07) generates a composite PAL video signal, which has a -15 degree hue shift relative to the 2C02 due to a different colorburst reference phase generated by the PPU ($x7 rather than $x8), in addition to the PAL colorburst phase being defined as -U ± 45 degrees.

Like NTSC, the PAL composite signal is affected by differential phase distortion to the same degree, if not slightly worse. Unlike NTSC, PAL composite has a mechanism to correct hue errors by shifting the phase of the colorburst reference on every line; thus the colors on the 2C07 will remain consistent on most PAL sets at the cost of slight further saturation loss.

The following palette was generated using Pally v0.22.1 with the following arguments:

python3 pally.py --skip-plot -cld -ppu "2C07" -phd 4 --delay-line-filter -e -o docs/NESDev/2C07_wiki.pal

Download:
File:2C07 wiki.pal

| $00 | $01 | $02 | $03 | $04 | $05 | $06 | $07 | $08 | $09 | $0A | $0B | $0C | $0D | $0E | $0F |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| $10 | $11 | $12 | $13 | $14 | $15 | $16 | $17 | $18 | $19 | $1A | $1B | $1C | $1D | $1E | $1F |
| $20 | $21 | $22 | $23 | $24 | $25 | $26 | $27 | $28 | $29 | $2A | $2B | $2C | $2D | $2E | $2F |
| $30 | $31 | $32 | $33 | $34 | $35 | $36 | $37 | $38 | $39 | $3A | $3B | $3C | $3D | $3E | $3F |

### 2C03 and 2C05

This palette is intentionally similar to the NES's standard palette, but notably is missing the greys in entries $2D and $3D.
The 2C03 is used in Vs. Duck Hunt, Vs. Tennis, all PlayChoice games, and the Sharp C1 (Famicom TV).
The 2C05 is used in some later Vs. games as a copy protection measure. A variant of the 2C05 without copy protection measures is present in the Sharp Famicom Titler, albeit with adjustments to the output (see below).
Both have historically been used in RGB mods for the NES, as a circuit implementing A0' = A0 xor (A1 nor A2) can swap PPUCTRL and PPUMASK to make a 2C05 behave as a 2C03.

The formula for mapping the DAC integer channel value to 8-bit per channel color is C = 255 * DAC / 7.

Download:
File:2C03 wiki.pal

| | $x0 | $x1 | $x2 | $x3 | $x4 | $x5 | $x6 | $x7 | $x8 | $x9 | $xA | $xB | $xC | $xD | $xE | $xF |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| $0x | 333 | 014 | 006 | 326 | 403 | 503 | 510 | 420 | 320 | 120 | 031 | 040 | 022 | 000 | 000 | 000 |
| $1x | 555 | 036 | 027 | 407 | 507 | 704 | 700 | 630 | 430 | 140 | 040 | 053 | 044 | 000 | 000 | 000 |
| $2x | 777 | 357 | 447 | 637 | 707 | 737 | 740 | 750 | 660 | 360 | 070 | 276 | 077 | 000 | 000 | 000 |
| $3x | 777 | 567 | 657 | 757 | 747 | 755 | 764 | 772 | 773 | 572 | 473 | 276 | 467 | 000 | 000 | 000 |

Note that some of the colors are duplicates: $0B and $1A = 040, $2B and $3B = 276.

Monochrome works the same as the 2C02 (consistent across all RGB PPUs), but unlike the 2C02, emphasis on the RGB chips works differently; rather than "darkening" the specific color chosen, it sets the corresponding channel to full brightness instead.

### 2C05-99

The Sharp Famicom Titler (AN-510) contains a RC2C05-99 PPU, whose RGB output is fed into a X0858CE transcoder chip. This chip handles the conversion from RGBS into Composite, and S-Video. The additional components outside the chip reduce the saturation of the output palette by half with the use of a voltage divider[1], specifically both the R-Y and B-Y color difference channels on pin 11 and pin 12 (U = U * 0.5; V = V * 0.5). This was likely done due to the highly saturated colors of RGB video, especially the 2C05-99's raw RGBS output, which may cause excessive color bleed on composite video.

Otherwise, the raw internal palette of the 2C05-99 PPU itself is believed to be identical to that of the 2C03.

| $00 | $01 | $02 | $03 | $04 | $05 | $06 | $07 | $08 | $09 | $0A | $0B | $0C | $0D | $0E | $0F |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| $10 | $11 | $12 | $13 | $14 | $15 | $16 | $17 | $18 | $19 | $1A | $1B | $1C | $1D | $1E | $1F |
| $20 | $21 | $22 | $23 | $24 | $25 | $26 | $27 | $28 | $29 | $2A | $2B | $2C | $2D | $2E | $2F |
| $30 | $31 | $32 | $33 | $34 | $35 | $36 | $37 | $38 | $39 | $3A | $3B | $3C | $3D | $3E | $3F |

### 2C04

All four 2C04 PPUs contain the same master palette, but in different permutations. It's almost a superset of the 2C03/5 palette, adding four greys, six other colors, and making the bright yellow more pure.

Much like the 2C03 and 2C02, using the greyscale bit in PPUMASK will remap the palette by index, not by color. This means that with the scrambled palettes, each row will remap to the colors in the $0X column for that 2C04 version.

Visualization tool: RGB PPU Palette Converter

No version of the 2C04 was ever made with the below ordering, but it shows the similarity to the 2C03:

| 333 | 014 | 006 | 326 | 403 | 503 | 510 | 420 | 320 | 120 | 031 | 040 | 022 | 111 | 003 | 020 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 555 | 036 | 027 | 407 | 507 | 704 | 700 | 630 | 430 | 140 | 040 | 053 | 044 | 222 | 200 | 310 |
| 777 | 357 | 447 | 637 | 707 | 737 | 740 | 750 | 660 | 360 | 070 | 276 | 077 | 444 | 000 | 000 |
| 777 | 567 | 657 | 757 | 747 | 755 | 764 | 770 | 773 | 572 | 473 | 276 | 467 | 666 | 653 | 760 |

The PPUMASK monochrome bit has the same implementation as on the 2C02, and so it has an unintuitive effect on the 2C04 PPUs; rather than forcing colors to grayscale, it instead forces them to the first column.

#### RP2C04-0001

MAME's source claims that Baseball, Freedom Force, Gradius, Hogan's Alley, Mach Rider, Pinball, and Platoon require this palette.

755,637,700,447,044,120,222,704,777,333,750,503,403,660,320,777
357,653,310,360,467,657,764,027,760,276,000,200,666,444,707,014
003,567,757,070,077,022,053,507,000,420,747,510,407,006,740,000
000,140,555,031,572,326,770,630,020,036,040,111,773,737,430,473

Expand
Palette colors

| | $x0 | $x1 | $x2 | $x3 | $x4 | $x5 | $x6 | $x7 | $x8 | $x9 | $xA | $xB | $xC | $xD | $xE | $xF |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| $0x | 755 | 637 | 700 | 447 | 044 | 120 | 222 | 704 | 777 | 333 | 750 | 503 | 403 | 660 | 320 | 777 |
| $1x | 357 | 653 | 310 | 360 | 467 | 657 | 764 | 027 | 760 | 276 | 000 | 200 | 666 | 444 | 707 | 014 |
| $2x | 003 | 567 | 757 | 070 | 077 | 022 | 053 | 507 | 000 | 420 | 747 | 510 | 407 | 006 | 740 | 000 |
| $3x | 000 | 140 | 555 | 031 | 572 | 326 | 770 | 630 | 020 | 036 | 040 | 111 | 773 | 737 | 430 | 473 |

#### RP2C04-0002

MAME's source claims that Castlevania, Mach Rider (Endurance Course), Raid on Bungeling Bay, Slalom, Soccer, Stroke & Match Golf (both versions), and Wrecking Crew require this palette.

000,750,430,572,473,737,044,567,700,407,773,747,777,637,467,040
020,357,510,666,053,360,200,447,222,707,003,276,657,320,000,326
403,764,740,757,036,310,555,006,507,760,333,120,027,000,660,777
653,111,070,630,022,014,704,140,000,077,420,770,755,503,031,444

Expand
Palette colors

| | $x0 | $x1 | $x2 | $x3 | $x4 | $x5 | $x6 | $x7 | $x8 | $x9 | $xA | $xB | $xC | $xD | $xE | $xF |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| $0x | 000 | 750 | 430 | 572 | 473 | 737 | 044 | 567 | 700 | 407 | 773 | 747 | 777 | 637 | 467 | 040 |
| $1x | 020 | 357 | 510 | 666 | 053 | 360 | 200 | 447 | 222 | 707 | 003 | 276 | 657 | 320 | 000 | 326 |
| $2x | 403 | 764 | 740 | 757 | 036 | 310 | 555 | 006 | 507 | 760 | 333 | 120 | 027 | 000 | 660 | 777 |
| $3x | 653 | 111 | 070 | 630 | 022 | 014 | 704 | 140 | 000 | 077 | 420 | 770 | 755 | 503 | 031 | 444 |

#### RP2C04-0003

MAME's source claims that Balloon Fight, Dr. Mario, Excitebike (US), Goonies, and Soccer require this palette.

507,737,473,555,040,777,567,120,014,000,764,320,704,666,653,467
447,044,503,027,140,430,630,053,333,326,000,006,700,510,747,755
637,020,003,770,111,750,740,777,360,403,357,707,036,444,000,310
077,200,572,757,420,070,660,222,031,000,657,773,407,276,760,022

Expand
Palette colors

| | $x0 | $x1 | $x2 | $x3 | $x4 | $x5 | $x6 | $x7 | $x8 | $x9 | $xA | $xB | $xC | $xD | $xE | $xF |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| $0x | 507 | 737 | 473 | 555 | 040 | 777 | 567 | 120 | 014 | 000 | 764 | 320 | 704 | 666 | 653 | 467 |
| $1x | 447 | 044 | 503 | 027 | 140 | 430 | 630 | 053 | 333 | 326 | 000 | 006 | 700 | 510 | 747 | 755 |
| $2x | 637 | 020 | 003 | 770 | 111 | 750 | 740 | 777 | 360 | 403 | 357 | 707 | 036 | 444 | 000 | 310 |
| $3x | 077 | 200 | 572 | 757 | 420 | 070 | 660 | 222 | 031 | 000 | 657 | 773 | 407 | 276 | 760 | 022 |

#### RP2C04-0004

MAME's source claims that Clu Clu Land, Excitebike (Japan), Ice Climber (both versions), and Super Mario Bros. require this palette.

430,326,044,660,000,755,014,630,555,310,070,003,764,770,040,572
737,200,027,747,000,222,510,740,653,053,447,140,403,000,473,357
503,031,420,006,407,507,333,704,022,666,036,020,111,773,444,707
757,777,320,700,760,276,777,467,000,750,637,567,360,657,077,120

Expand
Palette colors

| | $x0 | $x1 | $x2 | $x3 | $x4 | $x5 | $x6 | $x7 | $x8 | $x9 | $xA | $xB | $xC | $xD | $xE | $xF |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| $0x | 430 | 326 | 044 | 660 | 000 | 755 | 014 | 630 | 555 | 310 | 070 | 003 | 764 | 770 | 040 | 572 |
| $1x | 737 | 200 | 027 | 747 | 000 | 222 | 510 | 740 | 653 | 053 | 447 | 140 | 403 | 000 | 473 | 357 |
| $2x | 503 | 031 | 420 | 006 | 407 | 507 | 333 | 704 | 022 | 666 | 036 | 020 | 111 | 773 | 444 | 707 |
| $3x | 757 | 777 | 320 | 700 | 760 | 276 | 777 | 467 | 000 | 750 | 637 | 567 | 360 | 657 | 077 | 120 |

#### Games compatible with multiple different PPUs

Some games don't require that arcade owners have the correct physical PPU.

At the very least, the following games use some of the DIP switches to support multiple different PPUs:

- Atari RBI Baseball

- Battle City

- Star Luster

- Super Sky Kid

- Super Xevious

- Tetris (Tengen)

- TKO Boxing

#### LUT approach

Emulator authors may implement the 2C04 variants as a LUT indexing the "ordered" palette. This has the added advantage of being able to use preexisting .pal files if the end user wishes to do so.

Repeating colors such as 000 and 777 may index into the same entry of the "ordered" palette, as this is functionally identical.

const unsigned char PaletteLUT_2C04_0001 [64] ={
0x35,0x23,0x16,0x22,0x1C,0x09,0x1D,0x15,0x20,0x00,0x27,0x05,0x04,0x28,0x08,0x20,
0x21,0x3E,0x1F,0x29,0x3C,0x32,0x36,0x12,0x3F,0x2B,0x2E,0x1E,0x3D,0x2D,0x24,0x01,
0x0E,0x31,0x33,0x2A,0x2C,0x0C,0x1B,0x14,0x2E,0x07,0x34,0x06,0x13,0x02,0x26,0x2E,
0x2E,0x19,0x10,0x0A,0x39,0x03,0x37,0x17,0x0F,0x11,0x0B,0x0D,0x38,0x25,0x18,0x3A
};

const unsigned char PaletteLUT_2C04_0002 [64] ={
0x2E,0x27,0x18,0x39,0x3A,0x25,0x1C,0x31,0x16,0x13,0x38,0x34,0x20,0x23,0x3C,0x0B,
0x0F,0x21,0x06,0x3D,0x1B,0x29,0x1E,0x22,0x1D,0x24,0x0E,0x2B,0x32,0x08,0x2E,0x03,
0x04,0x36,0x26,0x33,0x11,0x1F,0x10,0x02,0x14,0x3F,0x00,0x09,0x12,0x2E,0x28,0x20,
0x3E,0x0D,0x2A,0x17,0x0C,0x01,0x15,0x19,0x2E,0x2C,0x07,0x37,0x35,0x05,0x0A,0x2D
};

const unsigned char PaletteLUT_2C04_0003 [64] ={
0x14,0x25,0x3A,0x10,0x0B,0x20,0x31,0x09,0x01,0x2E,0x36,0x08,0x15,0x3D,0x3E,0x3C,
0x22,0x1C,0x05,0x12,0x19,0x18,0x17,0x1B,0x00,0x03,0x2E,0x02,0x16,0x06,0x34,0x35,
0x23,0x0F,0x0E,0x37,0x0D,0x27,0x26,0x20,0x29,0x04,0x21,0x24,0x11,0x2D,0x2E,0x1F,
0x2C,0x1E,0x39,0x33,0x07,0x2A,0x28,0x1D,0x0A,0x2E,0x32,0x38,0x13,0x2B,0x3F,0x0C
};

const unsigned char PaletteLUT_2C04_0004 [64] ={
0x18,0x03,0x1C,0x28,0x2E,0x35,0x01,0x17,0x10,0x1F,0x2A,0x0E,0x36,0x37,0x0B,0x39,
0x25,0x1E,0x12,0x34,0x2E,0x1D,0x06,0x26,0x3E,0x1B,0x22,0x19,0x04,0x2E,0x3A,0x21,
0x05,0x0A,0x07,0x02,0x13,0x14,0x00,0x15,0x0C,0x3D,0x11,0x0F,0x0D,0x38,0x2D,0x24,
0x33,0x20,0x08,0x16,0x3F,0x2B,0x20,0x3C,0x2E,0x27,0x23,0x31,0x29,0x32,0x2C,0x09
};

## Backdrop color

The backdrop color ($3F00) is shown wherever backgrounds and sprites are both transparent, as well as in the border region on NTSC and RGB PPUs. When only one of background or sprite rendering is disabled, the disabled component is treated as transparent. Disabling components in the left column via PPUMASK also treats them as transparent there.

### Backdrop override

When both background and sprite rendering are disabled, this is called forced blank. During forced blank, the PPU normally draws the backdrop color. However, if the current VRAM address in v points into palette RAM ($3F00-$3FFF), then the color at that address will be drawn, instead, overriding the backdrop color. This can allow the CPU to deliberately select colors to draw to the screen, including in the border region, for effects such as color bars or to draw more colors on the screen than is possible during rendering. It can also be used to show the normally-unused transparent colors in $3Fx4/$3Fx8/$3FxC.

More commonly, though, backdrop override results in a glitch where updating palettes outside of vblank causes the palette to be briefly drawn on the screen. Because of this, palette updates should be timed to occur during vblank. Backdrop override can also occur naturally based on scroll position if v points into palette RAM when rendering disables, usually resulting in colors flashing in the 2 scanlines of border at the bottom of the screen. Fortunately, this border flashing is normally hidden by overscan.

Backdrop override is not a deliberate PPU feature, but rather a side effect of how palette RAM is addressed. Palette RAM has just one address input, which is normally the address produced by the rendering hardware. However, when palettes are written by the CPU, it instead needs to use the CPU's target address. The PPU draws whatever color is output from palette RAM, so changing the address to allow CPU access also changes the color drawn to the screen. This is similar to color RAM dots on some Sega consoles, but occurs continuously rather than just during the CPU access.

## Color names

When programmers and artists are communicating, it's often useful to have human-readable names for colors.
Many graphic designers who have done web or game work will be familiar with HTML color names.

### Luma

- $0F: Black

- $00: Dark gray

- $10: Light gray or silver

- $20: White

- $01-$0C: Dark colors, medium mixed with black

- $11-$1C: Medium colors, similar brightness to dark gray

- $21-$2C: Light colors, similar brightness to light gray

- $31-$3C: Pale colors, light mixed with white

### Chroma

Names for hues:

- $x0: Gray

- $x1: Azure

- $x2: Blue

- $x3: Violet

- $x4: Magenta

- $x5: Rose

- $x6: Red or maroon

- $x7: Orange

- $x8: Yellow or olive

- $x9: Chartreuse

- $xA: Green

- $xB: Spring

- $xC: Cyan

### RGBI

These NES colors approximate colors in 16-color RGBI palettes, such as the CGA, EGA, or classic Windows palette, though the NES doesn't really have particularly good approximations:

- $0F: 0/Black

- $02: 1/Navy

- $1A: 2/Green

- $1C: 3/Teal

- $06: 4/Maroon

- $14: 5/Purple

- $18: 6/Olive ($17 for the brown in CGA/EGA in RGB)

- $10: 7/Silver

- $00: 8/Gray

- $12: 9/Blue

- $2A: 10/Lime

- $2C: 11/Aqua/Cyan

- $16: 12/Red

- $24: 13/Fuchsia/Magenta

- $28: 14/Yellow

- $30: 15/White

## See also

- NTSC video - details of the NTSC signal generation and how it produces the palette

- PAL video

- Palettes (gallery) - a few different visualizations of PPU palettes.

- Another palette test - Simple test ROM to display the palette.

- Full palette demo - Demo that displays the entire palette with all emphasis settings at once.

- RGB PPU Palette Converter - RGB PPU palette conversion and visualization tool, written by WhoaMan.

## References

- Re: Various questions about the color palette and emulators - 2012 collection of 2C03, 2C04, 2C05 palettes.

- ↑ Sharp CZ-880 service manual schematic, which uses the same X0858CE transcoder chip and voltage divider resistors on the color-difference I/O pins


