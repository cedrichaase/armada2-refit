---
ui: patch
games: [armada2]
status: Installed, not yet seen in game
---

### Fixed
- The engine's text is read as Windows-1252, its code page, where it was read as Latin-1 (`text::game`, at the tooltips' string streams and `engine::buttons::clean`): a bullet, dash or curly quote no longer draws as `?`; a tab draws as a space and other control characters as nothing; the font bakes the rest of Windows-1252.
