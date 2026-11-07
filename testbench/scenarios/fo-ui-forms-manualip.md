# Fleet Operations: Roots with the UI: Manual IP over FleetOpsHook's form, the address kept by Roots' own handler

Resolutions: 1600x900
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh Menus.Forms=1 Menus.Scene=1
Timeout: 14 min

Run with `a2test --game fo-roots` and `--install .` (`PLAN-FLEETOPS.md`, FO-6 step 7,
option C; `ui/MENUS.md`, "Roots' forms"). Multiplayer opens Roots' connection form, drawn
as Armada II's (`menus/roots/multiplayer.rs`); its Manual IP button opens Roots' Manual IP
form, drawn as Armada II's small dialogs (`menus/roots/manualip.rs`: its address field is
Roots' own text field, which a click on ours gives the keyboard). A click on the address field
gives Roots' field the keyboard; the address is typed into it, ours shows it, and Cancel
is pressed: Roots' Cancel writes nothing (Join would write the address into
Roots' settings file and start a connection, which Wine's DirectPlay cannot make).


1. Launch the game.
2. Wait up to 180 seconds for "UI.log" to say "forms: TMainForm is ours".
3. Wait 2 seconds.
4. Click "Multiplayer".
5. Wait up to 20 seconds for "UI.log" to say "forms: TMultiplayerConnectionForm is ours".
6. Wait 3 seconds.
7. Take a screenshot called "connection".
8. Check that a window of ours titled "MULTIPLAYER", laid out as Armada II's Multiplayer Connection screen, shows a Name caption over a field reading the player name, three wide buttons for the Internet, Manual IP and LAN connections, and a Previous Menu button.
9. Click "Manual".
10. Wait up to 20 seconds for "UI.log" to say "forms: TManualIPForm is ours".
11. Wait 3 seconds.
12. Click at 800,452.
13. Wait 1 second.
14. Type "10.11.12.13".
15. Wait 2 seconds.
16. Take a screenshot called "manual ip".
17. Check that a window of ours titled "MANUAL IP" over the dimmed connection window shows a Host address caption over a field reading 10.11.12.13 with a caret, and two buttons, OK and Cancel.
18. Expect "10.11.12.13" is visible.
19. Click "Cancel".
20. Wait up to 20 seconds for "UI.log" to say "TManualIPForm".
21. Wait 2 seconds.
22. Take a screenshot called "back".
23. Wait up to 20 seconds for the logs the next steps expect.
24. Expect "UI.log" contains "press on buttonNo".
25. Expect "UI.log" contains "TManualIPForm has 4 controls".
26. Expect "UI.log" not contain "PANIC".
27. Expect "Display8.log" not contain "VUID".
28. Stop the game.
29. Expect no crash.
