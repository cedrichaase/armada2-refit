---
layers: [qol]
bump: patch
games: [armada2]
status: Installed, not yet seen in game
---

### Fixed
- Hook `qol.astar_path` (no key): `A_Star_Search::Plan_Path` stops building a found path after 20,000 nodes and answers no path; a `Get_From` chain that never reaches its start (after a wormhole exit) no longer runs the game out of memory late in long games (issue #174).
