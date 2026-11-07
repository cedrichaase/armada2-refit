# Fleet Operations: Roots with the UI: the map editor's own menu clear of our top-right buttons

Resolutions: 1280x960, 3440x1440
Mod: refit
Layers: display asiloader render3d qol ui
Overlay: UI.Menus:Forms=1 UI.Menus:Enable=1 UI.Menus:Scene=1 UI.Menus:Credits=1 UI.UI:Cursor=1 UI.HUD:Minimap=1 UI.HUD:Info=1 UI.HUD:UnitView=1 UI.HUD:Tooltips=1 UI.HUD:Bar=1 UI.HUD:BarLabels=1 QOL.QOL:BarKeys=1
Timeout: 12 min

Run with `a2test --game fo-roots` and `--install .` (PLAN-FLEETOPS.md, decision 504). In the
map editor Roots draws its own menu ("Edit Menu", the team line, F1 to F9 and "Navigate with
TAB and F keys") in the top-right corner, under where our top-right buttons stand; the
heading was covered. In the editor (`UserProfile+0x84`) the buttons stand beside the resource
strip at the top left instead. The overlay is the user's own. Every click is by a label.

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
14. Click "ACCEPT".
15. Wait up to 20 seconds for "UI.log" to say "buttons: in the map editor".
16. Wait 8 seconds.
17. Take a screenshot called "editor".
18. Check that the editor's own menu at the top right ("Edit Menu", a team line and the F1 to F9 entries: Map Objects, Borg, Dominion, ...) is whole on the screen with no panel or button of ours over it (the map itself may show behind the text), and the row of square buttons stands at the top left just right of the resource strip.
19. Expect "UI.log" not contain "PANIC".
20. Expect "Display8.log" not contain "VUID".
21. Stop the game.
22. Expect no crash.
