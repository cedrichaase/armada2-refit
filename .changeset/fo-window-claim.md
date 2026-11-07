---
display: minor
ui: patch
root: patch
---

### Fixed
- Contract 1.5: CLAIM_WINDOW (gpu/src/display.rs). Display8 makes its one window ([Window] OneWindow) only when a plugin claims it at the first CreateDevice; the UI claims it while Roots' forms are its own ([Menus] Forms=1). With Roots' own screens (Forms=0, Roots' default) the one window took the pointer from Roots' own Instant Action setup (PLAN-FLEETOPS.md, decision 405).
