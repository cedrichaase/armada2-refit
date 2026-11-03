# A retired backend in the overlay: [Display8.Display8] Backend=d3d9 is read past, auto runs

Resolutions: 1280x720
Mod: refit
Setup: display/bench-mode.sh stub Backend=d3d12,d3d9 Mode=forward
Overlay: Display8.Display8:Backend=d3d9
Timeout: 15 min

Run with `--install .`. `display-retired-keys` with a `RefitPlayer.ini` that still says
`[Display8.Display8] Backend=d3d9`, as a player's could from before display 1.0.0. The
overlay's value wins over `Display8.ini`'s (`tools/settings/README.md`), and Display8
skips its retired `d3d9` the same way: nothing is left, so the list is `auto`, Vulkan
first (on the GPU, with the validation layer). Display8 writes nothing into the
overlay; the UI's hub is its only writer (it drops the token there, phase 11 R3).

    ./a2test run display-retired-keys-overlay --install . -j 1

1. Launch the game with "".
2. Wait up to 120 seconds for "Display8.log" to say "stub: the backend is".
3. Expect "Display8.log" contains "settings: Backend token d3d9 retired (Direct3D 9 removed)".
4. Expect "Display8.log" contains "Backend= names no backend: auto".
5. Expect "Display8.log" contains "settings: Mode=forward retired (forward mode removed); ignored".
6. Expect "Display8.log" contains "stub: the backend is Vulkan".
7. Expect "Display8.log" not contain "bail-out".
8. Wait 8 seconds.
9. Press Escape.
10. Wait 3 seconds.
11. Press Escape.
12. Wait 3 seconds.
13. Press Escape.
14. Wait 3 seconds.
15. Press Escape.
16. Wait up to 300 seconds for the main menu.
17. Wait 3 seconds.
18. Take a screenshot called "main menu".
19. Check that the screenshot shows the main menu of Star Trek: Armada II (Single Player, Instant Action, Multiplayer, Options ...), not a black or garbled screen.
20. Quit the game.
21. Expect the game to have exited.
22. Expect no crash.
