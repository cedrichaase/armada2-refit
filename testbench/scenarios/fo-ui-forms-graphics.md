# Fleet Operations: Roots with the UI: the graphics screen as Armada II's, over FleetOpsHook's form

Resolutions: 1600x900
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh Menus.Forms=1 Menus.Scene=1
Timeout: 14 min

Run with `a2test --game fo-roots` and `--install .` (`PLAN-FLEETOPS.md`, FO-6 step 7,
option C; `ui/MENUS.md`, "Roots' forms"). Roots' Graphics Settings form has a spec
(`menus/roots/graphics.rs`): Armada II's graphics screen over the form's controls, a
window with the display driver, resolution and anti-aliasing as dropdowns over the form's
combo boxes, graphics detail and gamma as sliders, the switches as check boxes that show
the state Roots' own controls hold (read from the controls' published properties), and
Previous Menu, which runs Roots' own Back and saves the profile. Each control is operated
and the effect is read back from the form (UI.log: `forms: <control> of
TGraphicOptionsForm now ...`).

1. Launch the game.
2. Wait up to 180 seconds for "UI.log" to say "forms: TMainForm is ours".
3. Wait 2 seconds.
4. Click "OPTIONS".
5. Wait up to 20 seconds for "UI.log" to say "forms: TOptionsMenuForm is ours".
6. Wait 2 seconds.
7. Click "Graphics Settings".
8. Wait up to 20 seconds for "UI.log" to say "forms: TGraphicOptionsForm is ours".
9. Wait 3 seconds.
10. Take a screenshot called "graphics".
11. Check that a window of ours titled "GRAPHICS", laid out as Armada II's settings screens, shows the captions Display Driver, Resolution and Anti-Aliasing over three dropdowns, Graphics Detail and Gamma Correction over two sliders, a column of check boxes (Windowed Mode, Team Colors, Hardware Vertex Processing, Bump Mapping, Asynchronous Cursor, Asynchronous Cursor (Windowed Mode)) of which Hardware Vertex Processing, Bump Mapping and Asynchronous Cursor are ticked, and a Previous Menu button at the bottom left, over the dimmed menu scene.
12. Expect "UI.log" contains "published properties of TJvgCheckBox".
13. Click "Team Colors".
14. Wait up to 10 seconds for "UI.log" to say "TeamColorsCheck of TGraphicOptionsForm now checked".
15. Click at 700,545.
16. Wait up to 10 seconds for "UI.log" to say "GammaCorrectionSlider of TGraphicOptionsForm now at".
17. Click "disabled".
18. Wait 1 second.
19. Click "4x".
20. Wait up to 10 seconds for "UI.log" to say "comboDevicePrimaryAA of TGraphicOptionsForm now chose 3".
21. Wait 2 seconds.
22. Take a screenshot called "changed".
23. Check that Team Colors is ticked, the Gamma Correction slider has moved right of its middle and the Anti-Aliasing dropdown reads 4x.
24. Click "Previous Menu".
25. Wait up to 20 seconds for "UI.log" to say "press on buttonBack".
26. Wait 3 seconds.
27. Take a screenshot called "options".
28. Check that Roots' Options screen of ours is shown again, a column of buttons in a window.
29. Expect "UI.log" not contain "PANIC".
30. Expect "Display8.log" not contain "VUID".
31. Stop the game.
32. Expect no crash.
