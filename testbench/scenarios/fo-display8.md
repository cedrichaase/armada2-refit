# Fleet Operations: Roots with Display8 in DxWrapper's place: menus and a skirmish on Vulkan

Resolutions: 1280x960
Mod: refit
Layers: display
Setup: display/bench-mode.sh stub Backend=vulkan
Timeout: 10 min

Run with `--install .` and `a2test --game fo-roots` (or `A2_PROFILE=fo-roots`): `./install`
puts Display8 into the clone's `Data/` as `d3d8.dll`, with DxWrapper's own as
`d3d8.dll.a2display-backup`; `dxwrapper.dll`, `dxwrapper.ini`, Roots' `d3d9.dll` and
ReShade stay (`display/README.md`, "On Fleet Operations: Roots"). Display8 reads the build
from the profile `fo-roots-5.0Z11`, where `display.modes` is verified beside Roots' changes
(FO-A), and draws the game on Vulkan with the validation layer on, every draw through the
passthrough: the shell on its windowed 1200x900 device, then the skirmish on the second
device Roots makes for the game, which the swap chain follows (`display/README.md`, "A
second window"); Roots' DDS art as BC textures, and FleetOpsHook's bump-mapped hulls
through the port of the Borg bump shader. Roots writes no log of its own; Display8's is in
the clone's `Data/`, which the bench reads as "Display8.log".

1. Launch the game.
2. Wait up to 120 seconds for "Display8.log" to say "Direct3DCreate8".
3. Expect "Display8.log" contains "engine: build armada2/fo-roots-5.0Z11".
4. Expect "Display8.log" contains "the engine draws nothing from here".
5. Expect "Display8.log" not contain "display.modes stands down".
6. Expect "Display8.log" not contain "bail-out".
7. Wait for the main menu.
8. Take a screenshot called "main menu".
9. Check that the Fleet Operations: Roots title stands over a space station backdrop, with a column of buttons (Single Player, Instant Action, Multiplayer, Map Editor, Options, Credits, Exit) in the middle and a version number below, no error dialog, and the picture is not a white or black screen.
10. Click "Instant Action".
11. Wait up to 20 seconds for "LAUNCH".
12. Click "LAUNCH".
13. Wait up to 180 seconds for "6000".
14. Wait 5 seconds.
15. Take a screenshot called "skirmish".
16. Check that a space map is shown from above with a ship or station of the player's in the middle, resource counters along the top, a minimap at the bottom left and a row of small buttons at the top right, the ships and stations textured (not white), and that the picture is not white.
17. Expect "Display8.log" contains "the swap chain follows it".
18. Expect "Display8.log" not contain "could not follow".
19. Expect "Display8.log" not contain "not one the passthrough has a port of".
20. Expect "Display8.log" not contain "vulkan 0x".
21. Expect "Display8.log" not contain "device removed".
22. Quit the game.
23. Expect the game to have exited.
24. Expect no crash.
