# Fleet Operations: Roots: two players with the refit's visual plugins, on Microsoft's DirectPlay: found, reached, refused by name

Resolutions: 1280x960
Mod: refit
Players: host, joiner
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh Menus.Forms=1 Menus.Scene=1 Menus.Enable=1 --dplay
Timeout: 25 min

Run with `a2test --game fo-roots` and `--install .` (PLAN-FLEETOPS.md, FO-8, the acceptance
scenario `fo-multiplayer`). Two games of Roots on this machine with Display8, Render3D, QOL
and the UI, on Microsoft's DirectPlay (`--dplay`: `online/reference-dplay.sh`, the clones
only), which also puts `Online.asi` in as a tracer: its entry is not chosen, so every call
goes to Microsoft's DirectPlay and `Online.log` records it. Roots' front-end forms are drawn
by the UI (`Menus.Forms=1`): the bench's clicks on Roots' own VCL buttons were lost, while
ours press them. The host chooses *Internet - Manual IP* with a blank address (the lobby,
then Create Game); the joiner types this machine's address, picks the game by its map (The
Very First Map, the default) and joins.

**What it shows today (2026-11-08): the joiner reaches the game and Roots refuses it by
name.** Roots (as shipped too, without a plugin of ours: DxWrapper, then DXVK) loses the
player's name as its connection screen closes: the name's first two bytes become a small
number (0x0006 with the UI's forms, 0x0012 with Roots' own; the shell's `save/shell.set`
shows it), so every player's name is one control character, `SetPeerInfo` hands DirectPlay
that (`Online.log`: `name "?" [0006]`), the game is "'s Game", and the host's game refuses
a second player with "The game you tried to join already has a player with your name."
The steps stop at that message and check the transport got that far. When the name is
mended (PLAN-FLEETOPS.md, FO-8, decisions 660 onwards), the launch and the match go back in
(`multiplayer-two-players.md` has them).

1. Launch the game.
2. Wait up to 180 seconds for "UI.log" to say "forms: TMainForm is ours".
3. Wait 3 seconds.
4. Click "MULTIPLAYER".
5. Wait up to 20 seconds for "UI.log" to say "forms: TMultiplayerConnectionForm is ours".
6. Wait 3 seconds.
7. Host: Click "Internet - Manual IP".
8. Host: Wait up to 20 seconds for "UI.log" to say "forms: TManualIPForm is ours".
9. Host: Wait 2 seconds.
10. Host: Click at 659,528.
11. Host: Wait up to 30 seconds for "Create Game".
12. Host: Wait 4 seconds.
13. Host: Click "Create Game".
14. Host: Wait up to 20 seconds for "UI.log" to say "game name drawn by its spec".
15. Host: Wait 2 seconds.
16. Host: Click "OK".
17. Host: Wait up to 30 seconds for "LAUNCH".
18. Host: Take a screenshot called "host setup".
19. Joiner: Click "Internet - Manual IP".
20. Joiner: Wait up to 20 seconds for "UI.log" to say "forms: TManualIPForm is ours".
21. Joiner: Wait 2 seconds.
22. Joiner: Click at 640,490.
23. Joiner: Wait 1 second.
24. Joiner: Type this machine's address.
25. Joiner: Wait 1 second.
26. Joiner: Click at 659,528.
27. Joiner: Wait up to 60 seconds for "The Very First Map".
28. Joiner: Take a screenshot called "game list".
29. Joiner: Click "The Very First Map".
30. Joiner: Wait 2 seconds.
31. Joiner: Click "Join Game".
32. Joiner: Wait up to 60 seconds for "already has a player with your name".
33. Joiner: Take a screenshot called "refused by name".
34. Joiner: Expect "Online.log" contains "CONNECT_COMPLETE".
35. Host: Expect "Online.log" contains "INDICATE_CONNECT".
36. Expect "Online.log" contains "[0006]".
37. Expect "Online.log" not contains "OUR TRANSPORT".
38. Expect "UI.log" not contain "PANIC".
39. Joiner: Click "OK".
40. Joiner: Wait 2 seconds.
41. Joiner: Quit the game.
42. Host: Quit the game.
43. Expect no crash.
