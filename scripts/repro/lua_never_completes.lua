-- A Lua script that yields forever (never completes on its own).
--
-- Used as the negative leg for the f11qa lua runner: run WITHOUT a ROM, the
-- runner's emulation step fails on the first iteration.  Before v2.0.4 r15
-- the runner broke out of its frame loop silently there and reported
-- "script completed (all assertions passed)" -- i.e. a script that never ran a
-- single frame was judged PASS.  The runner must now exit non-zero.
--
-- It also never finishes by itself, so the frame-budget timeout applies.

while true do
    emu.frameadvance()
end
