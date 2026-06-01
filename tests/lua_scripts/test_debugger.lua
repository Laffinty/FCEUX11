-- FCEUX11 Lua Engine Compatibility Test: debugger library

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

-- hitbreakpoint (existence + no crash)
check("debugger.hitbreakpoint exists", type(debugger.hitbreakpoint) == "function")

-- getcyclescount
local cycles = debugger.getcyclescount()
check("debugger.getcyclescount returns number", type(cycles) == "number")

-- getinstructionscount
local instr = debugger.getinstructionscount()
check("debugger.getinstructionscount returns number", type(instr) == "number")

-- resetcyclescount
debugger.resetcyclescount()
check("debugger.resetcyclescount does not crash", true)

-- resetinstructionscount
debugger.resetinstructionscount()
check("debugger.resetinstructionscount does not crash", true)

-- getsymboloffset
local offset = debugger.getsymboloffset("test")
check("debugger.getsymboloffset returns number", type(offset) == "number")

print(string.format("debugger library: %d passed, %d failed", passed, failed))
