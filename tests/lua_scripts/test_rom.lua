-- FCEUX11 Lua Engine Compatibility Test: rom library

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

-- gethash
local hash = rom.gethash()
check("rom.gethash returns string", type(hash) == "string")
check("rom.gethash is hex string", hash:match("^[0-9a-f]+$") ~= nil)

-- readbyte
local val = rom.readbyte(0)
check("rom.readbyte returns number", type(val) == "number")
check("rom.readbyte in range [0,255]", val >= 0 and val <= 255)

-- writebyte (existence check - modifying ROM may cause issues)
check("rom.writebyte exists", type(rom.writebyte) == "function")

print(string.format("rom library: %d passed, %d failed", passed, failed))
