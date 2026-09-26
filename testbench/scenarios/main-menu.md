# Main menu renders correctly at every aspect ratio

Resolutions: 4:3, 16:10, 16:9, 21:9
Mod: remastered

The shell is 800x600 art. Menus.asi scales it to the screen height and centres it. The
outpainted backdrop plate fills the rest of the screen (menus 1.0.0, `Backdrops=1`), so
nothing should be stretched and nothing should be black. The 4:3 case is the reference
the others are measured against.

1. Launch the game.
2. Wait for the main menu.
3. Take a screenshot called "main menu".
4. Check that the screenshot is not stretched compared with 4:3.
5. Check that "SINGLE PLAYER" is not stretched compared with 4:3.
6. Expect no black bars.
7. Expect "Menus.log" contains "hooks=7/7".
8. Expect "Menus.log" contains "backdrop:".
9. Check that the main menu fills the full height of the screen, that the backdrop continues without a visible seam or hard edge into the areas left and right of the central 4:3 menu, and that no menu element is cut off at the edges.
10. Quit the game.
11. Expect the game to have exited.
12. Expect no crash.
