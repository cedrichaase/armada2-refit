# Working in this directory

Tooling for remastering Star Trek: Armada II — Display8, our own Direct3D 8 on Vulkan
and Direct3D 12; the 3D renderer (`Render3D.asi`) and the HUD, menus and loading screen
(`UI.asi`) drawn on it, as run-time plugins; the quality-of-life, rules and online
plugins; the texture pipeline (`a2tex`), the cutscene proxy and the movie and
menu-backdrop pipelines; and `a2mod`, which switches the whole game between stock and
refit. `PLAYING.md` is for players, `MODDING.md` for modders; this file is the working
brief for agents.

**This repository is published.** It holds code, configuration, recipes and docs only —
nothing derived from the game's own files (hard rule 8, `publish/README.md`). **Every
asset lives in `$A2_DATA`**, never in a working tree: the stock art extracted from the
player's own install, the paid `ai/` layers, every intermediate and every build, one
work directory per texture target, movie and menu backdrop. `A2_DATA` comes from
`a2env.sh` (default `~/.local/share/armada2-refit`), and every checkout and
worktree shares it. **Every layer must work without assets**: with nothing built, an
asset layer installs nothing and says so, and the game keeps its own art.
`./a2test run no-assets` checks it; run it after changing any installer.

## Layout

The repo is laid out by **`a2mod` layer**: each folder is one thing `./a2mod` switches
between stock and refit (or, for `platform/`, something it deliberately leaves
alone). `layers.toml` is the one list of layers; `./a2mod status` names them, and
retired ones (`hud`, `menus`, `grid`, `renderer`, ...) show as retired rows. Every layer
folder has:

- a **`README.md`** — how the layer works and *why*: derivations, measurements, traps;
- a **`CHANGELOG.md`** — *what* changed, when, at which version, and whether it has been
  seen in game. **Read it before assuming the state of a layer**, together with the
  pending changesets in `.changeset/` that name it (`tools/changeset.py status`). Every
  change adds a changeset; the release writes the entry (see "Changelogs and versions").

The root `CHANGELOG.md` is the project's: the modpack version and the layer versions
it bundles.

