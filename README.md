# Armada II remastered

A remaster of Star Trek: Armada II — the GOG release, patch 1.1 plus Patch Project
1.2.5, run through Heroic with Proton on Arch + Hyprland. The layers here fix what the
engine does at modern resolutions — the HUD, the menus, anti-aliasing, bloom — at run
time, and all of it reverts.

**This repository holds no game content**: no textures, models, video or data from the
game, original or derived (`publish/README.md`). The upscaled textures, the upscaled
intro and the menu backdrop plates are a separate, private project for that reason.

Game directory: wherever `./a2env.sh` says — Heroic's default,
`~/Games/Heroic/Star Trek Armada II`, unless `A2_GAME` or
`~/.config/armada2-remastered.conf` (`A2_GAME=`, `A2_PREFIX=`, `A2_PROTON=` lines) says
otherwise.

## The layers

The modpack is a stack of independent layers. `./a2mod` switches the whole stack
between stock and remastered for before/after comparisons, and each layer has a folder
here with its own README.

| Folder | Layer | What it changes |
|---|---|---|
| [`hud/`](hud/README.md) | hud | `HUD.asi`: the in-game HUD layout, font and cursors undistorted at any aspect, at run time |
| [`menus/`](menus/README.md) | menus | `Menus.asi`: the 800x600 shell menus scaled to fill the screen, embedded in the game window, with outpainted backdrops |
| [`msaa/`](msaa/README.md) | msaa | `MSAA.asi`: 8x multisample anti-aliasing |
| [`postfx/`](postfx/README.md) | renderer, bloom | `dxvk.conf` (anisotropic filtering, LOD bias) and vkBasalt bloom |
| — | textures, models, cutscenes | upscaled textures, the widened loading screen, and the upscaled intro with the `binkw32.dll` proxy that plays it; built in the private repository, switched here by `a2mod` |

Not switched by `a2mod`, because the layers above stand on them:

| Folder | What it is |
|---|---|
| [`platform/`](platform/README.md) | Heroic and Proton, the DXVK d3d8 chain, the ASI loader, the widescreen patch |

## Entry points

    ./install                               install every layer in this repository
    ./a2mod status | stock | remastered     switch every layer at once
    ./a2test ...                            the game headless, for testing (testbench/)
    ./a2env.sh                              print where the game, prefix and Proton are

Each layer installs and removes itself with its own script: `install.sh` in `hud/`,
`menus/` and `msaa/`, or a `--revert` flag on the Python tools.
See the layer's README.

## Versions

Each layer folder has a `CHANGELOG.md` with its own semver version. The root
[`CHANGELOG.md`](CHANGELOG.md) versions the modpack as a whole and lists the layer
versions it bundles.

`publish/` says what this repository may contain and why. `CLAUDE.md` is the working
brief for Claude Code sessions in this repo.
