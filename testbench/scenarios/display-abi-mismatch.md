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

Since display 0.18.0 the shell's 800x600 request is raised by Display8, not by the UI
(`[Window]`); display 0.11.0's stub offers the desktop's mode alone, so the engine
would fail to create its device (a Wine stack overflow before the menu) with no one to
raise it. A player who mixes files like this needs what raised it before: in
**olddisplay** `display/bench-abi.sh` puts Menus.asi back (`ui/bench-legacy.sh menus`),
which raises the mode and scales the shell to the screen, the UI's menus standing
down beside it; the cube is then at 930,100. Menus.asi also changes what the engine draws,
which the old stub drops, so that Display8 falls back to forward mode and the UI's HUD draws on
Direct3D 9; the game is not quit through its menus (their buttons moved), the case's end stops it.

## Main menu
1. Launch the game.
2. Wait up to 300 seconds for the main menu.
3. Wait 5 seconds.
4. Take a screenshot called "main menu".
5. Olddisplay: Expect "UI.log" contains "is too old for this plugin (no contract handshake".
6. Olddisplay: Expect "UI.log" not contains "menus: hosted by the UI".
7. Olddisplay: Expect "UI.log" not contains "no Display8 (the game's d3d8.dll is another chain's)".

## Mission
8. Oldplugins: Click at 640,90.
9. Olddisplay: Click at 930,100.
10. Wait for "GAME SETUP".
11. Wait 2 seconds.
12. Click "LAUNCH".
13. Wait 60 seconds.
14. Take a screenshot called "mission".
15. Check that the screenshot shows a space battle map of Star Trek: Armada II drawn by the game (stars, ships or stations, or the game's own interface), not a black or garbled screen.
16. Oldplugins: Expect "Display8.log" contains "passthrough: the engine's first draw".
17. Olddisplay: Expect "Display8.log" contains "forward mode instead".
18. Olddisplay: Expect "Render3D.log" contains "is too old for this plugin (no contract handshake".
19. Oldplugins: Expect "Display8.log" contains "Render3D.asi is loaded and has not asked for the contract".
20. Oldplugins: Expect "Display8.log" contains "UI.asi is loaded and has not asked for the contract".
21. Oldplugins: Expect "Display8.log" not contains "contract: Render3D.asi asks for".
22. Expect "Display8.log" not contains "PANIC".
23. Expect "Render3D.log" not contains "PANIC".
24. Expect "UI.log" not contains "PANIC".
25. Oldplugins: Quit the game.
26. Expect no crash.
