# Changelog — textures

The texture pipeline (`./a2tex`, `lib/`, `targets/`, `tools/`) and everything it installs
into `Textures/RGB/`. Versioning rules: [`CLAUDE.md`](../CLAUDE.md), "Changelogs and
versions". Newest first. The reasoning behind each change is in [`README.md`](README.md),
not here.

## 2.1.0 — 2026-09-27

### Added
- `a2tex pack NAME [target...]`: the built targets in `$A2_DATA/packs/NAME.zip`, with a
  `pack.txt` of name, date, commit and every file's SHA-256 (`tools/pack.py`).
- `a2tex install --pack FILE [target...]`: refuses a pack with a missing, extra or
  altered file, then runs the ordinary install from it, every guard included.
- `a2tex extract`, the same command as `a2tex stock`.
- `PACKS.md`: making textures and packs, the short path through the pipeline.

Installs nothing different. Checked on the bench: a pack of `MBG02`, `LOADING` and
`Fsovereign` made from the real builds, installed into a clone with no assets, is
byte-identical to the installed game, `logo.SOD` included; a pack with one flipped byte
is refused and installs nothing.

## 2.0.1 — 2026-09-27

### Fixed
- `a2tex install` with no target named installs the targets that are built, and with
  none built says so and exits cleanly. It used to fill `stock/` for all 84 first,
  which on an install that is not the GOG release fails before installing anything.

Installs nothing different.

## 2.0.0 — 2026-09-27

### Changed
- **No asset lives in the checkout any more.** A target is a recipe in
  `textures/targets/<T>/` (`target.conf`, `stock.sha256`) and a work directory in
  `$A2_DATA/textures/<T>/` (`stock/`, `ai/`, `src/`, `src-alpha/`, `out/`, and the
  unit directories); scratch is `$A2_DATA/textures/.scratch/`. `A2_DATA` comes from
  `a2env.sh` (default `~/.local/share/armada2-refit`), so every checkout and
  worktree builds from and into the same place. `a2tex`, `lib/` (by the directory
  `a2tex` hands it), `verify.py`, `upscale-stock.sh`, `measure-invention.sh` and
  `fix-enterprise-registry.py` follow; `inventory.py` reads target membership from the
  committed `stock.sha256` instead of `stock/`. MAJOR: the layout changed, and an
  existing checkout's data has to be moved to `$A2_DATA`.
- The same for the menu backdrops (`menus/backdrop.sh`, `install-plates.sh`): the recipe
  is `menus/backdrops/<name>.conf`, the layers and plate are `$A2_DATA/backdrops/<name>/`.
- `./install` looks for builds in `$A2_DATA`; with none, every layer says so and the
  game keeps what it has.
- `a2env.sh` / `a2env.py` resolve `A2_DATA` (identical to the public copies).

### Removed
- `stock/` of every target, and `MBG02/src/`, from git (1673 files). `stock/` is filled
  from the game on demand as before; `MBG02/src/` is kept in `$A2_DATA` with `ai/` --
  back both up.

Installs nothing different. Checked: every target rebuilt from the same inputs by the old code and by this one --
1677 files, byte-identical to each other and to the builds installed today, with
`stock/` filled from the game into an empty `A2_DATA` (1669 files, every hash
matched). On the bench, public and private stacked into a clone from the real
`A2_DATA` reproduced the installed game byte for byte (2118 textures, `logo.SOD`,
the intro, both plates).

## 1.2.0 — 2026-09-27

### Added
- `./install` at the repository root: every built target (`a2tex install`), the
  cutscene proxy and movies, and the menu plates, into `$A2_GAME`. The public test
  bench stacks it after the public checkout (`a2test session start --install`).

### Changed
- `a2tex` and `tools/` find the game through `a2env.sh` / `a2env.py` (`A2_GAME`, then
  `~/.config/armada2-refit.conf`, then Heroic's default) instead of a hard-coded
  path; `classify-alpha.py`'s `A2_TEX` defaults to `$A2_GAME/Textures/RGB`.

Installs nothing different. Checked on the bench: the stacked install reproduced every
installed texture byte for byte.

## 1.1.0 — 2026-09-26

### Added
- `./a2tex stock [target...]` fills a target's `stock/` from the game install (the
  `.a2neb-backup`, else the live file), accepting a file only if it matches the
  target's new `stock.sha256`. `build`, `install`, `revert <target>`, `diff`, `verify`
  and `tools/upscale-stock.sh` fill it first. `--manifest <T>` writes the manifest.

### Changed
- The layer moved out of the public repository into this private one, with `models/`,
  `archive/` and `promo/`, because `stock/` and everything derived from it may not be
  published. `stock/`, `MBG02/src/` and `Mnebula4/src-generated/` stay committed here,
  beside the new manifests; the `.units/` slices are no longer tracked (derived).

