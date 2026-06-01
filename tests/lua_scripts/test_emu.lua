-- FCEUX11 Lua Engine Compatibility Test: emu library
-- Tests all emu library functions

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

-- framecount: should return a non-negative integer
local fc = emu.framecount()
check("framecount returns number", type(fc) == "number")
check("framecount >= 0", fc >= 0)

-- lagcount
local lc = emu.lagcount()
check("lagcount returns number", type(lc) == "number")

-- lagged
check("lagged returns boolean", type(emu.lagged()) == "boolean")

-- emulating
check("emulating returns true", emu.emulating() == true)

-- paused
check("paused returns boolean", type(emu.paused()) == "boolean")

-- message (no crash test)
emu.message("Rust Lua engine test message")
check("message does not crash", true)

-- print (no crash test)
emu.print("emu.print test", 42, true)
check("print does not crash", true)

-- speedmode (no crash test)
emu.speedmode("normal")
check("speedmode('normal') does not crash", true)

-- frameadvance (basic yield test - this should yield the coroutine)
-- We test that frameadvance doesn't error immediately
check("frameadvance exists", type(emu.frameadvance) == "function")

-- registerbefore/registerafter/registerexit
local before_called = false
local after_called = false
local exit_called = false

emu.registerbefore(function()
    before_called = true
end)
check("registerbefore does not crash", true)

emu.registerafter(function()
    after_called = true
end)
check("registerafter does not crash", true)

emu.registerexit(function()
    exit_called = true
end)
check("registerexit does not crash", true)

-- poweron/softreset (existence check - calling these may reset state)
check("poweron exists", type(emu.poweron) == "function")
check("softreset exists", type(emu.softreset) == "function")

-- pause/unpause (existence check)
check("pause exists", type(emu.pause) == "function")
check("unpause exists", type(emu.unpause) == "function")

print(string.format("emu library: %d passed, %d failed", passed, failed))
