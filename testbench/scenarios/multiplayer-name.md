# The multiplayer name field is scaled with its menu

Resolutions: 4:3, 21:9
Mod: refit
Reference: 800x600 stock
Stock shell: embed

Regression test for real edit controls in the scaled shell (menus 4.2.0). The
Multiplayer Connection screen's name field is a Win32 `Edit`, 399x32 at design
208,171, holding the player's name ("Player" by default). The shell draws the frame
around it into the scaled dialog, but the edit is a window of its own: unscaled, it sat
1:1 at its design coordinates, in the top-left corner of the screen, in a font sized for
800x600. The design rectangle below is the field's; the height floor is in design
pixels, where stock at 800x600 reads "Player" 17 tall. Typing a new name checks that
the moved field still takes the click and the keyboard. The field opens with the name
the player last saved (`save/shell.set`, which the clone copies), so it is cleared
first rather than expected to read "Player". (The edit has no Ctrl+A, and the bench's
`Type` sends capitals without a Shift press, which the game reads as lower case --
hence End, BackSpace and a lower-case name.)

1. Launch the game.
2. Wait for the main menu.
3. Click the multiplayer emblem.
4. Wait for the multiplayer connection screen.
5. Take a screenshot called "multiplayer connection".
6. Click at design 400,187.
7. Press End.
8. Press BackSpace 32 times.
9. Type "picard".
10. Wait 1 second.
11. Take a screenshot called "name typed".
12. Expect "picard" is visible inside design 208,171,399,32 and at least 10 design px tall.
13. Click "Previous Menu".
14. Wait for the main menu.
15. Quit the game.
16. Expect no crash.
