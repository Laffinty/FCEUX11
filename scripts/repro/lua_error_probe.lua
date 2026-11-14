-- A script that raises a real Lua error on purpose.
-- Used as the negative leg for the f11qa lua runner: the runner must report
-- FAIL (non-zero exit) when the script dies with a Lua error.

local ok, err = pcall(function()
    local t = nil
    return t.field -- nil index -> Lua runtime error
end)

print("caught: " .. tostring(err))
error("intentional failure for the negative leg")
