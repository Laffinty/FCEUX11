//! `gui` library binding
//!
//! Provides GUI drawing functions: `pixel`, `getpixel`, `line`, `box`, `text`, etc.
//!
//! The pixel buffer (256x240 ARGB) is stored in LuaEngine and blitted to XBuf
//! at each frame boundary via `gui_overlay()`.

use mlua::{Lua, Result, Table};

/// 5x7 pixel font bitmap for ASCII characters 0x20-0x7F (space through ~)
/// Each character is 5 bytes (one per column), bits top-to-bottom (bit 0 = top)
const FONT_DATA: &[u8] = include_bytes!("font5x7.bin");

/// Character glyph width in pixels
const GLYPH_WIDTH: i32 = 5;
/// Character glyph height in pixels
const GLYPH_HEIGHT: i32 = 7;
/// Horizontal character spacing
const CHAR_SPACING: i32 = 1;

/// Lookup offset for ASCII char in FONT_DATA (space = 0x20)
fn char_offset(c: char) -> Option<usize> {
    let code = c as u32;
    if !(0x20..=0x7F).contains(&code) {
        return None;
    }
    Some((code - 0x20) as usize * (GLYPH_WIDTH as usize))
}

/// Draw a single glyph column by column, top to bottom
fn draw_glyph(engine: &mut crate::LuaEngine, x: i32, y: i32, glyph: &[u8], color: u32) {
    // glyph has GLYPH_WIDTH bytes, each bit 0=top
    for (col, &col_bits) in glyph.iter().enumerate() {
        let px = x + col as i32;
        for row in 0..GLYPH_HEIGHT {
            let bit = (col_bits >> row) & 1;
            if bit != 0 {
                let _ = engine.set_pixel(px, y + row, color);
            }
        }
    }
}

