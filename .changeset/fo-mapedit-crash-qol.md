---
layers: [qol]
bump: patch
games: [armada2, fo-roots]
---

### Fixed
- Hook `qol.script_input` (no key): `ScriptInputCallbacks::AlwaysSimulate` calls a script's input monitor only while one is set; the map editor opened after a scripted mission no longer crashes on a cleared callback.
