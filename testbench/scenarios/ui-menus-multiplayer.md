# The UI's multiplayer screens: connection, Manual IP, the lobby, Game Setup with two players

Resolutions: 1280x720
Mod: refit
Players: host, joiner
Setup: ui/bench-asi.sh --dplay --menus
Timeout: 20 min

`ui/MENUS.md`, "The setup and multiplayer specs" and "The Internet lobby". Both games
draw every front-end screen through the UI's specs: Multiplayer Connection
(`multiplayer.rs`), the Manual IP form (`forms.rs`), the Internet lobby (`lobby.rs`),
the game's name (`forms.rs`) and Game Setup with two humans (`game_setup.rs`).
Microsoft's DirectPlay is in each clone (`--dplay`), since Proton's cannot host. The
flow is `multiplayer-two-players.md`'s, clicked on our layouts
(`testbench/operating/ui-multiplayer.md` has where things are at 1280x720): the host
renames itself in our name field, takes *Internet - Manual IP* with the field empty and
creates a game; the joiner types this machine's address into our field, finds the
host's game as a row of our lobby (a `GameSelectionSlot`, which the model reads since
this scenario was written) and joins; a chat line each way through our chat field (Enter
sends it); the joiner's side becomes Klingon through our drop-down; the three LAUNCH
presses start the match. The joiner quits first. Names are typed in lower case
(`multiplayer-name.md`). No judged step, so no 4:3 reference case doubles the games.

1. Launch the game.
2. Wait for the main menu.
3. Expect "UI.log" contains "menus: hosted by the UI".
4. Click at 228,346.
5. Wait up to 20 seconds for "Internet - Manual IP".
6. Host: Take a screenshot called "connection".
7. Host: Expect "Internet - Online" is visible.
8. Host: Click at 640,271.
9. Host: Press End.
10. Host: Press BackSpace 32 times.
11. Host: Type "host".
12. Host: Click "Internet - Manual IP".
13. Host: Wait up to 10 seconds for "leave blank to host".
14. Host: Take a screenshot called "manual ip".
15. Host: Click at 659,416.
16. Host: Wait up to 30 seconds for "Create Game".
17. Host: Take a screenshot called "host lobby".
18. Host: Expect "Local IP Address" is visible.
19. Host: Expect "CURRENT GAMES" is visible.
20. Host: Click "Create Game".
21. Host: Wait up to 10 seconds for "Password".
22. Host: Take a screenshot called "game name".
23. Host: Click at 659,430.
24. Host: Wait up to 30 seconds for "GAME SETUP".
25. Joiner: Click at 640,271.
26. Joiner: Press End.
27. Joiner: Press BackSpace 32 times.
28. Joiner: Type "joiner".
29. Joiner: Expect "hostjoiner" is not visible.
30. Joiner: Click "Internet - Manual IP".
31. Joiner: Wait up to 10 seconds for "leave blank to host".
32. Joiner: Click at 640,355.
33. Joiner: Type this machine's address.
34. Joiner: Click at 659,416.
35. Joiner: Wait up to 30 seconds for "host's Game".
36. Joiner: Take a screenshot called "game list".
37. Joiner: Click "host's Game".
38. Joiner: Click "Join Game".
39. Joiner: Wait up to 30 seconds for "GAME SETUP".
40. Joiner: Wait 3 seconds.
41. Joiner: Click at 300,640.
42. Joiner: Type "hello from joiner".
43. Joiner: Press Return.
44. Host: Wait up to 20 seconds for "hello from joiner".
45. Host: Click at 300,640.
46. Host: Type "hello from host".
47. Host: Press Return.
48. Joiner: Wait up to 20 seconds for "hello from host".
49. Joiner: Click at 370,172.
50. Joiner: Wait 1 second.
51. Joiner: Click at 370,263.
52. Joiner: Wait 2 seconds.
53. Joiner: Expect "Klingon" is visible.
54. Host: Take a screenshot called "host setup with joiner".
55. Host: Expect "joiner" is visible.
56. Host: Click "LAUNCH".
57. Host: Wait 5 seconds.
58. Joiner: Click "LAUNCH".
59. Joiner: Wait 5 seconds.
60. Host: Click "LAUNCH".
61. Host: Wait 90 seconds.
62. Expect "UI.log" contains "mission: frames start".
63. Take a screenshot called "match".
64. Expect the game is still running.
65. Joiner: Quit the game.
66. Host: Quit the game.
67. Expect no crash.
