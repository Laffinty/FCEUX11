-- FCEUX11 Lua Engine Compatibility Test: savestate library

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

-- save/load existence
check("savestate.save exists", type(savestate.save) == "function")
check("savestate.load exists", type(savestate.load) == "function")

-- create
local obj = savestate.create()
check("savestate.create returns number", type(obj) == "number")

-- object
check("savestate.object exists", type(savestate.object) == "function")

-- persist (may be stub)
check("savestate.persist exists", type(savestate.persist) == "function")

-- registersave
savestate.registersave(function() end)
check("savestate.registersave does not crash", true)

-- registerload
savestate.registerload(function() end)
check("savestate.registerload does not crash", true)

-- invalid slot test
local ok, err = pcall(savestate.save, 0)
check("savestate.save(0) rejects invalid slot", not ok)

local ok2, err2 = pcall(savestate.save, 11)
check("savestate.save(11) rejects invalid slot", not ok2)

print(string.format("savestate library: %d passed, %d failed", passed, failed))
