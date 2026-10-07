# Internet - Online with QOLRules: one player without it

Resolutions: 16:9
Mod: refit
Players: host, joiner
Setup: online/bench-asi.sh --no-rules joiner
Timeout: 20 min

QOL-7 (qol/README.md), the mixed case. `multiplayer-online-match` with QOLRules installed (`--install .`) on the host only: the joiner reports none, the host answers that not every player has it, and both log stock rules; the host's answer stays stock for the whole match (it logs "on for every player" only once the joiner has quit, when nobody without the plugin is left). Milestone 1 (online/README.md): two games host, join, chat and play a match through
*Internet - Online*, on `Online.asi`'s own UDP transport. The clones keep Proton's own
DirectPlay, which cannot host, so the match can only run on ours: `Setup:` installs
nothing but `Online.asi`. The joiner types this machine's address. The flow is
`multiplayer-two-players.md`'s, and the logs checked at the end are the transport's
own lines in `Online.log`.

1. Include "_online-setup".
2. Joiner: Type this machine's address.
3. Include "_online-play".
34. Host: Expect "Online.log" contains "rules: a joiner has no QOLRules".
35. Joiner: Expect "Online.log" not contains "rules: QOLRules on".
