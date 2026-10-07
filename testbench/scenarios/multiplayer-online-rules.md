# Internet - Online with QOLRules: both players agree on the rules

Resolutions: 16:9
Mod: refit
Players: host, joiner
Setup: online/bench-asi.sh
Timeout: 20 min

QOL-7 (qol/README.md). `multiplayer-online-match`, with QOLRules installed on both clones (`--install .`): the joiner reports it, the host answers that every player has it, and both log it. Milestone 1 (online/README.md): two games host, join, chat and play a match through
*Internet - Online*, on `Online.asi`'s own UDP transport. The clones keep Proton's own
DirectPlay, which cannot host, so the match can only run on ours: `Setup:` installs
nothing but `Online.asi`. The joiner types this machine's address. The flow is
`multiplayer-two-players.md`'s, and the logs checked at the end are the transport's
own lines in `Online.log`.

1. Include "_online-setup".
2. Joiner: Type this machine's address.
3. Include "_online-play".
34. Host: Expect "Online.log" contains "rules: QOLRules on for every player".
35. Joiner: Expect "Online.log" contains "rules: QOLRules on for every player".
