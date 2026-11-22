# Fleet Operations: Roots with the UI: the lobby, its Help and Create Game as Armada II's

Resolutions: 1280x960, 3440x1440
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh Menus.Forms=1 Menus.Scene=1 Menus.Enable=1
Timeout: 12 min

Run with `a2test --game fo-roots` and `--install .` (`ui/MENUS.md`, "Roots' dialogs, laid
out"; PLAN-FLEETOPS.md, decisions 600 to 606). Multiplayer's Local Area Network opens Roots'
"Please Wait" (`NetWaitingDlgProc`) and then the lobby (`InternetGameDlgProc`), both adopted
dialogs that were drawn as Roots' pictures. The lobby is Armada II's screen at 1.5 times its
design size, so Armada II's spec draws it (`screens::lobby::draw_with`); Help is Armada II's
text window, Create Game's name dialog a window of ours with the stock fields. The games list
is empty on the bench (no other game on the network). Every click is by a label.

1. Launch the game.
2. Wait up to 180 seconds for "UI.log" to say "forms: TMainForm is ours".
3. Wait 3 seconds.
4. Click "MULTIPLAYER".
5. Wait up to 20 seconds for "UI.log" to say "forms: TMultiplayerConnectionForm is ours".
6. Wait 2 seconds.
7. Click "Local Area Network".
8. Wait up to 30 seconds for "UI.log" to say "net waiting drawn by its spec".
9. Wait up to 30 seconds for "UI.log" to say "internet game drawn by its spec".
10. Wait 3 seconds.
11. Take a screenshot called "lobby".
12. Check that the screen shows a screen of ours titled LOBBY: a CURRENT GAMES window with a "Show all" drop-down and Join Game and Create Game buttons, a PLAYERS window at the right, a CHAT window across the bottom with a line of text and an empty field, and Previous Menu at the bottom left with Help and Main Menu at the bottom right; no picture of Roots' blue-framed lobby.
13. Click "Help".
14. Wait up to 20 seconds for "UI.log" to say "help drawn by its spec".
15. Wait 2 seconds.
16. Take a screenshot called "help".
17. Check that a window titled HELP over the dimmed lobby shows the network help guide's text in a scroll area and one OK button under it.
18. Click "OK".
19. Wait 3 seconds.
20. Click "Create Game".
21. Wait up to 20 seconds for "UI.log" to say "game name drawn by its spec".
22. Wait 2 seconds.
23. Take a screenshot called "game name".
24. Check that a window titled CREATE GAME over the dimmed lobby shows a "Game Name" caption over a field, a "Password" caption over an empty field, and OK and Cancel at its foot.
25. Click "Cancel".
26. Wait 3 seconds.
27. Click "Previous Menu".
28. Wait up to 20 seconds for "UI.log" to say "forms: TMultiplayerConnectionForm is ours".
29. Expect "UI.log" not contain "PANIC".
30. Expect "Display8.log" not contain "VUID".
31. Stop the game.
32. Expect no crash.
