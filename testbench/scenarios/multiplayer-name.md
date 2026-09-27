# The multiplayer name field is scaled with its menu

Resolutions: 4:3, 21:9
Mod: remastered
Reference: 800x600 stock
Stock shell: embed

Regression test for real edit controls in the scaled shell (menus 4.2.0). The
Multiplayer Connection screen's name field is a Win32 `Edit`, 399x32 at design
208,171, holding the player's name ("Player" by default). The shell draws the frame
around it into the scaled dialog, but the edit is a window of its own: unscaled, it sat
1:1 at its design coordinates, in the top-left corner of the screen, in a font sized for
800x600. The design rectangle below is the field's; the height floor is in design
pixels, where stock at 800x600 reads "Player" 17 tall. Typing a new name checks that
the moved field still takes the click and the keyboard. (The edit has no Ctrl+A, and the
bench's `Type` sends capitals without a Shift press, which the game reads as lower
case -- hence End, BackSpace and a lower-case name.)

1. Launch the game.
2. Wait for the main menu.
3. Click the multiplayer emblem.
4. Wait for the multiplayer connection screen.
5. Take a screenshot called "multiplayer connection".
6. Expect "Player" is visible inside design 208,171,399,32 and at least 10 design px tall.
7. Click at design 400,187.
8. Press End.
9. Press BackSpace 6 times.
10. Type "picard".
11. Wait 1 second.
12. Take a screenshot called "name typed".
13. Expect "picard" is visible inside design 208,171,399,32 and at least 10 design px tall.
14. Expect "Player" is not visible.
15. Click "Previous Menu".
16. Wait for the main menu.
17. Quit the game.
18. Expect no crash.
