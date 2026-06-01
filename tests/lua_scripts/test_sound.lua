-- FCEUX11 Lua Engine Compatibility Test: sound library

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

-- sound.get
local s = sound.get()
check("sound.get returns table", type(s) == "table")
check("sound.get has rp2a03", type(s.rp2a03) == "table")

-- square1
check("rp2a03 has square1", type(s.rp2a03.square1) == "table")
check("square1 has volume", type(s.rp2a03.square1.volume) == "number")
check("square1 has frequency", type(s.rp2a03.square1.frequency) == "number")
check("square1 has midikey", type(s.rp2a03.square1.midikey) == "number")
check("square1 has duty", type(s.rp2a03.square1.duty) == "number")
check("square1 has regs", type(s.rp2a03.square1.regs) == "table")

-- square2
check("rp2a03 has square2", type(s.rp2a03.square2) == "table")
check("square2 has volume", type(s.rp2a03.square2.volume) == "number")

-- triangle
check("rp2a03 has triangle", type(s.rp2a03.triangle) == "table")
check("triangle has volume", type(s.rp2a03.triangle.volume) == "number")

-- noise
check("rp2a03 has noise", type(s.rp2a03.noise) == "table")
check("noise has volume", type(s.rp2a03.noise.volume) == "number")

-- dmc
check("rp2a03 has dmc", type(s.rp2a03.dmc) == "table")
check("dmc has volume", type(s.rp2a03.dmc.volume) == "number")

-- top-level
check("sound.get has sample_rate", type(s.sample_rate) == "number")
check("sound.get has length_count", type(s.length_count) == "number")

print(string.format("sound library: %d passed, %d failed", passed, failed))
