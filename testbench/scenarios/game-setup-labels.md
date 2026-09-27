# Game Setup's option labels stay clear of the minimap

Resolutions: 4:3, 21:9
Mod: remastered

Regression test for `[Labels]` in `Menus.ini` (menus 4.3.0). On Instant Action's Game
Setup, stock draws "Shroud Off, Fog Off" and "Random Placement" on past their
rectangles and over the minimap's frame (x 618-621 in design space), and so do the
other states of the Shroud option. The plugin keeps them inside x 615 and says so in
Menus.log. The minimap itself is blank while the game rule "Minimap Hidden" is on;
clicking that option is how the host reveals it (the option then removes itself, as
in stock), so the map is shown before the second look.

1. Launch the game.
2. Wait for the main menu.
3. Click the instant action cube.
4. Wait for "GAME SETUP".
5. Move the mouse to design 200,450.
6. Take a screenshot called "game setup".
7. Expect "Menus.log" contains "label boxes listed: 1".
8. Expect "Menus.log" contains "Random Placement".
9. Expect "Random Placement" is visible inside design 520,200,95,25.
10. Click at design 507,170.
11. Move the mouse to design 200,450.
12. Wait 1 second.
13. Take a screenshot called "minimap shown".
14. Expect that the square frame to the right of the options holds a map, and that none of the option labels to its left ("Shroud ..., Fog ...", "Random Placement") runs over the grey frame line or into the map.
15. Click "Previous Menu".
16. Wait for the main menu.
17. Quit the game.
18. Expect no crash.
