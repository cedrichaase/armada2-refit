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

**display** (`display.modes`): Display8 takes the bail-out (display 0.17.0, P5: a stub
without the engine's mode requests died of a stack overflow; it forwarded to the old
chain until display 1.0.0), Display8.log names the hook and the test and says
`bail-out:`, and the game plays on the system's own Direct3D 8 with the plugins standing
down: the stock shell (4:3, centred with black sides, its Instant Action cube clicked in
screen pixels), then a skirmish.

**radar** (`ui.hud.radar`): UI.asi leaves the minimap to the engine. The Refit screen's
Interface tab shows the `Minimap` row greyed, and pointing at it shows the note
`Stands down: mapRadar: test mismatch (A2EngineTest.ini)`, and in a mission the minimap is the stock one (the
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
9. Display: Expect "Display8.log" contains "bail-out: without display.modes the stub could open no display; the game draws through ".
10. Display: Expect "Display8.log" not contains "PANIC".
11. Radar: Expect "UI.log" contains "engine: TEST: A2EngineTest.ini forces a mismatch for ui.hud.radar".
12. Radar: Expect "UI.log" contains "engine: ui.hud.radar stands down: ".
13. Radar: Expect "UI.log" contains "test mismatch (A2EngineTest.ini)".
14. Radar: Expect "UI.log" not contains "engine: ui.hud.panels stands down".
15. Radar: Expect "UI.log" not contains "PANIC".
16. Display: Check that the screenshot shows the main menu of Star Trek: Armada II (Single Player, Instant Action, Multiplayer, Options ...), not a black or garbled screen.
## Radar: the Refit screen
17. Radar: Click at 228,390.
18. Radar: Wait 3 seconds.
19. Radar: Expect "Refit settings" is visible.
20. Radar: Click "Refit settings".
21. Radar: Wait for the refit screen.
22. Radar: Click "Interface".
23. Radar: Wait 2 seconds.
24. Radar: Expect "Minimap" is visible.
25. Radar: Hover over 300,212.
26. Radar: Wait 1 second.
27. Radar: Expect "test mismatch" is visible.
28. Radar: Take a screenshot called "radar refit interface".
29. Radar: Press Escape.
30. Radar: Wait 2 seconds.
## Mission
31. Display: Click at 930,100.
32. Display: Wait for "GAME SETUP".
33. Display: Wait 2 seconds.
34. Display: Click "LAUNCH".
35. Display: Wait 60 seconds.
36. Display: Take a screenshot called "display mission".
37. Display: Check that the screenshot shows a space battle map of Star Trek: Armada II drawn by the game (stars, ships or stations, and the game's own interface), not a black or garbled screen.
38. Display: Expect the game is still running.
39. Radar: Quit the game.
40. Radar: Launch the game.
41. Radar: Wait for the main menu.
42. Radar: Click the single player emblem.
43. Radar: Wait for the single player screen.
44. Radar: Click the Borg campaign.
45. Radar: Wait for "Werewolf Pack".
46. Radar: Click "Werewolf Pack".
47. Radar: Wait 1 second.
48. Radar: Click "O.K.".
49. Radar: Wait up to 300 seconds for the briefing.
50. Radar: Wait 3 seconds.
51. Radar: Click "OK".
52. Radar: Wait 10 seconds.
53. Radar: Take a screenshot called "radar mission".
54. Radar: Check that the screenshot shows a Star Trek: Armada II mission whose minimap, in the bottom left, is the game's own (the stock, small, square one), with the panels of the Refit HUD around the map.
55. Radar: Expect "UI.log" contains "engine: ui.hud.radar stands down: ".
56. Radar: Expect "UI.log" not contains "minimap: ours at".
## End
57. Quit the game.
58. Expect no crash.
