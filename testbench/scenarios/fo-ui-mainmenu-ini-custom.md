# Fleet Operations: Roots with the UI: a Menu\mainmenu.ini of the bench's: right side, text title, one entry hidden, one relabelled

Resolutions: 2150x900
Mod: refit
Layers: display asiloader render3d ui
Setup: testbench/scenarios/fo-ui-mainmenu-ini.sh custom
Timeout: 15 min

Run with `a2test --game fo-roots` and `--install .`. The setup puts a `Data\Menu\mainmenu.ini`
in the clone (`fo-ui-mainmenu-ini.sh`): `Side=Right`, a text title (`TitleImage=none`, two
lines, the second orange), `Y=160`, `TitleGap=60`, wider gaps and larger entries, a dim shade,
`MapEditor=0` and `SinglePlayer=Campaign`. Every key is the file's: Roots' set says none of it.

1. Launch the game.
2. Wait up to 180 seconds for "UI.log" to say "forms: TMainForm is ours".
3. Wait 4 seconds.
4. Take a screenshot called "custom".
5. Expect "UI.log" contains "menus: main menu layout: Menu\mainmenu.ini".
6. Check that the column stands at the RIGHT of the screen with its text right-aligned: a small LAYOUT FILE line over a large orange TEST MENU with an orange rule under it, then a clear gap, then the entries Campaign, INSTANT ACTION, MULTIPLAYER, OPTIONS, CREDITS, EXIT; there is no MAP EDITOR entry and no gold lettering picture.
7. Expect "UI.log" not contain "PANIC".
8. Stop the game.
9. Expect no crash.
