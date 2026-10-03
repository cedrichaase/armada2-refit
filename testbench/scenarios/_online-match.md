# Fragment: host, join, chat and a match through Internet - Online

Shared by `multiplayer-online-match.md` and `multiplayer-online-loss.md`, which differ
only in their `Setup:`. The steps assume `Players: host, joiner`.

1. Launch the game.
2. Wait for the main menu.
3. Click the multiplayer emblem.
4. Wait for the multiplayer connection screen.
5. Host: Click at design 400,187.
6. Host: Press End.
7. Host: Press BackSpace 32 times.
8. Host: Type "host".
9. Host: Click "Internet - Online".
10. Host: Wait up to 10 seconds for "Join code or host address".
11. Host: Click "O.K.".
12. Host: Wait up to 30 seconds for "Create Game".
13. Host: Click "Create Game".
14. Host: Wait 3 seconds.
15. Host: Click "O.K.".
16. Host: Wait up to 30 seconds for "GAME SETUP".
17. Host: Take a screenshot called "host lobby".
18. Joiner: Click at design 400,187.
19. Joiner: Press End.
20. Joiner: Press BackSpace 32 times.
21. Joiner: Type "joiner".
22. Joiner: Click "Internet - Online".
23. Joiner: Wait up to 10 seconds for "Join code or host address".
24. Joiner: Click at design 400,292.
25. Joiner: Type this machine's address.
26. Joiner: Click "O.K.".
27. Joiner: Wait up to 30 seconds for "host's Game".
28. Joiner: Take a screenshot called "game list".
29. Joiner: Click "host's Game".
30. Joiner: Click "Join Game".
31. Joiner: Wait up to 30 seconds for "GAME SETUP".
32. Joiner: Click at design 155,547.
33. Joiner: Type "hello from joiner".
34. Joiner: Press Return.
35. Host: Wait up to 20 seconds for "hello from joiner".
36. Host: Click at design 155,547.
37. Host: Type "hello from host".
38. Host: Press Return.
39. Joiner: Wait up to 20 seconds for "hello from host".
40. Host: Take a screenshot called "host lobby with joiner".
41. Host: Click "LAUNCH".
42. Host: Wait 5 seconds.
43. Joiner: Click "LAUNCH".
44. Joiner: Wait 5 seconds.
45. Host: Click "LAUNCH".
46. Host: Wait up to 180 seconds for "999999".
47. Joiner: Wait up to 60 seconds for "999999".
48. Host: Wait 60 seconds.
49. Expect the game is still running.
50. Take a screenshot called "match".
51. Expect "Online.log" contains "OUR TRANSPORT".
52. Expect "Online.log" not contains "unsupported call".
53. Host: Expect "Online.log" contains "net host: joined".
54. Joiner: Expect "Online.log" contains "net joiner: connected".
55. Joiner: Quit the game.
56. Host: Quit the game.
57. Expect no crash.
58. Expect "Online.log" contains "closed; datagrams sent".
