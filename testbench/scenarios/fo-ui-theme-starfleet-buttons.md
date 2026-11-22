# Fleet Operations: Roots with the UI: Starfleet's dark buttons in the front end, at rest and blooming on hover

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh Menus.Forms=1 Menus.Scene=1
Timeout: 25 min

Run with `a2test --game fo-roots` and `--install .`. The fleetops set dresses Roots' menus in
Starfleet (`[Looks] Menus`, `ui/STYLE.md`, "Which theme shows"), whose buttons are dark with
their labels in the lamps' blue (`ui/STYLE.md`, "Starfleet"): Options shows a column of them;
hovering one makes its label bloom.

1. Launch the game.
2. Wait up to 180 seconds for "UI.log" to say "forms: TMainForm is ours".
3. Wait 3 seconds.
4. Move the mouse to 600,300.
5. Wait 1 second.
6. Click "OPTIONS".
7. Wait up to 20 seconds for "UI.log" to say "forms: TOptionsMenuForm is ours".
8. Wait 2 seconds.
9. Move the mouse into the bottom right corner.
10. Wait 1 second.
11. Take a screenshot called "options at rest".
12. Hover over "Sound Settings".
13. Wait 1 second.
14. Take a screenshot called "options hover".
15. Check that the Options window's buttons are near-black plates with blue labels, and that the hovered one's label glows with a soft blue halo and its edge turns blue.
16. Click "Previous Menu".
17. Wait up to 20 seconds for "UI.log" to say "forms: TMainForm is ours".
18. Expect "UI.log" not contain "PANIC".
19. Expect "Display8.log" not contain "VUID".
20. Stop the game.
21. Expect no crash.
