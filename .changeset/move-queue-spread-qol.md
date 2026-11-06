---
layers: [qol]
bump: minor
status: Installed, not yet seen in game
---

### Added
- Export `QOL_MoveQueueGroup(ship, own, pts, flags, cap, queue)`: a ship's own targets, the queue's points and its serial, for drawing a group as one path.
- Profile entries for the formations stock moves a group in, read only (`mq_formations`, `mq_form_types`, `mq_proc_formation`, `mq_frm_*`, `mq_slot_*`, `mq_min_radius`, `mq_ent_bounds`, `mq_bounds_radius`, `mq_class_shiptype`).
- `QOL.log`: `queue K spread (...)` with each ship's place, `ship ID passed point N, D from its target`, `at the end of its run`, and on a group's `done` whether each ship was at its own target at every point; the queue's log cap is 600 lines.

### Fixed
- A group flying a move queue no longer bunches up at each point: each ship flies to the point plus its place in the group's formation (the formation type chosen on the bar, stock's slots, scale and leader, turned to face each leg), one stock path order per ship. A formation picked mid-queue gives the points ahead the new places; ships it has no slot for keep their place in the group's layout. `QOL_MoveQueue` now answers the ship's own targets.
