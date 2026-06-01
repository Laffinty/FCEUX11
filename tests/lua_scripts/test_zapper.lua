-- FCEUX11 Lua Engine Compatibility Test: zapper library

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

-- read
local x = zapper.read()
check("zapper.read returns number", type(x) == "number")

-- set
zapper.set(100, 100, 1)
check("zapper.set does not crash", true)

print(string.format("zapper library: %d passed, %d failed", passed, failed))
