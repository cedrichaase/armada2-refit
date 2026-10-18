# The button bar is shown after a mission's opening cinematic (UI.asi, Game Settings' "Command Bar")

Resolutions: 1280x720
Mod: refit
Setup: ui/bench-asi.sh --no-grid
Timeout: 12 min

`ui/HUD.md`, "The button bar". Game Settings' "Command Bar" (`UIPlayer.ini`
`[HUD] BarHidden`, Shown when there is no file) is applied once per mission by
pressing the stock palette toggle when the palette mode (`+0x110`: 1 the bar, 0 the
floating palette) disagrees with it. Until ui 0.13.3 it compared the component's
hidden flag (`+0x16`) instead, which the engine also sets under a mission's opening
and before the first selection, so a "Shown" could press the toggle with the bar
already up and hide it for the mission. The player saw it at the start of new games;
on the bench the mode started at 0 with the flag set, where both checks agree, so
this run proves the mode is read and the bar comes up after a cinematic, and UI.log
records the mode and flag found. The first Federation mission opens on a cinematic;
Space skips it. Run it with `--install .` so the clone has the UI's chain.

The bench clone has no `UIPlayer.ini`, so the setting is Shown. After the briefing
the fleet is selected with a box over the middle of the screen, and the bar must
show its buttons.

1. Launch the game.
2. Wait for the main menu.
3. Click the single player emblem.
4. Wait for the single player screen.
5. Click the Federation campaign.
6. Wait for "Invasion".
7. Click "Invasion".
8. Click "O.K.".
9. Wait up to 300 seconds for the mission to start.
10. Wait 5 seconds.
11. Press space.
12. Wait up to 60 seconds for "BRIEFING".
13. Wait 3 seconds.
14. Click "OK".
15. Wait 2 seconds.
16. Expect "UI.log" contains "bar: shown as the player set it".
17. Drag from 150,120 to 1130,520.
18. Wait 2 seconds.
19. Expect "UI.log" contains "bar: menu=".
20. Move the mouse to 640,60.
21. Wait 1 second.
22. Take a screenshot called "fleet selected".
23. Check that at the bottom left, beside the minimap panel, there is a panel holding a 5x3 grid of round command buttons with a small letter (Q, W, E, R, A, S, Z, ...) at the top left of each.
24. Quit the game.
25. Expect no crash.
