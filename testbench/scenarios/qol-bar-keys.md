# The command bar's position keys are QOL.asi's (QOL-1)

Resolutions: 1280x720
Mod: refit
Timeout: 12 min

`qol/README.md`, "QOL-1". Run it on a checkout: `./a2test run qol-bar-keys --install . -j 1`.
Since phase 9 the keys (Q W E R T / A S D F G / Z X C V B) are `qol/bar.c`'s: it wraps the
bar's keyboard step, asks the UI's `UI_BarCell` which stock button is in a cell, and
presses it; the UI keeps the layout and draws the key `QOL_BarLabel` names. `QOL.log`
says `bar: position keys on (the Refit bar is drawn)` and names each key (`bar: key Q`);
`UI.log` keeps the layout (`bar: menu=0 ... | Q=build ...`) and the click.
`BarKeys` is a live setting of QOL.ini (`[QOL.QOL]` in `RefitPlayer.ini`): off, the keys
stand down at the next step and the labels go. `qol-bar-keys-stock.md` is the same with
no UI.asi in the game.

The match and the camera are `ui-hud-bar.md`'s (Home first; the first Assembler is then
at about 640,175, 1280x720).

1. Include "_enter-instant-action".
2. Expect "UI.log" contains "bar: ours".
3. Expect "QOL.log" contains "--- QOL BarKeys=1  -> Q W E R T / A S D F G / Z X C V B press the Refit bar's cells, patched".
4. Note "Home puts the camera on the base; select the first Assembler there".
5. Press Home.
6. Wait 2 seconds.
7. Click at 640,175.
8. Wait up to 5 seconds for the logs the next steps expect.
9. Expect "UI.log" contains "| Assembler |".
10. Expect "UI.log" contains "Q=build W=order E=form R=aiMenu A=stop S=transport Z=gtracbm".
11. Note "the UI draws the labels QOL names".
12. Take a screenshot called "labels on".
13. Check that the bar at the bottom shows a grid of square icon buttons, each with a small yellow letter in its top-left corner (Q W E R in the top row, A S in the middle, Z at the bottom).
14. Note "Q is the build opener: QOL presses the UI's button, the stock bar goes to the build list".
15. Press q.
16. Wait 1 second.
17. Expect "QOL.log" contains "bar: position keys on".
18. Expect "QOL.log" contains "bar: key Q".
19. Expect "UI.log" contains "T=cancel".
20. Expect "UI.log" contains "G=back".
21. Note "G is back".
22. Press g.
23. Wait 1 second.
24. Expect "QOL.log" contains "bar: key G".
25. Expect "Assembler" is visible.
26. Note "BarKeys off, live: the keys and the labels go".
27. Overlay QOL.QOL:BarKeys=0
28. Wait up to 10 seconds for the logs the next steps expect.
29. Expect "QOL.log" contains "settings: applied QOL.BarKeys=0".
30. Press q.
31. Wait 1 second.
32. Expect "QOL.log" contains "bar: position keys off (BarKeys=0)".
33. Take a screenshot called "labels off".
34. Check that the bar at the bottom shows the grid of icon buttons with no letter in their corners.
35. Note "back on, live".
36. Overlay --unset QOL.QOL:BarKeys
37. Wait up to 10 seconds for the logs the next steps expect.
38. Expect "QOL.log" contains "settings: applied QOL.BarKeys=1".
39. Press Return.
40. Type "qqq".
41. Press Return.
42. Wait 1 second.
43. Expect the game is still running.
44. Quit the game.
45. Expect no crash.
