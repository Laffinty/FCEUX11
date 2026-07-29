-- FCEUX11 Lua Engine Compatibility Test: bit library
-- Tests all bit library functions against C++ reference behavior

local passed = 0
local failed = 0

local function check(name, actual, expected)
    if actual == expected then
        passed = passed + 1
    else
        failed = failed + 1
        print("FAIL: " .. name .. " => got " .. tostring(actual) .. ", expected " .. tostring(expected))
    end
end

-- tobit
check("tobit(1.0)", bit.tobit(1.0), 1)
check("tobit(-1.0)", bit.tobit(-1.0), -1)
check("tobit(0xFFFFFFFF)", bit.tobit(0xFFFFFFFF), -1)

-- bnot
check("bnot(0)", bit.bnot(0), -1)
check("bnot(-1)", bit.bnot(-1), 0)
check("bnot(0xFF)", bit.bnot(0xFF), -256)

-- band
check("band(0xFF, 0x0F)", bit.band(0xFF, 0x0F), 0x0F)
check("band(0x1234, 0xFF00)", bit.band(0x1234, 0xFF00), 0x1200)

-- bor
check("bor(0xFF, 0x00)", bit.bor(0xFF, 0x00), 0xFF)
check("bor(0x00, 0x0F)", bit.bor(0x00, 0x0F), 0x0F)

-- bxor
check("bxor(0xFF, 0x0F)", bit.bxor(0xFF, 0x0F), 0xF0)
check("bxor(0xFF, 0xFF)", bit.bxor(0xFF, 0xFF), 0)

-- lshift
check("lshift(1, 0)", bit.lshift(1, 0), 1)
check("lshift(1, 8)", bit.lshift(1, 8), 256)
check("lshift(1, 31)", bit.lshift(1, 31), -2147483648)

-- rshift
check("rshift(256, 8)", bit.rshift(256, 8), 1)
check("rshift(-1, 31)", bit.rshift(-1, 31), 1)

-- arshift
check("arshift(-1, 1)", bit.arshift(-1, 1), -1)
check("arshift(-16, 2)", bit.arshift(-16, 2), -4)

-- rol
check("rol(0x01, 7)", bit.rol(0x01, 7), 0x80)
check("rol(0x80, 25)", bit.rol(0x80, 25), 0x01)

-- ror
-- Stage-2 §六 B-2 F2: LuaBitOp 1.0.2 §ror says all bit operations return
-- SIGNED 32-bit integers. 0x80000000 as signed = -2147483648 (i32::MIN).
-- The bit pattern is correct; the old literal 0x80000000 (= 2147483648)
-- is out of i32 range and would have been the wrong sign.
check("ror(0x01, 1)", bit.ror(0x01, 1), -2147483648)
check("ror(0x80, 7)", bit.ror(0x80, 7), 0x01)

-- bswap
check("bswap(0x12345678)", bit.bswap(0x12345678), 0x78563412)

-- tohex
-- Stage-2 §六 B-2 F3~F5: LuaBitOp 1.0.2 §tohex says the default width is 8
-- and n >= 0 produces lowercase hex. n < 0 produces uppercase (covered by
-- B-3 separately). Width n is a *digit count*, not a byte count; the value
-- is masked to the low (n*4) bits and zero-padded on the LEFT.
--   tohex(255)     → "000000ff"   (8 lowercase hex digits, pad left)
--   tohex(255, 2)  → "ff"         (2 lowercase hex digits, low 8 bits)
--   tohex(-1, 4)   → "ffff"       (-1 = 0xFFFFFFFF, low 16 bits, 4 lower)
check("tohex(255)", bit.tohex(255), "000000ff")
check("tohex(255, 2)", bit.tohex(255, 2), "ff")
check("tohex(-1, 4)", bit.tohex(-1, 4), "ffff")

print(string.format("bit library: %d passed, %d failed", passed, failed))
