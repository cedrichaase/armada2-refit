# Plan: architecture follow-ups (phase 13)

This plan comes from the architecture review of 2026-11-04. The review was read-only and
covered `main` at root 15.6.21. It was judged against three goals:

- **Modularity for modders.** A modder can take one unit and extend it without forking
  the others.
- **Simplicity for players.** There is one way to install, one place for settings, and
  nothing to keep in step by hand.
- **Extensibility to the other Armadas.** Fleet Operations: Roots (`PLAN-FLEETOPS.md`)
  and Armada 1 (`PLAN-ARMADA1.md`) build on the shared code without copying it.

It is a plan, not a commitment of dates. Effort is given in ROADMAP units: agent hours,
bench passes and sign-off round.

## What holds

The big structure is right, and this plan keeps it:

- one device owner (Display8) behind a versioned C contract (`display8v1_*`);
- claims instead of plugins finding each other by DLL name;
- engine addresses as data, in per-build profiles, with a stand-down for each hook;
- self-describing settings tables, and one player overlay (`RefitPlayer.ini`);
- render3d as module crates, with `lint-layers.sh` enforcing that no module depends on
  another;
- `layers.toml` as the one registry of layers.

## What the review found

| # | Finding | Evidence | Goal it hurts |
|---|---|---|---|
| F1 | The same install exists three times: per-layer scripts behind `./install`, the self-contained release scripts `publish/installer/install.sh` (815 lines) and `install.ps1` (923 lines), and the GUI installer's own discovery and checks. The ini merge is written in Python, awk and PowerShell, and the retire logic three times. `tools/layers.py --check` compares only `::step` ids, never behaviour | `install.sh:432-440` ("Keep them in step"); `layers.py:172-322` | players, maintenance |
| F2 | There is no "game" at run time or in the tooling. `game = "armada2"` in `build.toml` only groups profiles in the generator, and `Table.game` is never read. `exe = [...]` is only linted. `"Armada2.exe"` is hard-coded in `a2engine` and `csdk/a2engine.h`. `a2env` has one game, and every `layers.toml` `games` field is `['armada2']`, which nothing filters on | `crates/a2engine/src/table.rs:130`, `engine.rs:67,177,558`, `pe.rs:147`; `a2env.py:24-49` | other games |
| F3 | Struct offsets are not per build. `field` and `value` entries are generated as compile-time constants (`ui/src/syms_core.rs:51-60`; `crates/a2engine/README.md:119-121`). render3d's profiles hold four `field` entries in total, against about 110 `const` offsets, 170 `field(x, 0x..)` calls and 50 `rd(x + 0x..)` reads in its sources. The C plugins have about 21 raw `+0x` offsets, which `lint.py` treats as advisory only | `render3d/mods/particles/src/hooks.rs:31-40`; `tools/engine/lint.py:762-771` | other games (crash, not stand-down) |
| F4 | The UI has no engine boundary. `hud/` and `menus/` import `crate::engine::*` directly, and there is no `Panel` or `Screen` trait. The 1600x1200 canvas is hard-coded, and race lists live in `widgets/theme.rs` | `ui/src/engine/hud.rs:87`, `ui/src/widgets/theme.rs:235-353` | modders, other games |
| F5 | There is no file system layer. `Textures\RGB` is hard-coded in `render3d/core/src/assets/mod.rs:84`, `a2mod` and `tools/layers.py`. Roots packs its ODFs in `ODF.fpq` and mixes DDS with TGA; Armada 1 uses `.zfs` archives and `Index8` textures | — | other games |
| F6 | Content lives partly in code. All nine `content.rs` files have only a `"stock"` arm. Some names sit outside them: `"8472_cocoon"` (`effects/src/chunks.rs:103`), `"Klingon"` (`effects/src/shields.rs:143`), the `Borg*` keys (`hulls/src/look.rs`) and the UI's race lists. Rule 4 of `lint-layers.sh` is advisory | — | other games, modders |
| F7 | There are two GPU stacks. `display/src/vk` and `display/src/dx12` (about 5.2k lines) sit beside `gpu/src/vulkan` and `gpu/src/dx12` (about 8.8k lines), and both create pipelines. Display7 would make a third | — | other games, maintenance |
| F8 | render3d can be extended only by forking. Modules are a static list (`render3d/src/lib.rs:29-46`). Services are a fixed struct of five traits in a `static mut` (`core/src/services.rs`). `Step` is a closed enum named after `Armada2.exe` functions | — | modders, other games |
| F9 | The player faces 7 shipped inis with about 172 keys. Only 54 of those are Refit rows, so most defaults live in both the ini and the code | — | players |
| F10 | Twins drift silently. `a2env.sh` and `a2env.py` are a hand-synced pair. `csdk/a2asi.h`'s ini, overlay and settings code has no test against `a2rt` and `a2ini`, unlike `a2engine.h` | — | maintenance |
| F11 | The docs drift. ROADMAP's "Where we are" was two rounds stale until this plan. There are about 400k words of Markdown and 78 remote branches. The bench's `testbench/scene/scene.c` holds 98 raw addresses outside the profiles | — | modders |

