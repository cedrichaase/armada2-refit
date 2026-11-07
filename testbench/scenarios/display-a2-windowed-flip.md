# Armada II's own flip to windowed: a new device on a new window, and the swap chain follows it

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh showcase Race=federation Hud=1
Timeout: 10 min

`display/README.md`, "A second window". Armada II makes its display again when the
fullscreen flag, the device type or the display device changes; in a retail game that is its
assert and exception handlers' flip to windowed (`Debug_SystemCleanup`, with
`cfgFLIP_TO_WINDOWED_ON_ASSERT = 1` in the stock `RTS_CFG.h`). The old device is deleted
with its window, and a new device is made on a new window of the engine's class, windowed,
shown by the engine once the device exists. Display8 moves the swap chain to it and logs what
it found: the new window's class (`Armada2`), hidden still, on the engine's thread, and the
old window gone. Scene.asi's `windowed` calls the engine's function as the handlers do
(`../scene/README.md`), without a crash: the mission must go on drawing on the new window.
A player met this path after a crash in render3d (`render3d-welds-next-mission`): the black
screen was the engine hung in its crash dump, not the swap chain's move.

1. Launch the game.
2. Wait up to 240 seconds for "Scene.log" to say "scene ready".
3. Wait 3 seconds.
4. Take a screenshot called "fullscreen".
5. Scene "windowed".
6. Wait up to 30 seconds for "Display8.log" to say "the swap chain follows it".
7. Expect "Display8.log" contains "hidden, thread".
8. Expect "Display8.log" contains ": gone, thread".
9. Wait up to 30 seconds for "UI.log" to say "UI frames in".
10. Wait 3 seconds.
11. Take a screenshot called "windowed".
12. Expect no black bars.
13. Expect the window manager to see one window.
14. Expect "Display8.log" not contain "could not follow".
15. Expect the game is still running.
16. Quit the game.
17. Expect no crash.
