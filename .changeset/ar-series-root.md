---
layers: [root]
bump: minor
status: Installed, not yet seen in game
---

The AR series (`PLAN-ARCHITECTURE.md`: AR-2 game descriptor, AR-5 file system, AR-6 content sets, AR-3 fields per build, AR-4 UI engine boundary); what belongs to no one layer. The layers' own changesets carry their items.

### Added
- `games/`: one descriptor per game (`games/<id>.toml`; `games/README.md`): label, executables and data folder, Heroic and GOG ids, default folder and prefix names, settings file kind, shell resolution and HUD canvas, `[files]` (`textures`, `texture_formats`, `sod`, `sprites`, `odf`, `archives`), data directory stem and the layers on offer. `armada2.toml` is the default; `fo-roots.toml` is a stub, `offered = false`.
- `A2_PROFILE` and `--game <id>` choose the game in `a2env.py`, `a2env.sh`, `a2mod`, `./install`, `./a2test` and `tools/layers.py` (default `armada2`, the same paths as before); `a2env` also gives `A2_PROFILE`, `A2_EXE` and `A2_TEXTURES`, the config file takes a `[<id>]` section per other game, and `python3 a2env.py --list` lists the games.
- `tools/layers.py --check` checks the descriptors (games, layers, profiles' builds, `[files]`, `a2env.py`'s built-in default) and holds render3d's and the UI's `LAYOUT` tables to the descriptor.
- `a2fmt::vfs` (`crates/a2fmt/README.md`, "The file system"): one `no_std` file system over a game's files (`trait Vfs`, `Loose` over an `FnIo`, `Layered` mounts; `Fpq` and `Zfs` declared, `Unsupported`), host-tested in `crates/a2fmt/tests/vfs.rs`.
- `a2ini::Table`: an ini table read borrowing from its text, for the content sets (`MODDING.md`, "Content sets").
- `tools/engine/moved.py` and `moved-gog-1.1.tsv` (in `selftest.sh`): every offset moved out of a plugin's source resolves to its old value on `gog-1.1`.

### Changed
- Profiles: `field` and `value` entries are resolved per build at run time, as addresses are (AR-3). `tools/engine/gen.py` emits `field::X()`/`value::X()` in Rust and `a2f_<p>_<id>` (behind `<P>_F_X`, `<P>_V_X`) in C; `crates/a2engine` gains `Engine::field`/`value`, `csdk/a2engine.h` `a2e_field`; a hook whose field the build lacks stands down. The fixtures gain a child build that moves a field.
- `tools/engine/lint.py` rule 4: a literal struct offset in any plugin source fails (was advisory); a set named for a plugin with a `content.rs` must be a folder of its `content/sets/` (fixture `29-content-set-no-folder`).
- The resolver (`crates/a2engine`, `csdk/a2engine.h`, `tools/engine/resolve.py`) has no `Armada2.exe` literal: the identity line and notes name the matched build's `exe` (`Build.exe`, emitted by `gen.py`), or the process's module file name on an unknown build; `a2engine::is_identity_line`, `Engine::exe()`, `Engine::game()` are new.
- `a2env.sh` takes its values from `a2env.py` (`--sh`); `tools/layers.py --resolve` and `./install` offer only the chosen game's layers; `a2mod` reads the executable, the player's files it never switches and the texture paths from the descriptor.
