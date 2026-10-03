# Fragment: join, chat, a match, quit, and the transport's own log lines

Follows `_online-setup.md` and the joiner typing where the game is.

1. Joiner: Click "O.K.".
2. Joiner: Wait up to 30 seconds for "host's Game".
3. Joiner: Take a screenshot called "game list".
4. Joiner: Click "host's Game".
5. Joiner: Click "Join Game".
6. Joiner: Wait up to 30 seconds for "GAME SETUP".
7. Joiner: Click at design 155,547.
8. Joiner: Type "hello from joiner".
9. Joiner: Press Return.
10. Host: Wait up to 20 seconds for "hello from joiner".
11. Host: Click at design 155,547.
12. Host: Type "hello from host".
13. Host: Press Return.
14. Joiner: Wait up to 20 seconds for "hello from host".
15. Host: Take a screenshot called "host lobby with joiner".
16. Host: Click "LAUNCH".
17. Host: Wait 5 seconds.
18. Joiner: Click "LAUNCH".
19. Joiner: Wait 5 seconds.
20. Host: Click "LAUNCH".
21. Host: Wait up to 180 seconds for "999999".
22. Joiner: Wait up to 60 seconds for "999999".
23. Host: Wait 60 seconds.
24. Expect the game is still running.
25. Take a screenshot called "match".
26. Expect "Online.log" contains "OUR TRANSPORT".
27. Expect "Online.log" not contains "unsupported call".
28. Host: Expect "Online.log" contains "net host: joined".
29. Joiner: Expect "Online.log" contains "net joiner: connected".
30. Joiner: Quit the game.
31. Host: Quit the game.
32. Expect no crash.
33. Expect "Online.log" contains "closed; datagrams sent".
