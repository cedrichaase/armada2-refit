# The UI hosts the shell's menus: the main menu's spec, and Options' three settings screens

Resolutions: 1280x720
Mod: refit
Setup: ui/bench-asi.sh --menus
Timeout: 12 min

`ui/MENUS.md`. `--menus` sets `[Menus] Enable=1` and takes Menus.asi out of the clone:
every shell dialog then runs hidden and the UI draws it. The main menu is the example
spec (`ui/src/menus/screens/main_menu.rs`): a column of labelled buttons over the stock
art, faded. Its buttons are clicked at their screen place (1280x720 only), because OCR
can also find the words painted into the faded art behind them. Every other screen is
the generic, mirrored one, so the stock screens' `ui.json` targets and design
coordinates still land (UI.log carries the same `MoveWindow 800x600 @0,0 -> ...` line
Menus.asi logs, and the bench reads it). A setting changed through the UI must
persist: Bump Mapping is switched off, Graphics Settings left and opened again, and it
must still read Off. Sound Settings' Unit Speech is switched the same way, and Game
Settings opened. No step is judged (a judged step would add a 4:3 reference game).

1. Launch the game.
2. Wait for the main menu.
3. Expect "UI.log" contains "menus: hosted by the UI".
4. Expect "UI.log" contains "open dialog 291".
5. Expect "ARMADA II" is visible.
6. Expect "REPLAY INTRO" is visible.
7. Take a screenshot called "main menu spec".
8. Click at 228,390.
9. Wait 3 seconds.
10. Expect "Graphics Settings" is visible.
11. Expect "UI.log" contains "(proc 0x5cddb0)".
12. Click "Graphics Settings".
13. Wait 2 seconds.
14. Expect "Bump Mapping" is visible.
15. Click at design 570,240.
16. Wait 1 second.
17. Take a screenshot called "bump mapping off".
18. Click "Previous Menu".
19. Wait 2 seconds.
20. Click "Graphics Settings".
21. Wait 2 seconds.
22. Expect "Off" is visible.
23. Take a screenshot called "graphics settings again".
24. Click "Previous Menu".
25. Wait 2 seconds.
26. Click "Sound Settings".
27. Wait 2 seconds.
28. Click at design 400,403.
29. Wait 1 second.
30. Expect "Off" is visible.
31. Click "Previous Menu".
32. Wait 2 seconds.
33. Click "Game Settings".
34. Wait 2 seconds.
35. Expect "Game Difficulty" is visible.
36. Click "Previous Menu".
37. Wait 2 seconds.
38. Click "Previous Menu".
39. Wait 3 seconds.
40. Expect "ARMADA II" is visible.
41. Expect "UI.log" contains "lost) in".
42. Quit the game.
43. Expect no crash.
