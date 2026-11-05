# games — which game the tools work on

One descriptor per game, `games/<id>.toml`: everything the tooling knows of "which game",
so the shared code names none. Not a layer; versioned by the root.

| Descriptor | Game | Offered |
|---|---|---|
| `armada2.toml` | Star Trek: Armada II, GOG 1.1 — the default | yes |
| `fo-roots.toml` | Fleet Operations: Roots ([`PLAN-FLEETOPS.md`](../PLAN-FLEETOPS.md)) | no: a stub, so its paths resolve; FO-1 fills it in |

Armada 1 (`armada1`) gets one of its own by [`PLAN-ARMADA1.md`](../PLAN-ARMADA1.md), A1-1.

## Choosing a game

`A2_PROFILE=<id>` (the environment, or a top-level line of
`~/.config/armada2-refit.conf`), or `--game <id>` on `./install`, `./a2mod`, `./a2test`,
`tools/layers.py`, `python3 a2env.py` and `./a2env.sh`. Without either, the descriptor
marked `default = true` (armada2), so nothing changes for anyone who never sets it.
`python3 a2env.py --list` lists them and marks the chosen one.

`a2env.py` is the one reader: it resolves `A2_GAME`, `A2_PREFIX` and `A2_DATA` from the
descriptor's defaults (the environment and the config file still win), and adds
`A2_PROFILE` and `A2_EXE`; `a2env.sh` takes its values from it. Another game's paths go in
a section of the config file:

    A2_GAME=/mnt/games/Star Trek Armada II
    [fo-roots]
    A2_GAME=/mnt/games/Fleet Operations Roots

Top-level `A2_GAME` and `A2_PREFIX` are the default game's; top-level `A2_HEROIC`,
`A2_PROTON` and `A2_DATA` are every game's.

## The fields

| Field | What it is | Read by |
|---|---|---|
| `id`, `label` | the id (the file's name) and a name for people | everything |
| `offered` | whether players get it: `./install` refuses a game that is not | `tools/layers.py` |
| `default` | the game resolved when none is chosen; exactly one | `a2env.py` |
| `layers` | the `layers.toml` ids on offer (none retired); each layer lists the game in its `games` | `tools/layers.py`, `./install` |
| `exe.names` | the executables, the first the one launched and looked for; a profile build's `exe` is one of them | `a2env.py` (`A2_EXE`), `a2mod`, the bench, `tools/layers.py --check` |
| `exe.data_dir` | where the executable and the game's files are under the install folder (`""`, or `Data` for Roots); the layers install there | `a2env.py`, `a2mod`, the bench |
| `install.heroic`, `install.gog` | Heroic's app names and GOG's product ids | the GUI and release installers, once AR-9 moves them on |
| `install.folder`, `install.prefix` | the default names under `~/Games/Heroic/` and `~/Games/Heroic/Prefixes/` | `a2env.py` |
| `settings.kind`, `settings.file` | the player's settings file and its format (`armada-prf`, `fo-settings-xml`) | the bench's adapter |
| `settings.keep` | the player's own files `a2mod` never switches | `a2mod` |
| `display.shell`, `display.hud_canvas` | the shell's resolution and the HUD's design canvas | the bench (`config.SHELL_RES`); the UI later (AR-4) |
| `assets.textures` | where the textures are, under `data_dir` | `tools/layers.py --check`; `a2tex` and render3d later (AR-5) |
| `paths.stem` | the data directory's name (`A2_DATA`'s default); shared by the games | `a2env.py` |

`tools/layers.py --check` holds them together: every field present and typed, one
default, every `layers.toml` `games` entry naming a descriptor, every descriptor's layers
existing and listing it back, each `profiles/*/build.toml` naming a described game and
one of its executables, and `a2env.py`'s built-in copy of the default game (what the
release package's `a2env.py` resolves without this folder) equal to the descriptor.

Nothing here is derived from the game's files (hard rule 8): names, folder names, ids and
sizes from the plans.

## What does not read it yet

The GUI installer (`gui-installer/`) and the release scripts (`publish/installer/`) keep
their own discovery until AR-9 puts them on one installer core. A few literals remain
where only Armada II is handled today: `Textures/RGB` in `./a2tex` and render3d's assets
(AR-5), the UI's canvas (`ui/src/engine/hud.rs`, AR-4), Display8's trace line naming
`Armada2.exe`, and the bench's `ARMADA.PRF` parsers, which only the `armada-prf` kind
uses.
