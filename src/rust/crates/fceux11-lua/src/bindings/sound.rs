//! `sound` library binding
//!
//! Provides sound/APU state queries: `get`.
//!
//! FFI bridge: reads sound state from C++ `sound.cpp` globals via
//! fceux11_lua_sound_get_* FFI accessors.

use mlua::{Lua, Table, Result};

/// Register the `sound` table into the Lua global namespace
pub fn register(lua: &Lua) -> Result<Table> {
    let sound = lua.create_table()?;

    sound.set(
        "get",
        lua.create_function(|lua, ()| {
            let state = lua.create_table()?;

            // rp2a03 (main APU)
            let rp2a03 = lua.create_table()?;

            // square1
            let square1 = lua.create_table()?;
            square1.set("volume", unsafe { crate::fceux11_lua_sound_get_square1_volume() })?;
            square1.set("frequency", unsafe { crate::fceux11_lua_sound_get_square1_frequency() })?;
            square1.set("midikey", unsafe { crate::fceux11_lua_sound_get_square1_midikey() })?;
            square1.set("duty", unsafe { crate::fceux11_lua_sound_get_square1_duty() })?;
            let square1_regs = lua.create_table()?;
            square1_regs.set("frequency", unsafe { crate::fceux11_lua_sound_get_square1_regs() })?;
            square1.set("regs", square1_regs)?;
            rp2a03.set("square1", square1)?;

            // square2
            let square2 = lua.create_table()?;
            square2.set("volume", unsafe { crate::fceux11_lua_sound_get_square2_volume() })?;
            square2.set("frequency", unsafe { crate::fceux11_lua_sound_get_square2_frequency() })?;
            square2.set("midikey", unsafe { crate::fceux11_lua_sound_get_square2_midikey() })?;
            square2.set("duty", unsafe { crate::fceux11_lua_sound_get_square2_duty() })?;
            let square2_regs = lua.create_table()?;
            square2_regs.set("frequency", unsafe { crate::fceux11_lua_sound_get_square2_regs() })?;
            square2.set("regs", square2_regs)?;
            rp2a03.set("square2", square2)?;

            // triangle
            let triangle = lua.create_table()?;
            triangle.set("volume", unsafe { crate::fceux11_lua_sound_get_triangle_volume() })?;
            triangle.set("frequency", unsafe { crate::fceux11_lua_sound_get_triangle_frequency() })?;
            triangle.set("midikey", unsafe { crate::fceux11_lua_sound_get_triangle_midikey() })?;
            let triangle_regs = lua.create_table()?;
            triangle_regs.set("frequency", unsafe { crate::fceux11_lua_sound_get_triangle_linear() })?;
            triangle.set("regs", triangle_regs)?;
            rp2a03.set("triangle", triangle)?;

            // noise
            let noise = lua.create_table()?;
            noise.set("volume", unsafe { crate::fceux11_lua_sound_get_noise_volume() })?;
            noise.set("frequency", unsafe { crate::fceux11_lua_sound_get_noise_frequency() })?;
            noise.set("midikey", unsafe { crate::fceux11_lua_sound_get_noise_midikey() })?;
            noise.set("short", unsafe { crate::fceux11_lua_sound_get_noise_mode() })?;
            let noise_regs = lua.create_table()?;
            noise_regs.set("frequency", unsafe { crate::fceux11_lua_sound_get_noise_regs() })?;
            noise.set("regs", noise_regs)?;
            rp2a03.set("noise", noise)?;

            // DMC/dpcm
            let dmc = lua.create_table()?;
            dmc.set("volume", unsafe { crate::fceux11_lua_sound_get_dmc_volume() })?;
            dmc.set("frequency", unsafe { crate::fceux11_lua_sound_get_dmc_frequency() })?;
            dmc.set("midikey", unsafe { crate::fceux11_lua_sound_get_dmc_midikey() })?;
            dmc.set("dmcaddress", unsafe { crate::fceux11_lua_sound_get_dmc_address() })?;
            dmc.set("dmcsize", unsafe { crate::fceux11_lua_sound_get_dmc_size() })?;
            dmc.set("dmcloop", unsafe { crate::fceux11_lua_sound_get_dmc_loop() })?;
            dmc.set("dmcseed", unsafe { crate::fceux11_lua_sound_get_dmc_seed() })?;
            let dmc_regs = lua.create_table()?;
            dmc_regs.set("frequency", unsafe { crate::fceux11_lua_sound_get_dmc_regs() })?;
            dmc.set("regs", dmc_regs)?;
            rp2a03.set("dmc", dmc)?;

            state.set("rp2a03", rp2a03)?;

            // Overall info
            state.set("sample_rate", unsafe { crate::fceux11_lua_sound_get_sample_rate() })?;
            state.set("length_count", unsafe { crate::fceux11_lua_sound_get_length_count() })?;

            Ok(state)
        })?,
    )?;

    Ok(sound)
}