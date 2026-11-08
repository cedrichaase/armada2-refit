# Fleet Operations: Roots: Internet - Online, joining by join code through a bench server: reached, refused by name

Resolutions: 1280x960
Mod: refit
Players: host, joiner
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh Menus.Forms=1 Menus.Scene=1 Menus.Enable=1 --online
Timeout: 25 min

Run with `a2test --game fo-roots` and `--install .` (PLAN-FLEETOPS.md, FO-8;
online/README.md, "On Fleet Operations: Roots"). Proton's own DirectPlay stays in the clones:
the game runs on `Online.asi`'s transport. `--online` puts `Online.asi` in with a bench
`a2online-server` on 127.0.0.1:23990 (`online/bench-asi.sh --server`). Roots' connection
screen is FleetOpsHook's form, drawn by the UI (`Menus.Forms=1`): its first button, the dead
internet service's, is *Internet - Online*, which tells `Online.asi` the entry is chosen
(`Online_Choose`) and presses Roots' Manual IP button. The host leaves the field blank and
creates a game; the server gives it a join code (of Roots' kind), shown in GAME SETUP's chat;
the joiner types the code it reads from the host's `Online.log`. On one machine the direct
path wins.

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
7. Take a screenshot called "connection".
8. Host: Click "Internet - Online".
9. Host: Wait up to 20 seconds for "UI.log" to say "forms: TManualIPForm is ours".
10. Host: Wait 2 seconds.
11. Host: Expect "Join code or host address" is visible.
12. Host: Click at 659,528.
13. Host: Wait up to 30 seconds for "Create Game".
14. Host: Wait 4 seconds.
15. Host: Click "Create Game".
16. Host: Wait up to 20 seconds for "UI.log" to say "game name drawn by its spec".
17. Host: Wait 2 seconds.
18. Host: Click "OK".
19. Host: Wait up to 30 seconds for "Join code".
20. Host: Take a screenshot called "host setup with code".
21. Joiner: Click "Internet - Online".
22. Joiner: Wait up to 20 seconds for "UI.log" to say "forms: TManualIPForm is ours".
23. Joiner: Wait 2 seconds.
24. Joiner: Click at 640,490.
25. Joiner: Wait 1 second.
26. Joiner: Type what follows "join code" in "Online.log" of host.
27. Joiner: Wait 1 second.
28. Joiner: Click at 659,528.
29. Joiner: Wait up to 60 seconds for "The Very First Map".
30. Joiner: Take a screenshot called "game list".
31. Joiner: Click "The Very First Map".
32. Joiner: Wait 2 seconds.
33. Joiner: Click "Join Game".
34. Joiner: Wait up to 60 seconds for "already has a player with your name".
35. Joiner: Take a screenshot called "refused by name".
36. Joiner: Expect "Online.log" contains "CONNECT_COMPLETE".
37. Host: Expect "Online.log" contains "INDICATE_CONNECT".
38. Expect "Online.log" contains "[0006]".
39. Expect "Online.log" contains "OUR TRANSPORT".
40. Expect "Online.log" contains "kind 0x14e91822".
41. Expect "Online.log" not contains "unsupported call".
42. Host: Expect "Online.log" contains "a joiner is coming".
43. Host: Expect "Online.log" contains "net host: joined".
44. Joiner: Expect "Online.log" contains "direct to the host".
45. Joiner: Expect "Online.log" contains "net joiner: connected".
46. Expect "UI.log" not contain "PANIC".
47. Joiner: Click "OK".
48. Joiner: Wait 2 seconds.
49. Joiner: Quit the game.
50. Host: Quit the game.
51. Expect no crash.
52. Expect "Online.log" contains "closed; datagrams sent".
