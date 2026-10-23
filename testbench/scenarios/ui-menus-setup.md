# The UI's Game Setup: three players, a map, the Advanced panel, launch

Resolutions: 1280x720
Mod: refit
Setup: ui/bench-asi.sh --menus
Timeout: 15 min

`ui/MENUS.md`, "Game Setup". Instant Action's Game Setup is drawn by its spec
(`src/menus/screens/game_setup.rs`): a Players window of drop-downs over the stock
`ComboBox`es, a map window with the stock preview, settings buttons, and the bottom row.
Every choice goes through the stock controls: a drop-down's choice is the two clicks a
player gives the stock pull-down (open it, click the item), so the stock screen's own
handler sets it.

Player 2 becomes "Computer AI:Easy" and the empty third slot "Computer AI:Hard"; player
1's side becomes Borg. Random Placement is switched to Fixed Placement (the start
locations then show in the preview). SELECT MAP opens Map Select's spec: the list is
scrolled by its down arrow and Four Sisters is chosen. The Advanced panel's spec turns
Warp off and moves a slider (the stock scroll bar, mirrored, takes the pointer). LAUNCH
starts the match.

Coordinates are the spec's layout at 1280x720 (`testbench/operating/menus-setup.md`).
The drop-downs' items are clicked by place: their order is the stock combo boxes'.

1. Launch the game.
2. Wait for the main menu.
3. Expect "UI.log" contains "menus: hosted by the UI".
4. Click at 228,302.
5. Wait for "GAME SETUP".
6. Wait 2 seconds.
7. Expect "PLAYERS" is visible.
8. Expect "Computer AI:Medium" is visible.
9. Take a screenshot called "game setup".
10. Click at 200,147.
11. Wait 1 second.
12. Expect "Computer AI:Hard" is visible.
13. Click at 150,195.
14. Wait 1 second.
15. Click at 200,172.
16. Wait 1 second.
17. Click at 150,265.
18. Wait 1 second.
19. Click at 450,122.
20. Wait 1 second.
21. Expect "Species 8472" is visible.
22. Click at 400,169.
23. Wait 1 second.
24. Expect "Computer AI:Easy" is visible.
25. Expect "Computer AI:Hard" is visible.
26. Expect "Borg" is visible.
27. Click "Random Placement".
28. Wait 1 second.
29. Expect "Fixed Placement" is visible.
30. Take a screenshot called "three players".
31. Click "SELECT MAP".
32. Wait 3 seconds.
33. Expect "Acheron (2 Players)" is visible.
34. Take a screenshot called "map select".
35. Click at 918,577.
36. Wait 1 second.
37. Expect "Fat City (6 Players)" is visible.
38. Click "Four Sisters (4 Players)".
39. Wait 1 second.
40. Click at 420,612.
41. Wait 3 seconds.
42. Expect "FOUR SISTERS" is visible.
43. Expect "Acheron (2 Players)" is not visible.
44. Click "Advanced".
45. Wait 3 seconds.
46. Expect "ADVANCED OPTIONS" is visible.
47. Click "Warp Enabled".
48. Wait 1 second.
49. Expect "Warp Disabled" is visible.
50. Click at 560,156.
51. Wait 1 second.
52. Take a screenshot called "advanced".
53. Click "Close".
54. Wait 2 seconds.
55. Expect "ADVANCED OPTIONS" is not visible.
56. Expect "Computer AI:Hard" is visible.
57. Click "LAUNCH".
58. Wait up to 60 seconds for the logs the next steps expect.
59. Expect "UI.log" contains "mission: frames start".
60. Take a screenshot called "match".
61. Expect the game is still running.
62. Quit the game.
63. Expect no crash.
