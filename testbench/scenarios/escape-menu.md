# Esc opens and closes the in-mission menu, repeatedly

Resolutions: 16:9
Mod: remastered
Launch: -nointro a2_borg01

Regression tests for menus 2.1.0 (`EscapeReturns=1`: Esc in the in-mission menu acts as
Return to Game) and menus 2.0.2 (Esc reopens the in-mission menu after it has been
closed: an embedded menu left the keyboard focus NULL, so every key was dropped until
a restart).

1. Launch the game.
2. Wait 45 seconds.
3. Press Escape.
4. Wait for the options menu.
5. Take a screenshot called "options open".
6. Press Escape.
7. Wait 2 seconds.
8. Expect "Graphics Settings" is not visible.
9. Press Escape.
10. Wait for the options menu.
11. Take a screenshot called "options reopened".
12. Click "Return to Game".
13. Wait 2 seconds.
14. Expect "Graphics Settings" is not visible.
15. Press Escape.
16. Wait for the options menu.
17. Expect "Menus.log" contains "Esc in the in-mission menu".
18. Quit the game.
19. Expect no crash.
