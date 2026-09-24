# Armada II remastered

A remaster of Star Trek: Armada II — the GOG release, patch 1.1 plus Patch Project
1.2.5, run through Heroic with Proton on Arch + Hyprland. Everything here is derived
from the game's own art and binary, and all of it reverts.

Game directory: `/home/cedric/Games/Heroic/Star Trek Armada II`

## The layers

The modpack is a stack of independent layers. `./a2mod` switches the whole stack
between stock and remastered for before/after comparisons, and each layer has a folder
here with its own README.

| Folder | Layer | What it changes |
|---|---|---|
| [`textures/`](textures/README.md) | textures | about 1,660 textures upscaled from their own stock art: skyboxes, nebulae, planets, UI, hulls |
| [`models/`](models/logo-sod.py) | models | the mission loading screen, widened to fill 21:9 |
| [`hud/`](hud/README.md) | hud layout | the in-game HUD re-laid out for widescreen; the cursors un-stretched |
| [`menus/`](menus/README.md) | menu scale | `MenuScale.asi`: the 800x600 shell menus scaled to fill the screen, with outpainted backdrops |
| [`msaa/`](msaa/README.md) | msaa | `MSAA.asi`: 8x multisample anti-aliasing |
| [`cutscenes/`](cutscenes/binkproxy/README.md) | cutscenes | a `binkw32.dll` proxy that plays upscaled AV1 movies full screen |
| [`postfx/`](postfx/README.md) | renderer, bloom | `dxvk.conf` (anisotropic filtering, LOD bias) and vkBasalt bloom |
| [`font/`](font/README.md) | font | the in-game bitmap font condensed so text is not drawn 1.9x too wide |

Not switched by `a2mod`, because the layers above stand on them:

| Folder | What it is |
|---|---|
| [`platform/`](platform/README.md) | Heroic and Proton, the DXVK d3d8 chain, the ASI loader, the widescreen patch |
| [`gameplay/`](gameplay/README.md) | map scroll speed; the cutscene draw distance (understood, not changed) |

## Entry points

    ./a2mod status | stock | remastered     switch every layer at once
    ./a2tex list | build | install | revert | verify     the texture pipeline

Each of the other layers installs and removes itself with its own script: `install.sh`
in `menus/`, `msaa/` and `cutscenes/binkproxy/`, or a `--revert` flag on the Python
tools. See the layer's README.

`archive/` holds material that is paid for or cannot be regenerated; nothing in it is on
a build path. `CLAUDE.md` is the working brief for Claude Code sessions in this repo.
