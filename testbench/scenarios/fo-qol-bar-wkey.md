# Fleet Operations: Roots with the grid's keys: no button under W, every build-list item under a key that presses it

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui qol
Launch: -nointro save\ia-fed-kli.sav
Setup: testbench/bench/roots-save.sh ia-fed-kli display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh --scene game HUD.Bar=1
Overlay: UI.HUD:Bar=1 UI.HUD:BarLabels=1 QOL.QOL:BarKeys=1
Timeout: 14 min

Run with `a2test --game fo-roots` and `--install .`. The user's bug (2026-11-07): with a Federation
shipyard selected, the Steamrunner sat in the W cell of the build list with no key badge, and W did
nothing but deselect the shipyard. Roots' plain W is its own `select_next_ship`, reworked by Roots to
select the damaged ships of the selection, so it empties a shipyard's, and the overview reads it
before the bar's keys (`PLAN-FLEETOPS.md`, decision 347; on the bench, with W's key given back, W
emptied the Starbase's selection and pressed nothing). So W stays Roots' own: QOL gives the W cell no
key and the grid puts no button there, and every item sits under a key that presses it.

The Starbase stands for the shipyards (the same kind of build list). The game starts from the
Instant Action save (Federation against the Klingon AI) and `Scene.asi` selects the Starbase by its
class (`README.md`, "Objects"); only keys are pressed, so the game's pointer plays no part. Q opens
the build list; its line in `UI.log` has no `W=`; A (the second item, filled down the columns:
Q, A, Z, then the second column from S) queues its ship, which turns the build list's cancel on
(`T=cancel `, without `(off)`) while the Starbase stays selected.

1. Launch the game.
2. Wait up to 240 seconds for "Scene.log" to say "scene ready".
3. Expect "UI.log" contains "bar: ours as the grid".
4. Expect "QOL.log" contains "BarKeys=1  -> Q W E R T / A S D F G / Z X C V B press the Refit bar's cells, patched".
5. Select the station of class "Starbase" owned by the player.
6. Wait up to 10 seconds for "UI.log" to say "Z=all_bastion".
7. Expect "UI.log" not contain "| W=".
8. Press q.
9. Wait up to 10 seconds for "QOL.log" to say "bar: key Q".
10. Wait up to 10 seconds for "UI.log" to say "T=cancel(off)".
11. Expect "UI.log" not contain " W=".
12. Wait 2 seconds.
13. Take a screenshot called "starbase build list".
14. Check that the grid shows the Starbase's ships each with a key letter, and that the second cell of the top row is empty.
15. Press a.
16. Wait up to 10 seconds for "QOL.log" to say "bar: key A".
17. Wait up to 10 seconds for "UI.log" to say "T=cancel ".
18. Wait 2 seconds.
19. Take a screenshot called "second ship queued by A".
20. Check that the Starbase is still selected and its build list still shows, with a ship building (a queued count or progress on a button, or the cancel button top right now lit).
21. Expect "UI.log" not contain "PANIC".
22. Expect "Display8.log" not contain "VUID".
23. Quit the game.
24. Expect the game to have exited.
25. Expect no crash.
