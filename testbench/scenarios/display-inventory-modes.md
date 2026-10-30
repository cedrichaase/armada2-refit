# Display8's inventory: the display mode, the mouse and a mode switch

Resolutions: 1280x720
Reference: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Setup: display/bench-mode.sh forward showcase
Timeout: 12 min

Run with `--install .`. The rest of phase 0 of the display plan (`display-inventory`
is the whole game's way): Display8 in forward mode, tracing, in a mission with the
showcase scene. Contexts: "mission", "options", "mode switch" (a display mode chosen in
Graphics Settings: the engine gives up its device and creates another, so `Reset` or
`CreateDevice` and whoever reads the mode show), "after switch" and "pan" (right-drags,
which move the map and the pointer; after the switch, because right-drags made first left
the Options menu deaf to clicks). `Display8.inventory.tsv` and `Display8.log` (`MODE`,
`MOUSE`, `WINDOW` lines) are gathered into the case's `logs/`; no step expects anything
of the counts. Stub variant: `display/stub-scenario.sh display-inventory-modes`.

1. Launch the game.
2. Wait up to 50 seconds for the logs the next steps expect.
3. Expect "Scene.log" contains "scene ready".
4. Mark the display context "mission".
5. Wait 5 seconds.
6. Mark the display context "options".
7. Press Escape.
8. Wait for the options menu.
9. Wait 2 seconds.
10. Click "Graphics Settings".
11. Wait for "Display Mode".
12. Mark the display context "mode switch".
13. Click the display mode box.
14. Wait 2 seconds.
15. Click "800x600x32".
16. Wait 3 seconds.
17. Click "Previous Menu".
18. Wait for the options menu.
19. Press Escape.
20. Wait 5 seconds.
21. Mark the display context "after switch".
22. Wait 15 seconds.
23. Take a screenshot called "after switch".
24. Mark the display context "pan".
25. Right-drag from 960,500 to 760,500.
26. Wait 1 second.
27. Right-drag from 600,300 to 700,350.
28. Wait 1 second.
29. Right-drag from 960,500 to 760,400.
30. Wait 3 seconds.
31. Take a screenshot called "after pan".
33. Wait 3 seconds.
34. Expect the game is still running.
35. Quit the game.
36. Expect no crash.
