# Block-compressed textures on the stub: DXT1, DXT3 and DXT5 made, filled and drawn by Display8

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Clock: bench
Setup: display/bench-mode.sh stub Passthrough=1 BenchTextures=300 render3d/bench-ini.sh Modules:hulls=0 showcase Seed=7 Step=16 Pause=200 Race=federation Hud=1
Timeout: 15 min

Run with `--install .`. What Fleet Operations: Roots asks of the Direct3D 8 device, tried on
the stock game (`display/README.md`, "Block-compressed textures"): from the 300th `Present`
on, Display8's bench-only `BenchTextures` hook makes a 64x64 texture of each of DXT1, DXT3 and
DXT5 with a whole chain (seven levels, down to the 1x1 one), through `CreateTexture`, fills
every level through `LockRect` (the pitch it is given must be a row of blocks) and draws
them with `DrawPrimitiveUP` through the passthrough, over the paused showcase frame. A panel
at the top left shows, for each format, a 160x160 quad (level 0: a checker of red and blue
blocks; DXT3 and DXT5 with a ramp of alpha across each block over the panel) and five
minified quads of 32, 16, 8, 4 and 2 pixels below it, which sample the flat-coloured lower
levels (green, yellow, cyan, magenta, white). The check is the screenshot, by eye and by
crop; no baseline is frozen.

1. Launch the game.
2. Wait up to 600 seconds for "Scene.log" to say "paused at tick".
3. Wait 5 seconds.
4. Take a screenshot called "compressed".
5. Expect "Display8.log" contains "a 64x64 texture of format 0x31545844 made".
6. Expect "Display8.log" contains "a 64x64 texture of format 0x33545844 made".
7. Expect "Display8.log" contains "a 64x64 texture of format 0x35545844 made".
8. Expect "Display8.log" not contain "LockRect level".
9. Expect "Display8.log" not contain "has no mirror".
10. Expect "Display8.log" not contain "vulkan 0x".
11. Expect no crash.
