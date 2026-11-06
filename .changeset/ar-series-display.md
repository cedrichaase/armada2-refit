---
layers: [display]
bump: minor
status: Installed, not yet seen in game
---

### Changed
- The engine resolver (`crates/a2engine`, `csdk/a2engine.h`; root 15.8.0) names the executable in its log lines and notes by the matched build's `exe` (`profiles/*/build.toml`), or on an unknown build by the process's own module file name, not by a literal: `engine: Armada2.exe timestamp ...` on Armada II as before. The generated symbol tables carry each build's `exe`.
- Engine fields and values come from the matched build at run time (`field::X()`, `value::X()`), no longer compile-time consts of `gog-1.1` (PLAN-ARCHITECTURE.md, AR-3).
- The stub's walk of the engine's mode list (`stub/modes.rs`) reads the device's list and each mode's width, height and depth through new `display.toml` fields (`dev_modes`, `mode_node_value`, `mode_width`, `mode_height`, `mode_bpp`, hook `modes`), no longer literal offsets.
