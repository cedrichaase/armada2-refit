# Display8 and plugins of another contract stand down cleanly

Resolutions: 1280x720
Mod: refit
Players: oldplugins, olddisplay
Setup: display/bench-mode.sh stub display/bench-abi.sh
Timeout: 25 min

Run with `--install .`. The contract between Display8 and the plugins
(`gpu/src/display.rs`, "Versions"; `display/README.md`, "Versions of the contract")
has a handshake since display 0.12.0, and its exports are named with their major, so
files from two releases that do not belong together must not misread each other's
structs. Two games at once, each on the stub with `Backend=auto` as shipped (Vulkan
on the bench's GPU, with the validation layer), `display/bench-abi.sh` putting files of
c50bbc0 (13.42.4, the last release before the handshake) into the clone:

In **oldplugins**, that release's `Render3D.asi` and `UI.asi` sit beside this
checkout's `d3d8.dll`. They look for the unversioned exports (`display8_backend`, ...),
find none, and draw on Direct3D 9 only, which on Vulkan is nothing; Display8 logs that
they never asked for the contract. In **olddisplay**, this checkout's plugins sit
beside that release's `d3d8.dll`. `gpu` finds a Display8 with no `display8_abi`, logs
it as too old (not as no Display8 at all) and stands down the same way.

Either way the plugins draw nothing, the engine's own draws pass through (both
Display8s have passthrough on Vulkan), the game reaches the menu and a mission, and
nothing crashes. With the UI standing down the shell is the stock one, 800x600 at the
screen's top left, so the Instant Action cube is clicked in screen pixels.

## Main menu
1. Launch the game.
2. Wait up to 300 seconds for the main menu.
3. Wait 5 seconds.
4. Take a screenshot called "main menu".
5. Olddisplay: Expect "UI.log" contains "is too old for this plugin (no contract handshake".
6. Olddisplay: Expect "UI.log" contains "runs stock: no Direct3D 9, Vulkan or Direct3D 12 device to draw with".
7. Olddisplay: Expect "UI.log" not contains "no Display8 (the game's d3d8.dll is another chain's)".

## Mission
8. Click at 640,90.
9. Wait for "GAME SETUP".
10. Wait 2 seconds.
11. Click "LAUNCH".
12. Wait 60 seconds.
13. Take a screenshot called "mission".
14. Check that the screenshot shows a space battle map of Star Trek: Armada II drawn by the game (stars, ships or stations, or the game's own interface), not a black or garbled screen.
15. Expect "Display8.log" contains "passthrough: the engine's first draw".
16. Olddisplay: Expect "Render3D.log" contains "is too old for this plugin (no contract handshake".
17. Oldplugins: Expect "Display8.log" contains "Render3D.asi is loaded and has not asked for the contract".
18. Oldplugins: Expect "Display8.log" contains "UI.asi is loaded and has not asked for the contract".
19. Oldplugins: Expect "Display8.log" not contains "contract: Render3D.asi asks for".
20. Expect "Display8.log" not contains "PANIC".
21. Expect "Render3D.log" not contains "PANIC".
22. Expect "UI.log" not contains "PANIC".
23. Quit the game.
24. Expect no crash.
