# The UI hosts the shell's menus: the generic screen as our widgets (`Generic=widgets`)

Resolutions: 1280x720
Mod: refit
Setup: ui/bench-asi.sh --menus Menus.Generic=widgets
Timeout: 8 min

`ui/MENUS.md`, "The generic screen". A screen without a spec, drawn from the model of
the hidden stock dialog as `crate::widgets`: here the Multiplayer connection screen, its
name edit as the widgets' text field (a click focuses it, typing edits our copy and is
sent to the stock edit as its new text), its buttons as ours. The name is read back from
the stock edit each frame, so what shows is the stock edit's text. (The generic
screen's list and drop-down have no screen on the bench that uses them: the lists the
shell draws itself are made of buttons.)

1. Launch the game.
2. Wait for the main menu.
3. Click at 228,346.
4. Wait 3 seconds.
5. Expect "MULTIPLAYER CONNECTION" is visible.
6. Expect "Internet - Manual IP" is visible.
7. Click at 700,224.
8. Press End.
9. Type "zq".
10. Wait 1 second.
11. Expect "zq" is visible.
12. Take a screenshot called "name typed".
13. Press BackSpace 2 times.
14. Wait 1 second.
15. Expect "zq" is not visible.
16. Click "Previous Menu".
17. Wait 3 seconds.
18. Expect "ARMADA II" is visible.
19. Quit the game.
20. Expect no crash.
