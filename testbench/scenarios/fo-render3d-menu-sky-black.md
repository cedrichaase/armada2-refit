# Fleet Operations: Roots' menu scene under env_black

Resolutions: 2100x900
Mod: refit
Layers: display asiloader render3d ui
Setup: testbench/scenarios/fo-render3d-menu-sky.sh env_black
Timeout: 40 min

Run with `a2test --game fo-roots` and `--install .`. Roots' main menu over the menu scene
with `Sky=env_black` (PLAN-FLEETOPS.md, decision 370): four shots 35 seconds apart, about a
quarter of the 150 second orbit each, then the campaign selection's view.

1. Launch the game.
2. Wait up to 180 seconds for "UI.log" to say "forms: TMainForm is ours".
3. Wait 4 seconds.
4. Take a screenshot called "orbit0".
5. Wait 35 seconds.
6. Take a screenshot called "orbit1".
7. Wait 35 seconds.
8. Take a screenshot called "orbit2".
9. Wait 35 seconds.
10. Take a screenshot called "orbit3".
11. Click "SINGLE PLAYER".
12. Wait 8 seconds.
13. Take a screenshot called "single".
14. Expect "UI.log" not contain "PANIC".
15. Stop the game.
16. Expect no crash.
