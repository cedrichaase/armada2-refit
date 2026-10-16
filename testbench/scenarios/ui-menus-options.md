# The UI hosts the shell's menus: the main menu's spec, and Options with its three settings screens as specs

Resolutions: 1280x720
Mod: refit
Setup: ui/bench-asi.sh --menus
Timeout: 12 min

`ui/MENUS.md`. `--menus` sets `[Menus] Enable=1` and takes Menus.asi out of the clone:
every shell dialog then runs hidden and the UI draws it. The main menu, Options and the
Graphics, Sound and Game Settings screens are specs (`ui/src/menus/screens/`): a column
of buttons, and forms of captions over sliders (the stock `ShellScrollBar`s), buttons
that step through their choices, and drop-downs (Display Mode). The words are the stock
controls' own, so the steps click by text; the controls that have none (a switch is a
button labelled "On") are clicked at their screen place, at 1280x720 only. A setting
changed through the UI must persist, exactly as in stock: Bump Mapping is switched off,
Graphics Settings left and opened again, and it must still read Off; the display mode
list opens and a choice closes it; the Music Volume slider is clicked in the middle and
Unit Speech switched, then Sound Settings opened again; Game Difficulty is stepped. No
step is judged (a judged step would add a 4:3 reference game).

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
12. Take a screenshot called "options spec".
13. Click "Graphics Settings".
14. Wait 2 seconds.
15. Expect "Bump Mapping" is visible.
16. Expect "Gamma Correction" is visible.
17. Click at 839,294.
18. Wait 1 second.
19. Take a screenshot called "bump mapping off".
20. Click at 440,239.
21. Wait 1 second.
22. Expect "800x600x32" is visible.
23. Take a screenshot called "display mode list".
24. Click at 440,348.
25. Wait 1 second.
26. Expect "800x600x32" is not visible.
27. Click "Previous Menu".
28. Wait 2 seconds.
29. Click "Graphics Settings".
30. Wait 2 seconds.
31. Expect "Off" is visible.
32. Take a screenshot called "graphics settings again".
33. Click "Previous Menu".
34. Wait 2 seconds.
35. Click "Sound Settings".
36. Wait 2 seconds.
37. Expect "Music Volume" is visible.
38. Click at 640,179.
39. Click at 640,424.
40. Wait 1 second.
41. Expect "Off" is visible.
42. Take a screenshot called "sound settings".
43. Click "Previous Menu".
44. Wait 2 seconds.
45. Click "Sound Settings".
46. Wait 2 seconds.
47. Expect "Off" is visible.
48. Take a screenshot called "sound settings again".
49. Click "Previous Menu".
50. Wait 2 seconds.
51. Click "Game Settings".
52. Wait 2 seconds.
53. Expect "Game Difficulty" is visible.
54. Expect "AI Build Cost Handicap" is visible.
55. Click at 839,198.
56. Wait 1 second.
57. Expect "Hard" is visible.
58. Take a screenshot called "game settings".
59. Click "Previous Menu".
60. Wait 2 seconds.
61. Click "Previous Menu".
62. Wait 3 seconds.
63. Expect "ARMADA II" is visible.
64. Expect "UI.log" contains "lost) in".
65. Quit the game.
66. Expect no crash.
