---
layers: [ui]
bump: minor
games: [fo-roots]
---

### Added
- Roots: Game Setup (Instant Action's and the multiplayer games' setup) drawn by a spec of ours: Armada II's Game Setup over Roots' layout, the players' rows as our drop-downs over the stock `ComboBox`es, the map's window and settings, the bottom row

### Changed
- `setup_kit` acts through the trait `menus::Host` and Game Setup's places are a `game_setup::Layout`, shared by Armada II's screens and Roots' specs; Armada II's screens behave as before
