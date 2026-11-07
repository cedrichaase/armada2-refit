# Fleet Operations: Roots with the UI: Advanced Options as Armada II's, over FleetOpsHook's form

Resolutions: 1600x900
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh Menus.Forms=1 Menus.Scene=1
Timeout: 14 min

Run with `a2test --game fo-roots` and `--install .` (`PLAN-FLEETOPS.md`, FO-6 step 7,
option C; `ui/MENUS.md`, "Roots' forms"). Instant Action's setup screen is Roots' own dialog
(adopted and mirrored); its Advanced button opens Roots' Advanced Options form, which has a
spec (`menus/roots/advanced.rs`): four dropdowns over the form's combo boxes, check boxes
with the state Roots' controls hold, six sliders with their end captions, and OK, Cancel
and Default. A check box, a slider and a dropdown are each operated and read back from
the form; Cancel is Roots' own.

1. Launch the game.
2. Wait up to 180 seconds for "UI.log" to say "forms: TMainForm is ours".
3. Wait 2 seconds.
4. Click "INSTANT ACTION".
5. Wait 10 seconds.
6. Move the mouse to 864,800.
7. Wait 1 second.
8. Click at 864,800.
9. Wait up to 30 seconds for "UI.log" to say "forms: TAdvancedOptionsForm is ours".
10. Wait 3 seconds.
11. Take a screenshot called "advanced".
12. Check that a window of ours titled "GAME SETTINGS", laid out as Armada II's settings screens, shows the dropdowns Starting Resources, Cease Fire, AI Personality Type and Battleship Build Limit, three ticked check boxes (Give Units On, Ferengi On, Special Weapons Auto) and at the right six sliders (Game Speed, Weapon Strength, Ship Build Time, AI Build Time Handicap, AI Build Cost Handicap, Global Build Cost) with their end captions, over the dimmed setup screen, and OK, Cancel and Default buttons.
13. Click "Ferengi On".
14. Wait up to 10 seconds for "UI.log" to say "chkFerengi of TAdvancedOptionsForm now unchecked".
15. Click at 1200,293.
16. Wait up to 10 seconds for "UI.log" to say "sldrDamageModifier of TAdvancedOptionsForm now at".
17. Click at 550,360.
18. Wait 1 second.
19. Click at 500,430.
20. Wait up to 10 seconds for "UI.log" to say "cbbTechlevel of TAdvancedOptionsForm now chose 1".
21. Wait 2 seconds.
22. Take a screenshot called "changed".
23. Check that Ferengi On is not ticked, the Weapon Strength slider has moved to the right and the AI Personality Type dropdown reads Balanced AI Type.
24. Click "Cancel".
25. Wait up to 20 seconds for "UI.log" to say "press on btnClose".
26. Wait 3 seconds.
27. Expect "UI.log" not contain "PANIC".
28. Expect "Display8.log" not contain "VUID".
29. Stop the game.
30. Expect no crash.
