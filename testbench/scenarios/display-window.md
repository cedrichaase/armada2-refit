# The window and the display mode are Display8's: the shell, the intro, Native and KeepClip

Resolutions: 1280x720
Reference: 1280x720
Mod: refit
Timeout: 15 min

Run with `--install .` (the stub on `Backend=auto`), in forward mode through
`display/stub-scenario.sh`, and without the UI through `--layers`:

    ./a2test run display-window --install . -j 1
    ./a2test run "$(display/stub-scenario.sh display-window forward)" --install . -j 1
    ./a2test run display-window --install . --layers display,render3d -j 1   # the Display8.log steps fail: no UI, no raise

`display/README.md`, "Window and display mode" (contract C3, display 0.18.0). Display8
is the only plugin that changes a display-mode request of the engine or the pointer's
clip: the shell's 800x600x16 is raised to the desktop's while the UI's menu host claims
`MENUS`, the intro's 640x480 under `[Window] RaiseIntro`, a stock profile mode under
`[Window] Native`, and Wine's clip is put back under `[Window] KeepClip`. The cutscene
proxy leaves the intro's mode to Display8 (`BinkProxy.log`). With no UI nothing claims
`MENUS`, so the shell stays at its stock 800x600 (the stock dialogs, no raise).

1. Launch the game with "".
2. Wait up to 60 seconds for "BinkProxy.log" to say "Paramount.bik".
3. Wait 4 seconds.
4. Take a screenshot called "paramount".
5. Expect black bars.
6. Wait up to 60 seconds for "BinkProxy.log" to say "Intro.".
7. Wait 6 seconds.
8. Press Escape.
9. Wait up to 120 seconds for the main menu.
10. Wait 2 seconds.
11. Expect "Display8.log" contains "window: shell mode raised to".
12. Expect "Display8.log" contains "(MENUS claimed)".
13. Expect "Display8.log" contains "window: intro mode raised to".
14. Expect "BinkProxy.log" contains "intro: Display8 raises the mode".
15. Expect "Display8.log" contains "window: KeepClip on (Wine)".
16. Expect "UI.log" not contains "menus: shell display mode".
17. Take a screenshot called "main menu".
18. Check that the main menu fills the whole screen: its backdrop reaches every edge and the buttons are at the usual size, not a small 800x600 picture in a corner.
19. Quit the game.
20. Expect no crash.