## Work packages

The packages are listed in order. AR-1 to AR-3 are specified in full below, and the rest
are sketched.

| WP | What | Depends on | Agent time · bench · round |
|---|---|---|---|
| AR-1 | Behavioural equivalence test for the installers (F1) | nothing | 3–6 h · none · none |
| AR-2 | Game descriptor for the tooling; no `Armada2.exe` in shared code (F2). This is FO-1's mechanism | nothing | 4–8 h · 1–2 · none |
| AR-3 | Field and value offsets resolved per build at run time; render3d's offsets into its profiles; strict lint (F3) | nothing; best after AR-2 | 8–16 h · 2–4 pixel-identical passes · with the next round |
| AR-4 | UI engine boundary: `Panel` and `Screen` traits, `engine/` the only reader of `syms_*`, the canvas from the profile, a UI `lint-layers` (F4) | AR-3 | 8–16 h · 3–5 · a round |
| AR-5 | `a2fmt::vfs`: one trait, with loose-file, `.zfs`, `.fpq` and engine-backed implementations; texture roots from the game descriptor (F5) | AR-2 | 6–12 h · 1–2 |
| AR-6 | Content sets as data folders (`content/<set>/`); names out of code; rule 4 strict (F6) | AR-2 | 6–10 h · 2–3 pixel-identical |
| AR-7 | Display8's device and pipeline layer on `gpu`, before C1 and Display7 (F7) | AR-3 | 10–20 h · pixel-identical A/B, 3–5 |
| AR-8 | render3d services as a typed registry; neutral `Step` names mapped per game in `hooks.rs`; a "write a module" walkthrough (F8) | — | 3–6 h · 1 pixel-identical |
| AR-9 | One installer core, used by the GUI, the release and `./install`; `install.sh` and `install.ps1` become thin launchers (F1) | AR-1 | 12–24 h · `no-assets`, 2–3 · a release round |
| AR-10 | Shipped inis as defaults only: every key gets a row (most `advanced`), defaults come from the table, and the ini merge and upgrade tables go (F9) | AR-9 | 6–10 h · 2 · with AR-9 |
| — | Small items: an `a2env` single source (F10); an `a2asi.h` twin test (F10); the bench's `scene.c` on the profile (F11); a doc diet and branch pruning (F11) | — | 1–3 h each |

### AR-1: installer equivalence test

**Goal:** CI fails when the installers diverge in what they do, not only in which
steps they name.

