---
display: minor
ui: patch
root: patch
games: [armada2, fo-roots]
---

### Fixed
- Contract 1.5: CLAIM_WINDOW (gpu/src/display.rs). Display8 makes its one window ([Window] OneWindow) only when a plugin claims it at the first CreateDevice; the UI claims it while a game's front-end forms are its own ([Menus] Forms=1).
