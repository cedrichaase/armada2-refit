# Internet - Online: a whole match on our own transport

Resolutions: 16:9
Mod: refit
Players: host, joiner
Setup: online/bench-asi.sh
Timeout: 20 min

Milestone 1 (online/README.md): two games host, join, chat and play a match through
*Internet - Online*, on `Online.asi`'s own UDP transport. The clones keep Proton's own
DirectPlay, which cannot host, so the match can only run on ours: `Setup:` installs
nothing but `Online.asi`. The joiner types this machine's address. The flow is
`multiplayer-two-players.md`'s, and the logs checked at the end are the transport's
own lines in `Online.log`.

1. Include "_online-match".
