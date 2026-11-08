# Fleet Operations: Roots with the UI: the HUD's icons are Roots' own art (the bar, the top-right buttons)

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui qol
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh HUD.Bar=1
Overlay: UI.HUD:Bar=1 UI.HUD:BarLabels=1 UI.HUD:Minimap=1 QOL.QOL:BarKeys=1
Timeout: 14 min

Run with `a2test --game fo-roots` and `--install .`. FO-6, "The bar as the grid" (`PLAN-FLEETOPS.md`,
decision 337): Roots keeps most of its button art in DDS sheets (`all_interface_buttons03` to `11`),
and the UI read a sprite's texture as TGA only, so every button whose sprite sits on one of them
showed its name instead (the user's `cycle_support` and `Wgesconv`). The texture is now read with
the game's extensions in its order (`.tga`, then `.dds`) and either format decoded
(`engine/sprite.rs`). The Starbase's four weapons (the bastion and the three trades) and the
construction ship's Tractor Beam draw from those sheets; `UI.log` names each texture the first time
(`sprite: <name> -> texture <file> 512x512`). And on Roots every HUD icon of ours that stands for a
game sprite is Roots' own art first (decision 340, `icons::game_art_first`): the bar's buttons, the
top-right buttons (menu, comms, idle ship, view, objectives; their sprites were never drawn by the
engine, so their art comes from their lines in Roots' `.spr` files, decision 343); the UI's flat
icons only where Roots' sprite has no art. Roots blanks the minimap's filter sprites (an empty
rectangle in its `.spr`), so that strip keeps its words (decision 341).

1. Include "_fo-render3d-skirmish".
2. Expect "UI.log" contains "bar: ours as the grid".
3. Move the mouse to 700,300.
4. Wait 1 second.
5. Select the Federation Starbase (the big station) alone, by a click on it or a box round it that holds no ship, then, as your last action, run the step "Move the mouse to 700,200", and stop once UI.log's last "bar: grid" line names all_bastion.
6. Wait 2 seconds.
7. Expect "UI.log" contains "texture all_interface_buttons0".
8. Expect "UI.log" not contain "NOT FOUND".
9. Take a screenshot called "starbase icons".
10. Check that every filled cell of the grid shows a picture (the bottom row's four weapons included), none a word in place of one, and that each Roots tile fills its cell with no second plate of ours around it.
11. Expect "UI.log" contains "icon: menu sprite menu_button art yes".
12. Expect "UI.log" contains "icon: objectives sprite objectives_button art yes".
13. Check that each of the five buttons at the top right is a square tile picture in the race's colours (lines, a swirl, a question mark, an eye, a tick), with no round button or second plate drawn around or behind it.
14. Expect "UI.log" contains "mapfilter_ship_button not drawn by the engine yet: its .spr line".
15. Expect "UI.log" not contain "PANIC".
16. Expect "Display8.log" not contain "VUID".
17. Quit the game.
18. Expect the game to have exited.
19. Expect no crash.
