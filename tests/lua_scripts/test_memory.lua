-- FCEUX11 Lua Engine Compatibility Test: memory library
-- Tests all memory library functions

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

-- readbyte: should return a number in [0, 255]
local val = memory.readbyte(0x0000)
check("readbyte returns number", type(val) == "number")
check("readbyte in range [0,255]", val >= 0 and val <= 255)

-- readbytesigned
local sval = memory.readbytesigned(0x0000)
check("readbytesigned returns number", type(sval) == "number")
check("readbytesigned in range [-128,127]", sval >= -128 and sval <= 127)

-- readword
local wval = memory.readword(0x0000)
check("readword returns number", type(wval) == "number")
check("readword in range [0,65535]", wval >= 0 and wval <= 65535)

-- readwordsigned
local wsval = memory.readwordsigned(0x0000)
check("readwordsigned returns number", type(wsval) == "number")

-- writebyte + readbyte round-trip
local orig = memory.readbyte(0x0000)
memory.writebyte(0x0000, 0x42)
local readback = memory.readbyte(0x0000)
check("writebyte/readbyte round-trip", readback == 0x42)
memory.writebyte(0x0000, orig)  -- restore

-- getregister
local pc = memory.getregister("pc")
check("getregister('pc') returns number", type(pc) == "number")
local a = memory.getregister("a")
check("getregister('a') returns number", type(a) == "number")
local x = memory.getregister("x")
check("getregister('x') returns number", type(x) == "number")
local y = memory.getregister("y")
check("getregister('y') returns number", type(y) == "number")
local s = memory.getregister("s")
check("getregister('s') returns number", type(s) == "number")
local p = memory.getregister("p")
check("getregister('p') returns number", type(p) == "number")

-- registerwrite/registerread/registerexec (no crash test)
memory.registerwrite(0x0000, function() end)
check("registerwrite does not crash", true)

memory.registerread(0x0000, function() end)
check("registerread does not crash", true)

memory.registerexec(0x0000, function() end)
check("registerexec does not crash", true)

print(string.format("memory library: %d passed, %d failed", passed, failed))
