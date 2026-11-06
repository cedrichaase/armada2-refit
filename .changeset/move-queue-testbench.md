---
layers: [testbench]
bump: minor
status: Installed, not yet seen in game
---

### Added
- Scenario `qol-move-queue`: Tab queues on the `warp` scene, flown a run at a time (a point appended while flying, a 3D point then a minimap warp point, Stop and a plain right-click ending a queue, a point for one of two queued ships splitting it) (issue #17).

### Changed
- Scenario `render3d-move-paths` reworked for the queues drawn for the selected ships only (`QOL_MoveQueue`): five Tab points appended one by one with render3d's count of points ahead following, 0 selected ships on a path while another ship is selected and QOL has cleared nothing, a plain right-click and the bar's Stop clearing the drawing, a go leg then a minimap warp leg (no warp run sent before the go legs end). Waits are on the logs' lines; shots: held, released, deselected, mid-flight, cleared, mixed, go-leg, warp-leg, stopped.
