---
layers: [display]
bump: patch
games: [armada2, fo-roots]
---

### Changed
- A later CreateDevice on another window logs the new window's class, whether it is shown and its thread, and whether the old window is still there, before the swap chain follows it; the README says Armada II meets this path when its engine makes its display again (its flip to windowed after an assert or a crash), where following is right.
