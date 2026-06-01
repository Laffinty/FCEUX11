-- FCEUX11 Lua Engine Compatibility Test: input library

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

-- input.get (may return empty table if stub)
local inp = input.get()
check("input.get returns table", type(inp) == "table")

-- input.popup (stub may just print)
check("input.popup exists", type(input.popup) == "function")

-- input.openfilepopup
check("input.openfilepopup exists", type(input.openfilepopup) == "function")

-- input.savefilepopup
check("input.savefilepopup exists", type(input.savefilepopup) == "function")

print(string.format("input library: %d passed, %d failed", passed, failed))
