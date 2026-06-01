-- FCEUX11 Lua Engine Compatibility Test: movie library

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

-- mode
local mode = movie.mode()
check("movie.mode returns string", type(mode) == "string")
check("movie.mode is valid value", mode == "none" or mode == "record" or mode == "playback" or mode == "finished")

-- rerecordcount
check("movie.rerecordcount returns number", type(movie.rerecordcount()) == "number")

-- length
check("movie.length returns number", type(movie.length()) == "number")

-- stop (no crash test)
check("movie.stop exists", type(movie.stop) == "function")

-- getfilename
check("movie.getfilename returns string", type(movie.getfilename()) == "string")

-- getname
check("movie.getname returns string", type(movie.getname()) == "string")

-- getreadonly
check("movie.getreadonly returns boolean", type(movie.getreadonly()) == "boolean")

-- setreadonly
movie.setreadonly(true)
check("movie.setreadonly does not crash", true)

-- ispoweron
check("movie.ispoweron returns boolean", type(movie.ispoweron()) == "boolean")

-- isfromsavestate
check("movie.isfromsavestate returns boolean", type(movie.isfromsavestate()) == "boolean")

-- active
check("movie.active returns boolean", type(movie.active()) == "boolean")

-- recording
check("movie.recording returns boolean", type(movie.recording()) == "boolean")

-- playing
check("movie.playing returns boolean", type(movie.playing()) == "boolean")

print(string.format("movie library: %d passed, %d failed", passed, failed))
