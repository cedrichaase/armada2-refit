# Internet - Online: the menu entry hosts and joins

Resolutions: 16:9
Mod: refit
Players: host, joiner
Setup: online/reference-dplay.sh
Timeout: 15 min

The Multiplayer Connection screen's IPX button is *Internet - Online* with `Online.asi`
installed (online/README.md, "The menu entry"), in that button's place (design 204,368,
400x34). Both players take it: its dialog asks for a join code or the host's address, a
blank one hosts, and the game connects as Manual IP does, here over Microsoft's
DirectPlay. Until the transport exists an address is what joins. Then a chat line each
way proves the two are in one session. The rest of the flow is
`multiplayer-two-players.md`'s. The games are stopped, not quit: the bench quits through
the main menu or a mission's, and these are in the lobby.

1. Launch the game.
2. Wait for the main menu.
3. Click the multiplayer emblem.
4. Wait for the multiplayer connection screen.
5. Host: Take a screenshot called "connection screen".
6. Host: Expect "Internet - Online" is visible.
7. Host: Expect "Internet - Online" is visible inside design 204,368,400,34.
8. Host: Expect "Local Area Network (TCP/IP)" is visible.
9. Host: Click at design 400,187.
10. Host: Press End.
11. Host: Press BackSpace 32 times.
12. Host: Type "host".
13. Host: Click "Internet - Online".
14. Host: Wait up to 10 seconds for "Join code or host address".
15. Host: Take a screenshot called "online dialog".
16. Host: Click "O.K.".
17. Host: Wait up to 30 seconds for "Create Game".
18. Host: Click "Create Game".
19. Host: Wait 3 seconds.
20. Host: Click "O.K.".
21. Host: Wait up to 30 seconds for "GAME SETUP".
22. Joiner: Click at design 400,187.
23. Joiner: Press End.
24. Joiner: Press BackSpace 32 times.
25. Joiner: Type "joiner".
26. Joiner: Click "Internet - Online".
27. Joiner: Wait up to 10 seconds for "Join code or host address".
28. Joiner: Click at design 400,292.
29. Joiner: Type this machine's address.
30. Joiner: Click "O.K.".
31. Joiner: Wait up to 30 seconds for "host's Game".
32. Joiner: Click "host's Game".
33. Joiner: Click "Join Game".
34. Joiner: Wait up to 30 seconds for "GAME SETUP".
35. Joiner: Click at design 155,547.
36. Joiner: Type "hello from joiner".
37. Joiner: Press Return.
38. Host: Wait up to 20 seconds for "hello from joiner".
39. Host: Click at design 155,547.
40. Host: Type "hello from host".
41. Host: Press Return.
42. Joiner: Wait up to 20 seconds for "hello from host".
43. Host: Take a screenshot called "lobby".
44. Expect "Online.log" contains "menu entry: LAN (IPX) is Internet - Online".
45. Expect "Online.log" contains "menu: Internet - Online chosen".
46. Host: Expect "Online.log" contains "INDICATE_CONNECT".
47. Joiner: Expect "Online.log" contains "CONNECT_COMPLETE".
48. Joiner: Stop the game.
49. Host: Stop the game.
50. Expect no crash.
