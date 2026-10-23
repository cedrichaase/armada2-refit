# The UI's single-player screens: campaign, missions, save, load, tech tree, Admiral's Log

Resolutions: 1280x720
Mod: refit
Setup: ui/bench-asi.sh --menus
Timeout: 25 min

`ui/MENUS.md`, "Writing a screen spec". The campaign selection, the mission list, Save
Game, Load Game, the Technology Tree and the Admiral's Log are specs of `UI.asi` in the
shell theme, acting only through the stock controls. The campaign screen's cards are cuts
of the stock art (the Federation card opens the mission list, a click on the stock panel's
point); a mission list row and O.K. start the first Federation mission (about 100 s to its
briefing). In the mission the in-mission menu's Save Game takes a name typed into our text
field (the stock edit receives it: the save appears in the list), the Admiral's Log
shows its Score table and switches page, the Technology Tree shows the tree from the
edit's text, and Load Game loads the save after the abort confirmation.

Clicks that are not on a label are by place (1280x720 only): the main menu's column,
Save Game's name field and its Save button.

1. Launch the game.
2. Wait for the main menu.
3. Expect "UI.log" contains "menus: hosted by the UI".
4. Click at 228,258.
5. Wait 3 seconds.
6. Expect "Federation Campaign" is visible.
7. Take a screenshot called "campaign selection".
8. Check that the screenshot shows either four cards (Tutorials, Federation Campaign, Klingon Campaign, Borg Campaign), each with the faction's picture from the stock screen under its name, a Previous Menu button at the bottom left and a Load Game button at the bottom right, drawn in the dark blue and amber shell style (in a 2x2 grid over a dark backdrop); or, when render3d draws the menu scene, a column at the left of the four names each with a white emblem, then LOAD GAME and PREVIOUS MENU, over a 3D scene.
9. Click "Federation Campaign".
10. Wait 3 seconds.
11. Expect "Invasion" is visible.
12. Take a screenshot called "mission list".
13. Check that the mission names are listed in a window titled Select Mission over the dimmed campaign screen, with O.K. and Cancel under the list.
14. Click "Invasion".
15. Wait 1 second.
16. Take a screenshot called "mission picked".
17. Check that the row Invasion is highlighted.
18. Click "O.K.".
19. Wait up to 300 seconds for the briefing.
20. Wait 3 seconds.
21. Click "OK".
22. Wait 2 seconds.
23. Press Escape.
24. Wait 3 seconds.
25. Click "Save Game".
26. Wait 3 seconds.
27. Expect "SAVE GAME" is visible.
28. Take a screenshot called "save game".
29. Check that the saved games are listed with their dates at the right, a Name field with a name in it, and Previous Menu, Save Game and Delete Game at the bottom, over the dimmed mission.
30. Click at 700,607.
31. Press End.
32. Press BackSpace 30 times.
33. Type "uisingle".
34. Wait 1 second.
35. Expect "uisingle" is visible.
36. Click at 640,639.
37. Wait 4 seconds.
38. Expect "Save Successful" is visible.
39. Click "O.K.".
40. Wait 2 seconds.
41. Expect "uisingle" is visible.
42. Click "Previous Menu".
43. Wait 2 seconds.
44. Click "View Admiral's Log".
45. Wait 4 seconds.
46. Expect "GAME POINTS" is visible.
47. Take a screenshot called "admirals log".
48. Check that the Admiral's Log shows its eight tabs along the top (Score selected), the players at the left and a table of scores, with Save to text and Done at the bottom right.
49. Click "Timeline".
50. Wait 2 seconds.
51. Expect "Relative Power" is visible.
52. Take a screenshot called "admirals log timeline".
53. Click "Done".
54. Wait 2 seconds.
55. Click "Technology Tree".
56. Wait 3 seconds.
57. Expect "Starbase" is visible.
58. Take a screenshot called "technology tree".
59. Check that the tree is drawn with thin branch lines and dots before the names, the roots in amber, and an OK button below.
60. Click "OK".
61. Wait 2 seconds.
62. Click "Load Game".
63. Wait 3 seconds.
64. Take a screenshot called "load game".
65. Expect "uisingle" is visible.
66. Click "uisingle".
67. Wait 1 second.
68. Click at 640,639.
69. Wait 3 seconds.
70. Click "Yes".
71. Wait up to 60 seconds for the logs the next steps expect.
72. Expect "UI.log" contains "mission: frames resume".
73. Expect the game is still running.
74. Quit the game.
75. Expect no crash.
