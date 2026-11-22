# Armada II Refit

**Make Star Trek: Armada II (2001) enjoyable on a modern system, without turning it
into a different game.**

Armada II still plays well, but it was built for a 4:3 CRT at 1024x768. At a modern
resolution the HUD stretches, the menus sit in an 800x600 box, edges shimmer without
anti-aliasing, and the art falls apart at 4K. This project fixes these problems one at
a time, and every fix can be undone.

## Goals

- **Enjoyable on modern systems.** Widescreen and ultrawide without stretching, the
  menus filling the screen, anti-aliasing, and a working Vulkan renderer under Proton.
- **Faithful to the original.** The game can look better, but it should still feel
  like Armada II. The same atmosphere and style, and nothing that pulls you out of it.
  The mod is modular. The core fixes (HUD, menus, anti-aliasing) replace no art and
  patch no files on disk. New textures, backdrops and movies are separate, optional
  layers. `./a2mod stock` puts the whole game back the way it shipped, for a
  before/after comparison.
- **Share the tooling.** Most of the work went into tools for reading the engine,
  measuring textures and testing the game headless, and they're here too. So are the
  notes on what the engine actually does, with the dead ends included. They should be
  useful to anyone modding this game or others like it.

## Where to read next

- [`PLAYING.md`](PLAYING.md): for players: installing, the Refit screen, `RefitPlayer.ini`, multiplayer, the graphics backend, logs and troubleshooting, uninstalling.
- [`MODDING.md`](MODDING.md): for modders: the plugin contracts, symbol and content profiles, building plugins, and what may be published.
- [`BUILDING.md`](BUILDING.md): building the plugins and the release zip from source, step by step, on Windows (WSL2) or Linux.

## Features

| | |
|---|---|
| **Stretch-free HUD** | `UI.asi` draws the in-game HUD, its font and the cursors correctly at any aspect ratio (16:9, 21:9, 32:9), from the display mode the game actually sets ([`ui/HUD.md`](ui/HUD.md)) |
| **Full-screen menus** | `UI.asi` hosts the shell menus and draws them at any size inside the game window, with optional widescreen backdrops (black sides without them; [`ui/MENUS.md`](ui/MENUS.md)) |
| **Our own 3D renderer** | `Render3D.asi` takes over the 3D view and draws it into a half-float, multisampled target, bloomed by energy and tone-mapped into the back buffer, with modules of its own for every category: ships and stations lit per pixel by a warm key, a dim blue fill and a faint sky light, with shadows and light from nebulae, planets, explosions and weapons; each map's sky computed in a shader from a recipe, with no cube seams; planets and the dilithium moon made from recipes (relief, craters, oceans, ice, clouds, a night side); every nebula as a cloud of gas you can fly through; fireballs and shockwaves, shield hits, repair welding, venting plasma, the warp flash and the special weapons drawn by shaders; and the fog of war and the shroud with smooth edges. It replaces the earlier per-part plugins (MSAA, lighting, sky, nebulae, surfaces, explosions, shields, repair, venting, specials, fog, planets). It draws through Display8 (below), on Vulkan or Direct3D 12 ([`render3d/README.md`](render3d/README.md)) |
| **Our own Direct3D 8** | `Display8` is the game's `d3d8.dll`, ours: it draws on Vulkan or Direct3D 12, whichever starts first, with nothing behind it (no d3d8to9, no DXVK), and hands the game the system's `d3d8.dll` when neither starts. Anisotropic filtering and LOD bias are `Display8.ini`'s `[Samplers]` ([`display/README.md`](display/README.md)) |
| **Quality of life, stock-compatible** | `QOL.asi`: faster right-drag panning, `Shift+number` adds to a control group, selections and groups beyond 16 ships (40 by default), stations in control groups with one build menu for several, and long moves on the map going to warp as minimap moves do. Everything stays on your machine, so you can still play against unmodded players |
| **Grid hotkeys** | `UI.asi`'s button bar lays the bar out as a 5x3 grid with one key per cell by keyboard position (`QWERT`/`ASDFG`/`ZXCVB`), labelled on the buttons and placed between the minimap and the info panel on wide screens |
| **Smoother geometry** | the dilithium moons and the asteroids are smoothed, and the selection bubble is round instead of a visible polygon (SOD rewrites from your own stock files, `models/install.sh`) |
| **Texture replacement** | `./a2tex`, a pipeline for replacement textures, whether upscaled, generated or drawn by hand. It checks each one against the engine's format rules (bit depth, mip chains, size limits), installs it with a backup, and packs a set so others can install it. There are 84 recipes so far: skyboxes, nebulae, planets, UI and ship hulls |
| **Video replacement** | a replacement `binkw32.dll` that plays the launch reels full screen and plays AV1 replacements in place of the original Bink movies, plus a pipeline to build them |
| **Widened loading screen** | the 3D loading-screen model, rebuilt for widescreen |
| **Online multiplayer** *(in progress)* | multiplayer under Proton is broken (Wine stubs DirectPlay). `Online.asi` adds *Internet – Online*, which runs on its own UDP transport with join codes, hole punching and a relay through a self-hostable server. In the release zip and in `./install` ([`online/README.md`](online/README.md)) |
| **One switch** | `./a2mod stock` / `refit` flips every layer at once for before/after comparisons |
| **Headless test bench** | `./a2test` runs the game on a copy of the install on a virtual display at any resolution, and takes screenshots and runs regression scenarios |

