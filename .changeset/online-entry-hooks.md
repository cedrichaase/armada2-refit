---
layers: [online]
bump: minor
games: [armada2]
---

### Changed
- Online.asi's engine hook online.entry is split into entry (the connection screen's click and IPX branch), labels (read_text_label's online texts) and chat (the join code's lines in the lobby's and GAME SETUP's chat boxes), each checked and installed on its own; on Armada II all are ready and nothing changes. New exports Online_Available, Online_Choose and Online_Chosen let a screen of the UI choose Internet - Online where the exe's connection screen does not run.
