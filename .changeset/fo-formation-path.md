---
render3d: minor
testbench: patch
games: [armada2]
status: Installed, not yet seen in game
---

### Added
- overlays: the move paths draw a formation move's followers. A move of several ships gives the group's leader the move and every other ship FOLLOW (command 11, stock's), which the paths skipped; a follower now draws its leader's path (once for the group), a leg to its own place at the leader's next point and a small dot at its place at each point after. Profile entries `cmd_follow`, `proc_follow_target`, `proc_follow_offset` (gog-1.1).
- overlays: `EngineMoves` in a content set's `overlays.ini`: `sketched` (the stock set, as before: without QOL's queue only sketched paths draw) or `all` (every move of a selected ship draws).

### Changed
- testbench: `engine-syms` expects the overlays' 110 symbols (was 107).
- testbench: `engine-syms` expects the effects module's 127/128 symbols (was 126/126, stale since the shield-hit entries).
