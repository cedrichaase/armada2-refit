# An old UIPlayer.ini's "Command Bar: Hidden" moves to RefitPlayer.ini and still hides the bar (UI.asi, the settings hub)

Resolutions: 1280x720
Mod: refit
Setup: tools/settings/bench-overlay.sh --into UIPlayer.ini HUD:BarHidden=1 ui/bench-asi.sh
Timeout: 12 min

`tools/settings/README.md`, "The overlay", and `ui/HUD.md`, "The button bar". Until ui
0.29.0 Game Settings' "Command Bar" was kept in `UIPlayer.ini` as `[HUD] BarHidden`.
Since then it is `[UI.HUD] BarHidden` of the player's overlay, `RefitPlayer.ini`, and
at its first frame the UI's settings hub carries a non-zero old value over (when the
overlay has none) and renames the old file `UIPlayer.ini.migrated`. The clone starts
with an old file saying Hidden and no overlay; after the migration the overlay holds the
key, the old file is renamed, and the first mission applies Hidden: the stock palette
toggle is pressed so that no bar of ours shows after the briefing. Run it with
`--install .` so the clone has the UI's chain.

1. Launch the game.
2. Wait for the main menu.
3. Wait up to 30 seconds for the logs the next steps expect.
4. Expect "UI.log" contains "settings: UIPlayer.ini migrated to RefitPlayer.ini [UI.HUD] (BarHidden=1 carried over); renamed UIPlayer.ini.migrated".
5. Expect "UI.log" contains "settings: RefitPlayer.ini [UI.HUD] BarHidden=1 written".
6. Expect "RefitPlayer.ini" contains "BarHidden=1".
7. Expect "UIPlayer.ini.migrated" contains "BarHidden=1".
8. Click the single player emblem.
9. Wait for the single player screen.
10. Click the Federation campaign.
11. Wait for "Invasion".
12. Click "Invasion".
13. Click "O.K.".
14. Wait up to 300 seconds for the mission to start.
15. Wait 5 seconds.
16. Press space.
17. Wait up to 60 seconds for "BRIEFING".
18. Wait 3 seconds.
19. Click "OK".
20. Wait 2 seconds.
21. Expect "UI.log" contains "bar: hidden as the player set it".
22. Expect "UI.log" contains "settings: RefitPlayer.ini [UI.HUD] BarHidden=1 (UI.ini has no BarHidden)".
23. Drag from 150,120 to 1130,520.
24. Wait 2 seconds.
25. Move the mouse to 640,60.
26. Wait 1 second.
27. Take a screenshot called "fleet selected".
28. Check that at the bottom left, beside the minimap panel, there is no panel holding a 5x3 grid of round command buttons.
29. Quit the game.
30. Expect no crash.
