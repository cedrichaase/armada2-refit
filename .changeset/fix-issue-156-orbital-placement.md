---
layers: [ui]
bump: patch
games: [armada2]
---

### Fixed
- The placement markers (`[HUD] Placement=1`): a taken cell under a clear footprint is no longer drawn red and crossed, only under a blocked one, so the Orbital Processing Facility over its planet reads as buildable (green over amber tiles, as stock draws it) instead of every cell taken (#156). `UI.log`'s placement line now says how many cells it crossed ("placement: footprint clear, 0 crossed, ...").
