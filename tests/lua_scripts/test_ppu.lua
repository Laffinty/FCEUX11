-- FCEUX11 Lua Engine Compatibility Test: ppu library

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

-- readbyte
local val = ppu.readbyte(0x2000)
check("ppu.readbyte returns number", type(val) == "number")
check("ppu.readbyte in range [0,255]", val >= 0 and val <= 255)

-- readbyterange
local range = ppu.readbyterange(0x2000, 8)
check("ppu.readbyterange returns table", type(range) == "table")
check("ppu.readbyterange length == 8", #range == 8)

print(string.format("ppu library: %d passed, %d failed", passed, failed))
