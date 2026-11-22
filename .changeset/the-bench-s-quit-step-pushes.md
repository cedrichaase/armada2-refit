---
layers: [testbench]
bump: patch
games: [armada2]
---

### Fixed
- The bench's quit step pushes the pointer into the corner before each press, so the exit confirmation's Yes is hit when the compositor's pointer and the game's stand apart (multiplayer-online-loss failed there intermittently).
