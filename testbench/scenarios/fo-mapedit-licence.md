# Fleet Operations: Roots with the UI: the map editor's licence stands whole in the middle

Resolutions: 1280x960, 2560x1080, 3440x1440
Mod: refit
Layers: display asiloader render3d qol ui
Overlay: UI.Menus:Forms=1 UI.Menus:Enable=1 UI.Menus:Scene=1 UI.UI:Cursor=1 UI.HUD:Minimap=1 UI.HUD:Info=1 UI.HUD:Bar=1
Timeout: 12 min

Run with `a2test --game fo-roots` and `--install .` (PLAN-FLEETOPS.md, decision 503). The
user's report of 2026-11-07, at 3440x1440: the editor's licence (`LicenseAgreementWindowDlgProc`,
an adopted dialog with no spec, drawn as its picture) stood in the top left corner, mostly off
the screen, so ACCEPT could not be reached. Roots' procedure leaves the dialog at the device
window's origin; the adoption took that for its place in the design space, across the design
space's top left corner. A dialog that does not fit in the design space is now centred in it.
Every click here is by a label, so the case runs at any resolution; Edit Map on a chosen map
ends in the licence, as Create new map does.

1. Launch the game.
2. Wait up to 180 seconds for "UI.log" to say "forms: TMainForm is ours".
3. Wait 3 seconds.
4. Click "MAP EDITOR".
5. Wait up to 20 seconds for "UI.log" to say "forms: TMapEditForm is ours".
6. Wait 3 seconds.
7. Click "A Good Day to Die".
8. Wait up to 10 seconds for "UI.log" to say "chosen".
9. Wait 2 seconds.
10. Click "Edit Map".
11. Wait up to 20 seconds for "UI.log" to say "press on buttonEditMap".
12. Wait up to 150 seconds for "UI.log" to say "centred in it".
13. Wait 5 seconds.
14. Take a screenshot called "licence".
15. Check that the licence agreement (a dark window with the agreement's text and ACCEPT and CANCEL under it) stands whole in the middle of the screen, not cut at any edge of the screen.
16. Click "ACCEPT".
17. Wait 10 seconds.
18. Take a screenshot called "editor".
19. Check that the map editor is up (a map in space with the HUD) and no licence window is left on the screen.
20. Expect the game is still running.
21. Expect "UI.log" not contain "PANIC".
22. Expect "Display8.log" not contain "VUID".
23. Stop the game.
24. Expect no crash.
