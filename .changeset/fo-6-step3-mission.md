---
ui: minor
games: [armada2]
status: Installed, not yet seen in game
---

### Added
- `[UI] RefitKey`: with Ctrl, opens the Refit screen as a modal over a running mission (0, none, in the shipped `UI.ini`); the key is kept from the engine
- UI.log names the render lists once per mission (`hud: render lists`), the thread's windows and their window procedures at the first loading and mission frames and the first device releases, the callers of the two message functions at their entries, and the first button downs kept from the engine

### Changed
- Whether the engine hides its HUD is read from the info panel the render lists name (`hud::ship_display`), not the `shipDisplay` global; hook `core.reads_hud_hidden` and its symbol are gone from the profile
