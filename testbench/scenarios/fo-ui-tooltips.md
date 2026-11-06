# Fleet Operations: Roots with the UI: the tooltips at the pointer ours, Roots' icons and figures as words and digits

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh HUD.Tooltips=1
Timeout: 14 min

Run with `a2test --game fo-roots` and `--install .`. The user's report of 2026-11-06: with
`HUD.Tooltips=1` (Roots ships it at 0; the player turned it on in the Refit screen) almost
every tooltip showed odd symbols and question marks. Roots' tooltips are written in its
fonts' own codes: red figures (0x7f for 0, 0x81 for 1, ...), resource icons (0x85
dilithium, 0x86 tritanium, 0x87 supplies, ...), unit classes drawn as words (0xa4
"Scout", 0xf8 "Station"), cost lines set apart with `$`, and tabs; the UI drew those
bytes as Latin-1 in its own font. It now reads the game's text as Windows-1252 and writes
the content set's `[Glyphs]` (`ui/content/sets/fleetops/ui.ini`) as words and digits
(`ui/src/text/game.rs`; PLAN-FLEETOPS.md, decision 160). `fo-ui-tooltips-off` is the A
side, Roots' own tooltips.

The hovers are `fo-render3d-rings`'s box selection of the start fleet (the station and two
construction ships): the bar then holds thirteen buttons 70 px apart, the station's
Bastion at 882, Buy Dilithium at 952 and the Tractor Beam last at 1162; the starbase is the
big station at 840,380. The log checks read the text the UI drew (`tooltip: "..."`, its
control characters escaped as `\u{..}`). A build item's cost line (`$` between figures and
icons) is not reached here: a box round the station alone does not select it reliably on the
bench, and the bar's buttons move with the selection. `render3d/hosttest` (`sets.rs`,
`ui_fleetops_glyphs`) holds the cost line, the hitpoints, the classes and the bullet from a
player's `UI.log` to what the UI now draws.

1. Include "_fo-render3d-skirmish".
2. Wait 5 seconds.
3. Move the mouse to 700,300.
4. Wait 1 second.
5. Drag from 30,120 to 1250,640.
6. Wait 3 seconds.
7. Take a screenshot called "selected".
8. Move the mouse to 1162,788.
9. Wait 1 second.
10. Move the mouse to 1164,790.
11. Wait 4 seconds.
12. Take a screenshot called "tractor beam tooltip".
13. Expect "400" is visible.
14. Move the mouse to 700,300.
15. Wait 1 second.
16. Move the mouse to 840,380.
17. Wait up to 15 seconds for "UI.log" to say "Weak vs: Artillery".
18. Wait 2 seconds.
19. Take a screenshot called "unit tooltip".
20. Check that the tooltip beside the pointer names the starbase and reads as plain text: its DPS and hitpoints in digits, the word "Station", and "Weak vs:" followed by unit class names, with no question marks, boxes or stray symbols.
21. Move the mouse to 700,300.
22. Wait 1 second.
23. Move the mouse to 882,788.
24. Wait 1 second.
25. Move the mouse to 884,790.
26. Wait up to 15 seconds for "UI.log" to say "1000 Dilithium / 1000 Tritanium".
27. Wait 2 seconds.
28. Take a screenshot called "bastion tooltip".
29. Check that the Activate Bastion tooltip reads as plain text: bullets before its lines, "33 percent", and a cost of 1000 Dilithium and 1000 Tritanium, with no question marks, boxes or stray symbols.
30. Move the mouse to 700,300.
31. Wait 1 second.
32. Move the mouse to 952,788.
33. Wait 1 second.
34. Move the mouse to 954,790.
35. Wait up to 15 seconds for "UI.log" to say "Buy Dilithium for".
36. Wait 2 seconds.
37. Take a screenshot called "buy dilithium tooltip".
38. Expect "UI.log" contains " Hitpoints".
39. Expect "UI.log" contains "Weak vs: Artillery, Berserker, Battleship".
40. Expect "UI.log" contains "Station".
41. Expect "UI.log" not contain "\u{".
42. Expect "UI.log" not contain "PANIC".
43. Expect "Display8.log" not contain "VUID".
44. Expect "Display8.log" not contain "device removed".
45. Quit the game.
46. Expect the game to have exited.
47. Expect no crash.
