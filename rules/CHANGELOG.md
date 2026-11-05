# Changelog — rules

`QOLRules.asi`: the rules every player of a network game must share (pay when queuing,
refund on cancel: QOL-7). Versioning rules: [`CLAUDE.md`](../CLAUDE.md), "Changelogs and
versions". Newest first. Details are in [`README.md`](README.md).

## 1.1.0 — 2026-11-05

### Changed
- Engine fields and values are the matched build's, filled at start-up (`QOLRULES_F_*`, `QOLRULES_V_*`), no longer compile-time per game; the advisor's topics are looked up per shortfall (`topic_of`) (PLAN-ARCHITECTURE.md, AR-3).

Installed, not yet seen in game.

## 1.0.4 — 2026-11-04

### Changed
- The engine resolver (`crates/a2engine`, `csdk/a2engine.h`; root 15.8.0) names the executable in its log lines and notes by the matched build's `exe` (`profiles/*/build.toml`), or on an unknown build by the process's own module file name, not by a literal: `engine: Armada2.exe timestamp ...` on Armada II as before. The generated symbol tables carry each build's `exe`.

Installed, not yet seen in game.

## 1.0.3 — 2026-11-04

### Fixed
- `s_num` (`csdk/a2asi.h`, compiled in) prints the most negative value correctly in log lines (an unsigned negate); no behaviour change otherwise.
Installed, not yet seen in game.

## 1.0.2 — 2026-11-04

### Fixed
- A network game's rules follow what the online layer announced on connect (`QOLRules_Network`), not the setting as it is when the game opens: switching `PayOnQueue` off after connecting left this player queuing stock while the peers paid on queue, and the games diverged. A local game still reads the setting.

Installed, not yet seen in game.

## 1.0.1 — 2026-11-03

### Changed
- `PayOnQueue`'s row moves from the Multiplayer tab to Gameplay (`tab = "gameplay"`); key, section and hint unchanged, so a player's `RefitPlayer.ini` value carries over. Installed, not yet seen in game.

## 1.0.0 — 2026-11-03

### Added
- The layer, made from `qol/`'s `QOLRules.asi` (qol 1.10.1) unchanged in behaviour: `rules.c`, `settings.toml` and its generated `settings_table.h`, `engine_syms.h` (profile `profiles/gog-1.1/qolrules.toml`), `QOLRules.ini`, and its own `build.sh` and `install.sh` (`--remove`). Exports (`QOLRules_Wanted`, `_Network`, `_Reset`, `_Settings`, `_SettingsNote`, `_SettingsChanged`) and the `[QOLRules.Rules]` overlay section keep their names.
- Contract C7: with the layer left out or set aside the player reports no rules and the host plays stock for everyone; Online finds the plugin by its export. `PayOnQueue` ships on and is selected by default in the installers. Installed, not yet seen in game.
