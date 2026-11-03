# A key a row took over is moved in the player's overlay (C2: UI.HUD.BarKeys to QOL.QOL.BarKeys)

Resolutions: 1280x720
Mod: refit
Overlay: UI.HUD:BarKeys=0
Timeout: 12 min

`tools/settings/README.md`, "Moved keys". `[UI.HUD] BarKeys` is where the player's choice of
the grid keys lived until phase 9; the key is QOL's now (`QOL.QOL.BarKeys`, row
`moved_from = "UI.HUD.BarKeys"`). The header seeds the old key with 0 before launch. At its
first frame the UI's settings hub finds the row, moves the value to `[QOL.QOL]`, removes the old
key and tells QOL: `UI.log` says `settings: RefitPlayer.ini [UI.HUD] BarKeys moved to [QOL.QOL]
BarKeys`, `QOL.log` says `settings: applied QOL.BarKeys=0`, and the position keys are off:
the bar's keys are the stock ones and no letter is drawn in its cells.

The match is `ui-hud-bar.md`'s.

1. Include "_enter-instant-action".
2. Expect "UI.log" contains "settings: RefitPlayer.ini [UI.HUD] BarKeys moved to [QOL.QOL] BarKeys".
3. Expect "QOL.log" contains "settings: applied QOL.BarKeys=0".
4. Press Home.
5. Wait 2 seconds.
6. Click at 640,175.
7. Wait up to 5 seconds for the logs the next steps expect.
8. Expect "UI.log" contains "| Assembler |".
9. Press q.
10. Wait 1 second.
11. Expect "QOL.log" contains "bar: position keys off (BarKeys=0)".
12. Expect "QOL.log" not contain "bar: key".
13. Take a screenshot called "keys off".
14. Check that the bar at the bottom shows a grid of square icon buttons with no letter in their corners.
15. Expect the game is still running.
16. Quit the game.
17. Expect no crash.
