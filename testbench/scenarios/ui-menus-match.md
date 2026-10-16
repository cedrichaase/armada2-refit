# The UI hosts the shell's menus: Instant Action's setup, a match, the in-mission menu, Save and Load

Resolutions: 1280x720
Mod: refit
Setup: ui/bench-asi.sh --menus
Timeout: 15 min

`ui/MENUS.md`. Every dialog runs hidden in the UI's dialog host and is drawn and driven
by the UI. From the main menu's spec (Instant Action at its screen place, 1280x720
only), Game Setup is the generic mirrored screen: the local player and a computer
player must show in it as they do in stock (a hosting bug once left it at its first,
empty picture: `ui/MENUS.md`), and the two clicks of `ui-runtime.md` change player 2 to
"Computer AI:Easy" and player 1's side to Borg. LAUNCH starts the match.

In the match, Esc opens the in-mission menu over the mission's last frame, dimmed, and
a second Esc returns to the game (`[Menus] EscapeReturns`, as Menus.asi does). Save
Game is then used with a name typed into its edit box (the click puts the stock edit's
keyboard focus there; the field is cleared first: it opens with the map's name), the
confirmation is dismissed, and Load Game loads that save (the second row: the clone
holds one save of the player's own; OCR reads the row as one word, "Action-uimenus", so
it is clicked by place), which aborts the running mission after a Yes and loads it.

1. Launch the game.
2. Wait for the main menu.
3. Expect "UI.log" contains "menus: hosted by the UI".
4. Click at 228,302.
5. Wait for "GAME SETUP".
6. Wait 2 seconds.
7. Expect "Computer AI:Medium" is visible.
8. Take a screenshot called "game setup".
9. Click at design 124,115.
10. Wait 1 second.
11. Click at design 124,143.
12. Wait 1 second.
13. Click at design 246,95.
14. Wait 1 second.
15. Click at design 246,123.
16. Wait 1 second.
17. Expect "Computer AI:Easy" is visible.
18. Expect "Borg" is visible.
19. Click "LAUNCH".
20. Wait 45 seconds.
21. Expect "UI.log" contains "dialog".
22. Take a screenshot called "match".
23. Press Escape.
24. Wait 3 seconds.
25. Expect "Return to Game" is visible.
26. Expect "UI.log" contains "over the mission's last frame".
27. Take a screenshot called "options over the mission".
28. Press Escape.
29. Wait 3 seconds.
30. Expect "Return to Game" is not visible.
31. Expect "UI.log" contains "Esc -> Return to Game".
32. Press Escape.
33. Wait 3 seconds.
34. Click "Save Game".
35. Wait 3 seconds.
36. Expect "SAVE GAME" is visible.
37. Click at design 600,516.
38. Wait 1 second.
39. Press End.
40. Press BackSpace 12 times.
41. Type "uimenus".
42. Wait 1 second.
43. Expect "uimenus" is visible.
44. Take a screenshot called "save name".
45. Click at design 400,579.
46. Wait 4 seconds.
47. Expect "Save Successful" is visible.
48. Click "O.K.".
49. Wait 2 seconds.
50. Click "Previous Menu".
51. Wait 2 seconds.
52. Click "Load Game".
53. Wait 3 seconds.
54. Expect "Instant Action" is visible.
55. Click at design 400,116.
56. Wait 2 seconds.
57. Click at design 400,579.
58. Wait 3 seconds.
59. Click "Yes".
60. Wait 45 seconds.
61. Expect "UI.log" contains "mission: frames resume".
62. Take a screenshot called "loaded".
63. Expect the game is still running.
64. Quit the game.
65. Expect no crash.