- **The harness.** `publish/installer/equiv.sh` takes a release package (the CI
  `package` job's output, or one staged from stub files in the same layout) and a fake
  game directory with stand-ins for the files the installers look at. It runs
  `install.sh` and `install.ps1` (pwsh, on Linux in CI) into two separate copies.
- **The comparison.** It compares the two trees: the paths, the bytes of every file
  copied, and each ini merged key by key. It does the same after an uninstall, after
  an upgrade from an older install that has retired files and old ini keys, and after a
  deselected layer.
- **`./install`.** The repository's `./install` is compared too, where it can run
  without a game (`--only` of the layers that need no build, or with prebuilt
  artifacts). What it cannot cover yet is listed in the harness, not hidden.
- **Ini fixtures.** The fixtures for the ini merge are shared with
  `tools/ini-merge.py`, so the three ini mergers (Python, awk, PowerShell) are tested
  against one set of expected outputs.
- **Wiring.** A CI job is added, and `publish/installer/README.md` (or the nearest
  README) says how to run it locally.
- **Acceptance.** The job passes on today's scripts, or the differences it finds are
  fixed or listed as known with a reason. A deliberate one-line divergence makes it
  fail.

### AR-2: game descriptor

**Goal:** "which game" is one data file that every tool reads, and the shared code
names no game.

- **`games/<id>.toml`**, one per game: `armada2` now; `fo-roots` and `armada1` later,
  by their plans. Each holds:
  - the label;
  - the executable names and the data subfolder (`Data/` for Roots);
  - install discovery: Heroic or GOG ids, the default folder names, the prefix name;
  - the settings file kind (`armada-prf`, later `fo-settings-xml`);
  - the shell resolution and the HUD canvas;
  - the texture roots;
  - the log and data directory stem;
  - the layers on offer, cross-checked with `layers.toml` `games`.
- **Choosing a game.** `A2_PROFILE` and `--game` choose the game in `a2env.sh`,
  `a2env.py`, `a2mod`, `./install`, the bench's config and `tools/layers.py`. The
  default is `armada2`, so nothing changes for today's user. `a2env.sh` gets its values
  from `a2env.py` (or a shared generated file), so the pair stops being hand-synced.
- **The shared resolver.** `crates/a2engine` and `csdk/a2engine.h` take the executable
  name for log lines from the matched build (`build.toml` `exe`), or from the process's
  own module name when no build matches, not from a literal. The log format keeps its
  shape (`engine: <exe> timestamp ...`), so the bench and the tools that parse it still
  work, and those parsers are updated to accept any name. `Table.game` is read where it
  helps the log.
- **Lint.** `tools/layers.py --check` checks that every `games` entry names a
  descriptor, and that each descriptor's layers exist.
- **Out of scope.** The GUI installer and the release scripts keep their own discovery
  for now; AR-9 moves them onto the descriptor.
- **Acceptance.**
  - All selftests pass (`tools/engine/selftest.sh`, `tools/settings/selftest.sh`,
    `tools/layers.py --check`, the `a2engine` tests).
  - Every tool prints the same paths as before with no `A2_PROFILE` set.
  - A stub `fo-roots` descriptor, marked as not offered, shows that a second game
    resolves.
- **Versions.** Root MINOR; MINOR for each layer whose tool interface gains `--game`.

### AR-3: field offsets per build

**Goal:** a build or game whose structs differ stands down or works. It never reads
the wrong field.

1. **Run-time resolution.** `tools/engine/gen.py` emits `field` and `value` entries as
   per-build values resolved at start-up, as addresses are. A field missing from a
   build blocks the hooks that use it, which then stand down and log it. The Rust API
   stays close to today's (`field::X` becomes a cheap accessor). The C twin
   (`csdk/a2engine.h`) does the same. The generated tables are regenerated, and on
   `gog-1.1` every value is unchanged.
2. **render3d's offsets into its profiles.** The `const` offsets, `field(x, 0x..)` and
   `rd(x + 0x..)` in `render3d/core/src/engine/` and `render3d/mods/*/src/hooks.rs`
   become `field` entries in `profiles/gog-1.1/render3d[.<m>].toml`, each with
   `used_by` naming its hook. Do the same for the C plugins' raw offsets and
   `ui/src/engine/sprite.rs`.
3. **Lint.** Rule 4's offset check becomes strict for plugin sources, outside PE
   header parsing, which is not engine knowledge. It is wired into `lint-layers.sh`
   and `tools/engine/lint.py`.
4. **Inheritance.** An inherited profile (the Roots shape) may override a field.
   `tools/engine/fixtures` gains a case where a child build moves a field, and the
   resolvers in all three languages agree on it.

**Acceptance:**
- `gen.py --check`, `lint.py`, the selftests and the `a2engine` tests all pass.
- `render3d/build.sh`, `ui/build.sh`, `display/build.sh` and the C plugins' builds link
  for `i686-pc-windows-msvc`.
- A host test shows every stock value equals the value it replaced.
- In-game: the `render3d-*` and `ui-*` scenarios are pixel-identical under
  `--clock bench`. Those runs are the user's, or a later agent's on the bench; this
  plan's agents have no bench.

**Versions.** render3d, ui, display, qol, rules, online and cutscenes each take a
MINOR, and the root a MINOR. The status line is "Installed, not yet seen in game"
until the bench and the user confirm.

**Risks.** A wrong offset moved into a profile is a crash, so the host test that
compares old constants with new values is mandatory. Run-time accessors cost a load
each; keep them out of the inner loops by reading them once per frame or per object
where a hot path reads many.

### Sketches: AR-4 to AR-10

- **AR-4.**
  - `trait Panel` (rectangle in canvas units, draw, input) and `trait Screen` (dialog
    key, draw).
  - `hud/` and `menus/` reach the engine only through a narrow `engine::api` facade.
  - The canvas comes from the profile, or from the game descriptor per aspect variant.
  - A `ui/lint-layers.sh` rejects `crate::syms_*` outside `engine/`.
  - This is the precondition for FO-6 and A1-8.
- **AR-5.**
  - `trait Vfs { open(path) -> Option<Bytes>; list(dir) }`.
  - Implementations: `Loose(root)`, `Zfs` (Armada 1), `Fpq` (Roots), and `Engine`
    (through the game's own file functions, where an archive's format is not ours to
    read).
  - render3d's assets, `a2fmt`'s ODF reader, `a2tex` and `a2mod` read texture roots
    from the descriptor.
- **AR-6.**
  - `render3d/content/<set>/...` and `ui/content/<set>/...` as data.
  - `by_set` becomes a folder lookup, and a set is a folder.
  - Names out of code, and rule 4 strict.
- **AR-7.** Display8 records through `gpu`'s device and pipeline cache. C1 (the shared
  fixed function, `PLAN-ARMADA1.md`) then lands on one stack.
