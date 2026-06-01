-- FCEUX11 Lua Engine Compatibility Test: gui library

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

-- pixel
gui.pixel(0, 0, 0xFF0000)
check("gui.pixel does not crash", true)

gui.pixel(255, 239, 0x00FF00)
check("gui.pixel boundary does not crash", true)

-- out-of-bounds pixel should be silently ignored
gui.pixel(-1, -1, 0xFFFFFF)
gui.pixel(256, 240, 0xFFFFFF)
check("gui.pixel OOB does not crash", true)

-- getpixel
local c = gui.getpixel(0, 0)
check("gui.getpixel returns number", type(c) == "number")

-- line
gui.line(0, 0, 100, 100, 0xFFFFFF)
check("gui.line does not crash", true)

-- box
gui.box(10, 10, 50, 50, 0xFF0000)
check("gui.box does not crash", true)

-- text
gui.text(10, 10, "Hello FCEUX11", 0xFFFFFF)
check("gui.text does not crash", true)

gui.text(10, 20, "Line2\nLine3", 0x00FF00)
check("gui.text with newline does not crash", true)

-- opacity
gui.opacity(128)
check("gui.opacity does not crash", true)

-- transparency
gui.transparency(true)
check("gui.transparency(true) does not crash", true)
gui.transparency(false)
check("gui.transparency(false) does not crash", true)

-- clear
gui.clear()
check("gui.clear does not crash", true)

-- register
gui.register(function()
    gui.pixel(0, 0, 0xFF0000)
end)
check("gui.register does not crash", true)

-- popup (no crash test only)
check("gui.popup exists", type(gui.popup) == "function")

-- savescreenshot (existence check)
check("gui.savescreenshot exists", type(gui.savescreenshot) == "function")

print(string.format("gui library: %d passed, %d failed", passed, failed))