/// Register the `gui` table into the Lua global namespace
pub fn register(lua: &Lua) -> Result<Table> {
    let gui = lua.create_table()?;

    // gui.pixel(x, y, color) — set a pixel (0xAARRGGBB format)
    gui.set(
        "pixel",
        lua.create_function(|_, (x, y, color): (i32, i32, Option<u32>)| {
            let engine = crate::get_engine_mut();
            if let Some(eng) = engine {
                let c = color.unwrap_or(0xFFFFFFFF);
                if let Err(e) = eng.set_pixel(x, y, c) {
                    eprintln!("gui.pixel error: {:?}", e);
                }
            }
            Ok(())
        })?,
    )?;

    // gui.getpixel(x, y) — get pixel color at position
    gui.set(
        "getpixel",
        lua.create_function(|_, (x, y): (i32, i32)| -> Result<u32> {
            let engine = crate::get_engine_mut();
            if let Some(eng) = engine {
                let idx = ((y as isize * 256) + x as isize) * 4;
                if idx >= 0 && ((idx + 3) as usize) < eng.gui_data.len() {
                    let rgba = [
                        eng.gui_data[idx as usize],
                        eng.gui_data[idx as usize + 1],
                        eng.gui_data[idx as usize + 2],
                        eng.gui_data[idx as usize + 3],
                    ];
                    return Ok(u32::from_le_bytes(rgba));
                }
            }
            Ok(0)
        })?,
    )?;

    // gui.line(x1, y1, x2, y2, color) — draw a line
    gui.set(
        "line",
        lua.create_function(
            |_, (x1, y1, x2, y2, color): (i32, i32, i32, i32, Option<u32>)| {
                let c = color.unwrap_or(0xFFFFFFFF);
                let engine = crate::get_engine_mut();
                if let Some(eng) = engine {
                    // Bresenham's line algorithm
                    let mut x1 = x1;
                    let mut y1 = y1;
                    let dx = (x2 - x1).abs();
                    let dy = -((y2 - y1).abs());
                    let sx = if x1 < x2 { 1 } else { -1 };
                    let sy = if y1 < y2 { 1 } else { -1 };
                    let mut err = dx + dy;

                    loop {
                        let _ = eng.set_pixel(x1, y1, c);
                        if x1 == x2 && y1 == y2 {
                            break;
                        }
                        let e2 = 2 * err;
                        if e2 >= dy {
                            if x1 == x2 {
                                break;
                            }
                            err += dy;
                            x1 += sx;
                        }
                        if e2 <= dx {
                            if y1 == y2 {
                                break;
                            }
                            err += dx;
                            y1 += sy;
                        }
                    }
                }
                Ok(())
            },
        )?,
    )?;

    // gui.box(x1, y1, x2, y2, color) — draw a rectangle outline
    gui.set(
        "box",
        lua.create_function(
            |_, (x1, y1, x2, y2, color): (i32, i32, i32, i32, Option<u32>)| {
                let c = color.unwrap_or(0xFFFFFFFF);
                let engine = crate::get_engine_mut();
                if let Some(eng) = engine {
                    // Draw 4 lines of the rectangle
                    for x in x1..=x2 {
                        let _ = eng.set_pixel(x, y1, c);
                        let _ = eng.set_pixel(x, y2, c);
                    }
                    for y in y1..=y2 {
                        let _ = eng.set_pixel(x1, y, c);
                        let _ = eng.set_pixel(x2, y, c);
                    }
                }
                Ok(())
            },
        )?,
    )?;

    // gui.text(x, y, msg, color) — draw text using 5x7 pixel font
    gui.set(
        "text",
        lua.create_function(|_, (x, y, msg, color): (i32, i32, String, Option<u32>)| {
            let c = color.unwrap_or(0xFFFFFFFF);
            let engine = crate::get_engine_mut();
            if let Some(eng) = engine {
                let mut cur_x = x;
                for ch in msg.chars() {
                    if ch == '\n' {
                        cur_x = x;
                        continue;
                    }
                    if ch == '\r' {
                        continue;
                    }
                    let offset = match char_offset(ch) {
                        Some(o) => o,
                        None => {
                            // Unknown char, draw a fallback block
                            for bx in 0..GLYPH_WIDTH {
                                for by in 0..GLYPH_HEIGHT {
                                    let _ = eng.set_pixel(cur_x + bx, y + by, c);
                                }
                            }
                            cur_x += GLYPH_WIDTH + CHAR_SPACING;
                            continue;
                        }
                    };

                    // Extract glyph columns from embedded font data
                    let glyph = &FONT_DATA[offset..offset + (GLYPH_WIDTH as usize)];
                    draw_glyph(eng, cur_x, y, glyph, c);
                    cur_x += GLYPH_WIDTH + CHAR_SPACING;
                }
            }
            Ok(())
        })?,
    )?;

    // gui.savescreenshot([filename]) — save screenshot
    gui.set(
        "savescreenshot",
        lua.create_function(|_, filename: Option<mlua::String>| {
            match filename {
                Some(name) => {
                    let bytes = name.as_bytes_with_nul();
                    unsafe { crate::fceux11_lua_gui_savescreenshot(bytes.as_ptr() as *const i8) };
                }
                None => {
                    unsafe { crate::fceux11_lua_gui_savescreenshot(std::ptr::null()) };
                }
            }
            Ok(())
        })?,
    )?;

    // gui.opacity(value) — set overlay opacity (0-255)
    gui.set(
        "opacity",
        lua.create_function(|_, value: i32| {
            let engine = crate::get_engine_mut();
            if let Some(eng) = engine {
                eng.transparency_modifier = value as u8;
            }
            Ok(())
        })?,
    )?;

    // gui.transparency(bool) — enable/disable transparency
    gui.set(
        "transparency",
        lua.create_function(|_, enabled: bool| {
            let engine = crate::get_engine_mut();
            if let Some(eng) = engine {
                eng.transparency_modifier = if enabled { 128 } else { 255 };
            }
            Ok(())
        })?,
    )?;

    // gui.popup(msg) — show a popup message
    gui.set(
        "popup",
        lua.create_function(|_, msg: mlua::String| {
            let bytes = msg.as_bytes_with_nul();
            unsafe { crate::fceux11_lua_gui_popup(bytes.as_ptr() as *const i8) };
            Ok(())
        })?,
    )?;

    // gui.register(func) — register a GUI drawing callback (called each frame)
    gui.set(
        "register",
        lua.create_function(|lua, cb: mlua::Function| {
            let engine = crate::get_engine_mut();
            if let Some(eng) = engine {
                let key = lua.create_registry_value(cb.clone())?;
                eng.callbacks().gui.push(crate::CallbackEntry {
                    func: cb,
                    _key: key,
                });
            }
            Ok(())
        })?,
    )?;

    // gui.clear() — clear the overlay buffer
    gui.set(
        "clear",
        lua.create_function(|_, ()| {
            let engine = crate::get_engine_mut();
            if let Some(eng) = engine {
                for b in &mut eng.gui_data {
                    *b = 0;
                }
            }
            Ok(())
        })?,
    )?;

    Ok(gui)
}
