# The style gallery shows every theme and widget state (UI.asi, ui/STYLE.md)

Resolutions: 16:9
Mod: refit
Setup: ui/bench-asi.sh Demo.Style=1
Timeout: 8 min

`ui/STYLE.md`. `[Demo] Style=1` puts the style gallery where the Credits were: eight
cards over the menu plate (or, without one, a starfield gradient): the shell's theme
and the six races', each with buttons in every state, a toggle, a slider, a bar, a
list, a focused field, tabs and a tooltip, and a card with the type scale. The same
gallery is drawn over a mission and the loading look on the loading screen, but a
case that enters a match cannot quit through the menus with the gallery over them,
so those are looked at by hand (`testbench/operating/ui-style.md`).

1. Launch the game.
2. Wait for the main menu.
3. Expect "UI.log" contains "shell: hooked".
4. Click the credits.
5. Wait 3 seconds.
6. Take a screenshot called "gallery".
7. Expect "STYLE" is visible.
8. Expect "KLINGON" is visible.
9. Expect "SPECIES 8472" is visible.
10. Expect "UI.log" contains "theme: menu plate".
11. Check that the screen shows eight panels in two rows of four, each in a different style (gunmetal, LCARS lavender and gold, red Klingon, green Romulan, amber Cardassian, square Borg, purple 8472, and a type-scale card), with buttons, a toggle, a slider, a list and tabs in each, and nothing overlapping or cut off.
12. Press Escape.
13. Wait for the main menu.
14. Quit the game.
15. Expect no crash.
