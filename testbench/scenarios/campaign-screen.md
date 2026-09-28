# Campaign selection screen renders correctly at every aspect ratio

Resolutions: 4:3, 16:10, 16:9, 21:9
Mod: refit

The single-player screen has the second backdrop plate (menus 1.0.0, `[Backdrops] 2=`,
with `2.soften=` for the Tutorials glow). It is reached by clicking the emblem, not the
label, which also checks that Menus.asi maps input back into design space at every
resolution.

1. Launch the game.
2. Wait for the main menu.
3. Click the single player emblem.
4. Wait for the single player screen.
5. Take a screenshot called "campaign selection".
6. Check that the screenshot is not stretched compared with 4:3.
7. Check that "PREVIOUS MENU" is not stretched compared with 4:3.
8. Expect no black bars.
9. Check that the four campaign panels (Starfleet Academy, Federation, Klingon, Borg) are all fully visible and undistorted, and that the widescreen backdrop behind them has no visible seam or box around the central 4:3 area.
10. Click the previous menu button.
11. Wait for the main menu.
12. Quit the game.
13. Expect no crash.
