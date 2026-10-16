# The UI hosts the shell's menus: the in-mission Options, a message, yes-or-no questions and the widgets-mode screens

Resolutions: 1280x720
Mod: refit
Setup: ui/bench-asi.sh --menus
Timeout: 15 min

`ui/MENUS.md`. In a mission Esc opens Options (`do_escapeMenu`) with its nine entries and
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
13. Expect "UI.log" contains "over the mission's last frame".
14. Take a screenshot called "options in the mission".
15. Click "Abort Mission".
16. Wait 2 seconds.
17. Expect "Are you sure you want to abort the mission?" is visible.
18. Take a screenshot called "abort question".
19. Click "No".
20. Wait 2 seconds.
21. Expect "Return to Game" is visible.
22. Expect "Are you sure you want to abort the mission?" is not visible.
23. Click "Exit to Windows".
24. Wait 2 seconds.
25. Expect "Are you sure you want to exit to Windows?" is visible.
26. Click "No".
27. Wait 2 seconds.
28. Click "Sound Settings".
29. Wait 2 seconds.
30. Expect "Music Volume" is visible.
31. Take a screenshot called "sound settings in the mission".
32. Click "Previous Menu".
33. Wait 2 seconds.
34. Click "Save Game".
35. Wait 3 seconds.
36. Expect "SAVE GAME" is visible.
37. Click at 640,639.
38. Wait 4 seconds.
39. Expect "Save Successful" is visible.
40. Take a screenshot called "save message".
41. Click "O.K.".
42. Wait 2 seconds.
43. Click "Previous Menu".
44. Wait 2 seconds.
45. Expect "Return to Game" is visible.
46. Click "Return to Game".
47. Wait 3 seconds.
48. Expect "Return to Game" is not visible.
49. Expect the game is still running.
50. Quit the game.
51. Expect no crash.
