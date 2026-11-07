---
qol: minor
games: [armada2]
---

### Added
- QOL: `[BarKeys] Rebind`, a key rebind table (rows action:FROM>TO) applied to the engine's parsed key map in memory while the grid's position keys are on (hook `qol.rebind`; a clash refused and logged); a row that does not apply leaves its FROM key the game's, its cell without a key and without a button, so a key never does two things.
