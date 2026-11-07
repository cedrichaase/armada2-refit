---
layers: [render3d]
bump: patch
---

### Fixed
- overlays: with QOL's queue found and a ship it keeps no queue for, the ship's ordinary move is drawn as the engine's path when the content set draws every engine move (Roots, EngineMoves = all); before, a ship QOL had no queue for drew nothing, so with QOL on Roots a plain move showed no path. QOL's queue still takes precedence where a ship has one; stock (EngineMoves = sketched) is unchanged
