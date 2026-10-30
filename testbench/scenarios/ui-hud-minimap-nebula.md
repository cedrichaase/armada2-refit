# The minimap shows the nebulae

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Setup: ui/bench-asi.sh --no-grid --scene ui-minimap-nebula HUD.Minimap=1
Timeout: 10 min

`ui/HUD.md`, "The minimap". Two nebulae and a Galaxy, no fog or shroud. The minimap
is the ours panel; the nebulae must show on it as stock's `CacheMap` draws them.

1. Launch the game.
2. Wait up to 90 seconds for the logs the next steps expect.
3. Expect "Scene.log" contains "scene ready".
4. Expect "UI.log" contains "minimap: ours".
5. Wait 3 seconds.
6. Take a screenshot called "minimap".
7. Check that the minimap at the bottom left shows two or more coloured blotches (the nebulae) on its dark map, besides a green dot for the ship.
8. Expect the game is still running.
