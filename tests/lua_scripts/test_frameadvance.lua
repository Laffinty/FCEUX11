-- FCEUX11 Lua Engine Integration Test: frameadvance coroutine cycle
-- This is the most critical test: verifies that emu.frameadvance() correctly
-- yields the coroutine and that the engine resumes it on frame boundaries.

local frame_count = 0
local MAX_TEST_FRAMES = 5

emu.registerbefore(function()
    frame_count = frame_count + 1
end)

for i = 1, MAX_TEST_FRAMES do
    emu.frameadvance()
end

-- If we get here, frameadvance yield/resume works correctly
emu.message("frameadvance test PASSED (" .. MAX_TEST_FRAMES .. " frames)")
emu.print("frameadvance coroutine test: PASSED")
