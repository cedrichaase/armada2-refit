---
layers: [display]
bump: minor
---

### Added
- [Window] OneWindow (default 1): where the build's profile says the game makes its device on a new window at each switch (Fleet Operations: Roots, profile value device_windows), Display8 keeps one top-level window of its own for the whole game and puts each device window inside it, so the window manager sees no window close or open and the focus stays; empty frames presented right after a switch are not shown until a frame with content, so the picture holds instead of going black (PLAN-FLEETOPS.md, decisions 314 and 315). Armada II is unchanged.
