# The UI hosts the shell's menus: Instant Action's setup, a match, the in-mission menu, Save and Load

Resolutions: 1280x720
Mod: refit
Setup: ui/bench-asi.sh --menus
Timeout: 15 min

`ui/MENUS.md`. Every dialog runs hidden in the UI's dialog host and is drawn and driven
by the UI. From the main menu's spec (Instant Action at its screen place, 1280x720
only), Game Setup shows the local player and a computer player as stock does (a hosting
bug once left it at its first, empty picture: `ui/MENUS.md`); its settings are
`ui-menus-setup.md`'s, and LAUNCH starts the match with the defaults.

In the match, Esc opens the in-mission menu over the mission's last frame, dimmed, and
a second Esc returns to the game (`[Menus] EscapeReturns`, as Menus.asi does). Save
Game is then used with a name typed into its edit box (the click puts the stock edit's
keyboard focus there; the field is cleared first: it opens with the map's name), the
confirmation is dismissed, and Load Game loads that save (clicked by its name), which
aborts the running mission after a Yes and loads it. Save and Load are the specs'
layouts, their buttons at the foot of the window.

1. Launch the game.
2. Wait for the main menu.
3. Expect "UI.log" contains "menus: hosted by the UI".
4. Click at 228,302.
5. Wait for "GAME SETUP".
6. Wait 2 seconds.
7. Expect "Computer AI:Medium" is visible.
8. Take a screenshot called "game setup".
9. Click "LAUNCH".
10. Wait 45 seconds.
11. Expect "UI.log" contains "dialog".
12. Take a screenshot called "match".
13. Press Escape.
14. Wait 3 seconds.
15. Expect "Return to Game" is visible.
16. Expect "UI.log" contains "over the mission's last frame".
17. Take a screenshot called "options over the mission".
18. Press Escape.
19. Wait 3 seconds.
20. Expect "Return to Game" is not visible.
21. Expect "UI.log" contains "Esc -> Return to Game".
22. Press Escape.
23. Wait 3 seconds.
24. Click "Save Game".
25. Wait 3 seconds.
26. Expect "SAVE GAME" is visible.
27. Click at 700,607.
28. Wait 1 second.
29. Press End.
30. Press BackSpace 30 times.
31. Type "uimenus".
32. Wait 1 second.
33. Expect "uimenus" is visible.
34. Take a screenshot called "save name".
35. Click at 640,639.
36. Wait 4 seconds.
37. Expect "Save Successful" is visible.
38. Click "O.K.".
39. Wait 2 seconds.
40. Click "Previous Menu".
41. Wait 2 seconds.
42. Click "Load Game".
43. Wait 3 seconds.
44. Expect "Instant Action" is visible.
45. Click "uimenus".
46. Wait 2 seconds.
47. Click at 640,639.
48. Wait 3 seconds.
49. Click "Yes".
50. Wait 45 seconds.
51. Expect "UI.log" contains "mission: frames resume".
52. Take a screenshot called "loaded".
53. Expect the game is still running.
54. Quit the game.
55. Expect no crash.
