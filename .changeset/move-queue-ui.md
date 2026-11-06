---
layers: [ui]
bump: minor
status: Installed, not yet seen in game
---

### Added
- The minimap: with the waypoint key (Tab) held, a command click is handed to QOL's move queue (`QOL_MoveQueuePoint`, found by its export) before any order is given; when QOL takes it the point is appended to the selection's queues and the UI gives no order (UI.log: `minimap: point 0x2b at x,z queued (QOL)`). Without QOL, or with its queue off, the click orders as before (`engine/radar.rs`).