Installs nothing different: the textures in the game are unchanged. `a2tex` itself was
checked (all 84 targets refilled byte-identical to git's copy); not a game change, so
there is nothing to see in game.

## 1.0.0 — 2026-09-25

Baseline: the first versioned release. It versions the layer as it stands; nothing
changed in the texture files themselves.

- **Installed and confirmed in game:** all 23 skybox sets (the `MBG02` atlas plus 22
  six-face sets); all 8 map-puff atlases; 11 story planets and 4 moons; 16 class
  planets and cloud layers; the UI (`UIicon`, `UIpanel`); the hull and prop
  classes (Federation ships and stations, Klingon, Romulan, Cardassian, Borg, Species
  8472, the odd-sized `Xeno*` plates, and the final run of weapons, effects, props and
  map features); the widened mission loading screen (`LOADING`, shipped together with
  the `models` layer).
- **Kept stock on purpose**, with the reasons in `README.md`: `Mbgstars`, `Stars`,
  `colors`, `logos`, `gminicon`/`gminisys`, `MBuild`, `UImid` (`install=no`), and
  `WeaponFX`, which `mip_strays()` refuses.
- `./a2tex list | build | install | revert | diff | verify`. `install` refuses a
  whole target when a mip chain would be left invalid, when an output is over `maxsize`,
  when an interface texture is over 256 px, or when a stock stray mip level would be
  left stale.
- `./a2tex verify` at this release: 1060 textures, 0 problems.

## Before versioning

Dated history from git. The hash is the commit that made the change.

### 2026-09-25
- The layer moved to `textures/`, and `a2tex` exports `ROOT=textures/`. Builds and
  verify output are identical to the old layout (`c3ae017`).

### 2026-09-24
- `LOADING`: the loading-screen art outpainted to 21:9 and installed together with
  `SOD/logo.SOD`. New keys `panel=` and `fit=none`. Confirmed in game (`f55e0d1`,
  `5ecfb50`).

### 2026-09-22
- The final run: 10 targets, 71 textures (weapons, effects, props, map features,
  unclaimed hulls). `Stars` and `Wshldrmd` were dropped on measurement (`d223c5a`).
- Fixed: a fresh clone could not build, because `mktemp` failed in the missing
  `.scratch/` and the build carried on with an empty path (`d223c5a`).

### 2026-09-21
- The Federation ships: 6 targets, 44 textures. Added `mips=auto`, `alphafilter=`,
  `measure-invention.sh` and `selftest-mips.py` (`e75500a`).
- Fixed: the chain guard could not see stock's non-underscore mip spellings.
  `mip_name()` now resolves them and validates by width (`e75500a`).
- Fixed: `verify.py` silently skipped textures whose name is another texture's name
  plus a digit (43 of them) (`e75500a`).
- Federation stations (3 targets) and `classify-alpha.py` (`a774e94`). The `Xeno*`
  odd-sized plates. `alphafilter=` now covers the whole alpha path (`ba7875b`).
- Five factions: 198 textures in 12 targets (`034187b`).
- Fixed: `borgUI3` installed at 1024 from a hull target. The 256 px ceiling now applies
  to every interface texture, whichever target builds it (`interface-textures.py`)
  (`034187b`).
- `INVENTORY.md`, generated by `tools/inventory.py` (`187a10a`).
- The Enterprise registry's two `C` apertures re-cut (`fix-enterprise-registry.py`)
  (`fcd4cb5`).

### 2026-09-20
- The pipeline: `a2tex` over per-target directories. `MBG02` and `Mnebula4` shipped
  (`c0d3f9b`).
- Skyboxes upscaled from their own stock faces (`upscale-stock.sh`). All 22 six-face
  sets were installed and confirmed in game (`53da86d`, `b0e8aee`, `2a49afc`).
- All 8 puffs upscaled from stock. `Mnebula2` crashed the Klingon campaign because
  its mip chain was left behind; it was reverted, and then shipped with `mips=N`
  (`8760059`, `4fd9166`, `f8f616b`).
- Planets and moons: the first 32-bit textures, handled by `write_tga()` depth
  matching, `attach_alpha()` and `blackedge=` (`435ce66`).
- Class planets, and the UI via contact sheets (`sheet=`) (`0fd63ee`).
- Fixed: UI sprites at 512 crashed the game. The UI now ships at 256, enforced by
  `maxsize=` (`d787ff2`).
- Fixed: `Mmoon` drew a white box around its sun. Added `./a2tex verify`, which reads
  raw TGA bytes. `UImid` retired with `install=no` (`b3003ba`).
- The first hull: the Sovereign, with `alpha=ai`. The blend was raised from 35 to 70
  after an in-game check (`7c49cc9`, `0f87f23`).
