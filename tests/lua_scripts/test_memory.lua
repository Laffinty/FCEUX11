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

-- readbyterange: returns a string of the requested length (upstream FCEUX
-- semantics: "Get a length bytes starting at the given address and return
-- it as a string")
local r = memory.readbyterange(0x0000, 4)
check("readbyterange returns string", type(r) == "string")
check("readbyterange length", type(r) == "string" and #r == 4)

-- readbyterange length 0: empty string
local r0 = memory.readbyterange(0x0000, 0)
check("readbyterange length 0 is empty string", type(r0) == "string" and #r0 == 0)

-- readbyterange negative length: returns nothing (upstream returns no values)
check("readbyterange negative length is nil", memory.readbyterange(0x0000, -1) == nil)

-- readbyterange past the end of the address space: no clamp in the range
-- function itself, follows GetMem's 16-bit masking — must not crash
local rwrap = memory.readbyterange(0xFFFF, 2)
check("readbyterange at 0xFFFF wraps, no crash", type(rwrap) == "string" and #rwrap == 2)

-- writebyterange + readbyterange round-trip (string in, string out)
local orig2 = memory.readbyterange(0x0100, 4)
local bytes = string.char(0x11, 0x22, 0x33, 0x44)  -- Lua 5.1: no \x escapes
memory.writebyterange(0x0100, bytes)
local rb = memory.readbyterange(0x0100, 4)
check("writebyterange/readbyterange round-trip", rb == bytes)
check("writebyterange byte-level values",
      rb:byte(1) == 0x11 and rb:byte(2) == 0x22 and rb:byte(3) == 0x33 and rb:byte(4) == 0x44)
memory.writebyterange(0x0100, orig2)  -- restore

-- writebyterange empty string: no-op, must not crash
memory.writebyterange(0x0100, "")
check("writebyterange empty string", true)

-- writebyterange out of range (>= 0x8000 PRG ROM): silently ignored like
-- writebyte (BWrite only handles < 0x8000) — value must be unchanged
local rom0 = memory.readbyte(0x8000)
memory.writebyterange(0x8000, string.char(0xAA, 0xBB))
check("writebyterange to ROM ignored", memory.readbyte(0x8000) == rom0)

-- writebyterange past the end of the address space: wraps like the read
-- side, must not crash
memory.writebyterange(0xFFFF, string.char(0x5A))
check("writebyterange at 0xFFFF wraps, no crash", true)

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
