---
layers: [render3d]
bump: patch
games: [armada2, fo-roots]
---

### Fixed
- Repair welds: the welds whose particle is gone are dropped before their light is placed, not only before they are drawn; at the first view of a match after an aborted one the old welds' hardpoint nodes (freed memory) were read through Entity::GetWorldPosition, which faulted at the end of the load (the engine then flipped to windowed and hung: a black screen). Logs `effects: repair: N weld(s) of a view T s ago dropped before their light` after a gap.
