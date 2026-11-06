# Fleet Operations: Roots with the UI: the graphics screen, generic, over FleetOpsHook's form

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh Menus.Forms=1
Timeout: 12 min

Run with `a2test --game fo-roots` and `--install .` (`PLAN-FLEETOPS.md`, FO-6 step 7,
option C; `ui/MENUS.md`, "Roots' forms"). Roots' Graphics Settings form has no spec: it is
drawn generic (`Generic=widgets`, Roots' default): its combo boxes as dropdowns of ours
(their entries read through their windows), its labels as ours, its check boxes and
sliders as crops of the form's own picture with the mouse forwarded to them. Previous Menu
runs Roots' own Back, which saves the profile, and Options comes back.

1. Launch the game.
2. Wait up to 180 seconds for "UI.log" to say "forms: TMainForm is ours".
3. Wait 2 seconds.
4. Click "Options".
5. Wait up to 20 seconds for "UI.log" to say "forms: TOptionsMenuForm is ours".
6. Wait 2 seconds.
7. Click "Graphics Settings".
8. Wait up to 20 seconds for "UI.log" to say "forms: TGraphicOptionsForm is ours".
9. Wait 3 seconds.
10. Take a screenshot called "graphics".
11. Check that a window of ours titled "GRAPHICS" shows the labels Display Driver, Resolution and Anti-Aliasing over three dropdowns of ours, check boxes (Windowed Mode, Team Colors, Hardware Vertex Processing and others) and two sliders, and a Previous Menu button at the bottom left.
12. Click "Previous Menu".
13. Wait up to 20 seconds for "UI.log" to say "closed after".
14. Wait 2 seconds.
15. Take a screenshot called "options".
16. Expect "UI.log" contains "press on buttonBack".
17. Expect "UI.log" not contain "PANIC".
18. Expect "Display8.log" not contain "VUID".
19. Stop the game.
20. Expect no crash.
