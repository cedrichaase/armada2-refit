---
render3d: minor
games: [armada2]
status: Confirmed in game 2026-11-07
---

### Added
- Menu scene: a content set can supply its own map (sets/<set>/menu/map.ini; Menu\map.ini still wins), a view takes Orbit= (degrees a second, the eye turning about its target), and the menu sky falls back to the stock recipe and to bstar.0 for bright stars
- Menu scene keys `SunAt=` (the way towards the sun, apart from the Key), `SunScale=` (the sun SOD's scale) and `StarTiles=`/`StarLevel=` (a star sheet laid on a fixed cube, for builds whose star sprite is a whole tile)
