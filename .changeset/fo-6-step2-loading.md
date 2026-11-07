---
root: minor
ui: minor
testbench: minor
status: Installed, not yet seen in game
---

### Added
- The UI's loading screen on Fleet Operations: Roots (FO-6 step 2): `loading.overlay` and `loading.own` are verified beside Roots' load bar and `GameDoOne` edits (`profiles/fo-roots-5.0Z11/build.toml`, `tools/engine/fo-scan.toml` `[verified]`); `inner_db` and `inner_pixels` stay down. On the bench the screen covers Roots' own, names the map and the players, and animates between the load's steps
- Scenarios `fo-ui-loading` (`[Loading] Hold`, waits on the UI's log) and `fo-ui-loading-off` (`Loading.Enable=0`, Roots' own screen), both `a2test --game fo-roots --install .`

### Changed
- `ui/bench-asi.sh` installs into the executable's folder (`Data/` on Roots), keeps a game's own `UI.ini` defaults as its base and runs `retire/` for Armada II only