- **AR-8.**
  - `Services` keyed by a typed id, so a module can offer one without a core edit.
  - `Step` renamed to engine-neutral phases, with each game's `hooks.rs` mapping its
    functions onto them.
  - The walkthrough in `MODDING.md`.
- **AR-9.**
  - One Python installer core (the GUI is already Python, frozen by PyInstaller) with a
    headless entry point.
  - The release's `install.sh` and `install.ps1` start it.
  - `./install` uses it for the copy, merge and retire steps, and keeps its build steps.
  - The selection is remembered the same way everywhere.
  - Retire lists come from `retire/known.txt` only.
- **AR-10.**
  - Every key gets a row; rows are `advanced` where they are not for players.
  - The plugins read defaults from their tables.
  - The shipped inis become commented references or go.
  - `ini-merge.py`, `UPGRADES` and the awk and PowerShell ports are deleted.

## A DLL per game

This is a proposal, open for the user. `PLAN-ARMADA1.md` left it to phase 7, which did
not settle it.

- **Another build of the same game** (Roots) is the same DLL, with its profile chosen
  at run time by inheritance, as designed.
- **Another game** (Armada 1) is a separate DLL built from the same crates. A Cargo
  feature `game-armada1` selects that game's `hooks.rs`, generated tables and content
  sets.
- **Shared code** is the drawing code, `r3d-core`, the UI runtime and widgets, and
  `gpu`. The compiler then enforces "per-game hooks, shared drawing", and each binary
  carries one game.
- **What it needs first:** AR-3 and AR-4, so that no engine read sits outside a game's
  hook files.

## Open decisions

1. A DLL per game (above): a game feature of the same crates, or separate crates under
   `armada1/`.
2. AR-9's language: a Python core (as proposed), or the release scripts kept and only
   tested (AR-1).
3. AR-10: whether the shipped inis go entirely or stay as commented references.
4. Retire code end of life: from which root version an old install must be reinstalled
   from clean, so that about 1.4k lines of migration code can go.
