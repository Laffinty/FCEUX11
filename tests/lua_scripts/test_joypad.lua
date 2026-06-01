-- FCEUX11 Lua Engine Compatibility Test: joypad library

local passed = 0
local failed = 0

local function check(name, cond)
    if cond then
        passed = passed + 1
    else
        failed = failed + 1
        print("FAIL: " .. name)
    end
end

-- get returns a number
local state = joypad.get(1)
check("joypad.get returns number", type(state) == "number")

-- set with button table
joypad.set(1, {A=true, B=false})
check("joypad.set does not crash", true)

-- set with multiple buttons
joypad.set(1, {up=true, start=true})
check("joypad.set multiple buttons does not crash", true)

-- set to clear override
joypad.set(1, {})
check("joypad.set({}) does not crash", true)

print(string.format("joypad library: %d passed, %d failed", passed, failed))
