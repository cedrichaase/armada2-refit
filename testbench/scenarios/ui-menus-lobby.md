# The UI's lobby, its Help and the game's name, one game (Armada II)

Resolutions: 1280x720
Mod: refit
Setup: ui/bench-asi.sh --dplay --menus
Timeout: 12 min

`ui/MENUS.md`, "The Internet lobby". One game, no joiner: *Internet - Manual IP* with the
address left blank hosts and opens the lobby (`lobby.rs`) with an empty games list; its Help
is the text window (`text_window.rs`) and Create Game the game's name (`forms.rs`). Written
when both specs became shared layouts with Roots (`lobby::draw_with`,
`text_window::text_window`; PLAN-FLEETOPS.md, decision 606): a run of this branch and one of
the tree before it give the same shots. Positions as `ui-menus-multiplayer.md`'s at 1280x720.

1. Launch the game.
2. Wait for the main menu.
3. Expect "UI.log" contains "menus: hosted by the UI".
4. Click at 228,346.
5. Wait up to 20 seconds for "Internet - Manual IP".
6. Click "Internet - Manual IP".
7. Wait up to 10 seconds for "leave blank to host".
8. Click at 659,416.
9. Wait up to 30 seconds for "Create Game".
10. Wait 3 seconds.
11. Take a screenshot called "lobby".
12. Check that the screen shows the lobby of ours: a title, a CURRENT GAMES window with an empty list and Join Game and Create Game buttons, a players window at the right, a CHAT window across the bottom with a "Local IP Address" line and an empty field, Previous Menu at the bottom left and Help and Main Menu at the bottom right.
13. Click "Help".
14. Wait up to 10 seconds for "NETWORK HELP GUIDE".
15. Wait 2 seconds.
16. Take a screenshot called "help".
17. Check that a window over the dimmed lobby shows the network help guide's text in a scroll area and one button under it.
18. Click "OK".
19. Wait 3 seconds.
20. Click "Create Game".
21. Wait up to 10 seconds for "Password".
22. Wait 2 seconds.
23. Take a screenshot called "game name".
24. Click "Cancel".
25. Wait 3 seconds.
26. Click "Previous Menu".
27. Wait up to 20 seconds for "Internet - Manual IP".
28. Expect "UI.log" not contain "PANIC".
29. Stop the game.
30. Expect no crash.
