---
layers: [render3d]
bump: minor
---

### Added
- Menu scene: a content set can supply its own map (sets/<set>/menu/map.ini; Menu\map.ini still wins), a view takes Orbit= (degrees a second, the eye turning about its target), and the menu sky falls back to the stock recipe and to bstar.0 for bright stars; the fleetops set's map is Deep Space Nine with a Defiant
- Menu scene keys `SunAt=` (the way towards the sun, apart from the Key), `SunScale=` (the sun SOD's scale) and `StarTiles=`/`StarLevel=` (a star sheet laid on a fixed cube, for builds whose star sprite is a whole tile, Roots'); the fleetops map uses them for a small visible sun and a sky that holds still
