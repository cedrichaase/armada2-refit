# Fleet Operations: Roots with the UI: the Warning prompt (Exit to Desktop, End Mission) is a dialog of ours

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui
Launch: -nointro save\ia-fed-kli.sav
Setup: testbench/bench/roots-save.sh ia-fed-kli display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh --scene game Menus.Forms=1
Timeout: 12 min

Run with `a2test --game fo-roots` and `--install .` (`ui/MENUS.md`, "Roots' forms"). Esc in a skirmish opens
Roots' Options form in its mission mode; Exit to Desktop and End Mission each ask Roots'
Warning form (`TYesNoForm`), which the UI draws as a dialog of its own (a themed panel, the
question in our font, the form's icon smaller, Yes and No); No closes it, Esc answers No.

The game starts straight in an Instant Action save (Federation against the Klingon AI,
`testbench/bench/roots-save.sh`; `../README.md`, "Roots straight into a map, and scenes on
Roots"), so the prompt wears the Federation's Starfleet look.

1. Launch the game.
2. Wait up to 240 seconds for "Scene.log" to say "scene ready".
3. Move the mouse into the bottom right corner.
4. Move the mouse to 700,300.
5. Wait 2 seconds.
6. Press Escape.
7. Wait up to 20 seconds for "UI.log" to say "forms: TOptionsMenuForm is ours".
8. Wait 3 seconds.
9. Click "Exit to Desktop".
10. Wait up to 20 seconds for "UI.log" to say "forms: TYesNoForm is ours".
11. Wait 3 seconds.
12. Take a screenshot called "warning exit".
13. Check that a dialog of ours titled WARNING, in the Federation's dark Starfleet frame and buttons, shows a small warning triangle and the whole question "Are you sure you want to exit to Windows?", with Yes and No once each.
14. Click "No".
15. Wait up to 20 seconds for "UI.log" to say "closed after".
16. Wait 3 seconds.
17. Click "End Mission".
18. Wait up to 20 seconds for "UI.log" to say "forms: TYesNoForm is ours".
19. Wait 3 seconds.
20. Take a screenshot called "warning end mission".
21. Check that the same dialog shows the End Mission question whole, with Yes and No once each.
22. Press Escape.
23. Wait up to 20 seconds for "UI.log" to say "closed after".
24. Wait 3 seconds.
25. Take a screenshot called "escape answered no".
26. Check that the Warning dialog is gone and the Options window is shown again over the mission.
27. Click "Return".
28. Wait up to 20 seconds for "UI.log" to say "closed after".
29. Wait 3 seconds.
30. Take a screenshot called "mission".
31. Check that the skirmish is shown again with no menu over it.
32. Expect "UI.log" not contain "PANIC".
33. Expect "Display8.log" not contain "VUID".
34. Stop the game.
35. Expect no crash.
