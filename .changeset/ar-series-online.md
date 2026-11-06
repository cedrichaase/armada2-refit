---
layers: [online]
bump: minor
status: Confirmed in game 2026-11-06
---

### Changed
- The engine resolver (`crates/a2engine`, `csdk/a2engine.h`; root 15.8.0) names the executable in its log lines and notes by the matched build's `exe` (`profiles/*/build.toml`), or on an unknown build by the process's own module file name, not by a literal: `engine: Armada2.exe timestamp ...` on Armada II as before. The generated symbol tables carry each build's `exe`.
- Engine fields and values are the matched build's, filled at start-up (`ONLINE_F_*`), no longer compile-time per game (PLAN-ARCHITECTURE.md, AR-3).
