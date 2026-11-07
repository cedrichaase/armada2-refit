# Fleet Operations: Roots with QOL: the right-drag pan speed, Shift+number and the move queue run, the rest stands down

Resolutions: 1280x960
Mod: refit
Layers: display asiloader qol
Setup: display/bench-mode.sh stub Backend=vulkan
Timeout: 10 min

FO-7 round one (`PLAN-FLEETOPS.md`, "FO-7: QOL on Roots", decisions 100-107). QOL.asi is
installed into Roots' `Data/` with the game's own defaults (`qol/defaults/fo-roots.ini`):
`PanSpeed=2.5` on, `ShiftAddsToGroup` on since round two (`fo-qol-groups`), `PathStartsAtOnce` on since round
three (`fo-qol-move-queue`), `StationGroups`, `WarpDistance`, `RigIgnoresSelf`, `MaxSelection` and `BarKeys` off. The pan hook is verified on this build
(`RTS_Configure`'s site is stock's bytes in the file and in memory after FleetOpsHook), so
the log says it patched and the parsed coefficient is Roots' own RTS_CFG.h value (0.005)
times 2.5; the rest of the log says which features were left alone and why (the profile
marks their functions as changed by Roots and not yet verified, or the key is 0).
It waits on QOL.log, not on the screen: the first checks run as soon as the plugin has
started, before a map is loaded. Then an Instant Action game is started
(`_fo-sides`) and the map is panned with the right button, which has to move the view.
`QOL.log` is written beside `ArmadaL.exe`, in `Data/`.

1. Launch the game.
2. Wait for the main menu.
3. Expect "QOL.log" contains "build armada2/fo-roots-5.0Z11".
4. Expect "QOL.log" contains "PanSpeed=2.5  -> x2.50, patched".
5. Expect "QOL.log" contains "RTS_CFG.h parsed: right-drag FASTSCROLL_COEFFICIENT now 0.0125".
6. Expect "QOL.log" contains "ShiftAddsToGroup=1  -> Shift+N adds to group N, Alt+N selects and centres, patched".
7. Expect "QOL.log" contains "StationGroups=0  (stations select and group as in stock)".
8. Expect "QOL.log" contains "WarpDistance=0  (0: moves left alone)".
9. Expect "QOL.log" contains "PathStartsAtOnce=1  -> move queues: Tab points (and minimap points) queued per ship, flown at once, patched".
10. Expect "QOL.log" contains "RigIgnoresSelf=0  (a construction ship blocks its own blueprint, as in stock)".
11. Expect "QOL.log" contains "BarKeys=0  (the bar's keys left as in stock)".
12. Expect "QOL.log" contains "MaxSelection=16  (16 or less: selection and groups left at stock's 16)".
13. Expect "QOL.log" not contain "BarKeys=1".
14. Include "_fo-sides".
15. Click "LAUNCH".
16. Wait up to 180 seconds for "6000".
17. Wait 5 seconds.
18. Take a screenshot called "before".
19. Right-drag from 840,480 to 440,480.
20. Wait 2 seconds.
21. Take a screenshot called "after".
22. Check that the large round station with four rings is now in the right quarter of the screen (it was just right of the middle before the drag: a right-drag to the left pans the view so that the map moves right), with the HUD unchanged and nothing frozen or garbled.
23. Right-drag from 440,480 to 840,480.
24. Wait 2 seconds.
25. Take a screenshot called "back".
26. Expect "QOL.log" not contain "patch refused".
27. Quit the game.
28. Expect the game to have exited.
29. Expect no crash.
