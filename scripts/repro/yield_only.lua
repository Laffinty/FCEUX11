-- FCEUX11 --- Lua yield self-proof probe (used by the v2.0.4 "plan-out B" work)
--
-- Purpose: prove whether a yielding Lua script is actually RUNNING in a given
-- host (GUI vs headless runner).  The counter file is rewritten on every
-- iteration, so:
--     file missing      -> the script never started (host-side problem)
--     file == "frames=0" -> the script started but never came back from the
--                           first emu.frameadvance() (yield/resume problem)
--     file == "frames=N" -> the script yielded AND was resumed N times
--
-- The counter path comes from the FCEUX11_YIELD_COUNTER environment variable
-- (os.getenv), so one copy of this script works for any host.
--
-- IMPORTANT (do not "simplify" this): do NOT wrap emu.frameadvance() in
-- pcall() to capture errors.  pcall is a C function: in Lua 5.1 it raises
-- L->nCcalls above L->baseCcalls (ldo.c:447), which makes the yield fail
-- with "attempt to yield across metamethod/C-call boundary" -- an error
-- manufactured by the probe itself, not by the code under test.

local COUNTER = os.getenv("FCEUX11_YIELD_COUNTER") or "fceux11_yield_counter.txt"

local frames = 0

local function stamp(n)
    local ok, f = pcall(io.open, COUNTER, "w")
    if ok and f then
        f:write(string.format("frames=%d\n", n))
        f:close()
    end
end

stamp(0)

while true do
    emu.frameadvance()
    frames = frames + 1
    stamp(frames)
    if frames % 600 == 0 then
        emu.print(string.format("yield_only: %d frames yielded", frames))
    end
end