| Folder | What it is | Start with |
|---|---|---|
| `qol/` | `QOL.asi` — gameplay quality of life compatible with stock players (right-drag pan speed, Shift+number adds to a group, selections and groups beyond 16, stations in groups with one build menu for several, long moves on the map at warp, the button bar's position keys, `BarKeys`, with the UI drawing their labels), and the plan for the rest, split by whether other players need it | `qol/README.md` |
| `rules/` | `QOLRules.asi` — the one change every player of a network game must share (pay when queuing, refund on cancel: QOL-7, `PayOnQueue`, on by default). A layer of its own, `scope = 'match'`: a player without it makes the game play stock rules for everyone; Online finds it by its export (`QOLRules_Wanted`) | `rules/README.md` |
| `render3d/` | `Render3D.asi` — the 3D view through a renderer of our own, in `no_std` Rust, a workspace: the cdylib `render3d/src/lib.rs` (one Cargo feature per module), `core/` (`r3d-core`: frame, gfx, scene, assets, `core/src/engine/`, the menu scene, module/registry/hooks/services/recipes) and one crate per module under `mods/<m>/` that depends on the core alone; engine knowledge on the engine profile (`profiles/gog-1.1/render3d[.<module>].toml`), no address in its sources; recipes in `render3d/content/{sky,surfaces,nebulae,ordnance,menu}/…`. Both views of `Armada_RenderAllOurStuff` drawn into a half-float MSAA target, bloomed by energy and tone-mapped into the back buffer; eight modules of its own (`[Modules]`: hulls, sky, particles, effects, ordnance, specials, world, overlays) draw every category (hulls with the scene lights and shadows, sky, particles, ordnance, effects, the special weapons' effects, planets, moons, asteroids and nebulae, fog and selection) from the game's assets, with its own cosmetic simulation (`PLAN.md`). Each module is live (its Refit row applies at once) and a module at 0 hands its category back to Display8's passthrough. It replaced the per-part plugins (MSAA, Lighting, Sky, Nebulae, Surfaces, Explosions, Shields, Repair, Venting, Specials, Fog and Planets), which were removed from the repository; their recipes live under `render3d/content/`, and `retire/` takes out any of those plugins an earlier install left | `render3d/README.md`, `render3d/PLAN.md` |
| `ui/` | `UI.asi` — the game's whole 2D side, in `no_std` Rust like render3d: a 2D renderer on the Display8 device (SDF text from a vendored OFL font, rounded panels, nine-slices), immediate-mode widgets, input with capture, and hooks into a mission's frame, the front end (the menus it hosts and draws, `[Menus]`, `MENUS.md`, and the Refit screen) and the loading screen. Its HUD panels and button bar (`[HUD]`, `HUD.md`) replaced the legacy `HUD.asi`, `GridLayout.asi` and `Menus.asi`, which are gone; every part is on in the shipped `UI.ini` (a key at 0 gives a part back to the engine, drawn by Display8's passthrough). `ui/backdrops/` builds the widescreen plates the menus composite (`backdrop.sh`, `install-plates.sh`); `ui/bench-asi.sh` puts a working copy's build in a bench clone | `ui/README.md`, `ui/PLAN.md`, `ui/STYLE.md`, `ui/backdrops/BACKDROPS.md` |
| `display/` | `d3d8.dll`, "Display8": the game's Direct3D 8, ours — the only one: nothing stands behind it. Its devices accept everything the engine does and draw nothing; render3d and the UI draw the frame on Display8's own Vulkan or Direct3D 12 device (`Backend=auto\|vulkan\|d3d12\|none`, `auto` shipped), and Display8 draws, by passthrough, whatever no plugin claims. When neither backend starts, or with `Backend=none`, it hands the game the system's own `d3d8.dll` (the bail-out) and the plugins stand down. No forward mode, no Direct3D 9. The contract with the plugins is `gpu/src/display.rs` (1.5); `[Window]` (the display mode, `KeepClip`) and `[Samplers]` (anisotropy and LOD bias) are its ini sections. On the bench it traces every call by context and caller (`Display8.inventory.tsv`), names the engine's draws (`Display8.go`) and drops them by category (`[Drop]`). `display/d12probe/` and `display/vkprobe/` are the native gates' probes | `display/README.md` |
| `retire/` | not a layer: `retire.sh`, which takes out of a game directory everything an earlier version put in and nothing installs any more, by the table in `known.txt` — the legacy `HUD`, `GridLayout`, `Menus` and `MenuScale` plugins, the twelve per-part render3d plugins, the font atlases, cursors, UI configs and loading screen that `hud-files.py` puts back from their backups, the vkBasalt bloom, and with `--chain` (which `./install` passes) the old Direct3D chain (`d3d8_real.dll`, `d3d9.dll`, a marked `dxvk.conf`, whose samplers go into `Display8.ini` `[Samplers]` first). `./install`, `render3d/install.sh`, `ui/bench-asi.sh` and the release installers run it; `history/` holds the changelogs of the deleted `hud`, `grid`, `menus` and `renderer` layers | `retire/README.md` |
| `textures/` | the texture pipeline: `lib/`, `tools/`, and 83 `targets/` — recipes only | `textures/README.md` |
| `models/` | 3D geometry — the smoothed dilithium moons (`moon-sod.py`) and asteroids (`asteroid-sod.py`), and the other SOD rewrites. The planets' sphere is render3d's; the widened loading screen is gone (the UI draws the loading screen) | `models/README.md` |
| `cutscenes/` | `binkproxy/`, a `binkw32.dll` that plays AV1 replacements full screen, and the `movies/` recipes | `cutscenes/binkproxy/README.md` |
| `online/` | `Online.asi` — online multiplayer: the *Internet – Online* menu entry, our own `IDirectPlay8Peer` over UDP (`peer.c`), and `server/a2online-server.py` for join codes, hole punching and the relay, public at `c20e.de:2399` (the default `Server=`; the bench blanks it). A layer of `./install` and `a2mod`, `scope = 'match'` | `online/README.md` |
| `platform/` | what `a2mod` never switches: the ASI loader, the widescreen patch, Heroic/Proton and their Wine overrides (`winmm=n,b;d3d8=n,b`); `vendor/` holds the third-party binaries we may redistribute. It carries no Direct3D chain any more (d3d8to9 and DXVK left in platform 4.0.0; `d3d8-chain.py --retire` takes an old one out) | `platform/README.md` |
| `layers.toml` | not a folder: the layer registry, every layer once (id, install and remove commands, files, backups, `select` = core / on / off / retired, `requires`, `scope` = local / match, `games`, release `steps`). `tools/layers.py` is its only reader; `./install`, `a2mod`, the bench (`--layers`, `Layers:`), `publish/package.sh`, the release scripts and the GUI installer take their layer lists from it, and `tools/layers.py --check` (CI) fails when they disagree | `layers.toml`, `tools/layers.py` |
| `games/` | not a layer, versioned by the root: one descriptor per game (`armada2.toml`, the default; `fo-roots.toml`, a stub not offered), "which game" as data — its executable and data folder, Heroic and GOG ids, default folder and prefix names, settings file, shell resolution and HUD canvas, texture roots, data directory stem and the layers on offer. `a2env.py` reads it; `A2_PROFILE` or `--game` (on `./install`, `a2mod`, `a2test`, `tools/layers.py`, `a2env`) chooses the game; `tools/layers.py --check` holds it to `layers.toml` and `profiles/` | `games/README.md` |
| `profiles/` | not a layer, versioned by the root: one directory per build of `Armada2.exe` (`build.toml` plus one `<plugin>[.<part>].toml` per plugin), only names, addresses, offsets and checks tied to hooks; `tools/engine/` generates the plugins' tables from it, lints it and tests the three resolvers. Another build (Fleet Operations: Roots, `PLAN-FLEETOPS.md`) is a new profile | `crates/a2engine/README.md`, `tools/engine/README.md` |
| `crates/` | not a layer: `a2rt`, the one runtime of Display8, render3d and the UI (allocator, Win32, log, `sfmt!`, ini with the overlay lookup, files, in-memory patching, frame dumps), linked into each as its `rt`; `a2engine`, the resolver of `profiles/` (`no_std`; the C twin is `csdk/a2engine.h`, the host tools `tools/engine/`), which says which build of `Armada2.exe` is running and whether each hook may hook it; `a2fmt`, the readers of the game's SOD, TGA, `.spr` and ODF files | `crates/a2engine/README.md`, `crates/a2rt/src/lib.rs`, `crates/a2fmt/README.md` |
| `crates/a2ini` | not a layer: the ini parser (shared by reader and writer) and `edit`, the line-keeping writer of the player's overlay `RefitPlayer.ini`, `no_std`, host-tested on `tools/settings/fixtures/` | `tools/settings/README.md` |
| `tools/settings/` | not a layer: the settings contracts (A2SET_ABI 1.1) — `gen.py` writes each layer's `settings_table` (the descriptor row itself is written by hand in `a2rt` and `a2asi.h`: neither has generated regions), `lint.py` checks the tables, `host.sh` builds the `a2ini` host binary, the one overlay editor, which `overlay.py` wraps for the bench (the GUI installer only reads the overlay, for its report), `move-keys.py` moves a row's `moved_from` key in the plugin inis, `selftest.sh`, `bench-overlay.sh`. Installers, `a2mod` and the release never write, ship or remove `RefitPlayer.ini`; the Refit screen is its only writer in the game. Its sections are `[<Stem>.<Section>]`, the plugin ini's name and section (`[Render3D.Modules]`, `[UI.HUD]`, `[Display8.Display8]`) | `tools/settings/README.md` |
| `tools/` (the rest) | not layers: `ini-merge.py`, the one ini merge (keys added, the player's values kept, retired defaults moved); `layers.py` (above); `changeset.py`, the changesets and the release's version step (`.changeset/README.md`); `doclinks.py`, which checks the relative links and anchors of every tracked `.md` (run it after moving a file or renaming a heading); `engine/` (the profile tools); `gates/` (the native gates' kit: `gate-windows.ps1`, `gate-macos.sh`, `GATES.txt`; CI attaches it as `native-gates-<commit>`; a test tool like the bench, not an installer); `shaderc/` (below) | `tools/ini-merge.py`, `tools/gates/GATES.txt` |
| `csdk/` | not a layer: `a2asi.h`, the header-only base the C plugins (`qol/`, `rules/`, `online/`, `cutscenes/binkproxy/`) are built on (Win32 types and imports, string and log helpers, signature checks, `VirtualProtect` patching, the overlay lookup), `a2engine.h`, and `def/`, their one set of import-library `.def` files | `csdk/a2asi.h` |
| `gpu/` | not a layer: the graphics interface render3d and the UI draw through (`no_std`, linked into each), on Display8's Vulkan or Direct3D 12 device — no Direct3D 9 (gpu 2.0.0); stood down (nothing drawn) with none; `gpu/shaders/compat.hlsli` lets one HLSL source build SPIR-V and DXIL (dxc only) | `gpu/README.md` |
| `tools/shaderc/` | not a layer: the shader compiler scripts every crate with shaders builds through (`hlsl-rs.sh` to SPIR-V and DXIL, the pinned dxc fetcher, `verify-generated.sh`, which CI runs); installs nothing, versioned by the root | `render3d/README.md`, "One source, two forms" |
| `testbench/` | `./a2test`: the game headless at any resolution, scenarios, reports. Installs nothing. `bench/legacy.py` builds the legacy `HUD.asi`, `GridLayout.asi` and `Menus.asi` once from the last tag that has them into `$A2_DATA/bench/legacy/<tag>/`, for the three places the bench still needs them (the `Stock shell: embed` reference case, `display/bench-abi.sh old-display`, and the old install that `retire-old-install` seeds and cleans; `testbench/README.md`, "The legacy plugins") | `testbench/README.md` |
| `films/` | `./a2film`: storyboards (`films/<name>/storyboard.md`) filmed on the test bench — probes at 1280x720 in parallel, final takes at 4K — and cut into a rough cut and an edit manifest. Installs nothing | `films/README.md`, the `film` skill |
| `publish/` | what may be published and the check that enforces it. Installs nothing; versioned by the root | `publish/README.md` |
| `gui-installer/` | `armada2-refit-installer.py`, the graphical installer for Linux (GTK 4 and libadwaita, released as an AppImage), Windows and macOS (Tk, frozen by PyInstaller into a portable `.exe` and a universal2 `.app`), over a starfield drawn in code: installs GitHub release zips, cached, with a choice of layers, and writes the launch settings (Heroic's variables on Linux, the Wine prefix's registry on macOS; Windows needs none). Versioned on its own, with the release-package schema it reads (`manifest.json`, the scripts' `::step` lines). Leaves out textures and cutscenes for now | `gui-installer/README.md` |
| `PLAYING.md`, `MODDING.md` | not code: the two published guides. `PLAYING.md` is for players (installing, the Refit screen, `RefitPlayer.ini`, the backends and the bail-out, logs, troubleshooting, uninstalling); `MODDING.md` is for modders (which unit needs what, the contracts and their versions, the symbol profiles, building one layer). The root `README.md` is the front page and links both | `PLAYING.md`, `MODDING.md` |

`./a2mod` is the entry point and stays at the root, with `./a2test` beside it for
testing, `./a2film` for films, `./a2tex` for textures, `./install`, which installs every
layer `layers.toml` selects (`--only`, `--without`), and
`a2env.sh` / `a2env.py`, which say where the game and the assets are. `./a2tex` exports
`ROOT=textures/`, so everything under `textures/` addresses `$ROOT/lib`,
`$ROOT/targets` and `$ROOT/tools`.

**Never hard-code a path to the game, its prefix or Proton, or the game's executable.**
Source `a2env.sh` (shell) or `import a2env` (Python, with the repository root on
`sys.path`): they resolve `A2_GAME`, `A2_PREFIX`, `A2_PROTON` and `A2_DATA` (where assets
live, default `~/.local/share/armada2-refit`; never a working tree) from the environment,
then `~/.config/armada2-refit.conf`, then the chosen game's descriptor
(`games/<id>.toml`; `A2_PROFILE`, default `armada2`), and add `A2_PROFILE` and `A2_EXE`.
`a2env.py` is the one implementation: `a2env.sh` evaluates its `--sh` output, so there is
no pair to keep in step. A Python tool that takes `--game` calls
`a2env.use(a2env.take_game_arg(sys.argv))` before it reads the paths.

**To see a change working in the game without taking over the user's screen, use the
test bench**: `./a2test session start --res 21:9`, then `./a2test drive --session ID shot`
/ `click` / `key` / `step "…"`, and read the screenshots. It runs on a reflink clone of
the install and a headless display. **Keep the ID `session start` prints, pass
`--session ID` on every `drive`, and end with `./a2test session stop ID`** (the ID is
required). Other jobs run sessions on the same bench: stop only the sessions you
started, never one from `session list` you don't recognise (`testbench/README.md`,
"Which session a command acts on"). **The bench runs on the user's own GPU**: a GPU
hang in a bench game crashed the user's desktop once, so run at 1280x720 unless fine
detail is judged and stop sessions promptly. **Bench limits, the
user's, across all agents**: at most **two** bench games at once (a two-player
multiplayer scenario only when no other game runs), at most **one** on Direct3D 12
(`Backend=d3d12` runs on the GPU with no CPU driver and no validation that reaches the
log: 1280x720, a `Timeout:` of at most 15 minutes, stop the session as soon as its shots
are taken, and stop at the first `device removed` in `Display8.log`). Memory is the other
limit (15 GB; systemd-oomd has killed the session twice): `CARGO_BUILD_JOBS=2`, one
build at a time, no build while your own game runs, start a game only with at least
4 GB available, and keep about four agents, not six. The bench's slot queue (`./a2test slots`) allows three games on this machine unless `A2_SLOTS=2` is set; the limit of two holds either way. A run whose result is a GPU time in
milliseconds (`Timing=1`) runs alone, since other games skew it, while pixel
comparisons under `--clock bench` are unaffected. **Vulkan on the bench runs on the
user's GPU with the validation layer on**; `--vulkan-device lavapipe` (the CPU driver)
is for validation runs and is judged against the GPU baselines (`p10-vk-*`), never
against another backend (`testbench/README.md`, "Vulkan on the bench"). **Say what you are doing**: `./a2test drive --session ID note "..."`
when you start driving and whenever your intent changes (the dashboard, `./a2test watch`,
shows it). Before driving the game, read `testbench/README.md`, "Operating the
game: what agents learned", and the staging recipes in `testbench/operating/`. Write regression scenarios in `testbench/scenarios/` for fixes
worth keeping fixed, and wait on what the game logs rather than for a fixed time
(`testbench/scenarios/README.md`, "Waits and checks"). A bench pass is evidence for the user, not their sign-off
("Finishing work" still applies).
**To test a checkout as installed**, `./a2test session start --install <checkout>` (or
`a2test run ... --install ...`) starts the clone from stock and runs that checkout's
`./install` into it; the report records its commit. The builds come from `$A2_DATA`,
so every checkout and worktree installs the same ones. `A2_DATA=<empty dir>` shows the
game as someone without a texture pack gets it.

Required reading, by task:

- **Anything renderer-adjacent: `platform/README.md`** (DLL overrides, the widescreen
  patch, the d3d8 slot, window management under Hyprland, the save format) **and
  `display/README.md`** (the backends, the bail-out, the contract).
- **A change a player or a modder will meet: `PLAYING.md` / `MODDING.md`.** Keep them
  true when a key, a screen, a contract version or a build step changes.
- **Making textures or packs, as a user would: `textures/PACKS.md`**, the short path
  through the pipeline. Keep it true when the commands change.
- **Any texture work, or anything that touches a texture file** (the font atlases, the
  cursors): `textures/README.md`. It holds the engine reference — TGA format, mip
  chains, what counts as an interface sprite, the two nebula systems. Do not re-derive
  any of it; it was established by measurement and several wrong turns.
- **Any texture that is not a nebula: `textures/REMASTERING.md`** — over half the set
  has a real alpha channel that these scripts would destroy.
- **Changing how an effect looks: `ART-DIRECTION.md`**, the user's direction for the
  effects (beams, shield hits), with what the reference frames measure.
- **"What is left": `textures/INVENTORY.md`**, generated by
  `textures/tools/inventory.py --md` from the game directory and this repo. Do not
  hand-edit it and do not re-derive its numbers in prose; re-run the tool.

## Changelogs and versions

Every layer folder in the table above has a `CHANGELOG.md` with its own semver version,
and so do `gui-installer/`, `testbench/` and `films/` (`.changeset/config.toml`). The
root `CHANGELOG.md` versions the project as a whole. **Versions are never edited by hand.**
A change records itself as a **changeset**: a small file in `.changeset/` that names the
layers it touches, the bump each takes, its in-game status and what changed.
The release assigns the numbers (`.changeset/README.md` has the format).

    python3 tools/changeset.py add --layers render3d,qol --bump minor --section Added -m "..."
    python3 tools/changeset.py status      # each layer's current -> next version, and why

**A commit that changes a layer adds a changeset in the same commit**, or edits the
branch's changeset for that work. A changeset written "later" gets forgotten. Do not touch
a `## X.Y.Z` heading, a `Cargo.toml` version or `INSTALLER_VERSION`.
`tools/changeset.py check` (CI) fails when a version file disagrees with its changelog,
and a pull request fails when it changes a version by hand.

**What gets a changeset:** anything that changes what a layer installs into the game, how
it builds, or its interface (CLI flags, `target.conf`/`.ini`/`.conf` keys, backup
names). This includes tuning changes like a new blend, seed or threshold, and a revert
of shipped work. **None:** changes to docs and notes only, and scratch work that
installs nothing.

**Which bump** (the changeset's `bump:`, or `<layer>: <bump>` per layer):

| Bump | When | Examples here |
|---|---|---|
| MAJOR | an existing install cannot move to it by just re-installing: the install/revert contract, backup suffix or `a2mod` snapshot layout changes; a key or flag is removed or renamed; paid `ai/` layers are invalidated; the layer newly requires something of `platform/` | a new backup suffix; dropping `blend=` for another key |
| MINOR | new content or capability: new targets or texture classes installed, a new menu handled, a new key, flag or tool | the five faction targets; `Embed=1`; `NoiseFloor=` |
| PATCH | a fix or re-tune behind the same interface | Sovereign blend 35 → 70; main-menu plate seed 6; the `Mmoon` sun-box fix |

At release, each layer's next version is its current one plus the largest bump among the
pending changesets that name it. The **project version** is the current root plus the
largest bump of all of them. Changes that belong to no one layer, such as `a2mod`, the
repo layout or a cross-layer convention, name `root` (`layers: [root]`) and bump only
the project. The release writes the root entry with the `Layers:` line listing each
layer's new version.

**Entry format** (written by `changeset.py version`, newest first): a heading
`## X.Y.Z — YYYY-MM-DD`, then `Added` / `Changed` / `Fixed` / `Removed`. Each item is one
or two lines saying *what* changed, naming the key or target, and ends with the hash of
the commit that added its changeset. The *why* stays in the layer's README. Do not copy
derivations into a changeset, and do not copy status ("confirmed in game", dates,
accepted seeds) into this file: status belongs in the changeset and its entry.

**In-game status goes in the changeset.** Its `status:` line, by default "Installed, not
yet seen in game", ends the entry the release writes, and makes the release an
**untested release** (GitHub's *Pre-release* flag). So does a line starting "Untested"
(seen, but not past the Windows gate). When the user confirms the change **before it is
released**, edit the changeset: `status: Confirmed in game YYYY-MM-DD`. When the user
confirms it **after release**, replace "Installed, not yet seen in game" with
`Confirmed in game YYYY-MM-DD` in the layer and root entries and do not bump. That is
the one permitted edit to a released entry. Don't keep both: an entry that still says
"not yet seen" after it has been seen is misleading. A rejection is not a status line.
It is a new changeset that reverts the change, like the font's `--method runs`.

**Releasing** (`publish/README.md`, "Releasing"). The user starts it by hand: the
Release workflow runs `changeset.py version --commit` on main and opens the pull request
"Release X.Y.Z". **Merging that PR publishes vX.Y.Z** (with a merge commit or a rebase; do not squash). CI recognises the merge by the
release commit's `Release-Version` trailer, and no other push releases anything. Every
push to main also publishes a **pre-release**, `vX.Y.Z-pre.N`, the build of the latest
main (N is CI's run number; the newest three are kept). The GUI installer offers the
newest last, as "Pre-release (latest main)", but never picks it for the player. Do not confuse it with an **untested release**, a
`vX.Y.Z` not yet seen in game. Never run `changeset.py version` on a work branch: only
the release branch does that.

**Parallel worktrees** no longer clash over numbers: each branch adds its own changeset
files, which merge without conflict, and no branch assigns a version. Name a changeset
after its work (`move-paths.md`), not a number.

**A branch with hand-numbered entries** (begun before changesets, such as
`ar-integration` or `move-queue-2`) converts them before it merges:
`python3 tools/changeset.py convert` (against its merge-base with `origin/main`, or
`--base REF`; `--dry-run` first). This turns each layer's new entries into
`.changeset/converted-<layer>.md`, with the bump from main's version to the branch's
newest and the status lifted out of the text ("not yet seen" wins). It then takes the
entries out of the changelogs and puts the version files (`Cargo.toml`, `Cargo.lock`,
`INSTALLER_VERSION`) back to main's. Root entries are dropped when the branch has layer
entries, because those carry the items, so read the note it prints. Review the changesets,
commit, then merge `origin/main`. The changelogs no longer conflict.

## Finishing work

Work is done when the user has confirmed it working in game **and** signed it off as
done. A test launch, or a passing `verify`, does not count. Then, without being asked
again, finish in this order:

1. **Record the confirmation** on the branch: `status: Confirmed in game YYYY-MM-DD` in
   the branch's changeset(s) (above), and commit. If the work was already released, edit
   the released entries instead.
2. **Merge into `main`** from the main checkout, with
   `git merge --no-ff <branch> -m "Merge <branch>: <what>"`. There are no version numbers
   to resolve: changesets merge side by side. Don't push unless the user asks. A push to
   `main` publishes the pre-release (the latest main), and no versioned release:
   releasing is the user's call, through the Release workflow and its pull request
   (`publish/README.md`, "Releasing").
3. **Clean up.** Remove the worktree (`git worktree remove`) and delete its branch
   (`git branch -d`). Then do the same for any other worktree whose branch
   `git branch --merged main` lists. Use only the safe forms, never `--force` or
   `-D`. If one refuses because of uncommitted changes or unmerged commits, leave it
   and tell the user.

Until the sign-off, the work stays on its branch and in its worktree.

## Environment

- Game: `$A2_GAME` (`./a2env.sh` prints it; here, Heroic's `~/Games/Heroic/Star Trek Armada II`) — GOG release, patch 1.1, run
  through Heroic with Proton-CachyOS. **No DXVK and no d3d8to9 chain**: the game's
  `d3d8.dll` is Display8, which draws through Vulkan or Direct3D 12 itself (Wine's
  overrides are `winmm=n,b;d3d8=n,b`), and falls back to the system's own `d3d8.dll`.
- Textures: `Textures/RGB/`, flat, **2115 `.tga` files, 196 MB stock**, **mixed
  `.tga` / `.TGA` case**. (196 MB is the byte total; `du` says 205 MB, because 2115
  small files carry ~9 MB of block slack.)
- Available: ImageMagick 7 (`magick`), `python3`, `ffmpeg`.
  **Not available: numpy, PIL.** Do image work through ImageMagick, not Python.
- **32-bit Windows DLLs can be built here**, which is not obvious: there is no MSVC and
  no mingw, but `clang` + `lld-link` + `llvm-dlltool` are installed and that is enough
  for an ASI plugin. Declare the Win32 prototypes yourself, generate import libraries
  from `.def` files (**`llvm-dlltool --kill-at`** — the `.def` carries decorated names,
  the real DLLs export undecorated ones), and link `/nodefaultlib`. See
  `qol/build.sh`. `objdump` also reads `pei-i386`, and the game ships
  `armada2.map` — a full symbol map — so `Armada2.exe` can be read rather than guessed at.
- Scratchpad for intermediates; this directory is the user's, keep it tidy.

## Hard rules

1. **Never write into `Textures/RGB/` directly.** `./a2tex build` writes to
   `$A2_DATA/textures/<NAME>/out/`; `./a2tex install` copies it across, making a `.a2neb-backup`
   on first touch. `./a2tex revert all` sweeps every `*.a2neb-backup` in `Textures/RGB` —
   the cursors included — and restores it.
   **There is now no exception, only history**: `hud/ui-font-condense.py` (the legacy HUD
   layer, deleted) rewrote the twelve `FontFinal4_*` atlases in place, paired with the
   `.spr` metrics beside them in `Sprites/`, and owned its backups under
   `.a2font-backup` — a *different* suffix, precisely so `a2tex revert all` could not
   restore a stock atlas under condensed metrics and garble every glyph. An old install
   may still hold them; `retire/hud-files.py` (run by `./install` through `retire/`)
   restores atlas and metrics together from the same suffix, and likewise the UI configs
   in `misc/` and the cursors, whose `.a2neb-backup`s it also handles. Apart from that
   restore, nothing writes into `Textures/RGB/` outside `a2tex` now.
2. **Match the stock TGA format exactly**: image type 2, uncompressed, no ID field, no
   colour map — and **the same bit depth as the file you are replacing.** The nebula
   textures are 24-bit; **1113 of the 2115 textures in the game are 32-bit with a live
   alpha channel.** `write_tga()` reads the depth off the stock file it is given as
   its third argument and writes to match, and `attach_alpha()` carries the stock mask
   across, Lanczos-upscaled. **Alpha does not go through the generative upscaler unless
   the target says so** — a *mask* has only edges, which Lanczos resolves exactly, and a
   model that invents plausible detail into a mask invents holes in the object. The test
   is what the channel *means*, not that it is the fourth one: on a hull texture the
   alpha is a self-illumination map — lit windows, deflector and nacelle glow — which is
   picture content, and `alpha=ai` in `target.conf` upscales it like colour, into
   `src-alpha/`. Measured on the Sovereign at 0.001% invented light. Opt-in per target,
   and it stays opt-in; `textures/tools/classify-alpha.py` is the test.
   **Resize *and composite* colour and alpha separately, always.** Joining them early
   and touching the result is the single most repeated mistake in this project — it has
   bitten three times (the Lanczos layer in `upscale-stock.sh`, `gen_mips`, and
   `blackedge` on `Mmoon`, which rendered as a white box around a sun sprite in game).
   Resizing an RGBA image associates alpha and then un-associates it, dividing colour
   back out by a near-zero alpha; that put `Mmoon`'s Lanczos layer at mean 137 against
   stock's 43, and `fit()` then *hid* it by scaling the whole plate to match the mean.
   Colour and alpha are handled apart and joined only at the very end.
   For 24-bit nebula work: `-alpha off -type TrueColor -compress None`, then
   `textures/tools/bottomup.py <file>`. Verify the header after writing; a format mismatch will
   not announce itself. ImageMagick 7 cannot be told to write bottom-up TGAs — every
   `-define tga:image-origin=...` spelling is ignored — so the row flip must be a
   post-pass. `bottomup.py` is in-place and idempotent.
   **Stock is not uniformly bottom-up.** Of the 135 skybox faces, 53 are `0x00` and 82
   are `0x20`, and the split runs *within* a single set: `MbgBorg.2` and `.5` are
   bottom-up while the other four faces are top-down. Both render correctly, so the
   engine honours the descriptor. `write_tga()` copies the stock file's origin
   (`bottomup.py --like`), which makes an accidental vertical flip impossible by
   construction rather than by argument.
3. **Always glob both extension cases.** `$(ls "$n.tga" "$n.TGA" 2>/dev/null | head -1 || true)`
   — and note the `|| true`: under `set -o pipefail` the failing `ls` aborts the script.
4. **Never install a UI sprite larger than 256x256.** `@tmaterial=interface` textures at
   512 **crash the game** at the cinematic-to-HUD transition (`rep movsd` in
   `Armada2.exe` -- a memcpy into a buffer that is too small). Exactly one stock texture
   in the game is 512 (`WshladSW`, a weapon) and no stock interface sprite is over 256.
   3D model textures are unaffected -- 2048 skyboxes and a 4096 atlas run fine -- so this
   is the sprite path specifically. `maxsize=N` in `target.conf` enforces it per target,
   and `a2tex install` also refuses any interface texture over 256 whatever target
   builds it (`textures/tools/interface-textures.py`), because a target's membership
   does not tell you what the engine thinks a texture is.
5. **A mip chain pins its base size, and is spelled the way STOCK spells it.**
   Changing a base without its hand-authored chain **crashes the game** — `Mnebula2` did,
   in the Klingon campaign. `mips=N` (or **`mips=auto`**, which takes the depth per
   texture from stock) makes the build emit the whole chain, and `install` validates it
   pre-flight, refusing the entire target rather than writing a partial one. `revert
   <target>` restores the chain too. `textures/REMASTERING.md` counts 363 chains, so this
   comes up constantly on hull work.
   The spelling is not always `_N`: `fcruise1_B`'s levels are `fcruise1_B1` and
   `fcruise1_B2`; `fresearch`'s are `fresearch1` and `fresearch2`. Both are real
   hand-authored chains, confirmed by box-downsampling the base and measuring RMSE
   against the sibling (0.022/0.026 and 0.033/0.054, inside the 0.017–0.067 band the
   ordinary underscore chains occupy). The engine looks a chain up **by name**, so
   writing `fcruise1_B_1` produces a file nothing reads and leaves stock's 128px level
   under a 1024px base. `mip_name()` in `textures/lib/common.sh` is the single
   authority; it tries both spellings and **validates by width**, which is why it cannot
   be a regex over names: `FluidicRift2` is 128 beside a 256 `FluidicRift`, level 2 by
   name and level 1 by size, so it is not a mip of it at all.
   Stock also ships siblings that are named like levels and are not sized like them —
   `Fsensor_B_1`/`_2` are both 128 beside a 128 `Fsensor_B`, and `FpremNew_B_2` is 128
   where level 2 wants 64. (Each pair is byte-identical to itself, which is what a
   placeholder looks like.) `mip_strays()` enumerates them and `a2tex install` refuses
   the target rather than leaving one stale beside a bigger base.
6. **`--mp` is not the scale factor, and the mismatch is silent.** The app rounds
   `sqrt(mp*1e6/(w*h))` to an integer, so `--mp 4` on a 256px source returns **2048, an
   8x lift**, not 4x — and it caps at 2048, so a sheet edge that does not divide 2048
   comes back at a non-integer ratio the slicer truncates to 1x. Anything comparing an
   `ai/` plate against a Lanczos layer must read the plate's size off the file. Building
   the Lanczos layer at an assumed 1024 and compositing a 2048 plate onto it does not
   error: ImageMagick composites at the origin, so the comparison is the top-left
   quarter of one plate against the whole of the other. It reads as catastrophic
   erasure — markings "vanish" because they are outside the crop — and it produced a
   confident, wrong conclusion here that `blend=70` was destroying the Galaxy's
   deflector dish. `textures/tools/measure-invention.sh` reads the size off the plate.
7. **Don't touch `Sprites/nebula.spr`.** UVs are normalised against `@reference=128`, so
   they are fractions and larger textures land on the same quadrants unchanged. Editing
   it is never the fix.
8. **Commit nothing derived from the game's own files, and write no asset into a
   checkout.** This repository is published (`publish/README.md`): code, configuration,
   recipes and docs only. No stock copy, no upscale, no comparison plate, no screenshot,
   no game video — at any path. Assets go in `$A2_DATA`. `.gitignore` ignores every
   image, video and game format, and every data directory a pre-`A2_DATA` checkout may
   still have, so it takes a `git add -f` to break this; don't. `publish/check.sh`
   fails on any binary in the index; run it before committing anything unusual.
   **The one exception is third-party binaries whose licence expressly allows
   redistribution** (MIT, zlib and the like; "freeware" or no licence does not count).
   They are vendored unmodified under `platform/vendor/<name>-<version>/`, beside their
   licence text and a `SOURCE.txt` that says where they came from and lists each
   file's sha256, and `check.sh` accepts a binary there only if both are present and
   the hash matches.
9. **Describe the engine, never reproduce it — and only where a plugin needs it.** The
   same applies to the code: what `Armada2.exe` is, as opposed to what it does, stays
   out of the repository (`publish/README.md`, "Engine findings").
   - **Allowed** in docs, comments and changelogs: function and symbol names from
     `armada2.map`, addresses and field offsets, a single instruction named in a
     sentence (`call 0x62bc70`), a config or `.spr` line quoted in passing, and in the
     C sources the short byte signatures a plugin checks before it patches.
   - **Never, at any path**: `armada2.map` or any excerpt of it longer than a name, a
     disassembly or decompilation listing (not even a few lines in a code block), a hex
     dump, a reconstructed struct or function body, or a game file quoted beyond a few
     lines. Say what the code does in your own words. `publish/check.sh` refuses
     `*.map`; the rest is on you.
   - **Every finding read out of the disassembly belongs to a plugin.** It is published
     because a plugin in this repository needs it to work with `Armada2.exe`: that is
     the purpose German law (UrhG §69e) allows disassembly for, and the only purpose
     it allows passing the results on for. So write it in the README of the layer whose
     code uses it, next to the hook it justifies, and name the hook. A dead end that
     shaped a hook is part of that justification and stays. A finding no plugin uses —
     engine trivia, a curiosity found on the way — goes in the scratchpad, not the
     repository. What was learned by **observing** the running game or measuring its
     files (the test bench, texture statistics, crash reproduction) is not limited in
     this way.

## a2mod

**`./a2mod stock` / `refit` / `status`** flips the *whole game* for
  before/after, layer by layer from `layers.toml`: textures, models, `Display8` (the
  game's `d3d8.dll`), `Render3D.asi`, `UI.asi`, `QOL.asi`, the rules layer
  (`QOLRules.asi`), Online, cutscenes. The ASI loader, the widescreen patch and
  the player's own options (`ARMADA.PRF`, `RTS_CFG.h`, `RefitPlayer.ini`) stay as they
  are in both states. A game with Display8 set aside runs on the `d3d8.dll` that was in
  the slot before (GOG's, from `d3d8.dll.a2display-backup`) or, with none, on the system's own. The **retired rows** (`hud`, `menus`, `grid`, `parts`, `menuscale`,
  `vkbasalt`, `renderer`) are never installed: `status` lists any file of theirs it
  finds, `stock` sets a stale copy aside, and `refit` restores an old snapshot byte for
  byte and then says which retired layers the snapshot holds (`retire/retire.sh` takes
  them out). It **snapshots** rather than
  reinstalls: modded files move to `$GAME/.a2mod/` and back, hash-checked, because some
  installed layers exist only on unmerged branches and a reinstall would not reproduce
  them. While in stock, anything installed over a stock file makes `refit` refuse.
  Do not delete `$GAME/.a2mod` while in stock.

## The texture pipeline

**`./a2tex`** — `list`, `build [-j N]`, `install`, `revert`, `diff`, `verify`, `stock` —
the texture pipeline. `./a2mod` switches the installed result between stock and
refit for before/after.

Two checkers beside them:

- **`textures/tools/selftest-mips.py`** pins mip-chain *name* resolution against the real texture
  directory, in both implementations — bash `mip_name`/`mip_strays` and python
  `mip_parent`/`stock_mip`. A disagreement between them is how a level gets written
  under a name nothing reads, which is a crash and not a blemish. Run it after touching
  either.
- **`textures/tools/measure-invention.sh <target>`** reports, per texture and per blend, how much
  the generative layer invents and how much it erases, at the on-screen size. Use it to
  set `blend=`, not your eye at 1:1 — and read its header before writing any comparison
  of your own. It **ranks** textures; it does not decide them: on organic or
  near-greyscale art its numbers go flat and say nothing, so there it has to be a render.

**Run `./a2tex verify` after every build, before installing, and after any change to how
a build works.** It checks every output against its stock file *from the raw TGA bytes*
— header, power-of-two size, `maxsize`, exact per-channel means, mip chain, and whether
the installed copy still matches. Raw bytes because ImageMagick's opinions about
associated alpha are what corrupted `Mmoon`, so it cannot be the thing that checks for
it. `Mmoon` shipped broken because it *was* verified, and then the build changed and
only the newly-added property was re-checked: **changing a build invalidates the whole
invariant set, not just the part you changed.**

A target is a **recipe** in `textures/targets/<NAME>/` — `target.conf` and
`stock.sha256`, both committed — and a **work directory** in
`$A2_DATA/textures/<NAME>/` holding `stock/`, `ai/`, `src/`, `out/`, plus `src-alpha/`
when it sets `alpha=ai`. **No pixel is ever committed, and no asset is ever written
into a checkout**: `A2_DATA` comes from `a2env.sh` (default
`~/.local/share/armada2-refit`), and every checkout and worktree shares it.
`./a2tex stock` fills `stock/` from the game install (the `.a2neb-backup`, else the
live file), accepting a file only if it matches `stock.sha256`, and every `a2tex`
subcommand that reads `stock/` does it first. A **new target** gets its stock copied
into its `stock/` by hand, then `./a2tex stock --manifest <T>`, which writes the
manifest into the recipe to be committed. `ai/` (and `MBG02/src/`, which predates
`ai/`) cannot be regenerated for free: never delete or overwrite them.
**To add work, drop images in `src/` and build.** Do not write new per-texture scripts;
add a target directory instead.

`-j` is safe because every build gets its own `mktemp -d` scratch. Keep it that way:
nothing in `textures/lib/` may write to a fixed path.

**Always write intermediates with the `PNG24:` prefix.** A greyscale intermediate is
stored as a 2-channel PNG and `%[fx:mean.g]` then reads exactly **0** — which makes the
channel-synthesis branch "repair" channels that were never broken, and makes a
reference's green/blue gain a silent no-op. Neither `-type TrueColor` nor
`-colorspace sRGB` fixes it; only the on-disk format does. `fit()` refuses a reference
with a zero green and blue channel for this reason.

Two bash traps that have each cost a debugging round here:

- `local a=$1 b="$a"` — the whole `local` expands before any assignment lands, so `b`
  gets the *old* `a`.
- `read -r w h < <(magick ... -format '%w %h' info:)` — `read` returns 1 at EOF without
  a trailing newline, and under `set -e` that **aborts the script silently**. It killed
  `upscale-stock.sh` between the paid upscale and the blend, leaving a full `ai/` and an
  empty `src/`. Put `\n` in the format.

## Generative upscaling, in six rules

**Everything installed by this project is derived from its own stock art**, upscaled.
None of the generated art shipped: `Mnebula4` was built both ways and the stock upscale
won. Reach for `textures/PROMPTS.md` only when stock is too small or damaged to carry
detail, and measure the upscale first.

1. **`enhance_details` / `enhance_realism` are not a quality dial.** Off makes invented
   detail *more* visible, not less — the output is sharper, so the hallucinations read
   clearly. On merely blurs them. Use a blend toward Lanczos instead:
   `magick lanczos.png ai.png -define compose:args=35 -compose blend -composite out.png`
2. **Measure invention, don't eyeball it.** `-fx "abs(r-g)"` against the stock file's
   own value catches invented chroma (MBG02: stock 1.89, Lanczos 1.97, AI 5.90).
3. **`MONOHUE=1` only on single-hue textures.** It rebuilds colour from luminance and so
   removes chroma invention by construction — but on a multi-hue texture it flattens
   real colour. Check hue spread first, with `textures/tools/huespread.py` — **not** with the
   plain `-colorspace HSL -channel H -separate` standard deviation, which is dominated
   by hue noise in the near-black areas that make up most of a skybox and scores
   single-hue plates like `MbgDom1` at 102°. `huespread.py` weights each pixel's hue by
   the chroma it actually carries; `MBG02`, the one accepted `MONOHUE=1`, scores 0.8°.
4. **How much a generative upscaler invents depends on the scale factor, steeply.**
   `MBG02` was 16x from a 128px quadrant and tripled its R-G deviation (1.89 → 5.90).
   The 6-face sets are 8x from 256px and the same model at the same settings invents
   *no* measurable chroma: `MbgBorg`'s R-G ratio against stock is 0.998–1.000 across all
   six faces. So `MONOHUE` is needed for the atlas and is **not** needed for the 6-face
   sets, where per-channel matching keeps the real hue variation instead of flattening
   it. Measure before reaching for it.
5. **Set the blend on a render at the object's REAL ON-SCREEN SIZE.** 35 is right for a
   skybox because a face fills the viewport — 1:1 and the game agree about what is
   visible. A ship does not: a Sovereign draws ~340px wide at ordinary zoom, its saucer
   takes 140 of 256 texels, and the invented hull glyphs that 35 exists to suppress land
   at well under a pixel. Judged at 1:1 that target looked careful; in game it was
   indistinguishable from stock, which is why hull work runs at **70**. Crop the region,
   resize it to the width the object actually occupies — across the whole zoom range,
   which spans ~13x — and compare *there*.
6. **Keep the AI layer.** `$A2_DATA/textures/<T>/` holds three layers: `stock/` (original),
   `ai/` (the raw upscale — what the credits bought), `src/` (`ai/` blended over
   Lanczos — what `a2tex build` reads). `textures/tools/upscale-stock.sh --reblend --blend N`
   re-derives `src/` offline and free. `MBG02` predates this: its intermediates were
   scratch and are gone, so re-tuning its blend means re-upscaling its four quadrants
   (~$0.16).

## The four things most easily got wrong

- **There are two nebula systems**, and they need opposite treatment. Map puffs
  (`Mnebula*`) are greyscale, engine-tinted, isolated on black, 4 per atlas. Skybox
  (`mbg*`) keeps its own colour, fills the frame edge to edge, 6 faces, order matters.
  A nebula that appears on the minimap is a map puff; the skybox never does.
- **`MBG02` breaks the skybox rules.** It is not one face — it is a 2x2 atlas of four
  128x128 tiles, so it needs four images, and `kind=sky-atlas` in its `target.conf` routes it
  through the atlas path. It also has no corner notches, unlike every other
  6-face set. Before assuming any skybox texture is a single picture, run the seam test
  at x=127|128 and y=127|128 and compare against a baseline column. Measure first.
- **The skybox is not in any config file**, and **`strings` will lie to you about it.**
  Each map's binary `.bzn` holds the background name in a fixed field after the marker
  `02 00 00 00 64 00 00 00`. It is either `<name>.sod` (a cube model) *or* a bare
  `<prefix>` resolving to `<prefix>0.tga`..`<prefix>5.tga` with no SOD at all — 38 of
  the 72 maps use the prefix form. `strings | grep '\.sod'` silently skips all of them
  and emits buffer-tail fragments (`s.sod`, `e.sod`) for some. **Read the field.**
  Recipe in `textures/README.md`.
- **Blending is additive, so black is transparent.** Any non-black pixel at a map-puff
  quadrant edge renders as a glowing square in space. Skybox faces are the opposite and
  must stay bright to their borders.

## Method

Measure, don't eyeball. Every quality decision here has a number behind it:

| Check | Target |
|---|---|
| Mean luminance | match the stock file it replaces (additive blend ⇒ mean ≈ light contributed) |
| Quadrant edge maxima | `0` for map puffs |
| Peak | below 255 — anything at 255 is clipping |
| Greyscale | R, G, B means identical — true for most puffs but **not** `Mnebula1` (28/26/22) or `Mnebula2` (33/9/8). Match the stock file, not the rule |
| Skybox hue | each of R, G, B matches the stock file's channel mean — `fit()` in `textures/lib/common.sh` does this |
| Face resolution | `size=` in `target.conf` = per-FACE pixels; one face fills the viewport, so this is what the eye sees. Sizing the atlas instead is why a 4x increase once still looked soft |
| Chroma invention | R-G deviation vs the stock file's (MBG02: 1.89). A generative upscale triples it; `MONOHUE=1` fixes it |
| Seamless-able? | only homogeneous sources. Five methods failed on composed ones — see `textures/README.md` before trying a sixth |
| Tile self-seam | L\|R and T\|B RMSE below stock's 0.023 / 0.058 — `make-seamless.sh` |
| TGA descriptor byte | **the same as the stock file's** — `bottomup.py --like`. ImageMagick always writes `0x20`; stock is a mix |
| Is it an atlas? | diff columns at x=127\|128 and y=127\|128 vs a baseline column |
| Corner notches | 24x24 corner mean — `0` on the 6-face sets, non-zero on `MBG02`. Upscaling a set's own stock preserves them by construction |

**ImageMagick is Q16 here, so `-threshold N` means N/65535, not N/255.** Writing
`-threshold 40` to mean "40/255" thresholds at 0.15/255 instead and reports nearly the
whole image as bright. Always use a percentage: `-threshold $((v*100/255))%`. This
produced a completely wrong brightness histogram for `MBG02` before it was caught.

`a2tex build` prints `q0:38->28(x0.746)` per quadrant: source mean, result, gain.
**Gain pinned at 2.5 means the input was too faint** and no amount of processing will
fix it — the generation prompt has to change. Gain near 1.0 is ideal. Dimming (< 1.0)
is safe and uncapped.

Two corrections that are baked into the scripts and should not be undone:

- **Linear stretch, never gamma, for density matching.** A gamma steep enough to lift
  mean 7 to 28 maps background 1/255 to 32/255 and fills the sky with fake stars.
- **Noise floor = mean of the darkest small corner, capped at 3%** — not the max over a
  large corner, which reads real gas once a cloud fills the frame.

## When changing prompts

`textures/PROMPTS.md` is the single source of truth and the artifact reads from it
directly — do not restate prompts elsewhere. Written for ChatGPT, which has **no
negative prompt**, so constraints are phrased positively and the whole thing is framed
as an isolated VFX plate; that framing is what keeps stars out.

What the build cannot fix, so the prompt must:

1. **Density** — a substantial body of mid-grey gas, not a bright core with faint wisps.
2. **Fill** — occupy most of the frame; a small subject in a black field wastes resolution.
3. **Softness** — soft-edged smoke. A very bright core just clips to flat white.

Generation: **the ChatGPT web UI beat every model reachable through `belt`**, including
`openai/gpt-image-2-5-flare` at high quality. If the user offers an image, prefer it —
and ask them to save it to disk, since pasted images do not persist anywhere readable.
For a tile the binding constraints are compositional, not aesthetic: fill the frame, no
focal point, no stars, fractal detail at many scales. See `textures/PROMPTS.md`.

If no image model is reachable, `textures/tools/gen-nebula.sh <seed> <out.png>` produces a usable
tile offline. It matches stock statistics exactly but **cannot produce coherent fibrous
filaments** — three alternative noise formulations were tried and were worse. The
comparison table is in `textures/README.md`; read it before re-tuning the weights.

Tiling repetition is *not* visible in game — the installed `Mnebula4` uses the same
image in all four quadrants and reads fine, because billboards overlap at many scales.
Don't insist on four distinct generations.

## Per-layer notes

What an agent must know before touching each layer — the decisions not to re-open and
the traps not to re-derive. **For what is installed, at which version, and whether it
has been seen in game, read that layer's `CHANGELOG.md`**; for the derivations, its
`README.md`.

### Retired: hud, menus, grid, renderer

`HUD.asi`, `GridLayout.asi`, `Menus.asi` (once `MenuScale.asi`) and the `renderer/`
layer (`dxvk.conf`) are deleted; `retire/` takes them out of an old install, and their
changelogs are in `retire/history/`. What they established lives beside the code that
still uses it, not in this file: the HUD's three references (the 1600x1200 layout canvas,
the hard-coded 800x600 cursor, the 1280x1024 font tier), the button bar's slot plan and
hotkey tables, and the HUD-side traps are in `ui/HUD.md`; the shell (the 800x600 mode it
asks for, `BitBlt` and no `StretchBlt`, embedded dialogs, `GetDC`/`ReleaseDC` not
pairing) in `ui/MENUS.md`; the window and display
mode in `display/README.md`; the backdrop plates (built by `ui/backdrops/backdrop.sh`, drawn by the UI's menus) in `ui/backdrops/BACKDROPS.md`.
Two rules from them still bind:

- **Do not change `cfgSCREEN_WIDTH` (`RTS_CFG.h`) to fix a canvas**, and never edit
  `RTS_CFG.h` at all (see qol): it is the fixed 1600 every rect is converted into.
- **`--method runs` for the font was built, measured, and rejected in game.** Metrics
  of ink and structure are not legibility: never change an appearance on the strength
  of a table; put it in the game and look.

### renderer, platform

- **The game renders through Display8, on Vulkan or Direct3D 12, with nothing behind
  it.** There is no DXVK and no d3d8to9 any more (platform 4.0.0): the game directory's
  `d3d8.dll` is Display8, `d3d8_real.dll` and `d3d9.dll` are retired files that
  `retire/` and `platform/d3d8-chain.py --retire` take out (by hash or marker, never by
  name or size), and the Wine overrides are `winmm=n,b;d3d8=n,b` (the ASI loader and
  Display8; `d3d9=n,b` went in 15.6.0). When neither backend starts, Display8 hands the
  game the system's own `d3d8.dll` and the plugins stand down. Our DLLs go in the
  **game directory**: the prefix is not durable (Proton restores its `d3d8.dll` and
  `d3d9.dll` from symlinks). The stock `syswow64/d3d8.dll` is *Wine's builtin*; never
  identify a DLL by its byte count (`platform/d3d8-chain.py --status` names each by
  hash). Heroic's `autoInstallDxvk` redeploying DXVK into the prefix is it working as
  designed, and matters only to the bail-out's look. The Wine virtual desktop stays
  **off**: it would put the menus in one window, but inside it `ChangeDisplaySettingsEx`
  fails and the game falls back to 640x480 (on our devices as on DXVK); never turn it on
  to fix a window problem, fix it in the UI's menu host (`platform/README.md`,
  "Hyprland / window management").
- **Bloom, tone mapping and every other post-process are render3d's.** The vkBasalt
  bloom (and its ReShade preset) was removed in 13.0.0; `retire/` takes out what an
  earlier version set up. Don't bring back a swapchain post-process: it would bloom the
  frame twice and sees the HUD too.
- **MSAA is render3d's**, on its own scene target, because nothing outside the plugin
  can turn it on for the engine's back buffer.
- **Anisotropy and LOD bias are Display8's `[Samplers]`** (they were `dxvk.conf` keys;
  `retire/` carries a marked `dxvk.conf`'s values over).

### qol

- **Two plugins, by what other players need.** `QOL.asi` holds only changes that stay
  on this player's machine (camera, keys, which ordinary command a key sends): a player
  with it can play a player without it. Anything that changes what every node simulates
  (QOL-7's pay-on-enqueue) goes in the rules layer (`rules/`, `QOLRules.asi`), which every
  player must run, checked when a network game is set up. Never put a rules change into
  `QOL.asi`. Which idea is which: `qol/README.md`.
- **The rules layer is `rules/`, with its own installer and changelog; `qol/install.sh`
  neither installs nor removes `QOLRules.*`.** A player without it reports no rules and
  the host plays stock for all (`rules/README.md`, "Must-match"); Online looks the plugin up
  by its export (`a2_find_export`), never by file name.
- **The selection lives in `QOL.asi`, not in `cOverViewImp`, when `MaxSelection` > 16.**
  Its 31 `this+0xbc` sites are re-pointed at the plugin's array; anything new that
  reads the selection must go through `GetSelectList` (vtable +0x98), never +0xbc. An
  order for more than 16 ships is still an ordinary `NetOrderObjects`, which stock
  peers accept (`qol/README.md`, QOL-3).
- **Stock code assumes 16 selected in places that never read the selection directly.**
  The special-weapon button gathered capable ships into 16-entry stack arrays and
  crashed 1.1.0 with 17 Galaxies selected. Auditing who *reads* the selection was not
  enough; test any cap change with big selections of ships that have special weapons,
  in combat.
- **Stations have their own ten groups** (`cOverViewImp+0x1c4`; ships' at +0x14c), and
  `cOverViewImp::Select` is the one gate that kept two stations from being selected.
  `StationGroups=` opens it for one class at a time; the button bar then needs the
  build button and each build/cancel routed to one station (QOL-6), because stock sends
  a build to *every* selected producer. Construction ships are ships and stay stock.
  `./a2test run qol-station-groups --install .` checks it.
- **The button bar's position keys (`BarKeys`) are QOL's; the UI only draws the labels.**
  Never patch the bar's vtable (`PopupPaletteImp`): QOL wraps the call of
  `mProcessKeyboardInput` inside its `Update` (checked stock first) and presses a button
  through the button's own press slot. Don't touch the floating palette's width (+0x7c): it is also the slot
  plan's width (`row * [+0x7c] + col`) and changing it made buttons vanish. Stock's bar
  keys are global per selection, not per open menu (F5 builds from the top level), so
  QOL empties the hotkey tables around the stock handler and points the six menu
  toggles at a zero; nothing outside the bar reads them (`qol/README.md`, QOL-1;
  `ui/HUD.md`, "The button bar").
- **Never edit `RTS_CFG.h` for a QOL change.** Network games compare a CRC of its bytes
  ("EXE / RTS_CFG.h files do not match node %d"), so an edited file locks the player
  out of games with stock players. Scale the value in memory after the parse, as
  `QOL.asi` does for `FASTSCROLL_COEFFICIENT` (right-drag only). `ARMADA.PRF` carries no
  such check.

### render3d

- **It replaced twelve plugins (MSAA, lighting, sky, nebulae, surfaces, explosions, shields, repair, venting, specials, fog, planets), and they are gone.** What
  they found is in `render3d/README.md` under each module; the old plugin files are
  only recognised (`retire/` takes them out of an old install). A module switched off
  (`[Modules] <name>=0`, or its Refit row) is live: its category goes back to *Display8's
  passthrough* and the engine's stock look is drawn on the same target, so it is
  tone-mapped and bloomed with the rest. A module stands down while a stale copy of its
  old plugin is loaded (`core/src/lib.rs`).
- **Modules are crates and hook through the core.** A module (`mods/<m>/`) depends on
  `r3d-core` alone and installs its engine hooks through the core's hooks registry,
  which also hands a hook back (the engine's own call runs) when the module is off or a
  symbol is unresolved; engine addresses, offsets and checks come from the symbol
  profile (`profiles/gog-1.1/render3d[.<module>].toml`), never from a literal in a
  source: a struct offset is a `field` entry read at run time (`field::X()`, the
  matched build's), and `lint-layers.sh` rule 1 fails on a literal one. A hook whose
  symbols are not all resolved stands down and logs which one.
  Modules talk through capabilities and services, not by name; module order is an
  explicit priority.
- **The seam is the two calls of `Armada_RenderAllOurStuff`.** If either site is not
  the stock call, render3d stands down and the game draws as stock. There is no
  Direct3D 9 fallback: with no Display8 device behind d3d8 (the bail-out to the
  system's d3d8, `Backend=none`) `gpu` is stood down (`Backend::None`), nothing of
  ours draws, and the engine draws everything.
- **The HUD's action camera draws the scene a second time each frame.** Any per-view
  state (the nebula slice ladder, the shadow map's list) is kept per view, by
  viewport. Bench scenes with `Hud=0` hide this.
- **Render only.** Nothing the simulation reads is written and nothing draws from
  `g_random`; the cosmetic simulation (particles, debris, animation) is render3d's own,
  on the sim clock. That keeps it stock-compatible in multiplayer.
- **Shader loops have a fixed count and no branch inside** (a rule from vkd3d, gone, kept
  because it costs nothing under dxc). Write shader loops branch-free (`step`/`lerp`);
  the volume march and the Voronoi searches do. Quads of different recipes go in one
  draw call, chosen per quad in the shader.
- **A special weapon is told by its ODF, never its C++ class.** `Phaser`, `Missile` and
  `Bullet` are shared with the ordinary weapons (the Borg ion beam is a `Phaser`); the
  ordnance class's `cPrjID` (+0x150) names the ODF, and a recipe `[Ordnance.<odf>]`
  makes it a special. Waves likewise by the explosion class's `cPrjID` (+0x24). Write
  a recipe from the `Survey` log, not from the screen: on the bench, lines that looked
  like special weapons were often the ships' own phasers.
- **The hull look has a Borg profile of its own.** Bump-mapped meshes (in stock the
  Borg alone) take the `Borg*` keys: dark plating, so the night lights carry the look.
  The Federation tuning made them too bright in game. Normals are the SOD's winding
  (inward, as the engine's: `scene::NORMAL_SIGN`); a **mirrored node swaps the cull**,
  and needs no flip of its own (the Steamrunner drew inside out without it, and a hull
  that goes dark whole under shadows is inside out, not acne).
- **A cloak is two things**: while it comes on or goes off, the vertex alpha of a fade;
  fully on, the engine's cloak material in place of every mesh's own. A fully
  cloaked hull's texture is that material's, so never read the hull's look from it.
  Faded hulls are drawn at `Step::Translucent`, far to near, not in the opaque pass.
- **Point lights are the engine's list.** Any new light source is an ordinary point
  light registered by a module's `lights()` after `GameObject_PreRenderAll` and
  `Ordnance::PreRenderAll`, before the view reads the list; size its reach by the
  ship's bounding radius (the shield's mean radius on a flat ship reached almost none
  of the hull). A soft point light is given mirrored through the draw's origin, since
  stock normals point inward. A nebula lights only while it counts as on screen, so
  render3d still marks every nebula it reaches.
- **Nebulae slices sit on a ladder fixed in distance from the eye**, per view, and the
  blue-noise thresholds stay put from frame to frame; moving either made the gas
  shimmer. The march's thresholds move only with `Temporal=`, which adds the frames up
  (reprojected, clipped to the neighbourhood); never move them without it. The noise is spectral, not value noise (value noise's grid drew walls). In
  the shadow map a texel of 1 means no hull, never a hull at the far end.
- **The skies follow stock by measurement**: since 2026-10-16 the user wants each
  recipe's shape and texture to match its stock sky, Aqua and `mbgrg` included (their
  old dimmer looks were dropped). Shape is fitted by `render3d/perf/sky-shape.py
  --fit`, level and colour balance by `render3d/perf/sky-grade.py`, both from bench
  bakes, not hand-tuned; soft billows, no filaments, no speckle (`Grain`/`Dust`/`Fine`
  stay off). The cube is magnified a few times at 4K, so finer detail comes from
  the draw, not a bigger cube. A recipe in `render3d/content/sky/` is built in; a
  `Sky\<name>.ini` in the game directory takes precedence.
- **Surfaces are looked up about a mesh's centre, scaled to its extents**, never its
  origin (an asteroid's origin can sit on its surface); the asteroids are triplanar.
  Fields draw `Master`, `Master1..5` and `Master7`, never 6 or 8. A hull mesh's texture
  is `**(mesh + 0x114)`, a list: reading `*(mesh + 0x114)` as the texture crashed.
- **The shields-down arcs must not turn**: the user asked for a readable effect that
  does not show as a rotating pattern. A shell type's name comes from the engine's name
  table, never `m_logical_db` (reading a name at +8 of one of its entries crashed).
- **Hardpoints sit a little inside the hull**: a weld's flare drawn at one is hidden
  unless pulled towards the camera. A weld is told by the call that made it, never
  by its sprite (hull-hit sparks share the `xspark` art). A vent particle is told
  by its emitter's sprite by name; copy a particle when it is handed over, never keep
  the pointer. The plasma cools steeply with age; judge a change parked *and* moving.
- **Don't make the fog textures bigger**: one texel per scan-grid cell, and resizing
  them froze the game (`UImid`). The shroud is alpha-tested in stock; render3d blends it
  so its edge can be soft, and holds its taps inside the grid's rectangle or the
  shroud thins along the map's border.
- **The planets are a sphere of render3d's own** (the world module): `Planet_Database`
  re-tessellates its own sphere against a tolerance set for 640x480, and a denser
  `PB_CLSS*.sod` drew pixel for pixel like stock, so don't re-try new planet meshes.
  The dilithium moons are ordinary SODs smoothed by `models/moon-sod.py`.

### display

- **Display8 is the game's `d3d8.dll`, and nothing stands behind it.** Its devices
  accept everything the engine does and draw nothing; render3d and the UI draw on
  Display8's own Vulkan or Direct3D 12 device, and Display8 draws, by passthrough, the
  categories no plugin claims (a module or UI part at 0, a plugin absent or stood down).
  There is no forward mode and no Direct3D 9. `d3d8.dll.a2display-backup` is what
  `a2mod stock` puts back, and with none it leaves the slot empty so the game runs on the
  system's own `d3d8.dll`. Every Direct3D 8 interface is an object of ours with every
  slot's exact `__stdcall` argument list (Wine's `d3d8.h`): one argument off unbalances
  the engine's stack.
- **The bail-out.** `Backend=` is an order: `auto` (Vulkan, then Direct3D 12),
  `vulkan`, `d3d12`, `none`, or a list; anything after the list is the bail-out. When
  nothing starts, Display8 loads the system's `d3d8.dll`, hands its interface back
  unwrapped, logs `bail-out: <why>; the game draws through <path>; the plugins stand
  down`, and answers `display8_backend` NONE. The plugins log their stand-down once.
  Retired values (`Mode=`, `Real=`, `Backend=d3d9`) are read past and logged
  (`display/README.md`, "Choosing a backend"). `FailBackend=` (bench only) forces the
  fallback.
- **The plugins chain slots of our device's vtable** (`Reset`, `Present`), so the
  vtables are patchable statics and Display8 never calls its own methods through them.
  The display mode, the top-level window and the pointer's clip are Display8's (since
  0.18.0, `[Window]` in `Display8.ini`: `Native`, `KeepClip`, `RaiseIntro`), but
  `MoveWindow` and `SetWindowPos` in the exe's imports stay the UI's, for the hosted
  menus: the tracer leaves them alone, and the UI's menu host stands down if it finds
  them hooked.
- **Callers are read off the stack**: the exact return address, then a scan for values
  just past a `call` in the exe. The scan finds stale addresses too: the `BeginScene`
  "caller" of the order ring was one. Confirm with `Display8.go` (a detail capture of the
  draws) or a `[Drop]` A/B before acting on a deep caller.
- **The engine is offered one display mode, the desktop's**, so the engine, the HUD and
  the mouse all agree with the swap chain; never add the engine's mode list back. What
  the plugins read of the engine's state (transforms, viewport, `Clear`, the targets)
  must reach whatever they draw on; everything else stays in Display8. A texture a
  plugin samples is a mirror (`stub/mod.rs`); the gamma ramp is the frame's last pass.
- **One device and one frame for both plugins**: Display8 owns them and hands them out
  through the plain C exports of `gpu/src/display.rs`, the contract both sides build
  against (version 1.5; a major mismatch makes a plugin stand down and log both
  versions; minors only append). Change both sides together. The plugins register
  their claims through it (`display8v1_register`), never by looking for a DLL by name.
- **Samplers** (`[Samplers]` in `Display8.ini`) are made as DXVK made them under the
  retired `dxvk.conf`, and the plugins' samplers take the anisotropy and LOD bias from
  Display8.
- Bench: `--install .` installs Display8 (tracing in a clone, `Trace=auto`);
  `display/bench-mode.sh` sets its keys from a `Setup:` (the one mode is `stub`;
  `Backend=none` is the bail-out); `display/stub-scenario.sh`
  makes any scenario's stub variant; `display-inventory`, `display-engine-draws`,
  `display-drop`, `display-stub`, `display-fallback` (Vulkan, Direct3D 12, bail-out).
- **`Backend=vulkan` on the bench runs on the user's GPU with the validation layer on**
  (`testbench/bench/session.py`, `vulkan_env`); a GPU hang once crashed the desktop, so
  keep 1280x720, keep the layer on (`--no-validate` only for timing runs) and stop
  sessions promptly. `--vulkan-device lavapipe` gives the CPU driver for validation; judge
  a lavapipe run against a GPU baseline (`p10-vk-*`). **`Backend=d3d12` runs on the GPU
  through vkd3d-proton with no safety net** (the bench limits above: one at a time). The
  references for the passthrough scenarios are frozen (`p10-vk-*`, `p10-d12-*`,
  `p10-fwd-*`, the last from the forward chain that no longer exists).
- **The native gates are CI's and the user's, not the bench's**: `display/d12probe/` and
  `display/vkprobe/` run in CI (WARP with the Direct3D 12 debug layer; lavapipe) and in
  the kit `tools/gates/` attaches as `native-gates-<commit>`. A release stays an untested
  release (an "Untested" status line; "Pre-release" before 15.10.0) until the user's
  Windows gate has passed.

### ui

- **Every frame of UI is `surface::frame(context)`**, called by one hook per context in
  `src/engine/`; a screen, panel or overlay is code in `app.rs` over `widgets`, never a
  hook of its own. Each hook stands down on its own when its site is not stock.
- **`alloc::format!` does not link** (the prebuilt `alloc` unwinds and wants
  `__CxxFrameHandler3`): use `crate::sfmt!` or `log!`, and never add a stub for the
  handler.
- **Input is read where the window's messages enter** (`WindowProc` and the two
  `PeekMessageA` drains after it), not by a subclass: the engine drains each burst
  itself, so a subclass misses clicks. Capture drops a down and exactly its up, and
  lapses 250 ms after the last UI frame.
- **The mission hook flushes the engine's batched 2D quads first**, or HUD sprites land
  on top of the UI. A screen of ours in the front end is modelled on `Show_Credits`, the
  one shell screen that draws through Direct3D; the device can be re-created at every
  switch between the shell's display and the game's.
- Findings stay in `ui/README.md` (or `HUD.md` / `MENUS.md`) beside the hook that uses
  them; addresses, offsets and checks come from the engine profile
  (`profiles/gog-1.1/ui.<part>.toml`), read in `src/engine/`, one file per area
  (`ui/PLAN.md`, "What this exposes of Armada2.exe").
- **Every panel and screen is drawn through `widgets/theme.rs`** (`STYLE.md`): never
  hard-code a colour. A click gives keyboard focus only to a widget that takes typing
  or arrow keys; a focused button once kept every key from the engine.
- **A HUD panel of ours never replaces the stock component's logic**: the stock panel
  keeps simulating, the panel switch (`engine/hud.rs`, in `RenderAll` and
  `CursorInterface::RefreshOwner`, never a vtable) skips its drawing and mouse, and ours
  reads its state and presses its own functions (`HUD.md`).
- **A menu of ours never writes a setting**: the stock dialog runs hidden and a screen
  spec acts only through its controls (`MENUS.md`, "Writing a screen spec"); specs are
  keyed by dialog procedure, not template (one template is five screens).

### textures

- **Settled — do not re-open without a specific reason:**
  - The skybox class. `Mbgstars` (and `Stars`) stay stock: sparse starfields lose their
    bright pixels when upscaled.
  - `MBG02` is candidate D: stock art, AI-upscaled, blend 35, `monohue=1`, face 2048 in
    a 4096 atlas; `./a2tex build MBG02` reproduces it byte-for-byte from the four tiles
    in `$A2_DATA/textures/MBG02/src/`. The rejected candidates are in
    `$A2_DATA/archive/mbg02-candidates/`, and the generated sources (B, C) lost to
    stock's own composition — read `textures/README.md` before re-litigating.
  - `Mnebula4`'s generated art is kept in `$A2_DATA/textures/Mnebula4/src-generated/`
    and `$A2_DATA/archive/mnebula4-generated/`; `source=gen` returns to it.
- **Dilithium is not a nebula.** It is a moon: `mdmoon.tga` plus the glow `Mdmoonglo4`
  and the "Dmoon nimbus pulse" animation in `Sprites/animation.spr`. The nebula-looking
  resource cloud people mean is usually `Mlatinum`.
- **Planets.** `kind=plain` is an alias for `sky-faces` (one file, no alpha, no chain).
  `MQonos` is at `blend=15` because its AI layer was free-floating hairlines. The class
  planets (`PB_CLSS*`, `PA_*`) are a **four-lobe gore unwrap, not equirectangular**:
  what matters there is no bleed across the gores, not horizontal wrap.
  `blackedge=N` exists for `Mmoon` alone (additive sun sprites draw a glowing square on
  any stray 1–3/255 at a quadrant boundary).
- **`UImid` is `install=no` and must stay that way.** It carries the fog-of-war and
  minimap textures (`Gfog`, `Gshroud`, `Gminimap`, `Gmabelt`, `Gmneb1-5`), and doubling
  them **froze the game** as the HUD appeared. "Safe to upscale as an image" and "safe
  to change the size of" are different questions.
- **UI icons go through contact sheets** (`sheet=GxP`) for *quality*, not cost: the app
  takes `megapixels` as an integer, so 1MP from a 64px icon is a 16x lift that restyles
  it, while a 512px sheet at 4MP is a 4x lift. Panels are deliberately **not** sheeted —
  they are 9-slice pieces whose edges abut on screen. `colors`, `logos`, `gminicon`,
  `gminisys` and `MBuild` stay stock; reasons in `textures/README.md`.
- **Hulls: 1024, not 2048.** Both cost $0.005; the decision stands on disk (one ship's
  three files are 16 MB at 1024, 64 MB at 2048) and on invention rising with the lift.
  The size of `Textures/RGB` is an **on-disk** figure, not a memory one —
  `textures/README.md`, "2.27 GB resident was never a memory figure".
- **`textures/tools/fix-enterprise-registry.py`** re-cuts the two `C` apertures the model
  closed in `NCC-1701-E`. A deliberate one-off. **Re-run it after any `--reblend` of
  that target, before `build`** — `src/` is derived and a re-blend discards the fix.

### models

**A SOD asks for a bump map** with a material spelled the Borg way (type 6, a second
texture with word `0x200`). `hull-bump.py` gave the Federation hulls a flat one for
per-pixel light and was removed in models 4.0.0, since render3d lights every hull
per pixel; a height map derived from the hull art was rejected in game
(`models/README.md`, "Bump maps on Federation hulls").

**The loading screen is the UI's**: the widened `SOD/logo.SOD` and the `LOADING` target
are gone (`retire/` puts an old install's stock back from `.a2neb-backup`).

**The planets' shape is not in their SOD**: `Planet_Database` builds its own sphere, and the
planets are render3d's own (see its section). Don't re-try new planet meshes.

**The dilithium moons are ordinary SODs**, smoothed by `models/moon-sod.py` (PN patches,
lumpy shape kept). **Leave their glow shell stock**: it is blended, so the engine
transforms and sorts it on the CPU every frame, and smoothing it cost visible frame rate.

### online

- **Multiplayer does not work under Proton at all**, LAN included: Wine's builtin
  `dpnet.dll` stubs `IDirectPlay8Peer::Host` and `EnumHosts`. Microsoft's DirectPlay
  works. It is **not redistributable**: `online/reference-dplay.sh` puts it into a bench
  **clone** only, from `$A2_DATA/reference/directx/`. Never into the real prefix, never
  into the repo.
- The plan, the decisions (our own protocol, public + self-hostable server, a new
  *Internet – Online* menu entry, join codes first) and the traced DirectPlay surface
  are in `online/README.md`. Read it before designing anything here.
- **No code from DirectPlay Lite**: it is GPL-2.0 and this repo is MIT.
- ***Internet – Online* takes the IPX button's place** (IPX cannot work anywhere this
  runs); don't add a fifth button. Its hooks and why are in `online/README.md`, "The
  menu entry". `./a2test run multiplayer-online-entry multiplayer-two-players` checks the
  entry and the stock Manual IP path, two games each.
- **`peer.c` is our DirectPlay**, used only when *Internet – Online* is chosen; it must
  keep delivering the messages the trace shows, in that order. Two things the game
  depends on that are easy to break: `Close` completes a pending search with
  `ASYNC_OP_COMPLETE (USERCANCEL)` (without it quitting crashes), and `GetSPCaps` keeps
  reporting DirectPlay's 1.5 s enum interval (the game ages its game list by it). Every
  vtable slot needs its exact argument count (`__stdcall`). After any change run
  `./a2test run multiplayer-online-match` and `multiplayer-online-loss` (10% loss).
- **The server's peers are addresses too.** A relayed peer is `0.x.y.z` port 1 (its
  server id), so the game and the rest of `peer.c` treat it as any address; `raw_send`
  is the one place that wraps it for the server. Server changes: run
  `online/server/selftest.py`, then `./a2test run multiplayer-online-code
  multiplayer-online-relay` (direct path; relay with 5% loss).
- **Messages to the player go in the game's chat boxes** (`notice()` in `online.c`),
  never in a window of our own; a line waits for its screen's `Chat::Init`, which empties
  the box.

### cutscenes

`cutscenes/binkproxy/` is a replacement `binkw32.dll` that plays
`animations/<Name>.mp4` + `.wav` in place of `<Name>.bik` and raises
`PlayIntroMovie`'s hard-coded 640x480 mode, which a bigger `.bik` could never escape.
**H.264, VP9 and AAC do not decode under Proton outside Steam; AV1 does.** A movie's recipe is
`cutscenes/movies/<Name>/movie.conf`, and its layers are in `$A2_DATA/movies/<Name>/`,
laid out like a texture target: `ai/` is paid and never overwritten, and **`ai-movie.sh` is the only script that spends**. Stock `.bik` files are
only ever read. **With no `.mp4` beside a `.bik`, every call goes to the real DLL**
(`binkw32_orig.dll`), so the proxy is safe to install with no movie built; it still
scales the launch reels to full screen. The DLL is built like the ASI plugins
("Environment"); `a2mod` switches it as the `cutscenes` layer.
