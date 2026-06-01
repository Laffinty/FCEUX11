-- FCEUX11 Lua Engine Integration Test: TAS-style script
-- Simulates a typical TAS automation script pattern

local frames = 0
local MAX_FRAMES = 10

-- Register a before-frame callback for HUD
emu.registerbefore(function()
    gui.text(10, 10, "Frame: " .. emu.framecount(), 0xFFFFFF)
end)

-- Main loop: typical TAS pattern
while frames < MAX_FRAMES do
    -- Read memory
    local pc = memory.getregister("pc")
    local hp = memory.readbyte(0x0060)

    -- Draw HUD
    gui.text(10, 25, "PC: " .. string.format("%04X", pc), 0x00FF00)
    gui.text(10, 35, "HP: " .. hp, 0xFFFF00)

    -- Set joypad
    joypad.set(1, {A=true, right=true})

    -- Advance frame
    emu.frameadvance()
    frames = frames + 1
end

emu.message("TAS simulation test PASSED")
