---
layers: [ui]
bump: minor
status: Installed, not yet seen in game
---

### Changed
- Fleet Operations: Roots: the scripts' on-screen messages are the UI's (`[HUD] Overlay=1` in `ui/defaults/fo-roots.ini`; a reinstall moves an install still at the old Roots default 0 to 1).

### Fixed
- An overlay message that would touch a HUD panel of ours (Roots' mission name at the top left, under the resource strip) is centred across the screen at its height, and moved below the panel if it still touches one; its colour and the engine's fade are kept.
