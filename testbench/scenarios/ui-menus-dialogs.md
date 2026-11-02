# The UI hosts the shell's menus: the in-mission Options, a message, yes-or-no questions and the widgets-mode screens

Resolutions: 1280x720
Mod: refit
Setup: ui/bench-asi.sh --menus
Timeout: 15 min

`ui/MENUS.md`. In a mission Esc opens Options (`do_escapeMenu`) with its ten entries (the
nine stock ones and `Refit`, ours) and
Return to Game in the footer, as a spec over the mission's last frame, dimmed. Abort
Mission asks a yes-or-no question (`do_yesNo`), and No leaves Options as it was; Save
Game ends in a one-button message (`do_oneButton`, "Save Successful!") whose O.K. is
clicked; Exit to Windows asks again. Both questions are specs (`notice.rs`) over a
dimming of the screen under them, their words the stock labels'. The settings screens
open from the in-mission Options too. The words are the stock controls', so the steps click
by text. The screens are read through the model of the hidden dialogs (UI.log lists them).

1. Launch the game.
2. Wait for the main menu.
3. Click at 228,302.
4. Wait for "GAME SETUP".
5. Wait 2 seconds.
6. Click "LAUNCH".
7. Wait 45 seconds.
8. Press Escape.
9. Wait 3 seconds.
10. Expect "Technology Tree" is visible.
11. Expect "Exit to Windows" is visible.
12. Expect "Return to Game" is visible.
13. Expect "Refit" is visible.
14. Expect "UI.log" contains "over the mission's last frame".
15. Take a screenshot called "options in the mission".
16. Click "Abort Mission".
17. Wait 2 seconds.
18. Expect "Are you sure you want to abort the mission?" is visible.
19. Take a screenshot called "abort question".
20. Click "No".
21. Wait 2 seconds.
22. Expect "Return to Game" is visible.
23. Expect "Are you sure you want to abort the mission?" is not visible.
24. Click "Exit to Windows".
25. Wait 2 seconds.
26. Expect "Are you sure you want to exit to Windows?" is visible.
27. Click "No".
28. Wait 2 seconds.
29. Click "Sound Settings".
30. Wait 2 seconds.
31. Expect "Music Volume" is visible.
32. Take a screenshot called "sound settings in the mission".
33. Click "Previous Menu".
34. Wait 2 seconds.
35. Click "Save Game".
36. Wait 3 seconds.
37. Expect "SAVE GAME" is visible.
38. Click at 640,639.
39. Wait 4 seconds.
40. Expect "Save Successful" is visible.
41. Take a screenshot called "save message".
42. Click "O.K.".
43. Wait 2 seconds.
44. Click "Previous Menu".
45. Wait 2 seconds.
46. Expect "Return to Game" is visible.
47. Click "Return to Game".
48. Wait 3 seconds.
49. Expect "Return to Game" is not visible.
50. Expect the game is still running.
51. Quit the game.
52. Expect no crash.
