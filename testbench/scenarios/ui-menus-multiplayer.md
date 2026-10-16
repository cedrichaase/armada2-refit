# The UI's multiplayer screens: connection, Manual IP, the lobby, Game Setup with two players

Resolutions: 1280x720
Mod: refit
Players: host, joiner
Setup: ui/bench-asi.sh --dplay --menus
Timeout: 20 min

`ui/MENUS.md`, "The setup and multiplayer specs". Both games draw every front-end
screen through the UI's specs: Multiplayer Connection (`multiplayer.rs`), the Manual IP
form (`forms.rs`), the Internet lobby (`lobby.rs`), the game's name (`forms.rs`) and
Game Setup with two humans (`game_setup.rs`). Microsoft's DirectPlay is in each clone
(`--dplay`), since Proton's cannot host. The flow is `multiplayer-two-players.md`'s,
clicked on our layouts (`testbench/operating/ui-multiplayer.md` has where things are at
1280x720): the host renames itself, takes *Internet - Manual IP* with the field empty
and creates a game; the joiner types this machine's address, finds the host's game in
the lobby's list and joins; a chat line each way through our chat field; the joiner
changes its side through our drop-down; the three LAUNCH presses start the match. The
joiner quits first. Names are typed in lower case (`multiplayer-name.md`).

1. Launch the game.
2. Wait for the main menu.
3. Expect "UI.log" contains "menus: hosted by the UI".
4. Click at 228,346.
5. Wait up to 20 seconds for "Internet - Manual IP".
6. Host: Take a screenshot called "connection".
7. Check that the multiplayer connection screen is drawn in the UI's style: a window with the player's name in a text field and one wide button per connection type, including Internet - Online.
8. Host: Click at 640,271.
9. Host: Press End.
10. Host: Press BackSpace 32 times.
11. Host: Type "host".
12. Host: Click "Internet - Manual IP".
13. Host: Wait up to 10 seconds for "O.K.".
14. Host: Take a screenshot called "manual ip".
15. Host: Click "O.K.".
16. Host: Wait up to 30 seconds for "Create Game".
17. Host: Take a screenshot called "host lobby".
18. Check that the lobby is drawn in the UI's style: a current games window with Create Game and Join Game, a players window, a chat window whose log shows the local IP address, and the way out at the bottom.
19. Host: Click "Create Game".
20. Host: Wait 3 seconds.
21. Host: Take a screenshot called "game name".
22. Host: Click "O.K.".
23. Host: Wait up to 30 seconds for "GAME SETUP".
24. Joiner: Click at 640,271.
25. Joiner: Press End.
26. Joiner: Press BackSpace 32 times.
27. Joiner: Type "joiner".
28. Joiner: Click "Internet - Manual IP".
29. Joiner: Wait up to 10 seconds for "O.K.".
30. Joiner: Click at 640,355.
31. Joiner: Type this machine's address.
32. Joiner: Click "O.K.".
33. Joiner: Wait up to 30 seconds for "host's Game".
34. Joiner: Take a screenshot called "game list".
35. Check that the joiner's lobby lists the host's game as a row of the current games window.
36. Joiner: Click "host's Game".
37. Joiner: Click "Join Game".
38. Joiner: Wait up to 30 seconds for "GAME SETUP".
39. Joiner: Click at 300,640.
40. Joiner: Type "hello from joiner".
41. Joiner: Press Return.
42. Host: Wait up to 20 seconds for "hello from joiner".
43. Host: Click at 300,640.
44. Host: Type "hello from host".
45. Host: Press Return.
46. Joiner: Wait up to 20 seconds for "hello from host".
47. Host: Take a screenshot called "host setup with joiner".
48. Check that the host's Game Setup shows two human players in the players window and both chat lines in the chat under it.
49. Host: Click "LAUNCH".
50. Host: Wait 5 seconds.
51. Joiner: Click "LAUNCH".
52. Joiner: Wait 5 seconds.
53. Host: Click "LAUNCH".
54. Host: Wait up to 180 seconds for "999999".
55. Joiner: Wait up to 60 seconds for "999999".
56. Take a screenshot called "match".
57. Expect the game is still running.
58. Joiner: Quit the game.
59. Host: Quit the game.
60. Expect no crash.
