---
layers: [qol]
bump: minor
status: Confirmed in game 2026-11-06
---

### Added
- Exports `QOL_MoveQueueVersion` (1) and `QOL_MoveQueue(ship, xyz, flags, cap)` (the points still ahead of a ship, for render3d's dots), and `QOL_MoveQueuePoint(xyz, action)` (the UI's minimap hands a Tab click to the queues).
- `QOL.log`: `move queue: ...` lines (new queue, appended point, run sent / next run sent / next part sent, cleared for N ship(s), ship gone, queue done, minimap point), at most 400.

### Changed
- QOL-10 is a move queue (`waypoints.c`, still switched by `PathStartsAtOnce`, a live row): a point queued with Tab (or, through the UI, on the minimap) for ships already flying a queue is appended to it instead of restarting it; each leg keeps the mode it was queued with (GO, GO_WARP with Alt or by QOL-8 judged on the leg's own length, PATROL), and the queue is flown a run of same-mode legs at a time, each run as stock's path order, the next run sent when the group reaches the end of the one before; a run longer than one order's packet is sent in parts. The player's orders that replace a ship's movement (stop, move, attack, a path, a special weapon, ...) end its queue; settings orders (alerts, autonomy, formation type) do not. Hooks added to `qol.move_paths`: the calls of `GameObject::DeQueueCommand` in the `NetOrder*::Execute`s, `Broadcast_Go_Command`, `FormFormation` and the special attack's `SetCommand`, the step after `cOverViewImp::mProcessKeyboardInput` and the clock after `GameObject::PreSimulateAll`.
