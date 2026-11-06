---
layers: [testbench]
bump: minor
status: Installed, not yet seen in game
---

### Added
- `./a2test --game <id>` (or `A2_PROFILE`): the game the bench runs, by its descriptor (`games/<id>.toml`). `bench/config.py` exposes `GAME_ID`, `EXE_NAME`, `EXE_REL`, `SETTINGS_KIND`, `SETTINGS_FILE` and `SHELL_RES`; a session launches and matches processes by the descriptor's executable (in its `data_dir`), and writes the settings file through the `armada-prf` adapter only (another kind is refused until it has one). `a2test syms` defaults to the game's executable and prints the identity line the resolver logs. With no game chosen everything is as before.

### Fixed
- Scenario `engine-syms` expects the symbol counts with the fields and values each table now resolves per build (Display8 34, UI 40, hud 172, QOL 137, Render3D 166 and its modules).
