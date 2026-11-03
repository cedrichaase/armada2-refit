# Changelog — rules

`QOLRules.asi`: the rules every player of a network game must share (pay when queuing,
refund on cancel: QOL-7). Versioning rules: [`CLAUDE.md`](../CLAUDE.md), "Changelogs and
versions". Newest first. Details are in [`README.md`](README.md).

## 1.0.1 — 2026-11-03

### Changed
- `PayOnQueue`'s row moves from the Multiplayer tab to Gameplay (`tab = "gameplay"`); key, section and hint unchanged, so a player's `RefitPlayer.ini` value carries over. Installed, not yet seen in game.

## 1.0.0 — 2026-11-03

### Added
- The layer, made from `qol/`'s `QOLRules.asi` (qol 1.10.1) unchanged in behaviour: `rules.c`, `settings.toml` and its generated `settings_table.h`, `engine_syms.h` (profile `profiles/gog-1.1/qolrules.toml`), `QOLRules.ini`, and its own `build.sh` and `install.sh` (`--remove`). Exports (`QOLRules_Wanted`, `_Network`, `_Reset`, `_Settings`, `_SettingsNote`, `_SettingsChanged`) and the `[QOLRules.Rules]` overlay section keep their names.
- Contract C7: with the layer left out or set aside the player reports no rules and the host plays stock for everyone; Online finds the plugin by its export. `PayOnQueue` ships on and is selected by default in the installers. Installed, not yet seen in game.
