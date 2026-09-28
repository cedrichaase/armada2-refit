# Making textures, and texture packs

This repository ships no textures. It ships the **recipes** — 84 targets in
`targets/<NAME>/`, each a `target.conf` and a `stock.sha256` — and the pipeline that
turns the game's own art into replacements. Everything it makes lives in `A2_DATA`
(`../a2env.sh`; default `~/.local/share/armada2-refit`), never in the checkout.

This page is the short path. The why, and every trap already walked into, is in
[`README.md`](README.md) (the engine reference), [`REMASTERING.md`](REMASTERING.md)
(anything that is not a nebula — read it before touching a texture with alpha) and
[`PROMPTS.md`](PROMPTS.md) (generating art from scratch).

## What you need

- The GOG release of Armada II, patch 1.1 — the release every
  `stock.sha256` was taken from. `./a2env.sh` must print its directory as `A2_GAME`.
- ImageMagick 7 (`magick`) and `python3`. No numpy, no PIL.
- For generative upscaling only: [`belt`](https://inference.sh), the inference.sh CLI,
  at `~/.local/bin/belt` and logged in. Everything else runs offline.

## 1. Extract the stock art

    ./a2tex extract                 # every target
    ./a2tex extract MbgBorg Mmoon   # or some

Copies each target's stock files out of `Textures/RGB` into
`$A2_DATA/textures/<NAME>/stock/`, taking the `.a2neb-backup` where one exists, and
accepts a file only if its SHA-256 matches the recipe's `stock.sha256`. A mismatch is
reported as `MISSING` and is never "close enough": it means the install is not the
release the recipes describe. Every other subcommand extracts what it needs first, so
this step is only ever needed to look at the originals.

## 2a. Upscale a target's own stock art

This is how every texture this project has installed was made. It costs money — about
$0.005 per image at the default 1024–2048px — and it is the only step that does.

    textures/tools/upscale-stock.sh MbgBorg                # paid: fills ai/, blends src/
    textures/tools/upscale-stock.sh --reblend --blend 50 MbgBorg   # free: re-blend ai/

Three layers per target, in its work directory:

| Layer | What it is | To recreate |
|---|---|---|
| `ai/` | the raw generative upscale, one image per face | **money** — back it up |
| `src/` | `ai/` blended `blend=`% over a plain Lanczos upscale | free: `--reblend` |
| `out/` | the finished TGA, in the stock file's exact format | free: `./a2tex build` |

`blend=` in `target.conf` is the one dial. Set it on a render **at the size the object
is drawn on screen**, not at 1:1: `textures/tools/measure-invention.sh <NAME>` reports
what the model invented and erased at those sizes. Skyboxes sit at 35, hulls at 70.
`README.md`, "Controlling how much a generative upscaler invents", and `CLAUDE.md`'s
six rules of generative upscaling say why.

## 2b. …or make new art

Drop images into `$A2_DATA/textures/<NAME>/src/` — one per face, sorted by file name —
and set `source=gen` in `target.conf` for a nebula puff. `PROMPTS.md` has prompts that
produce usable nebulae, and why stock won over them where both were tried. The build
matches every channel's mean to the stock file's, so the art only has to get shape,
density and fill right.

## 3. Build, verify, install

    ./a2tex build MbgBorg -j 8      # src/ -> out/, in the stock file's format
    ./a2tex verify MbgBorg          # header, size, means, mip chain -- from the raw bytes
    ./a2tex install MbgBorg         # into the game, backing up each file first
    ./a2tex revert all              # every texture back to stock

`install` refuses a target outright rather than install part of it when a build would
crash the game: an interface sprite over 256px, or a base texture resized without its
hand-authored mip chain. `./a2mod stock` / `refit` switches the whole game for a
before/after either way.

**A new target** is a directory under `targets/` with a `target.conf` (keys:
`README.md`, "target.conf"); copy the stock files it covers into its work directory's
`stock/` and run `./a2tex stock --manifest <NAME>` to write its `stock.sha256`.

## 4. Pack and share

    ./a2tex pack mypack                     # every built, installable target
    ./a2tex pack skyboxes MbgBorg MbgDom1   # or some
    ./a2tex install --pack mypack.zip       # on another machine, into its game

`pack` writes `$A2_DATA/packs/<NAME>.zip`: each target's `out/` and a `pack.txt` with
the pack's name, date, commit and the SHA-256 of every file. `install --pack` refuses a
pack with a missing, extra or altered file, then runs the ordinary install from it,
every guard included — so a pack needs a checkout that has recipes for its targets,
and installs the same way a local build does. Revert it with `./a2tex revert all`.

**A pack is the game's art, upscaled.** This project does not publish one, and you are
responsible for what you share (`../publish/README.md`).
