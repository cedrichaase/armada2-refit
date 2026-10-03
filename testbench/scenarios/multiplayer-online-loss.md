# Internet - Online: a whole match with 10% of datagrams lost

Resolutions: 16:9
Mod: refit
Players: host, joiner
Setup: online/bench-asi.sh --loss 10
Timeout: 20 min

`multiplayer-online-match.md` on a network that loses packets. On one machine nothing
is ever lost, so there the transport's retransmission never runs. `Loss=10` in each
clone's `Online.ini` drops one datagram in ten that each game sends, acknowledgements
and retransmissions included, and the match has to play on regardless. The last check
reads the transport's closing line for its resend count.

1. Include "_online-match".
2. Expect "Online.log" contains "dropped on purpose".
