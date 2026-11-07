# Fleet Operations: Roots with the grid's keys: W presses the bar's W cell, Roots' select_next_ship moves to M

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui qol
Launch: -nointro save\ia-fed-kli.sav
Setup: testbench/bench/roots-save.sh ia-fed-kli display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh --scene game HUD.Bar=1
Overlay: UI.HUD:Bar=1 UI.HUD:BarLabels=1 QOL.QOL:BarKeys=1
Timeout: 14 min

Run with `a2test --game fo-roots` and `--install .`. The user's bug (2026-11-07): with a shipyard
selected, W did nothing for the bar's W cell (the Steamrunner) and then deselected the shipyard:
Roots binds plain W to `select_next_ship`, reworked by Roots to select the damaged ships of the
selection, and the overview reads it before the bar's keys. The user chose to move it: while the
grid's keys are on, QOL's rebind table (`[BarKeys] Rebind = select_next_ship:W>M`, Roots' defaults)
moves that chord's W to M in the engine's parsed key map, in memory (`PLAN-FLEETOPS.md`, decision
349; `qol/README.md`, "Key rebinds"), and puts W back when the keys stop.

The game starts from the Instant Action save (Federation against the Klingon AI) and `Scene.asi`
selects by class (`README.md`, "Objects"); only keys are pressed, so the game's pointer plays no
part. The Starbase stands for a shipyard: its build list's W cell is `all_halt` (Roots' halt of the
queue), so A queues a ship (the list's cancel turns on, `T=cancel `), W presses the W cell (the
queue halts: `T=cancel(off)` again) and the Starbase stays selected. Then, with the construction
ships selected, W keeps the selection and M, Roots' action now, empties it (none is damaged, so
"the damaged ships of the selection" is none, as W did before). With `BarKeys=0` the key map's own
keys come back.

1. Launch the game.
2. Wait up to 240 seconds for "Scene.log" to say "scene ready".
3. Expect "UI.log" contains "bar: ours as the grid".
4. Expect "QOL.log" contains "[BarKeys] Rebind=select_next_ship:W>M  -> applied in memory".
5. Select the station of class "Starbase" owned by the player.
6. Wait up to 10 seconds for "QOL.log" to say "rebind: select_next_ship W -> M".
7. Wait up to 10 seconds for "UI.log" to say "Z=all_bastion".
8. Press q.
9. Wait up to 10 seconds for "QOL.log" to say "bar: key Q".
10. Wait up to 10 seconds for "UI.log" to say "W=all_halt".
11. Take a screenshot called "starbase build list".
12. Check that the second cell of the top row has a W badge.
13. Press a.
14. Wait up to 10 seconds for "QOL.log" to say "bar: key A".
15. Wait up to 10 seconds for "UI.log" to say "T=cancel ".
16. Press w.
17. Wait up to 10 seconds for "QOL.log" to say "bar: key W".
18. Wait up to 10 seconds for "UI.log" to say "T=cancel(off)".
19. Wait 1 second.
20. Expect a station of class "Starbase" owned by the player that is selected.
21. Select the ships of class "Mediterranean" owned by the player.
22. Wait 2 seconds.
23. Press w.
24. Wait 2 seconds.
25. Expect 2 ships of class "Mediterranean" owned by the player that are selected.
26. Press m.
27. Wait 2 seconds.
28. Expect no ship of class "Mediterranean" owned by the player that is selected.
29. Overlay QOL.QOL:BarKeys=0
30. Select the ships of class "Mediterranean" owned by the player.
31. Wait up to 10 seconds for "QOL.log" to say "rebind: the key map's own keys back".
32. Expect "UI.log" not contain "PANIC".
33. Expect "Display8.log" not contain "VUID".
34. Quit the game.
35. Expect the game to have exited.
36. Expect no crash.
