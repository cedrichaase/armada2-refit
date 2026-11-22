# Fleet Operations: Roots with Display8 and Render3D alone: the reference fo-ui-standdown is compared with

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d
Setup: display/bench-mode.sh stub Backend=vulkan
Timeout: 12 min

The A side of `fo-ui-standdown` (FO-6 step 3, `PLAN-FLEETOPS.md`): Roots as Display8 and
Render3D give it, with no `UI.asi` at all. Run it first, with `a2test --game fo-roots` and
`--install .`; it records the main menu and the Instant Action setup screen as the baseline
`fo-ui-standdown`, which `fo-ui-standdown` (the UI installed, every part of it at 0) must
match. The steps are the same in both, so the pointer and the timing agree.

1. Launch the game.
2. Wait for the main menu.
3. Wait 5 seconds.
4. Take a screenshot called "menu".
5. Include "_fo-sides".
6. Take a screenshot called "setup".
7. Record the screenshot "menu" as baseline "fo-ui-standdown".
8. Record the screenshot "setup" as baseline "fo-ui-standdown".
9. Click "LAUNCH".
10. Wait up to 240 seconds for "Render3D.log" to say "(claimed at the instance draw)".
11. Wait 15 seconds.
12. Take a screenshot called "match".
13. Check that the match has started: the 3D view of a space map with ships and Fleet Operations' own HUD.
14. Expect "Display8.log" not contain "VUID".
15. Expect "Display8.log" not contain "device removed".
16. Quit the game.
17. Expect the game to have exited.
18. Expect no crash.
