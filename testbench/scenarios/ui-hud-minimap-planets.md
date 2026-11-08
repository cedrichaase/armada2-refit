# The minimap shows planets without a box around them

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Setup: ui/bench-asi.sh --scene ui-minimap-planets HUD.Minimap=1
Timeout: 10 min

`ui/HUD.md`, "The minimap"; issue #155. Three planets, a dilithium moon and a Galaxy,
no fog or shroud. A planet or moon on the minimap is its class's radar sprite
(`gui_map.spr`: `map_planet`, `map_dmoon`, of the `additive` material, on an opaque
black ground in `Gminicon`). Stock adds it, so the black adds nothing; drawn with an
alpha blend, each planet sat in a black square.

1. Launch the game.
2. Wait up to 90 seconds for the logs the next steps expect.
3. Expect "Scene.log" contains "scene ready".
4. Expect "UI.log" contains "minimap: ours".
5. Expect "UI.log" contains "minimap: ours at".
6. Wait 3 seconds.
7. Take a screenshot called "minimap".
8. Check that the minimap at the bottom left shows three small round orange-brown planet symbols, each in the middle of a large magenta disc (its gravity well, from the map's picture), and a small blue moon symbol, besides a green dot for the ship; and that the magenta (or the dark blue map) shows right up to the edge of each round symbol: no planet or moon symbol sits on a black, dark or grey square patch that cuts into the magenta disc around it.
9. Expect the game is still running.
