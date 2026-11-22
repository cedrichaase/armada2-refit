---
render3d: patch
games: [armada2]
status: Installed, not yet seen in game
---

### Fixed
- A build without the object loop's draw site (stock) no longer logs `render3d.frame.obj_draw stands down`: the hook is asked about with a non-logging `present` before `ready`.
