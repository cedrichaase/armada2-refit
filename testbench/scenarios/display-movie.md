# The movies on screen: the launch reels and the intro, drawn by the UI

Resolutions: 1280x720
Reference: 1280x720
Mod: refit
Timeout: 15 min

Run with `--install .` (the stub on `Backend=auto`, shipped since display 0.8.0: Vulkan
on the GPU), and in the other modes and backends through `display/stub-scenario.sh`:

    ./a2test run display-movie --install . -j 1
    ./a2test run "$(display/stub-scenario.sh display-movie forward)" --install . -j 1
    ./a2test run "$(display/stub-scenario.sh display-movie stub Backend=d3d9)" --install . -j 1

The cutscene proxy hands each frame to the UI (`ui_present_movie`), which draws it at
the engine's `Present` (`ui/README.md`, "Movies"). In stub mode Display8 used to copy
the locked back buffer into the frame at that `Present`, after the UI had drawn, and
every movie was black with its sound playing (`display/README.md`, "Stub mode"). A
4:3 movie on this 16:9 screen sits between black bars; a black frame has no picture to
bound. The intro is the AV1 replacement when `$A2_DATA/movies/Intro` is built.

1. Launch the game with "".
2. Wait up to 60 seconds for "BinkProxy.log" to say "Paramount.bik".
3. Wait 4 seconds.
4. Take a screenshot called "paramount".
5. Expect black bars.
6. Expect "UI.log" contains "movie: frames drawn by the UI".
7. Wait up to 60 seconds for "BinkProxy.log" to say "Intro.".
8. Wait 6 seconds.
9. Take a screenshot called "intro".
10. Expect black bars.
11. Press Escape.
12. Wait up to 120 seconds for the main menu.
13. Wait 2 seconds.
14. Take a screenshot called "main menu".
15. Quit the game.
16. Expect no crash.
