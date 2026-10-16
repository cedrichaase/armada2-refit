# The UI's multiplayer boxes: comms, giving, private chat, the load wait and a dropped player

Resolutions: 1280x720
Mod: refit
Players: host, joiner
Setup: ui/bench-asi.sh --dplay --menus HUD.Comm=1 HUD.DropPlayer=1 HUD.LoadWait=1 Loading.Enable=1 --for joiner Loading.Hold=8
Timeout: 20 min

`ui/HUD.md`, "Comms, the drop box and the load wait". Two games through the UI's menus
(as `ui-menus-multiplayer.md`, by Manual IP), the joiner on the host's team (Team 1:
the game lets only allies give each other resources) and the host's Advanced panel
switched from trading by convoys to instant trade. The joiner's loading screen waits 8 s half-way
(`[Loading] Hold`, `--for joiner`), so the host, done first, shows our load-wait box
(the joiner's bar under 100%; the stock drop box alternates with it while the joiner
does not answer). In the match: the joiner opens our comm panel with the
stock top-right comm button, selects the host and presses PRIVATE, then says a line,
which reaches the host; the host's panel gives the joiner 1000 dilithium, which the
joiner is told of. Then the joiner's game is stopped, and the host gets our
dropped-player box, whose Eliminate (enabled once the game lets it vote) removes the
joiner. Coordinates: `testbench/operating/ui-multiplayer.md` (the panel hangs from a fixed height, so its first resource row's
give button is at 1015,359 whatever the race and the number of resources; the themes'
insets move it a few pixels, so the clicks aim at the middle of each control).

1. Launch the game.
2. Wait for the main menu.
3. Expect "UI.log" contains "comm: CommDisplay ours".
4. Click at 228,346.
5. Wait up to 20 seconds for "Internet - Manual IP".
6. Host: Click at 640,271.
7. Host: Press End.
8. Host: Press BackSpace 32 times.
9. Host: Type "host".
10. Host: Click "Internet - Manual IP".
11. Host: Wait up to 10 seconds for "leave blank to host".
12. Host: Click at 659,416.
13. Host: Wait up to 30 seconds for "Create Game".
14. Host: Click "Create Game".
15. Host: Wait up to 10 seconds for "Password".
16. Host: Click at 659,430.
17. Host: Wait up to 30 seconds for "GAME SETUP".
18. Host: Click "Advanced".
19. Host: Wait up to 10 seconds for "ADVANCED OPTIONS".
20. Host: Click "Trade Using Convoys".
21. Host: Wait 1 second.
22. Host: Click "Close".
23. Joiner: Click at 640,271.
24. Joiner: Press End.
25. Joiner: Press BackSpace 32 times.
26. Joiner: Type "joiner".
27. Joiner: Click "Internet - Manual IP".
28. Joiner: Wait up to 10 seconds for "leave blank to host".
29. Joiner: Click at 640,355.
30. Joiner: Type this machine's address.
31. Joiner: Click at 659,416.
32. Joiner: Wait up to 30 seconds for "host's Game".
33. Joiner: Click "host's Game".
34. Joiner: Click "Join Game".
35. Joiner: Wait up to 30 seconds for "GAME SETUP".
36. Joiner: Wait 3 seconds.
37. Joiner: Click at 637,172.
38. Joiner: Wait 1 second.
39. Joiner: Click at 601,197.
40. Joiner: Wait 2 seconds.
41. Host: Click "LAUNCH".
42. Host: Wait 5 seconds.
43. Joiner: Click "LAUNCH".
44. Joiner: Wait 5 seconds.
45. Host: Click "LAUNCH".
46. Host: Wait 90 seconds.
47. Expect "UI.log" contains "mission: frames start".
48. Host: Expect "UI.log" contains "load wait: shown".
49. Joiner: Click at 1194,16.
50. Joiner: Wait 2 seconds.
51. Joiner: Expect "COMMS/ALLIANCES" is visible.
52. Joiner: Take a screenshot called "comm panel".
53. Joiner: Click "host".
54. Joiner: Click "PRIVATE".
55. Joiner: Wait 1 second.
56. Joiner: Click "DONE".
57. Joiner: Wait 1 second.
58. Joiner: Press Return.
59. Joiner: Type "psst host".
60. Joiner: Press Return.
61. Host: Wait up to 20 seconds for "psst host".
62. Joiner: Expect "UI.log" contains "comm: press msg3".
63. Host: Click at 1194,16.
64. Host: Wait 2 seconds.
65. Host: Click "joiner".
66. Host: Wait 1 second.
67. Host: Take a screenshot called "giving".
68. Host: Click at 1015,359.
69. Joiner: Wait up to 20 seconds for "gave you".
70. Host: Click "DONE".
71. Joiner: Stop the game.
72. Host: Wait up to 60 seconds for "unresponsive".
73. Host: Take a screenshot called "drop box".
74. Host: Expect "UI.log" contains "drop: shown".
75. Host: Wait 50 seconds.
76. Host: Click "Eliminate".
77. Host: Wait up to 20 seconds for "eliminated".
78. Host: Expect the game is still running.
79. Host: Quit the game.
80. Host: Expect no crash.
