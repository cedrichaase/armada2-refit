# Retired Display8 keys are read past: Mode=forward, Real= and a d3d9 token, on Direct3D 12

Resolutions: 1280x720
Mod: refit
Setup: display/bench-mode.sh stub Backend=d3d12,d3d9 Mode=forward Real=d3d8_real.dll
Timeout: 15 min

Run with `--install .`. Display 1.0.0 removed forward mode and Direct3D 9, and with them
`Mode=`, `Real=` and the backend `d3d9` (`display/README.md`, "The bail-out", "Choosing a
backend"). A `Display8.ini` an install has not merged yet (`tools/ini-merge.py`
`Defaults=3` drops them) may still carry them: Display8 reads past each, says so once,
and runs on what is left, here Direct3D 12 (on the GPU: one Direct3D 12 game at a time
on the bench). `display-retired-keys-overlay` is the same with the overlay's
`[Display8.Display8] Backend=d3d9`, on auto.

    ./a2test run display-retired-keys --install . -j 1

1. Launch the game with "".
2. Wait up to 120 seconds for "Display8.log" to say "stub: the backend is".
3. Expect "Display8.log" contains "settings: Mode=forward retired (forward mode removed); ignored".
4. Expect "Display8.log" contains "settings: Real= retired (forward mode removed); ignored".
5. Expect "Display8.log" contains "settings: Backend token d3d9 retired (Direct3D 9 removed)".
6. Expect "Display8.log" contains "stub on d3d12".
7. Expect "Display8.log" contains "stub: the backend is Direct3D 12".
8. Expect "Display8.log" not contain "bail-out".
9. Wait 8 seconds.
10. Press Escape.
11. Wait 3 seconds.
12. Press Escape.
13. Wait 3 seconds.
14. Press Escape.
15. Wait 3 seconds.
16. Press Escape.
17. Wait up to 300 seconds for the main menu.
18. Wait 3 seconds.
19. Take a screenshot called "main menu".
20. Check that the screenshot shows the main menu of Star Trek: Armada II (Single Player, Instant Action, Multiplayer, Options ...), not a black or garbled screen.
21. Expect "Display8.log" not contain "device removed".
22. Quit the game.
23. Expect the game to have exited.
24. Expect no crash.
