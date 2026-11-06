---
layers: [qol]
bump: minor
status: Installed, not yet seen in game
---

### Changed
- The engine resolver (`crates/a2engine`, `csdk/a2engine.h`; root 15.8.0) names the executable in its log lines and notes by the matched build's `exe` (`profiles/*/build.toml`), or on an unknown build by the process's own module file name, not by a literal: `engine: Armada2.exe timestamp ...` on Armada II as before. The generated symbol tables carry each build's `exe`.
- Engine fields and values are the matched build's, filled at start-up (`QOL_F_*`, `QOL_V_*` read `a2f_qol_*`), no longer compile-time per game; the asm stubs read them through those variables instead of splicing literals (`A2_STR` is gone). The bar's menu toggles are saved for at most 16 (`BK_TOGGLES_MAX`), the build's count otherwise.
- The patched functions' frame locals the stubs read (`CheckCanExecute`'s craft array, count, index and route locals, `mSetupSpecialWeapons`' read-back local, `mCheckGroupSelect`'s group number) are profile values (`cce_*`, `msw_read_local`, `cgs_group`, unit `ebp`), no longer literals in the asm (PLAN-ARCHITECTURE.md, AR-3).
