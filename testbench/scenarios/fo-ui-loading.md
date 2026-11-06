# Fleet Operations: Roots with the UI: the loading screen is the UI's, and nothing else of it is on

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh HUD.Enable=0 Loading.Hold=20
Timeout: 12 min

Run with `a2test --game fo-roots` and `--install .`: `./install --game fo-roots` puts
Display8, the ASI loader, Render3D and `UI.asi` into the clone's `Data/`, with `UI.ini` as
Roots has it by default (`ui/defaults/fo-roots.ini`: the loading screen on, the HUD's
panels, the menus, the Credits, the menu scene, the cursor and the movies off). FO-6 step 2
(`PLAN-FLEETOPS.md`, "FO-6: the UI on Roots") verified `loading.overlay` and `loading.own`
beside Roots' load bar and `GameDoOne` edits: the call in `RenderLoopAssetLoadStatusUpdate`
that the overlay takes, the call of `LoadBarNext` into it, and the splash call in
`GameDoOne` are stock's bytes in the file and in memory. `loading.inner_db` and
`loading.inner_pixels` stay down by design (`FindLogicalDatabase`, `FindVisibleDatabase` and
`DemandLoad` are replaced whole), so the screen gets its frames between the load's steps
only. `Hold=20` makes the load wait 20 seconds at its half-way point, the screen still drawn
and animated by frames of ours, so it can be looked at (a test aid; never set in play).

The skirmish is Federation against Klingon (`_fo-sides`), so the race is known: the screen
names the local player's race through `shell.reads_setup` (Roots' `unitedfederation` is an
alias of the fleetops set's Federation) and the map and both players come from the same
reads. The logs are the UI's (`UI.log`) and Display8's.

1. Launch the game.
2. Wait for the main menu.
3. Include "_fo-sides".
4. Click "LAUNCH".
5. Wait up to 240 seconds for "UI.log" to say "holding at".
6. Take a screenshot called "load".
7. Check that the screen is a loading screen drawn edge to edge: a mission or map title, a panel listing the players with their races, and a progress bar across the bottom around the middle, with no Fleet Operations loading bar or logo model showing through, and the picture is neither white nor black.
8. Wait 2 seconds.
9. Take a screenshot called "load later".
10. Check that the ring of dots at the left of the status line has turned since the previous screenshot (frames of ours between the engine's are animated).
11. Wait up to 120 seconds for the logs the next steps expect.
12. Expect "UI.log" contains "engine: build armada2/fo-roots-5.0Z11".
13. Expect "UI.log" contains "loading: hook at".
14. Expect "UI.log" contains "loading: a screen of ours, with frames between the engine's".
15. Expect "UI.log" contains "loading: frames inside a step after each SOD and texture loaded (0 of 4 sites)".
16. Expect "UI.log" contains "loading: map".
17. Expect "UI.log" contains "first frame of ours".
18. Expect "UI.log" contains "hold over".
19. Expect "UI.log" not contain "loading: standing down".
20. Wait 20 seconds.
21. Take a screenshot called "match".
22. Check that the match has started: the 3D view of a space map with ships and Fleet Operations' own HUD (its resource counters, minimap and info strip), no loading screen left over it, and no panel of the UI's over the HUD.
23. Expect "UI.log" not contain "hud: panels".
24. Expect "Display8.log" not contain "VUID".
25. Expect "Display8.log" not contain "device removed".
26. Expect "Display8.log" not contain "bail-out".
27. Quit the game.
28. Expect the game to have exited.
29. Expect no crash.
