---
ui: patch
games: [armada2]
status: Installed, not yet seen in game
---

### Changed
- The info panel is found by name in the engine's render lists (every view), not through the `shipDisplay` global, on every build: the panel switch takes it by name (`INFO_PANEL_TAKEN`) and the answer is the same object. The unused `hud_ship_display` symbol is gone from the profile
