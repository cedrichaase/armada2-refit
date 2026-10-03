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
`Menus.asi` draws black sides. `Planets.asi` needs no assets at all; its log line says
both of its sites in `Armada2.exe` were found and patched, and `QOL.asi`'s that it scaled
the right-drag pan speed 2.5x as the game read `RTS_CFG.h`.

1. Launch the game.
2. Wait for the main menu.
3. Take a screenshot called "main menu".
4. Expect "Menus.log" contains "hooks=7/7".
5. Expect "BinkProxy.log" contains "BinkProxy loaded".
6. Expect "BinkProxy.log" contains "bink bitmaps".
7. Expect "Planets.log" contains "sites patched 2/2".
8. Expect "QOL.log" contains "FASTSCROLL_COEFFICIENT now 0.0125".
9. Check that the main menu shows the game's own stock art — the Armada II logo and the Single Player, Instant Action and Multiplayer buttons — centred at full screen height, with plain black to its left and right.
10. Quit the game.
11. Expect the game to have exited.
12. Expect no crash.
