# Fleet Operations: Roots with the UI: the multiplayer connection screen as Armada II's, over FleetOpsHook's form

Resolutions: 1600x900
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh Menus.Forms=1 Menus.Scene=1
Timeout: 12 min

Run with `a2test --game fo-roots` and `--install .` (`PLAN-FLEETOPS.md`, FO-6 step 7,
option C; `ui/MENUS.md`, "Roots' forms"). Roots' Multiplayer Connection form has a spec
(`menus/roots/multiplayer.rs`) that uses Armada II's layout function
(`screens::multiplayer::connection_layout`): the name as a field over which Roots' own text
field has the keyboard, the three ways to connect as wide buttons in Roots' captions, and
Previous Menu. The name typed goes to Roots' field, which ours shows. Previous Menu runs
Roots' own Back.

1. Launch the game.
2. Wait up to 180 seconds for "UI.log" to say "forms: TMainForm is ours".
3. Wait 2 seconds.
4. Click "MULTIPLAYER".
5. Wait up to 20 seconds for "UI.log" to say "forms: TMultiplayerConnectionForm is ours".
6. Wait 3 seconds.
7. Take a screenshot called "connection".
8. Check that a window of ours titled "MULTIPLAYER", laid out as Armada II's Multiplayer Connection screen over the dimmed menu scene, shows a Name caption over a field reading the player's name, three wide buttons (the first reads Internet - Tunngle or Internet - GameSpy, then Internet - Manual IP and Local Area Network) and a Previous Menu button at the bottom left.
9. Click at 800,366.
10. Wait 1 second.
11. Type "zed".
12. Wait 2 seconds.
13. Take a screenshot called "typed".
14. Check that the name field has a highlighted frame and reads zed.
15. Click "Previous Menu".
16. Wait up to 20 seconds for "UI.log" to say "press on buttonBack".
17. Wait up to 20 seconds for "UI.log" to say "forms: TMainForm is ours".
18. Expect "UI.log" not contain "PANIC".
19. Expect "Display8.log" not contain "VUID".
20. Stop the game.
21. Expect no crash.
