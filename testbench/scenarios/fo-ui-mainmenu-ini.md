# Fleet Operations: Roots with the UI: the main menu's default layout (Menu\mainmenu.ini absent)

Resolutions: 2150x900, 3440x1440
Mod: refit
Layers: display asiloader render3d ui
Setup: testbench/scenarios/fo-ui-mainmenu-ini.sh
Timeout: 15 min

Run with `a2test --game fo-roots` and `--install .`. With no `Data\Menu\mainmenu.ini` the
column takes the `fleetops` set's `[MainMenu]` (`ui/content/sets/fleetops/ui.ini`): Roots'
lettering with room under it before the first entry (`TitleGap`; PLAN-FLEETOPS.md, decisions
410 to 419; `ui/MENUS.md`, "The main menu's layout file"). `fo-ui-mainmenu-ini-custom` puts a
file there; `ui-menus-scene` is Armada II's menu, which the layout must leave as it was.

1. Launch the game.
2. Wait up to 180 seconds for "UI.log" to say "forms: TMainForm is ours".
3. Wait 4 seconds.
4. Take a screenshot called "main".
5. Expect "UI.log" contains "menus: main menu layout: the set's [MainMenu]".
6. Expect "UI.log" contains "lettering".
7. Check that the screen shows Roots' gold FLEET OPERATIONS and silver ROOTS lettering at the left, and under it a clear space, about as tall as one to two entries, before the first entry SINGLE PLAYER; the entries SINGLE PLAYER to EXIT follow in a column, all of it over the 3D space scene.
8. Expect "UI.log" not contain "PANIC".
9. Stop the game.
10. Expect no crash.
