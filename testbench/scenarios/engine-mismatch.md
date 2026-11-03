# A hook forced to fail on a stock exe: the plugin that owns it stands down, the rest runs

Resolutions: 1280x720
Mod: refit
Players: display, radar
Setup: tools/engine/bench-mismatch.sh by-player display/bench-mode.sh stub
Timeout: 30 min

Run with `--install .`. `tools/engine/bench-mismatch.sh by-player` writes the bench-only
`A2EngineTest.ini` into each clone (`[Test] Mismatch=<globs>`): the hooks it names stand
down exactly as a failed check would, on a stock executable, so each plugin's
stand-down path runs for real. The globs follow the player's name
(`tools/engine/bench-mismatch.sh`). Two players, so two games at once; the variants that
cannot run yet are in their own files (`engine-mismatch-hulls.md`,
`engine-mismatch-cplugins.md`).

**display** (`display.modes`): Display8 forwards to the old chain (display 0.17.0, P5: a
stub without the engine's mode requests died of a stack overflow), so the shell is the
stock one (800x600 at the top left), and Display8.log names the hook and the test.

**radar** (`ui.hud.radar`): UI.asi leaves the minimap to the engine. The Refit screen's
Interface tab shows the `Minimap` row greyed with the note `Stands down: ...: test
mismatch (A2EngineTest.ini)`, and in a mission the minimap is the stock one (the
other panels are ours). Display8 is on its stub here, so UI.asi draws on it.

## Main menu
1. Launch the game.
2. Wait up to 300 seconds for the main menu.
3. Wait 5 seconds.
4. Display: Take a screenshot called "display shell".
5. Display: Expect "Display8.log" contains "engine: TEST: A2EngineTest.ini forces a mismatch for display.modes".
6. Display: Expect "Display8.log" contains "engine: display.modes stands down: ".
7. Display: Expect "Display8.log" contains "test mismatch (A2EngineTest.ini)".
8. Display: Expect "Display8.log" contains "engine: build armada2/gog-1.1".
9. Display: Expect "Display8.log" not contains "PANIC".
10. Radar: Expect "UI.log" contains "engine: TEST: A2EngineTest.ini forces a mismatch for ui.hud.radar".
11. Radar: Expect "UI.log" contains "engine: ui.hud.radar stands down: ".
12. Radar: Expect "UI.log" contains "test mismatch (A2EngineTest.ini)".
13. Radar: Expect "UI.log" not contains "engine: ui.hud.panels stands down".
14. Radar: Expect "UI.log" not contains "PANIC".
15. Display: Check that the screenshot shows the Star Trek: Armada II main menu of the game's own shell (the stock one, small at the top left of the screen), not a black or garbled screen.
## Radar: the Refit screen
16. Radar: Click at 228,390.
17. Radar: Wait 3 seconds.
18. Radar: Expect "Refit" is visible.
19. Radar: Click "Refit".
20. Radar: Wait for the refit screen.
21. Radar: Click "Interface".
22. Radar: Wait 2 seconds.
23. Radar: Expect "Minimap" is visible.
24. Radar: Expect "test mismatch" is visible.
25. Radar: Take a screenshot called "radar refit interface".
26. Radar: Press Escape.
27. Radar: Wait 2 seconds.
## Mission
28. Display: Click at 640,90.
29. Display: Wait for "GAME SETUP".
30. Display: Wait 2 seconds.
31. Display: Click "LAUNCH".
32. Display: Wait 60 seconds.
33. Display: Take a screenshot called "display mission".
34. Display: Check that the screenshot shows a space battle map of Star Trek: Armada II drawn by the game (stars, ships or stations, and the game's own interface), not a black or garbled screen.
35. Display: Expect "Display8.log" contains "passthrough: the engine's first draw".
36. Radar: Quit the game.
37. Radar: Launch the game.
38. Radar: Wait for the main menu.
39. Radar: Click the single player emblem.
40. Radar: Wait for the single player screen.
41. Radar: Click the Borg campaign.
42. Radar: Wait for "Werewolf Pack".
43. Radar: Click "Werewolf Pack".
44. Radar: Wait 1 second.
45. Radar: Click "O.K.".
46. Radar: Wait up to 300 seconds for the briefing.
47. Radar: Wait 3 seconds.
48. Radar: Click "OK".
49. Radar: Wait 10 seconds.
50. Radar: Take a screenshot called "radar mission".
51. Radar: Check that the screenshot shows a Star Trek: Armada II mission whose minimap, in the bottom left, is the game's own (the stock, small, square one), with the panels of the Refit HUD around the map.
52. Radar: Expect "UI.log" contains "engine: ui.hud.radar stands down: ".
53. Radar: Expect "UI.log" not contains "minimap: ours at".
## End
54. Quit the game.
55. Expect no crash.
