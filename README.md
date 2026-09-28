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

## Features

| | |
|---|---|
| **Stretch-free HUD** | `HUD.asi` lays out the in-game HUD, font and cursors correctly at any aspect ratio (16:9, 21:9, 32:9), from the display mode the game actually sets |
| **Full-screen menus** | `Menus.asi` scales the 800x600 shell menus to fill the screen and draws them inside the game window, with optional widescreen backdrops (black sides without them) |
| **Anti-aliasing** | `MSAA.asi` turns on up to 8x multisample anti-aliasing, which the game has no option for |
| **Bloom and renderer tuning** | vkBasalt bloom, plus anisotropic filtering and LOD bias through `dxvk.conf` |
| **Texture replacement** | `./a2tex`, a pipeline for replacement textures, whether upscaled, generated or drawn by hand. It checks each one against the engine's format rules (bit depth, mip chains, size limits), installs it with a backup, and packs a set so others can install it. There are 84 recipes so far: skyboxes, nebulae, planets, UI and ship hulls |
| **Video replacement** | a replacement `binkw32.dll` that plays the launch reels full screen and plays AV1 replacements in place of the original Bink movies, plus a pipeline to build them |
| **Widened loading screen** | the 3D loading-screen model, rebuilt for widescreen |
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
and the game keeps its own art, while the HUD, menus, MSAA and bloom still apply.
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

