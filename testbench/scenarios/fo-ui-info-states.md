# Fleet Operations: Roots with the UI: the info panel against Roots' own (a ship's weapon icon states, the Starbase's systems)

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui
Launch: -nointro save\ia-fed-kli.sav
Setup: testbench/bench/roots-save.sh ia-fed-kli display/bench-mode.sh stub Backend=vulkan OneWindow=0 ui/bench-asi.sh --scene game HUD.Info=1
Overlay: UI.HUD:Info=1 UI.HUD:Tooltips=1 UI.HUD:BottomRow=stacked
Timeout: 14 min

Run with `a2test --game fo-roots` and `--install .`. FO-6, "The info panel on Roots" (`PLAN-FLEETOPS.md`,
decisions 440 to 459 and the follow-ups): each face is shot with ours (`HUD.Info=1`) and then with
Roots' own (`Info=0`, live), for the side-by-side sheets; no step needs the pointer (the units are
selected by `Scene.asi` through the overview's own Select, `README.md`, "Objects"). The game starts
in Instant Action from a save (Federation against the Klingon AI). An enemy, a neutral, a planet and a ship under fire are not staged: the overview's Select (Scene.asi's `select`) leaves the selection empty for another team's units and for a planet, even spawned beside the Starbase, and a spawned attacker was gone before its order.

1. Launch the game.
2. Wait up to 240 seconds for "Scene.log" to say "scene ready".
3. Expect "UI.log" contains "info: ours, in Roots' layout over its panel's unseen Render".
4. Select the ship of class "Saber" owned by the player.
5. Wait up to 10 seconds for "UI.log" to say "info: Roots' panel ours: Single".
6. Wait 2 seconds.
7. Take a screenshot called "saber, ours".
8. Overlay UI.HUD:Info=0.
9. Wait up to 10 seconds for "UI.log" to say "info: Roots' own".
10. Wait 2 seconds.
11. Take a screenshot called "saber, Roots' own".
12. Check that the ship image in the bottom panel carries four small square weapon icons, the first of them grey and the others purple.
13. Overlay UI.HUD:Info=1.
14. Select the station of class "Starbase" owned by the player.
15. Wait up to 10 seconds for "UI.log" to say "info: Roots' panel ours: Builder".
16. Wait 2 seconds.
17. Take a screenshot called "starbase, ours".
18. Check that among the six square system icons right of the station's picture, the one in the middle row on the right is grey and the others are blue.
19. Overlay UI.HUD:Info=0.
20. Wait up to 10 seconds for "UI.log" to say "info: Roots' own".
21. Wait 2 seconds.
22. Take a screenshot called "starbase, Roots' own".
23. Overlay UI.HUD:Info=1.
24. Expect "UI.log" not contain "PANIC".
25. Expect "Display8.log" not contain "VUID".
26. Expect "Display8.log" not contain "device removed".
27. Quit the game.
28. Expect the game to have exited.
29. Expect no crash.
