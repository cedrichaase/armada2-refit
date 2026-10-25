# The movies without the cutscene proxy: the release package's layers, on the stub

Resolutions: 1280x720
Reference: 1280x720
Mod: refit
Setup: cutscenes/binkproxy/bench-unproxy.sh
Timeout: 15 min

Run with `--install .` (the stub on `Backend=auto`):

    ./a2test run display-movie-no-proxy --install . -j 1

The release package installs no cutscene proxy, and with Display8's stub the game
crashed before the first movie: `PlayIntroMovie` asked for a 640x480 the stub does not
offer (`ui/README.md`, "The intro without the cutscene proxy"). `UI.asi` raises the
intro's mode itself and draws the stock reels' frames fitted, from the engine's own
copy, so a 4:3 movie on this 16:9 screen sits between black bars.

1. Launch the game with "".
2. Wait up to 60 seconds for "UI.log" to say "movie: frames drawn by the UI".
3. Wait 4 seconds.
4. Take a screenshot called "reel".
5. Expect black bars.
6. Expect "UI.log" contains "intro: display mode ->".
7. Expect "UI.log" contains "intro: frames drawn by the UI".
8. Press Escape.
9. Wait up to 120 seconds for the main menu.
10. Wait 2 seconds.
11. Take a screenshot called "main menu".
12. Quit the game.
13. Expect no crash.
