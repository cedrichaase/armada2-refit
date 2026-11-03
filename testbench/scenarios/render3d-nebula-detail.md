# render3d: the nebulae at a graphics detail below the top

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Clock: bench
Setup: testbench/bench-detail.sh 2 render3d/bench-ini.sh World:Nebulae=0 nebula-detail Seed=7 Step=16 Pause=200
Timeout: 12 min

Run with `--install .`. Six nebula classes from above (`nebula-detail`: mnebula6 to 10
and mnebula12) at Graphics Detail 2 of 4 (`testbench/bench-detail.sh`), with
`[World] Nebulae=0`, so the engine draws their puffs and Display8's passthrough draws
them. Below the top detail the engine asks for 16-bit texture formats first
(`display/README.md`, "Texture formats"); until display 1.0.2 the stub said yes, no
mirror held them, and every puff was drawn with a white texture: hard-edged flat
squares in each class's colour. Now the stub refuses them and the engine creates its
textures 32-bit at every detail: the puffs are soft, as at detail 4, Display8.log
records the refusals and nothing drawn white, and the overlays find the fog's texture
mirrored.

For the gas (every module on) at the same detail, drop `World:Nebulae=0` from the
`Setup:`; on Direct3D 12, `display/stub-scenario.sh render3d-nebula-detail stub
Backend=d3d12`.

1. Launch the game.
2. Wait up to 300 seconds for "Scene.log" to say "paused at tick".
3. Wait 8 seconds.
4. Take a screenshot called "nebulae".
5. Expect "Display8.log" contains "CheckDeviceFormat(0, 1, 22, 0x0, 3, 23) -> 0x8876086a".
6. Expect "Display8.log" not contain "drawn with a white one".
7. Expect "Render3D.log" not contain "has no Display8 mirror".
8. Expect the game is still running.
9. Expect no crash.
