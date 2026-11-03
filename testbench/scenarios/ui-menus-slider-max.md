# A shell slider can be set back to its top: Graphics Detail, down to the least and up to the most

Resolutions: 1280x720
Mod: refit
Setup: ui/bench-asi.sh --menus Menus.Trace=1
Timeout: 8 min

`ui/MENUS.md`, "A `ShellScrollBar` is driven by one click". The stock bar maps a click on
its track onto its range over the track's length plus one, so the point that maps to the
range's top lies past the track: the player reached it with the far arrow button, and our
slider sent a click on nothing (round F: Graphics Detail could not be set back to its
maximum). The slider is clicked at its left end and at its right end; the UI logs what it
sent and, when the bar's own value matches, the value read back.

1. Launch the game.
2. Wait for the main menu.
3. Expect "UI.log" contains "menus: hosted by the UI".
4. Click at 228,390.
5. Wait 3 seconds.
6. Click "Graphics Settings".
7. Wait 2 seconds.
8. Expect "Graphics Detail" is visible.
9. Click at 653,453.
10. Wait 1 second.
11. Expect "UI.log" contains "scroll bar read back at 0 of 0..4".
12. Click at 1020,453.
13. Wait 1 second.
14. Expect "UI.log" contains "scroll bar read back at 4 of 0..4".
15. Take a screenshot called "graphics detail at max".
16. Click "Previous Menu".
17. Wait 2 seconds.
18. Click "Graphics Settings".
19. Wait 2 seconds.
20. Take a screenshot called "graphics detail still max".
21. Click "Previous Menu".
22. Wait 2 seconds.
23. Quit the game.
24. Expect no crash.
