use std::os::raw::c_int;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pal {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

// ---------------------------------------------------------------------------
// NTSC palette generation (translated from C++ CalculatePalette)
// ---------------------------------------------------------------------------

static COLS: [u8; 16] = [0, 24, 21, 18, 15, 12, 9, 6, 3, 0, 33, 30, 27, 0, 0, 0];
static BR1: [u8; 4] = [6, 9, 12, 12];
static BR2: [f64; 4] = [0.29, 0.45, 0.73, 0.9];
static BR3: [f64; 4] = [0.0, 0.24, 0.47, 0.77];

/// Compute the 64-entry NTSC base palette from tint and hue.
/// `out` must point to at least 64 `Pal` entries.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_palette_calc_ntsc(tint: c_int, hue: c_int, out: *mut Pal) {
    if out.is_null() {
        return;
    }
    let out = unsafe { std::slice::from_raw_parts_mut(out, 64) };
    let s_base = tint as f64 / 128.0;

    for x in 0..=3 {
        for z in 0..16 {
            let mut s = s_base;
            let mut luma = BR2[x];
            if z == 0 {
                s = 0.0;
                luma = BR1[x] as f64 / 12.0;
            }
            if z >= 13 {
                s = 0.0;
                luma = 0.0;
                if z == 13 {
                    luma = BR3[x];
                }
            }

            let theta = std::f64::consts::PI
                * ((COLS[z] as f64 * 10.0 + (hue as f64 / 2.0) + 300.0) / 180.0);
            let r = ((luma + s * theta.sin()) * 256.0) as i32;
            let g = ((luma - (27.0 / 53.0) * s * theta.sin() + (10.0 / 53.0) * s * theta.cos())
                * 256.0) as i32;
            let b = ((luma - s * theta.cos()) * 256.0) as i32;

            let idx = (x << 4) + z;
            out[idx].r = r.clamp(0, 255) as u8;
            out[idx].g = g.clamp(0, 255) as u8;
            out[idx].b = b.clamp(0, 255) as u8;
        }
    }
}

// ---------------------------------------------------------------------------
// Bisqwit de-emphasis (translated from C++ ApplyDeemphasisBisqwit)
// ---------------------------------------------------------------------------

fn bisqwit_wave(p: i32, color: i32) -> usize {
    ((color + p + 8) % 12 < 6) as usize
}

fn bisqwit_gammafix(f: f32, gamma: f32) -> f32 {
    if f < 0.0 {
        0.0
    } else {
        f.powf(2.2 / gamma)
    }
}

fn bisqwit_clamp(v: i32) -> i32 {
    v.clamp(0, 255)
}

fn apply_deemphasis_bisqwit(entry: i32, r: &mut u8, g: &mut u8, b: &mut u8) {
    if entry < 64 {
        return;
    }

    let color = entry & 0x0F;
    let level = if color < 0xE { (entry >> 4) & 3 } else { 1 };

    const BLACK: f32 = 0.518;
    const WHITE: f32 = 1.962;
    const ATTENUATION: f32 = 0.746;
    const LEVELS: [f32; 8] = [
        0.350, 0.518, 0.962, 1.550, 1.094, 1.506, 1.962, 1.962,
    ];

    let lo_and_hi = [
        LEVELS[(level + 4 * (color == 0x0) as i32) as usize],
        LEVELS[(level + 4 * (color < 0xD) as i32) as usize],
    ];

    let mut myr = 0i32;
    let mut myg = 0i32;
    let mut myb = 0i32;

    for pass in 0..2 {
        let mut y = 0.0f32;
        let mut i = 0.0f32;
        let mut q = 0.0f32;
        let gamma = 1.8f32;

        for p in 0..12 {
            let mut spot = lo_and_hi[bisqwit_wave(p, color)];
            if pass == 1 {
                if ((entry & 0x40) != 0 && bisqwit_wave(p, 12) != 0)
                    || ((entry & 0x80) != 0 && bisqwit_wave(p, 4) != 0)
                    || ((entry & 0x100) != 0 && bisqwit_wave(p, 8) != 0)
                {
                    spot *= ATTENUATION;
                }
            }
            let v = (spot - BLACK) / (WHITE - BLACK) / 12.0;
            y += v;
            i += v * (std::f32::consts::PI * p as f32 / 6.0).cos();
            q += v * (std::f32::consts::PI * p as f32 / 6.0).sin();
        }

        let rt = bisqwit_clamp(
            (255.0 * bisqwit_gammafix(y + 0.946882 * i + 0.623557 * q, gamma)) as i32,
        );
        let gt = bisqwit_clamp(
            (255.0 * bisqwit_gammafix(y - 0.274788 * i - 0.635691 * q, gamma)) as i32,
        );
        let bt = bisqwit_clamp(
            (255.0 * bisqwit_gammafix(y - 1.108545 * i + 1.709007 * q, gamma)) as i32,
        );

        if pass == 0 {
            myr = rt;
            myg = gt;
            myb = bt;
        } else {
            if myr != 0 {
                let rscale = rt as f32 / myr as f32;
                *r = (*r as f32 * rscale).clamp(0.0, 255.0) as u8;
            }
            if myg != 0 {
                let gscale = gt as f32 / myg as f32;
                *g = (*g as f32 * gscale).clamp(0.0, 255.0) as u8;
            }
            if myb != 0 {
                let bscale = bt as f32 / myb as f32;
                *b = (*b as f32 * bscale).clamp(0.0, 255.0) as u8;
            }
        }
    }
}

