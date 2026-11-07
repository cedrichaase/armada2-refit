---
layers: [qol]
bump: patch
---

### Fixed
- Hook `qol.script_input` (no key): `ScriptInputCallbacks::AlwaysSimulate` calls a script's input monitor only while one is set; the map editor opened after a scripted mission (Roots' first tutorial) no longer crashes on a cleared callback, a crash stock Roots has too.
