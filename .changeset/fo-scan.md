---
root: minor
games: [fo-roots]
status: Installed, not yet seen in game
---

### Added
- `tools/engine/fo-scan.py`: profiles a new Fleet Operations: Roots build in one command (identity and section CRC-32s, file diff against stock 1.1 by function, MemDiff run-time diff from a bench clone with `--bench`, marks and dropped callee checks written to `profiles/fo-roots-<version>/`, a private report in `$A2_DATA`); `fo-scan.toml` holds its callee list; host test `fo_scan_test.py` in `selftest.sh`; `negative.sh` case (e) reads the build from `FO_ROOTS_BUILD`
