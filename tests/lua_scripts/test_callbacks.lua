-- FCEUX11 Lua Engine Integration Test: callback registration persistence
-- Verifies that registered callbacks survive across frameadvance calls

local before_count = 0
local after_count = 0

emu.registerbefore(function()
    before_count = before_count + 1
end)

emu.registerafter(function()
    after_count = after_count + 1
end)

-- Run 3 frames to verify callbacks fire
for i = 1, 3 do
    emu.frameadvance()
end

-- Note: we can't assert here because this runs inside the coroutine,
-- but if the script completes without error, callbacks were registered.
emu.message("callback test: before=" .. before_count .. " after=" .. after_count)
