-- FCEUX11 --- minimal Lua yield/resume probe (v2.0.4 "plan-out B" work)
--
-- Answers ONE question: does the script survive its first emu.frameadvance()
-- and get resumed by the host?  It writes one line per stage to the file named
-- by FCEUX11_YIELD_DIAG (default: fceux11_yield_diag.txt).
--
-- Stage lines:
--   A: before first frameadvance
--   B: after first frameadvance      <- B present => yield worked AND the host resumed us
--   C: after second frameadvance
--   D: after third frameadvance
--
-- If only "A" appears, the script died inside the first emu.frameadvance().
-- Do NOT wrap the call in pcall(): pcall is a C function and would itself
-- break the yield in Lua 5.1 (ldo.c:447, nCcalls > baseCcalls).

local LOG = os.getenv("FCEUX11_YIELD_DIAG") or "fceux11_yield_diag.txt"

local function w(msg)
    local f = io.open(LOG, "a")
    if f then
        f:write(msg .. "\n")
        f:close()
    end
end

w("A: before first frameadvance")
emu.frameadvance()
w("B: after first frameadvance")
emu.frameadvance()
w("C: after second frameadvance")
emu.frameadvance()
w("D: after third frameadvance")