/// Expand a 64-entry base palette into 512 entries with de-emphasis applied.
/// `src` must point to 64 `Pal` entries.
/// `dst` must point to 512 `Pal` entries.
/// It is safe for `src == dst` (in-place expansion).
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_palette_apply_deemphasis(src: *const Pal, dst: *mut Pal) {
    if src.is_null() || dst.is_null() {
        return;
    }
    let src = unsafe { std::slice::from_raw_parts(src, 64) };
    let dst = unsafe { std::slice::from_raw_parts_mut(dst, 512) };

    // copy base 64 entries first so that in-place expansion works safely
    if src.as_ptr() != dst.as_mut_ptr() {
        dst[..64].copy_from_slice(src);
    }

    for i in 1..8 {
        for p in 0..64 {
            let idx = i * 64 + p;
            dst[idx] = dst[p];
            apply_deemphasis_bisqwit(
                idx as i32,
                &mut dst[idx].r,
                &mut dst[idx].g,
                &mut dst[idx].b,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Grayscale conversion (extracted from C++ ChoosePalette)
// ---------------------------------------------------------------------------

/// Convert a 512-entry palette to grayscale.
/// `src` and `dst` must each point to 512 `Pal` entries.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_palette_make_grayscale(src: *const Pal, dst: *mut Pal) {
    if src.is_null() || dst.is_null() {
        return;
    }
    let src = unsafe { std::slice::from_raw_parts(src, 512) };
    let dst = unsafe { std::slice::from_raw_parts_mut(dst, 512) };

    for x in 0..512 {
        let gray = (src[x].r as f32 * 0.299
            + src[x].g as f32 * 0.587
            + src[x].b as f32 * 0.114) as u8;
        dst[x].r = gray;
        dst[x].g = gray;
        dst[x].b = gray;
    }
}

// ---------------------------------------------------------------------------
// NTSC control bars pixel drawing (extracted from FCEU_DrawNTSCControlBars)
// ---------------------------------------------------------------------------

/// Draw the NTSC tint/hue control bars into a pixel buffer.
/// `xbuf` must be a non-null pointer to an 8-bit indexed pixel buffer.
/// `width` is the horizontal stride (typically 256).
/// `which` is the bar length in pixels (typically ntschue*2 or ntsctint*2).
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_palette_draw_control_bars(
    xbuf: *mut u8,
    width: c_int,
    which: c_int,
) {
    if xbuf.is_null() || width <= 0 {
        return;
    }
    unsafe {
        let xbaf = xbuf.add((200 * width) as usize);
        for x in (0..which).step_by(2) {
            for x2 in -6..=6 {
                let offset = x as isize - (width as isize) * x2;
                *xbaf.offset(offset) = 0x85;
            }
        }
        for x in (which..256).step_by(2) {
            for x2 in -2..=2 {
                let offset = x as isize - (width as isize) * x2;
                *xbaf.offset(offset) = 0x85;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calc_ntsc_range() {
        let mut buf = [Pal { r: 0, g: 0, b: 0 }; 64];
        fceux11_rust_palette_calc_ntsc(56, 72, buf.as_mut_ptr());
        for p in &buf {
            assert!(
                p.r <= 255 && p.g <= 255 && p.b <= 255,
                "calc_ntsc produced out-of-range value"
            );
        }
    }

    #[test]
    fn test_calc_ntsc_null_safe() {
        fceux11_rust_palette_calc_ntsc(56, 72, std::ptr::null_mut());
    }

    #[test]
    fn test_apply_deemphasis_identity() {
        let mut src = [Pal { r: 0, g: 0, b: 0 }; 64];
        for i in 0..64 {
            src[i].r = i as u8;
            src[i].g = (i * 2) as u8;
            src[i].b = (i * 3) as u8;
        }
        let mut dst = [Pal { r: 0, g: 0, b: 0 }; 512];
        fceux11_rust_palette_apply_deemphasis(src.as_ptr(), dst.as_mut_ptr());

        // entries 0..63 should be identical to source (deemph 0 = identity)
        for i in 0..64 {
            assert_eq!(dst[i], src[i], "base palette entry {} was modified", i);
        }

        // all entries should be in valid range
        for p in &dst {
            assert!(
                p.r <= 255 && p.g <= 255 && p.b <= 255,
                "apply_deemphasis produced out-of-range value"
            );
        }
    }

    #[test]
    fn test_apply_deemphasis_in_place() {
        let mut buf = [Pal { r: 0, g: 0, b: 0 }; 512];
        for i in 0..64 {
            buf[i].r = i as u8;
            buf[i].g = (i * 2) as u8;
            buf[i].b = (i * 3) as u8;
        }
        let original = buf;
        fceux11_rust_palette_apply_deemphasis(buf.as_ptr(), buf.as_mut_ptr());

        // base 64 should be unchanged
        for i in 0..64 {
            assert_eq!(buf[i], original[i]);
        }
        // deemph entries should exist and be in range
        for i in 64..512 {
            assert!(
                buf[i].r <= 255 && buf[i].g <= 255 && buf[i].b <= 255,
                "in-place deemph entry {} out of range",
                i
            );
        }
    }

    #[test]
    fn test_apply_deemphasis_null_safe() {
        fceux11_rust_palette_apply_deemphasis(std::ptr::null(), std::ptr::null_mut());
    }

    #[test]
    fn test_make_grayscale() {
        let mut src = [Pal { r: 0, g: 0, b: 0 }; 512];
        for i in 0..512 {
            src[i].r = (i & 0xFF) as u8;
            src[i].g = ((i * 3) & 0xFF) as u8;
            src[i].b = ((i * 5) & 0xFF) as u8;
        }
        let mut dst = [Pal { r: 0, g: 0, b: 0 }; 512];
        fceux11_rust_palette_make_grayscale(src.as_ptr(), dst.as_mut_ptr());

        for p in &dst {
            assert_eq!(p.r, p.g);
            assert_eq!(p.g, p.b);
            assert!(p.r <= 255);
        }
    }

    #[test]
    fn test_make_grayscale_null_safe() {
        fceux11_rust_palette_make_grayscale(std::ptr::null(), std::ptr::null_mut());
    }

    #[test]
    fn test_draw_control_bars() {
        let mut buf = [0u8; 256 * 240];
        fceux11_rust_palette_draw_control_bars(buf.as_mut_ptr(), 256, 100);

        // Check that something was drawn around row 200
        let mut has_pixels = false;
        for y in 194..=206 {
            for x in 0..256 {
                if buf[y * 256 + x] == 0x85 {
                    has_pixels = true;
                }
            }
        }
        assert!(has_pixels, "control bars should have drawn pixels");
    }

    #[test]
    fn test_draw_control_bars_null_safe() {
        fceux11_rust_palette_draw_control_bars(std::ptr::null_mut(), 256, 100);
        fceux11_rust_palette_draw_control_bars(
            [0u8; 256].as_mut_ptr(),
            0, // invalid width
            100,
        );
    }

    #[test]
    fn test_calc_ntsc_determinism() {
        let mut a = [Pal { r: 0, g: 0, b: 0 }; 64];
        let mut b = [Pal { r: 0, g: 0, b: 0 }; 64];
        fceux11_rust_palette_calc_ntsc(46 + 10, 72, a.as_mut_ptr());
        fceux11_rust_palette_calc_ntsc(46 + 10, 72, b.as_mut_ptr());
        assert_eq!(a, b);
    }
}
