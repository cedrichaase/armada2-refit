---
layers: [textures]
bump: minor
status: Confirmed in game 2026-11-06
---

### Changed
- `./a2tex` takes the game's texture root from the game descriptor (`A2_TEXTURES`, the first of `games/<id>.toml` `[files] textures`, under the data folder) instead of `$A2_GAME/Textures/RGB`; with the default game it is the same folder.
