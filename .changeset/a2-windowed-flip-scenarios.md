---
layers: [testbench]
bump: minor
games: [armada2]
status: none
---

### Added
- Scene.asi's `windowed` command: the engine's own flip to windowed (Debug_SystemCleanup, as its assert and exception handlers call it), a new device on a new window; scenarios `display-a2-windowed-flip` (the swap chain follows that window and the mission draws on) and `render3d-welds-next-mission` (welds of an aborted match dropped at the next match's first view, no flip to windowed).