## No game content in this repository

This repository holds **code, configuration, recipes and documentation only**. It
contains no textures, models, video or data from the game, whether original or derived
([`publish/README.md`](publish/README.md)). The asset pipelines start from **your own
copy** of the game: they extract the stock art as the reference each replacement is
checked against, then build and install. Builds live in `A2_DATA`, outside the
repository.

Every layer works without assets. With nothing built, the asset layers install nothing
and the game keeps its own art, while the HUD, menus, the renderer, quality of life
and grid (`UI.asi`, `Render3D.asi`, `QOL.asi`) still apply.
[`textures/PACKS.md`](textures/PACKS.md) is the short path to building your own
texture pack.

## Requirements

**The game:** Star Trek: Armada II, **GOG release** (patch 1.1), with
**[`STA2WidescreenPatch`](https://github.com/Ligushka/STA2WidescreenPatch) v1.0** by
Ligushka, which unlocks widescreen resolutions and ships the
[Ultimate ASI Loader](https://github.com/ThirteenAG/Ultimate-ASI-Loader) (`winmm.dll`)
by ThirteenAG that loads this project's `.asi` plugins.

Both are MIT, so they are bundled: the release zip's installers install them when they
are missing, and its `CREDITS.txt` says where they come from.

**The platform:** developed and tested on Linux (Arch, Hyprland), with the game run
through [Heroic](https://heroicgameslauncher.com/) and Proton:

- A Vulkan or Direct3D 12 driver. Display8, the game's `d3d8.dll` of ours, draws on
  one of them itself: no DXVK, no d3d8to9, no Direct3D 9 (under Wine and Proton,
  Vulkan). The installers put it in; `platform/d3d8-chain.py --retire` takes an earlier
  version's chain out.
- For building: `clang`, `lld-link` and `llvm-dlltool` for the plugins (no MSVC or
  mingw needed), ImageMagick 7, Python 3 and `ffmpeg`.
- Optional, for generative upscaling:
  [`belt`](https://inference.sh), the inference.sh CLI. Upscaling costs credits, so
  everything else runs without it.

[`platform/README.md`](platform/README.md) covers the full setup: Heroic's DLL
overrides, the d3d8 slot, setting the resolution, and window management under Hyprland.

## Quick start

**Just playing:** download the installer for your system from the newest
[release](https://github.com/cedrichaase/armada2-refit/releases) and run it. It finds the
game, shows what it will install, installs the newest release and sets the launch
options the game needs ([`gui-installer/README.md`](gui-installer/README.md)).

- **Windows**, the GOG game as GOG installs it: `armada2-refit-installer-windows.exe`,
  one portable file. It is not signed, so SmartScreen may ask first: *More info* →
  *Run anyway*.
- **macOS**: `armada2-refit-installer-macos.zip`, an app for Apple Silicon and Intel.
  The game runs in Wine. Download GOG's Windows offline installer for the game from
  your GOG library into Downloads, open this app and press **Set up Armada II**: it
  makes a free [Sikarugir](https://github.com/Sikarugir-App/Sikarugir) wrapper in
  `~/Applications/Sikarugir`, installs the game into it and then the refit
  ([`gui-installer/README.md`](gui-installer/README.md), "Setting the game up on a
  Mac"). A game already in a **CrossOver** bottle or another wrapper is found as it is.
  The app is not notarised, so the first launch is refused: System Settings → Privacy
  & Security → *Open Anyway*.
- **Linux**: `armada2-refit-installer-x86_64.AppImage`; make it executable and run it.
  It carries GTK 4 and libadwaita itself, and writes Heroic's launch options for you.

If the installer says an antivirus took or holds a file, see
[Antivirus false positives](#antivirus-false-positives) below.

**From the repository:**

    ./a2env.sh                    # check where it thinks the game, prefix and Proton are
    ./install                     # build and install every layer
    ./a2mod status                # what is installed, layer by layer
    ./a2mod stock                 # back to the game as it shipped
    ./a2mod refit                 # and forward again

Paths are taken from the environment, then `~/.config/armada2-refit.conf`
(`KEY=value` lines), then the defaults:

| | Default |
|---|---|
| `A2_GAME` | Heroic's `~/Games/Heroic/Star Trek Armada II` |
| `A2_PREFIX`, `A2_PROTON` | Heroic's prefix and Proton for it |
| `A2_DATA` | `~/.local/share/armada2-refit` (`%LOCALAPPDATA%` on Windows): extracted stock art, paid AI layers, builds. **Back up its `ai/` folders**; everything else can be rebuilt with a command |
| `A2_PROFILE` | `armada2`: which game, by its descriptor in [`games/`](games/README.md); `--game <id>` on `./install`, `./a2mod`, `./a2test` and `tools/layers.py` does the same for one run. The defaults above are the chosen game's |

## Antivirus false positives

Some antivirus programs (Bitdefender among them) flag the mod's plugins as malware and
quarantine them, or hold them open while they scan, so the installer cannot read or copy
them. It is a false positive: the plugins are DLLs that are **not code-signed** and that
**hook into another program's process**, which is what heuristic scanners look for. They
are built from this repository's source by GitHub's CI, and each carries a version
resource naming the project. The installer says which file it could not read or write
and that an antivirus is the likely cause, and **Copy report** in its error window puts
together what helps with an issue.

**Which files are ours:** everything in the release zip's `game/` folder that ends in
`.asi`, and `binkw32.dll` (the cutscene player), which is today `Render3D.asi`,
`UI.asi`, `QOL.asi`, `QOLRules.asi`, `Online.asi` (and, from earlier versions, `HUD.asi`, `Menus.asi`,
`GridLayout.asi`, which the installers take out); the zip's `display/d3d8.dll` (Display8, the game's Direct3D 8 of
ours); and the installer itself
(`armada2-refit-installer-windows.exe`).
The others are third-party and unmodified: `winmm.dll` (the Ultimate ASI Loader),
`STA2WidescreenPatch.asi`, credited in the zip's `CREDITS.txt`. (Earlier zips also carried
crosire's d3d8to9 and DXVK's `d3d9.dll`; the installers take those out of a game.) The zip's `SHA256SUMS` holds every file's hash. Each release's notes carry a VirusTotal
scan link per file, so you can see what the scanners make of each one before you trust
it.

**What to do:**

1. Open the antivirus's quarantine (in Bitdefender under *Protection*; in Windows
   Security under *Virus & threat protection → Protection history*) and restore the
   mod's files.
2. Add exclusions for the installer's folder, `%LOCALAPPDATA%\armada2-refit\` (on Linux
   `~/.cache/armada2-refit/`, on macOS `~/Library/Caches/armada2-refit/`), and for the
   game folder (for GOG, for example `C:\Program Files (x86)\GOG Galaxy\Games\Star Trek
   Armada II\`).
3. Run the installer again. If it still cannot clear its earlier unpack, restart the
   computer first (or delete `%LOCALAPPDATA%\armada2-refit\installer\cache\unpacked\`).

Reporting the false positive to the antivirus's vendor helps everyone who installs after
you; most vendors have a form for it.

## Installing by hand, layer by layer

`./install` does all of this for you on Linux with Heroic. These notes cover each code
layer on its own. Textures, backdrops, movies and the loading screen are left out
because they need assets built from your own copy of the game (`textures/PACKS.md`).
The layers are grouped from "copy a file, works anywhere" to "needs a specific stack".

**Building.** The plugins are 32-bit Windows DLLs, built with `clang` + `lld-link` +
`llvm-dlltool` by each folder's `build.sh`, which writes to `<folder>/build/`.
**Prebuilt**, the zip on each
[release](https://github.com/cedrichaase/armada2-refit/releases) has `UI.asi`,
`Render3D.asi`, `QOL.asi`, `Online.asi` and the Bink proxy with their `.ini` files, and Display8 with `Display8.ini`. It also has installers that do this
section for you: `install.sh` on Linux and macOS, `install.bat` on Windows (`README.txt`
in the zip), and the graphical installers for all three, which leave the Bink proxy out
for now. `publish/package.sh` builds the same zip locally ([`BUILDING.md`](BUILDING.md), step by step).

**Every plugin needs** the game's `Armada2.exe` from GOG patch 1.1,
plus the Ultimate ASI Loader (`winmm.dll`) that `STA2WidescreenPatch` puts in the game
directory. The plugins patch the exe in memory only, after checking byte signatures. If
you have a different exe, they do nothing and say so in their `.log`. Under Wine or
Proton the loader only runs with the `winmm=n,b` DLL override
([`platform/README.md`](platform/README.md#heroic-configuration)). Without it, nothing
loads and nothing reports an error. On Windows no override is needed.

### 1. Copy into the game directory. Any renderer, Windows or Wine/Proton

| Layer | Copy | Notes |
|---|---|---|
| Quality of life | `qol/build/QOL.asi`, `qol/QOL.ini` | Right-drag pan speed (`PanSpeed=`, default 2.5x), `Shift+number` adds to a group (`ShiftAddsToGroup=`), selections beyond 16 (`MaxSelection=`, default 40), stations in control groups (`StationGroups=`), long map moves at warp (`WarpDistance=`, default 1100). Leave `RTS_CFG.h` stock: network games compare it, and the plugin scales the value in memory ([`qol/README.md`](qol/README.md)) |
| Cutscenes (launch reels) | `cutscenes/binkproxy/build/binkw32.dll`, `BinkProxy.ini` | First rename the stock `binkw32.dll` to `binkw32_orig.dll`, because the proxy forwards to it. With no `.mp4` beside a `.bik` it only scales the launch reels to full screen. Replacement movies are AV1 through Media Foundation: under Proton that works (GStreamer + dav1d), and on Windows it presumably needs the AV1 Video Extension |

Uninstall by deleting the files you copied (plus the `.log` each one writes), and for
the Bink proxy by renaming `binkw32_orig.dll` back.

### 2. Display8, the game's Direct3D 8

`Render3D.asi` and `UI.asi` draw through Display8: `display/build/d3d8.dll` and
`display/Display8.ini` go in the game directory as `d3d8.dll` (GOG's, if any, kept as
`d3d8.dll.gog-backup`). It draws on Vulkan or Direct3D 12 itself, so there is no DXVK, no
d3d8to9 and no `d3d9.dll` to set up, and nothing to configure for it beyond Wine's
`winmm=n,b;d3d8=n,b` overrides ([`display/README.md`](display/README.md)). A leftover chain
from an earlier version is taken out by `platform/d3d8-chain.py --retire` and by the
release installers (`retire/`).

| Layer | Install | Notes |
|---|---|---|
| Display8 | `display/build/d3d8.dll`, `display/Display8.ini` | Backend, VSync and the sampler settings (`[Samplers]`: anisotropy, LOD bias) |
| 3D renderer | `render3d/build/Render3D.asi`, `render3d/Render3D.ini`, its recipes (`render3d/content/sky/` built in; `render3d/content/surfaces/` and `render3d/content/nebulae/` into `Surfaces` and `Nebulae` folders beside it) | Needs Display8. Replaces the earlier `MSAA.asi`, `Lighting.asi`, `Sky.asi` and the other per-part plugins; `render3d/install.sh` takes any of them out ([`render3d/README.md`](render3d/README.md)) |
| UI | `ui/build/UI.asi`, `ui/UI.ini` | The HUD, the menus and the button bar; needs Display8 ([`ui/README.md`](ui/README.md)) |

Bloom is `Render3D.asi`'s. A vkBasalt or ReShade bloom set up by an earlier version
(before 13.0.0) is taken out by `./install` and by the release installers,
since both would bloom the frame twice.

The helper scripts (`./install`, `./a2mod`, the per-layer `install.sh`) are bash and
assume Linux. They find the game through `a2env.sh`, which defaults to Heroic's paths.
Any other layout works if you set `A2_GAME` / `A2_PREFIX`.

## The layers

The mod is a stack of independent layers, each in its own folder with its own README
(how it works and why) and changelog.

| Folder | Layer | What it changes |
|---|---|---|
| [`retire/`](retire/README.md) | (root) | `retire.sh`: takes out of a game what earlier versions installed and nothing installs now (`HUD.asi`, `GridLayout.asi`, `Menus.asi`, the per-part plugins, the file-based HUD fixes, the vkBasalt bloom, the Direct3D 9 chain and `dxvk.conf`) |
| [`display/`](display/README.md) | display | `Display8`, the game's `d3d8.dll`: Vulkan or Direct3D 12 behind the engine's Direct3D 8, or the system's `d3d8.dll` when neither starts |
| [`ui/`](ui/README.md) | ui | `UI.asi`: the HUD, the menus, the button bar and the Refit screen; `RefitPlayer.ini` is written only by it |
| [`rules/`](rules/README.md) | rules | `QOLRules.asi`: gameplay rules that every player of a network game must run |
| [`qol/`](qol/README.md) | qol | `QOL.asi`: gameplay quality of life that stays compatible with stock players (right-drag pan speed, `Shift+number` adds to a group, selections and groups beyond 16, stations in groups with one build menu, long moves at warp), and the plan for the rest |
| [`render3d/`](render3d/README.md) | render3d | `Render3D.asi`: the 3D view through a renderer of our own, in Rust. Half-float multisampled target with bloom by energy, and modules for hulls (scene lights, shadows), sky, particles, ordnance, effects (explosions, shields, repair, venting, warp), planets, moons, asteroids and nebulae, and fog and selection. Replaced the per-part plugins |
| [`textures/`](textures/README.md) | textures | the texture pipeline, `./a2tex`: 84 targets (skyboxes, nebulae, planets, UI, hulls), each a recipe for one set of replacement textures |
| [`models/`](models/README.md) | models | the smoothed dilithium moons and asteroids, the round selection bubble, and the loading-screen model, widened with its `LOADING` art. The planets' sphere is render3d's |
| [`cutscenes/`](cutscenes/binkproxy/README.md) | cutscenes | `binkproxy`, a `binkw32.dll` that plays launch reels full screen and AV1 replacements in place of `.bik` movies, and the movie pipeline |

`a2mod` does not switch these, because the layers above depend on them or they install
nothing:

| Folder | What it is |
|---|---|
| [`platform/`](platform/README.md) | Heroic and Proton, the d3d8 slot, the ASI loader, the widescreen patch |
| [`testbench/`](testbench/README.md) | `./a2test`: the game headless at any resolution, scenarios, reports |
| [`publish/`](publish/README.md) | what this repository may contain, and the check that enforces it |
| [`online/`](online/README.md) | online multiplayer without port forwarding, in progress: `Online.asi` adds *Internet – Online*, which runs the game on its own UDP transport, with join codes, hole punching and a relay through a self-hostable server (`online/server/`; the public one is `c20e.de`) |

## Entry points

    ./install                               install every layer
    ./a2mod status | stock | refit          switch every layer at once
    ./a2tex extract | build | install | pack | ...   the texture pipeline (textures/PACKS.md)
    ./a2test ...                            the game headless, for testing (testbench/)
    ./a2env.sh                              print where the game, prefix, Proton and assets are

Each layer installs and removes itself with its own script: `install.sh` in `ui/`,
`qol/`, `rules/`, `online/`, `models/`, `render3d/` and `cutscenes/binkproxy/`, `a2tex install`/`revert` for textures, or
a `--revert` flag on the Python tools. The layer's README has the details.

## Versions

Each layer folder has a `CHANGELOG.md` with its own semver version. The root
[`CHANGELOG.md`](CHANGELOG.md) versions the mod as a whole and lists the layer
versions it bundles. Changes are recorded as changesets
([`.changeset/README.md`](.changeset/README.md)), and a release, made by hand, turns
them into versions and changelog entries
([`publish/README.md`](publish/README.md#releasing)). Between releases, the GUI
installer's *Pre-release (latest main)* entry installs the newest `main`, untested.

`CLAUDE.md` is the working brief for Claude Code sessions in this repository.

## Licence and trademarks

The code, recipes and docs in this repository are MIT-licensed ([`LICENSE`](LICENSE)).
That covers nothing of the game's: Star Trek: Armada II, its art and its files belong
to their owners, and this repository holds none of them.

**This is an unofficial fan project.** It is not affiliated with, endorsed by or
supported by Activision, CBS Studios or Paramount. Star Trek: Armada II is © Activision;
Star Trek and related marks are trademarks of CBS Studios / Paramount. The names appear
here only to say which game this works with. The project uses none of their logos or
artwork, and nothing it produces should be presented as official.

The engine notes in the layer READMEs describe what `Armada2.exe` does, in our own
words, so that the plugins here can work with it. They quote no code from it.
