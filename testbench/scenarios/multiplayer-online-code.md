# Internet - Online: joining by join code, through the server

Resolutions: 16:9
Mod: refit
Players: host, joiner
Setup: online/bench-asi.sh --server
Timeout: 20 min

Milestone 2 (online/README.md): the host gets a join code from an `a2online-server`,
shown in its GAME SETUP chat, and the joiner types that code instead of an address.
`Setup:` starts a server on 127.0.0.1:23990 and names it in each clone's `Online.ini`.
The two games find each other through it and then talk directly: on one machine the
probes always get through, so this is the direct path, and
`multiplayer-online-relay.md` is the relayed one. The code is typed from the host's
`Online.log`; the chat line is checked by OCR.

1. Include "_online-setup".
2. Host: Wait up to 20 seconds for "Join code".
3. Host: Take a screenshot called "host lobby with code".
4. Joiner: Type what follows "join code" in "Online.log" of host.
5. Include "_online-play".
6. Host: Expect "Online.log" contains "a joiner is coming".
7. Joiner: Expect "Online.log" contains "direct to the host".
