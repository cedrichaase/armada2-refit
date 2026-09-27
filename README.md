# Armada II remastered

A remaster of Star Trek: Armada II — the GOG release, patch 1.1 plus Patch Project
1.2.5, run through Heroic with Proton on Arch + Hyprland. The code layers fix what the
engine does at modern resolutions — the HUD, the menus, anti-aliasing, bloom — at run
time, and all of it reverts. The asset layers — upscaled textures, the widened loading
screen, an upscaled intro, widescreen menu backdrops — are a **pipeline for making your
own**: extract the stock art from your install, upscale or replace it, build, install.

**This repository holds no game content**: no textures, models, video or data from the
game, original or derived (`publish/README.md`) — only the code and the recipes. What
you build lives in `A2_DATA`, outside any checkout. Without it, every layer still works:
the asset layers install nothing and the game keeps its own art.

Where things are — `./a2env.sh` prints it; each is taken from the environment, then
`~/.config/armada2-remastered.conf` (`KEY=value` lines), then the default:

| | Default |
|---|---|
| `A2_GAME` | Heroic's `~/Games/Heroic/Star Trek Armada II` |
| `A2_PREFIX`, `A2_PROTON` | Heroic's prefix and Proton for it |
| `A2_DATA` | `~/.local/share/armada2-remastered` (`%LOCALAPPDATA%` on Windows): extracted stock art, paid AI layers, builds. **Back up its `ai/` folders**; everything else is rebuilt by a command |

## The layers

The modpack is a stack of independent layers. `./a2mod` switches the whole stack
between stock and remastered for before/after comparisons, and each layer has a folder
here with its own README.

| Folder | Layer | What it changes |
|---|---|---|
| [`hud/`](hud/README.md) | hud | `HUD.asi`: the in-game HUD layout, font and cursors undistorted at any aspect, at run time |
| [`menus/`](menus/README.md) | menus | `Menus.asi`: the 800x600 shell menus scaled to fill the screen, embedded in the game window, with outpainted backdrops ([`BACKDROPS.md`](menus/BACKDROPS.md) builds them) |
| [`msaa/`](msaa/README.md) | msaa | `MSAA.asi`: 8x multisample anti-aliasing |
| [`postfx/`](postfx/README.md) | renderer, bloom | `dxvk.conf` (anisotropic filtering, LOD bias) and vkBasalt bloom |
| [`textures/`](textures/README.md) | textures | the texture pipeline, `./a2tex`: 84 targets — skyboxes, nebulae, planets, UI, hulls — each a recipe to upscale from the game's own art |
| [`models/`](models/CHANGELOG.md) | models | the loading-screen model, widened with its `LOADING` art |
| [`cutscenes/`](cutscenes/binkproxy/README.md) | cutscenes | `binkproxy`, a `binkw32.dll` that plays launch reels full screen and AV1 replacements in place of `.bik` movies, and the movie pipeline |

Not switched by `a2mod`, because the layers above stand on them:

| Folder | What it is |
|---|---|
| [`platform/`](platform/README.md) | Heroic and Proton, the DXVK d3d8 chain, the ASI loader, the widescreen patch |

## Entry points

    ./install                               install every layer
    ./a2mod status | stock | remastered     switch every layer at once
    ./a2tex stock | build | install | ...   the texture pipeline (textures/README.md)
    ./a2test ...                            the game headless, for testing (testbench/)
    ./a2env.sh                              print where the game, prefix, Proton and assets are

Each layer installs and removes itself with its own script: `install.sh` in `hud/`,
`menus/`, `msaa/` and `cutscenes/binkproxy/`, `a2tex install`/`revert`, or a
`--revert` flag on the Python tools. See the layer's README.

## Versions

Each layer folder has a `CHANGELOG.md` with its own semver version. The root
[`CHANGELOG.md`](CHANGELOG.md) versions the modpack as a whole and lists the layer
versions it bundles.

`publish/` says what this repository may contain and why. `CLAUDE.md` is the working
brief for Claude Code sessions in this repo.
