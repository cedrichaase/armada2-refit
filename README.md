<div align="center">

# Armada II Refit

**Star Trek: Armada II, made for modern screens. Same game, every fix reversible.**

[![Release](https://img.shields.io/github/v/release/cedrichaase/armada2-refit?style=for-the-badge&logo=github&color=4F4C94)](https://github.com/cedrichaase/armada2-refit/releases)
[![CI](https://img.shields.io/github/actions/workflow/status/cedrichaase/armada2-refit/ci.yml?style=for-the-badge&logo=githubactions&logoColor=white&label=CI)](https://github.com/cedrichaase/armada2-refit/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-6291B1?style=for-the-badge)](LICENSE)
[![Downloads](https://img.shields.io/github/downloads/cedrichaase/armada2-refit/total?style=for-the-badge&color=C5CEF1&labelColor=222)](https://github.com/cedrichaase/armada2-refit/releases)

![Rust](https://img.shields.io/badge/Rust-no__std-DEA584?style=flat-square&logo=rust&logoColor=black)
![Vulkan](https://img.shields.io/badge/Vulkan-ready-AC162C?style=flat-square&logo=vulkan&logoColor=white)
![Direct3D 12](https://img.shields.io/badge/Direct3D-12-0078D4?style=flat-square&logo=windows&logoColor=white)
![Linux](https://img.shields.io/badge/Linux-Proton-FCC624?style=flat-square&logo=linux&logoColor=black)
![Windows](https://img.shields.io/badge/Windows-supported-0078D4?style=flat-square&logo=windows&logoColor=white)
![macOS](https://img.shields.io/badge/macOS-Wine-000000?style=flat-square&logo=apple&logoColor=white)
![Status](https://img.shields.io/badge/fan%20project-unofficial-lightgrey?style=flat-square)

[**Install**](#install) · [Features](#features) · [Layers](#layers) · [Build](BUILDING.md) · [Play](PLAYING.md) · [Mod](MODDING.md) · [Changelog](CHANGELOG.md)

</div>

---

## Why

Armada II was built for a 4:3 CRT at 1024x768. On a modern display the HUD stretches, the menus sit in an 800x600 box, edges shimmer, and the art falls apart at 4K.

Refit fixes this one piece at a time. The core fixes replace no art and patch no files on disk. Textures, backdrops and movies are optional layers. `./a2mod stock` puts the whole game back as it shipped, so you can compare before and after in seconds.

## Features

| | |
|---|---|
| **Stretch free HUD** | Drawn at any aspect ratio (16:9, 21:9, 32:9) with its own font and cursors. See [`ui/HUD.md`](ui/HUD.md) |
| **Full screen menus** | Shell menus fill the window, with optional widescreen backdrops. See [`ui/MENUS.md`](ui/MENUS.md) |
| **A new 3D renderer** | HDR, multisampled, bloomed and tone mapped. Lit hulls with shadows, procedural skies, planets, fly through nebulae, shader driven effects and a smooth fog of war. See [`render3d/`](render3d/README.md) |
| **Display8** | Our own `d3d8.dll`. Draws on Vulkan or Direct3D 12 with no DXVK and no d3d8to9, and falls back to the system DLL if neither starts. See [`display/`](display/README.md) |
| **Quality of life** | Faster panning, `Shift+number` grouping, selections beyond 16, stations in control groups, long moves at warp. Fully compatible with stock players. See [`qol/`](qol/README.md) |
| **Grid hotkeys** | A 5x3 button bar keyed by keyboard position (`QWERT`, `ASDFG`, `ZXCVB`), labelled on the buttons |
| **Online multiplayer** *(in progress)* | Join codes, hole punching and a relay, because Wine stubs DirectPlay. See [`online/`](online/README.md) |
| **Texture pipeline** | `./a2tex` validates, installs and packs replacement textures. 84 recipes so far |
| **Smoother geometry** | Rounded moons, asteroids and selection bubbles. Built from your own files |
| **Video replacement** | AV1 movies and full screen launch reels through a `binkw32.dll` proxy |
| **Headless test bench** | `./a2test` runs the game on a virtual display with scripted scenarios and screenshots |

> [!NOTE]
> This repository holds code, configuration, recipes and docs only. No game content, original or derived, is included. Asset pipelines start from your own copy of the game, and every layer works without assets.

## Install

You need the **GOG release** of Star Trek: Armada II (patch 1.1) and [STA2WidescreenPatch](https://github.com/Ligushka/STA2WidescreenPatch) v1.0, which brings the [Ultimate ASI Loader](https://github.com/ThirteenAG/Ultimate-ASI-Loader). Both are MIT licensed and bundled, so the installers add them if they are missing.

Download the installer for your system from the [latest release](https://github.com/cedrichaase/armada2-refit/releases). It finds the game, shows what it will change, and sets the launch options.

| System | File | Notes |
|---|---|---|
| **Windows** | `armada2-refit-installer-windows.exe` | One portable file. Unsigned, so SmartScreen may ask: *More info*, then *Run anyway* |
| **macOS** | `armada2-refit-installer-macos.zip` | Apple Silicon and Intel. Press **Set up Armada II** and it builds a free [Sikarugir](https://github.com/Sikarugir-App/Sikarugir) wrapper and installs the game. Not notarised, so allow it under *Privacy & Security*, then *Open Anyway*. Existing CrossOver bottles are detected |
| **Linux** | `armada2-refit-installer-x86_64.AppImage` | Make it executable and run it. Ships with GTK 4 and libadwaita and writes Heroic's launch options |

Antivirus complaining? See [Antivirus false positives](#antivirus-false-positives).

### From source

```sh
./a2env.sh        # check where the game, prefix and Proton are
./install         # build and install every layer
./a2mod status    # what is installed, layer by layer
./a2mod stock     # back to the game as it shipped
./a2mod refit     # and forward again
```

Paths come from the environment, then `~/.config/armada2-refit.conf`, then these defaults:

| Variable | Default |
|---|---|
| `A2_GAME` | `~/Games/Heroic/Star Trek Armada II` |
| `A2_PREFIX`, `A2_PROTON` | Heroic's prefix and Proton for the game |
| `A2_DATA` | `~/.local/share/armada2-refit`. Holds extracted art, paid AI layers and builds. **Back up its `ai/` folders**, everything else can be rebuilt |
| `A2_PROFILE` | `armada2`. Picks the game by its descriptor in [`games/`](games/README.md), or pass `--game <id>` for one run |

**Toolchain for building:** `clang`, `lld-link`, `llvm-dlltool`, ImageMagick 7, Python 3 and `ffmpeg`. No MSVC or mingw. Generative upscaling is optional and uses [`belt`](https://inference.sh). Developed on Arch with Hyprland and Heroic. Full setup lives in [`platform/README.md`](platform/README.md), and the build is in [`BUILDING.md`](BUILDING.md).

## Layers

Refit is a stack of independent layers. Each has its own folder, README and changelog. `a2mod` switches them between stock and refit.

| Layer | What it does |
|---|---|
| [`display`](display/README.md) | `d3d8.dll` on Vulkan or Direct3D 12 |
| [`render3d`](render3d/README.md) | `Render3D.asi`, the 3D view |
| [`ui`](ui/README.md) | `UI.asi`: HUD, menus, button bar and the Refit settings screen |
| [`qol`](qol/README.md) | `QOL.asi`: stock compatible quality of life |
| [`rules`](rules/README.md) | `QOLRules.asi`: rules every player in a match must share |
| [`online`](online/README.md) | `Online.asi` and a self hostable relay server |
| [`textures`](textures/README.md) | 84 texture recipes. Start with [`PACKS.md`](textures/PACKS.md) |
| [`models`](models/README.md) | Smoothed moons, asteroids, selection bubble and the loading screen model |
| [`cutscenes`](cutscenes/binkproxy/README.md) | Bink proxy and the movie pipeline |

Not switched by `a2mod`: [`platform`](platform/README.md) (Heroic, Proton, ASI loader), [`testbench`](testbench/README.md), [`publish`](publish/README.md) and [`retire`](retire/README.md), which removes whatever earlier versions installed.

Each layer installs and removes itself with its own `install.sh`, or with `a2tex install` and `revert` for textures.

## How it works

Plugins are 32 bit `.asi` DLLs loaded by the ASI loader. They patch `Armada2.exe` in memory only, after checking byte signatures. On any other build they do nothing and say so in their log. Engine addresses live in per build profiles under `profiles/`, never in the sources.

Under Wine or Proton the loader needs the `winmm=n,b` override, set for you by the installers. Without it, nothing loads and nothing errors. See [`platform/README.md`](platform/README.md#heroic-configuration).

Installing by hand is documented per layer in [`PLAYING.md`](PLAYING.md) and [`MODDING.md`](MODDING.md).

## Antivirus false positives

Some scanners, Bitdefender among them, quarantine the plugins. They are unsigned DLLs that hook another process, which is exactly what heuristics look for. They are built from this source by GitHub CI, and each release lists a VirusTotal link per file and a `SHA256SUMS`.

**Files that are ours:** every `.asi` in `game/`, `binkw32.dll`, `display/d3d8.dll` and the Windows installer. `winmm.dll` and `STA2WidescreenPatch.asi` are third party and unmodified.

**To fix it:**

1. Restore the files from your antivirus quarantine.
2. Exclude the installer cache (`%LOCALAPPDATA%\armada2-refit\`, `~/.cache/armada2-refit/` on Linux, `~/Library/Caches/armada2-refit/` on macOS) and the game folder.
3. Run the installer again. If it still cannot clear an earlier unpack, restart first.

Reporting the false positive to your vendor helps everyone who installs after you.

## Versions and contributing

Every layer has its own semver and `CHANGELOG.md`. The root [`CHANGELOG.md`](CHANGELOG.md) versions the mod as a whole. Changes are recorded as [changesets](.changeset/README.md) and turned into releases by hand ([`publish/README.md`](publish/README.md#releasing)). The installer's *Pre-release* entry installs the newest `main`, untested.

[`CLAUDE.md`](CLAUDE.md) is the working brief for agent sessions in this repo.

## License

Code, recipes and docs are [MIT](LICENSE). That covers nothing of the game's.

Refit is an unofficial fan project, not affiliated with or endorsed by Activision, CBS Studios or Paramount. Star Trek: Armada II is © Activision, and Star Trek marks belong to CBS Studios and Paramount. Their names appear only to say which game this works with, and nothing this project produces should be presented as official. The engine notes in the layer READMEs describe what `Armada2.exe` does in our own words and quote no code from it.
