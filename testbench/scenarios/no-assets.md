# The game runs with every layer installed and no assets built

Resolutions: 16:9
Mod: refit
Assets: none

What someone gets who installs this repository without making a texture pack. `A2_DATA`
is an empty directory, so the asset layers — textures, the loading-screen model, the
replacement movies, the menu plates — have nothing to install and must say so rather
than fail; the code layers install as usual, the cutscene proxy included. With no
`.mp4` beside any `.bik`, `binkw32.dll` passes every movie to the real DLL
(`binkw32_orig.dll`), which is what `bink ...` lines in its log mean. With no plate,
`Menus.asi` draws black sides.

1. Launch the game.
2. Wait for the main menu.
3. Take a screenshot called "main menu".
4. Expect "Menus.log" contains "hooks=7/7".
5. Expect "BinkProxy.log" contains "BinkProxy loaded".
6. Expect "BinkProxy.log" contains "bink bitmaps".
7. Check that the main menu shows the game's own stock art — the Armada II logo and the Single Player, Instant Action and Multiplayer buttons — centred at full screen height, with plain black to its left and right.
8. Quit the game.
9. Expect the game to have exited.
10. Expect no crash.