- [DXVK](https://github.com/doitsujin/dxvk)'s `d3d8.dll` and `d3d9.dll` in the game
  directory, so the game renders through Vulkan. MSAA, bloom and the renderer settings
  depend on it. `platform/d3d8-chain.py --use dxvk` sets it up.
- [vkBasalt](https://github.com/DadSchoorse/vkBasalt) for bloom. It is Linux-only.
- For building: `clang`, `lld-link` and `llvm-dlltool` for the plugins (no MSVC or
  mingw needed), ImageMagick 7, Python 3 and `ffmpeg`.
- Optional, for generative upscaling:
  [`belt`](https://inference.sh), the inference.sh CLI. Upscaling costs credits, so
  everything else runs without it.

[`platform/README.md`](platform/README.md) covers the full setup: Heroic's DLL
overrides, the d3d8 chain, setting the resolution, and window management under Hyprland.

## Quick start

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

## Installing by hand, layer by layer

`./install` does all of this for you on Linux with Heroic. These notes cover each code
layer on its own. Textures, backdrops, movies and the loading screen are left out
because they need assets built from your own copy of the game (`textures/PACKS.md`).
The layers are grouped from "copy a file, works anywhere" to "needs a specific stack".

**Building.** The plugins are 32-bit Windows DLLs, built with `clang` + `lld-link` +
`llvm-dlltool` by each folder's `build.sh`, which writes to `<folder>/build/`.
**Prebuilt**, the zip on each
[release](https://github.com/cedrichaase/armada2-refit/releases) has all four with their
`.ini` files, `dxvk.conf` and the bloom config. It also has installers that do this
section for you: `install.sh` on Linux, `install.bat` on Windows (`README.txt` in the
zip). `publish/package.sh` builds the same zip locally.

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
| HUD | `hud/build/HUD.asi`, `hud/HUD.ini` | Don't also run `hud/ui-widescreen.py`, `ui-font-condense.py` or `cursor-aspect.py`; they are its file-based predecessors. Handles both cursor paths: D3D8's hardware cursor and the sprite path DXVK takes |
| Menus | `menus/build/Menus.asi`, `menus/Menus.ini` | GDI only, so the renderer doesn't matter. Delete any old `MenuScale.asi`. Without backdrop plates it draws black sides |
| Cutscenes (launch reels) | `cutscenes/binkproxy/build/binkw32.dll`, `BinkProxy.ini` | First rename the stock `binkw32.dll` to `binkw32_orig.dll`, because the proxy forwards to it. With no `.mp4` beside a `.bik` it only scales the launch reels to full screen. Replacement movies are AV1 through Media Foundation: under Proton that works (GStreamer + dav1d), and on Windows it presumably needs the AV1 Video Extension |

Uninstall by deleting the files you copied (plus the `.log` each one writes), and for
the Bink proxy by renaming `binkw32_orig.dll` back.

### 2. Needs DXVK as the game's renderer

DXVK's `d3d8.dll` and `d3d9.dll` go in the game directory, with `d3d8=n,b;d3d9=n,b`
under Wine/Proton (`platform/d3d8-chain.py --use dxvk`). The same two DLLs also work on
Windows.

| Layer | Install | Why DXVK |
|---|---|---|
| MSAA | `msaa/build/MSAA.asi`, `msaa/MSAA.ini` | The minimap copies from the back buffer. Native D3D8 does not allow that from a multisampled surface, and DXVK resolves it. Without DXVK expect a black minimap ([`msaa/README.md`](msaa/README.md)) |
| Renderer | `postfx/renderer-config.sh --stage 3` writes `dxvk.conf` | Only DXVK reads the file. Anisotropic filtering, LOD bias and seamless cube maps. The release zip has it ready, in `game/` |

### 3. Linux only: vkBasalt, launched through Heroic

| Layer | Install | Needs |
|---|---|---|
| Bloom | `postfx/vkbasalt/build.sh`, then `postfx/postfx.py --on` | DXVK (vkBasalt is a Vulkan layer), a per-user vkBasalt build, and Heroic: `--on` sets `ENABLE_VKBASALT=1` in the game's Heroic config, so quit Heroic first. Home toggles it in game |

The helper scripts (`./install`, `./a2mod`, the per-layer `install.sh`) are bash and
assume Linux. They find the game through `a2env.sh`, which defaults to Heroic's paths.
Any other layout works if you set `A2_GAME` / `A2_PREFIX`.

## The layers

The mod is a stack of independent layers, each in its own folder with its own README
(how it works and why) and changelog.

| Folder | Layer | What it changes |
|---|---|---|
| [`hud/`](hud/README.md) | hud | `HUD.asi`: the in-game HUD layout, font and cursors undistorted at any aspect, at run time |
| [`menus/`](menus/README.md) | menus | `Menus.asi`: the 800x600 shell menus scaled to fill the screen, embedded in the game window, with outpainted backdrops ([`BACKDROPS.md`](menus/BACKDROPS.md) builds them) |
| [`msaa/`](msaa/README.md) | msaa | `MSAA.asi`: 8x multisample anti-aliasing |
| [`postfx/`](postfx/README.md) | renderer, bloom | `dxvk.conf` (anisotropic filtering, LOD bias) and vkBasalt bloom |
| [`textures/`](textures/README.md) | textures | the texture pipeline, `./a2tex`: 84 targets (skyboxes, nebulae, planets, UI, hulls), each a recipe for one set of replacement textures |
| [`models/`](models/CHANGELOG.md) | models | the loading-screen model, widened with its `LOADING` art |
| [`cutscenes/`](cutscenes/binkproxy/README.md) | cutscenes | `binkproxy`, a `binkw32.dll` that plays launch reels full screen and AV1 replacements in place of `.bik` movies, and the movie pipeline |

`a2mod` does not switch these, because the layers above depend on them or they install
nothing:

| Folder | What it is |
|---|---|
| [`platform/`](platform/README.md) | Heroic and Proton, the DXVK d3d8 chain, the ASI loader, the widescreen patch |
| [`testbench/`](testbench/README.md) | `./a2test`: the game headless at any resolution, scenarios, reports |
| [`publish/`](publish/README.md) | what this repository may contain, and the check that enforces it |

## Entry points

    ./install                               install every layer
    ./a2mod status | stock | refit          switch every layer at once
    ./a2tex extract | build | install | pack | ...   the texture pipeline (textures/PACKS.md)
    ./a2test ...                            the game headless, for testing (testbench/)
    ./a2env.sh                              print where the game, prefix, Proton and assets are

Each layer installs and removes itself with its own script: `install.sh` in `hud/`,
`menus/`, `msaa/` and `cutscenes/binkproxy/`, `a2tex install`/`revert` for textures, or
a `--revert` flag on the Python tools. The layer's README has the details.

## Versions

Each layer folder has a `CHANGELOG.md` with its own semver version. The root
[`CHANGELOG.md`](CHANGELOG.md) versions the mod as a whole and lists the layer
versions it bundles.

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
