# Fleet Operations: Roots with Display8 in DxWrapper's place: menus and a skirmish

Resolutions: 1280x960
Mod: refit
Layers: display
Timeout: 10 min

Run with `--install .` and `a2test --game fo-roots` (or `A2_PROFILE=fo-roots`): `./install`
puts Display8 into the clone's `Data/` as `d3d8.dll`, with DxWrapper's own as
`d3d8.dll.a2display-backup`; `dxwrapper.dll`, `dxwrapper.ini`, Roots' `d3d9.dll` and
ReShade stay (`display/README.md`, "On Fleet Operations: Roots"). Display8 reads the build
from the profile `fo-roots-5.0Z11` and, as profiled (FO-2), stands `display.modes` down
(`FindDisplayMode` is changed by Roots and not yet verified): the stub could open no
display, so Display8 bails out and the game draws through the system's own `d3d8.dll`
(Wine's) with the plugins standing down. When the display hooks are verified the two
bail-out lines below change to the backend coming up, and the skirmish becomes the check of
Display8's passthrough on Roots (a skirmish on a Display8 device goes white while Display8
does not follow Roots' second `CreateDevice`: `fo-display-readiness`). Roots writes no log
of its own; Display8's is in the prefix, which the bench reads as "Display8.log".

1. Launch the game.
2. Wait up to 120 seconds for "Display8.log" to say "Direct3DCreate8".
3. Expect "Display8.log" contains "engine: build armada2/fo-roots-5.0Z11".
4. Expect "Display8.log" contains "Backend=auto: on Linux: vulkan, then d3d12".
5. Expect "Display8.log" contains "display.modes stands down".
6. Expect "Display8.log" not contain "Vulkan could not start".
7. Wait for the main menu.
8. Take a screenshot called "main menu".
9. Check that the Fleet Operations: Roots title stands over a space station backdrop, with a column of buttons (Single Player, Instant Action, Multiplayer, Map Editor, Options, Credits, Exit) in the middle and a version number below, no error dialog, and the picture is not a white or black screen.
10. Click "Instant Action".
11. Wait up to 20 seconds for "LAUNCH".
12. Click "LAUNCH".
13. Wait up to 180 seconds for "6000".
14. Wait 5 seconds.
15. Take a screenshot called "skirmish".
16. Check that a space map is shown from above with a ship or station of the player's in the middle, resource counters along the top, a minimap at the bottom left and a row of small buttons at the top right, and that the picture is not white.
17. Quit the game.
18. Expect the game to have exited.
19. Expect no crash.
