# The Admiral's Log renders correctly on a map, at every aspect ratio

Resolutions: 4:3, 16:10, 16:9, 21:9
Mod: refit
Launch: -nointro a2_borg01

Regression test for menus 2.0.1. The log's tab panes are embedded as its children, so
they close with it. Its owner-drawn buttons (tabs, Save, Done) are scaled with the
screen instead of drawing 1:1 in the top-left corner. The game is launched straight into
the `a2_borg01` map (a bare argument is a mission name; see `menus/run-wine.sh`), which
skips the menus and the campaign cinematic.

1. Launch the game.
2. Wait 45 seconds.
3. Press Escape.
4. Wait for the options menu.
5. Click "View Admiral's Log".
6. Wait for the admiral's log.
7. Take a screenshot called "admirals log".
8. Expect "Done" is visible.
9. Expect "Military" is visible.
10. Check that the screenshot is not stretched compared with 4:3.
11. Check that "GAME POINTS" is not stretched compared with 4:3.
12. Check that the Admiral's Log is drawn as one scaled screen: its row of tabs (Score, Military, Economy, Timeline, Battles, Ships, Build, Tally) sits along the top of the log's frame, the Save and Done buttons sit at its bottom right, all at the same scale as the frame, and no button or small copy of the log appears in the top-left corner of the screen.
13. Click "Done".
14. Wait 2 seconds.
15. Expect "GAME POINTS" is not visible.
16. Expect the options menu.
17. Take a screenshot called "after the log".
18. Check that no score table or part of the Admiral's Log is left on screen over the options menu.
19. Quit the game.
20. Expect no crash.
