# Internet - Online: a match through the server's relay

Resolutions: 16:9
Mod: refit
Players: host, joiner
Setup: online/bench-asi.sh --relay --loss 5
Timeout: 20 min

`multiplayer-online-code.md` for two players whose routers let nothing through: with
`Direct=0` neither game tries the other directly, and every datagram of the match goes
through the `a2online-server` that `Setup:` started, as `SV_RELAY`. `Loss=5` drops one
datagram in twenty on each side as well, so the transport's retransmission runs over
the relay too.

1. Include "_online-setup".
2. Host: Wait up to 20 seconds for "Join code".
3. Joiner: Type what follows "join code" in "Online.log" of host.
4. Include "_online-play".
5. Joiner: Expect "Online.log" contains "through the relay".
6. Host: Expect "Online.log" contains "joined: player".
7. Expect "Online.log" contains "dropped on purpose".
