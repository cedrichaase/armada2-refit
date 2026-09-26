# Working in this directory

Tooling for remastering Star Trek: Armada II — the HUD, the menus, the cutscenes,
anti-aliasing and bloom, and `a2mod`, which switches the whole game between stock and
remastered.

**This repository is published.** It holds code, configuration and docs only — nothing
derived from the game's own files (hard rule 4, `publish/README.md`). The **texture
work** — the `a2tex` pipeline, its 84 targets, the loading-screen model, the archive and
the promo footage — lives in the private repository **`~/armada2-remastered-private`**,
which holds the game's own art and is never published. Texture and model tasks happen
there, under its own `CLAUDE.md`.

## Layout

The repo is laid out by **`a2mod` layer**: each folder is one thing `./a2mod` switches
between stock and remastered (or, for `platform/` and `gameplay/`, something it
deliberately leaves alone). `./a2mod status` names the layers — including `textures`
and `models`, which `a2mod` switches in the game directory although their source lives
in the private repository. Every folder has:

- a **`README.md`** — how the layer works and *why*: derivations, measurements, traps;
- a **`CHANGELOG.md`** — *what* changed, when, at which version, and whether it has been
  seen in game. **Read it before assuming the state of a layer**, and update it with
  every change (see "Changelogs and versions").

The root `CHANGELOG.md` is the project's: the modpack version and the layer versions
it bundles.

| Folder | What it is | Start with |
|---|---|---|
| `hud/` | `HUD.asi` — the in-game HUD layout, font and cursors at any aspect | `hud/README.md` |
| `menus/` | `Menus.asi` — the shell menus and their backdrops | `menus/README.md` |
| `msaa/` | `MSAA.asi` | `msaa/README.md` |
| `cutscenes/` | `binkproxy/` and the replacement `movies/` | `cutscenes/binkproxy/README.md` |
| `postfx/` | two layers: renderer (`dxvk.conf`) and bloom (vkBasalt) | `postfx/README.md` |
| `platform/` | what `a2mod` never switches: DXVK, the ASI loader, Heroic/Proton | `platform/README.md` |
| `gameplay/` | map scroll speed; cutscene draw distance (notes only) | `gameplay/README.md` |
| `testbench/` | `./a2test`: the game headless at any resolution, scenarios, reports. Installs nothing | `testbench/README.md` |
| `publish/` | what may be published and the check that enforces it. Installs nothing; versioned by the root | `publish/README.md` |

`./a2mod` is the entry point and stays at the root, with `./a2test` beside it for
testing.

**To see a change working in the game without taking over the user's screen, use the
test bench**: `./a2test session start --res 21:9`, then `./a2test drive shot` / `click` /
`key` / `step "…"`, and read the screenshots. It runs on a reflink clone of the install
and a headless display. Write regression scenarios in `testbench/scenarios/` for fixes
worth keeping fixed. A bench pass is evidence for the user, not their sign-off
("Finishing work" still applies).

Required reading, by task:

- **Anything renderer-adjacent: `platform/README.md`** — DLL overrides, the widescreen
  patch, the d3d8 chain, window management under Hyprland, the save format.
- **Anything that touches a texture file** (the font atlases, the cursors): the private
  repository's `textures/README.md` holds the engine reference — TGA format, mip
  chains, what counts as an interface sprite.

## Changelogs and versions

Every layer folder in the table above has a `CHANGELOG.md` with its own semver version.
The root `CHANGELOG.md` versions the project as a whole. **A commit that changes a layer
updates that layer's changelog and the root one in the same commit**, with the version
bumped. A changelog that is updated "later" gets forgotten.

**What gets an entry:** anything that changes what a layer installs into the game, how
it builds, or its interface (CLI flags, `target.conf`/`.ini`/`.conf` keys, backup
names). This includes tuning changes like a new blend, seed or threshold, and a revert
of shipped work. **No entry:** changes to docs and notes only, `archive/`, `promo/`,
and scratch work that installs nothing.

**Which part to bump:**

| Bump | When | Examples here |
|---|---|---|
| MAJOR | an existing install cannot move to it by just re-installing: the install/revert contract, backup suffix or `a2mod` snapshot layout changes; a key or flag is removed or renamed; paid `ai/` layers are invalidated; the layer newly requires something of `platform/` | a new backup suffix; dropping `blend=` for another key |
| MINOR | new content or capability: new targets or texture classes installed, a new menu handled, a new key, flag or tool | the five faction targets; `Embed=1`; `NoiseFloor=` |
| PATCH | a fix or re-tune behind the same interface | Sovereign blend 35 → 70; main-menu plate seed 6; the `Mmoon` sun-box fix |

The **project version** takes the largest bump among the layers the change touches.
Changes that belong to no one layer, such as `a2mod`, the repo layout or a cross-layer
convention, bump only the root. Every root entry lists the new version of each layer it
touches, for example `menus 1.1.0, textures 1.0.1`.

**Entry format.** Newest first, headed `## X.Y.Z — YYYY-MM-DD`, then `Added` / `Changed`
/ `Fixed` / `Removed` as needed. Each item is one or two lines saying *what* changed,
naming the key or target, with the commit hash once there is one. The *why* stays in
the layer's README. Do not copy derivations into a changelog, and do not copy status
("confirmed in game", dates, accepted seeds) into this file — that is what the
changelogs are for.

**In-game status goes in the entry.** End every entry with "Installed, not yet seen in
game" or "Confirmed in game". When the user confirms later, **replace** "Installed, not
yet seen in game" with `Confirmed in game YYYY-MM-DD` and do not bump. Don't keep both:
an entry that still says "not yet seen" after it has been seen is misleading. This is the
one permitted edit to an entry that has already been released. A rejection is not a status line. It is a new entry
that reverts the change, like the font's `--method runs`.

**Parallel worktrees** will both claim the next number. Whichever branch merges second
renumbers its entries (layer and root) while resolving the conflict. Never leave two
entries with the same version.

## Finishing work

Work is done when the user has confirmed it working in game **and** signed it off as
done. A test launch, or a passing `verify`, does not count. Then, without being asked
again, finish in this order:

1. **Record the confirmation** on the branch: `Confirmed in game YYYY-MM-DD` in place
   of "Installed, not yet seen in game", in the layer and root changelogs (above), and
   commit.
2. **Merge into `master`** from the main checkout, with
   `git merge --no-ff <branch> -m "Merge <branch>: <what>"`. Resolve version clashes as
   "Parallel worktrees" says. There is no remote, so there is nothing to push.
3. **Clean up.** Remove the worktree (`git worktree remove`) and delete its branch
   (`git branch -d`). Then do the same for any other worktree whose branch
   `git branch --merged master` lists. Use only the safe forms, never `--force` or
   `-D`. If one refuses because of uncommitted changes or unmerged commits, leave it
   and tell the user.

Until the sign-off, the work stays on its branch and in its worktree.

## Environment

- Game: `/home/cedric/Games/Heroic/Star Trek Armada II` — GOG release, patch 1.1 plus
  Patch Project 1.2.5, run through Heroic with Proton-CachyOS.
- Textures: `Textures/RGB/`, flat, **2115 `.tga` files, 196 MB stock**, **mixed
  `.tga` / `.TGA` case**. (196 MB is the byte total; `du` says 205 MB, because 2115
  small files carry ~9 MB of block slack. Both figures appear in older notes.)
- Available: ImageMagick 7 (`magick`), `python3`, `ffmpeg`.
  **Not available: numpy, PIL.** Do image work through ImageMagick, not Python.
- **32-bit Windows DLLs can be built here**, which is not obvious: there is no MSVC and
  no mingw, but `clang` + `lld-link` + `llvm-dlltool` are installed and that is enough
  for an ASI plugin. Declare the Win32 prototypes yourself, generate import libraries
  from `.def` files (**`llvm-dlltool --kill-at`** — the `.def` carries decorated names,
  the real DLLs export undecorated ones), and link `/nodefaultlib`. See
  `menus/build.sh`. `objdump` also reads `pei-i386`, and the game ships
  `armada2.map` — a full symbol map — so `Armada2.exe` can be read rather than guessed at.
- Scratchpad for intermediates; this directory is the user's, keep it tidy.

## Hard rules

1. **Never write into `Textures/RGB/` directly.** The texture pipeline (private
   repository) installs there through `a2tex install`, making a `.a2neb-backup` on first
   touch; `a2tex revert all` sweeps every `*.a2neb-backup` in `Textures/RGB` — the
   cursors included — and restores it.
   **The one sanctioned exception is `hud/ui-font-condense.py`**, which rewrites the
   twelve `FontFinal4_*` atlases in place because its edit is paired with the `.spr`
   metrics beside them in `Sprites/` and so cannot live in a target. It owns its own
   backups under `.a2font-backup` — a *different* suffix, precisely so `a2tex revert
   all` cannot restore a stock atlas under condensed metrics and garble every glyph.
   The UI configs in `misc/` also have `.a2neb-backup`s, but are reverted by
   `hud/ui-widescreen.py --revert`, not by `a2tex`. Any TGA written must match the stock
   file's header exactly, bit depth and origin byte included (private repository,
   `CLAUDE.md` hard rule 2).
2. **Always glob both extension cases.** `$(ls "$n.tga" "$n.TGA" 2>/dev/null | head -1 || true)`
   — and note the `|| true`: under `set -o pipefail` the failing `ls` aborts the script.
3. **Never install a UI sprite larger than 256x256.** `@tmaterial=interface` textures at
   512 **crash the game** at the cinematic-to-HUD transition (`rep movsd` in
   `Armada2.exe` -- a memcpy into a buffer that is too small). Exactly one stock texture
   in the game is 512 (`WshladSW`, a weapon) and no stock interface sprite is over 256.
   3D model textures are unaffected, so this is the sprite path specifically.
4. **Commit nothing derived from the game's own files.** This repository is published
   (`publish/README.md`): code, configuration and docs only. No stock copy, no upscale,
   no comparison plate, no screenshot, no game video — at any path. `.gitignore` ignores
   every image, video and game format, so it takes a `git add -f` to break this; don't.
   `publish/check.sh` fails on any binary in the index; run it before committing anything
   unusual. Material worth keeping that may not be published goes in the private
   repository, `~/armada2-remastered-private/`.

## a2mod

**`./a2mod stock` / `remastered` / `status`** flips the *whole game* for
  before/after: textures, font, HUD layout, the menus (`Menus.asi`), MSAA, cutscenes, the loading-screen
  model, `dxvk.conf` and bloom at launch. DXVK, the ASI loader, the widescreen patch and
  the scroll-speed files stay as they are in both states. It **snapshots** rather than
  reinstalls: modded files move to `$GAME/.a2mod/` and back, hash-checked, because some
  installed layers exist only on unmerged branches and a reinstall would not reproduce
  them. While in stock, anything installed over a stock file makes `remastered` refuse.
  Do not delete `$GAME/.a2mod` while in stock.

## Per-layer notes

What an agent must know before touching each layer — the decisions not to re-open and
the traps not to re-derive. **For what is installed, at which version, and whether it
has been seen in game, read that layer's `CHANGELOG.md`**; for the derivations, its
`README.md`.

### hud

- **`HUD.asi` makes all three corrections below at run time**, from the display mode
  actually set, with the game files stock. The three scripts are its derivation and its
  `--revert`; **never install them alongside it** — every correction would apply
  twice. `hud/install.sh` reverts them and the plugin stands down per part if it finds
  them. `hud/README.md` has the hook table and what was established to get there.
- **Do not change `cfgSCREEN_WIDTH` (`RTS_CFG.h`) to fix the canvas.** It is the fixed
  1600 every rect is converted into and the palette reads against, with 33 readers.
- **The UI stretch is a layout-canvas bug, not a texture problem.** `misc/gui_<race>.cfg`
  declares a 1600x1200 canvas that the engine scales to the back buffer per axis.
  `hud/ui-widescreen.py` re-declares it and moves the anchored panels. The bridge
  display is the one unhandled case.
- **`popupPaletteXA`/`XB` are read against a hard-coded 1600**, not the canvas — every
  rect in the same file takes the canvas. `popupPaletteXA` is the one key that
  deliberately does not reproduce stock: it goes flush with the info panel.
- **The cursors are a third reference, a hard-coded 800x600, and a hardware cursor.**
  `cursor.spr` cannot fix them (it reaches only the map-plane part), so
  `hud/cursor-aspect.py` squashes the art 32 → 18 texels **about each `@origin`
  hotspot**, and `cursor.spr` stays stock — applying both would square the art twice.
  **The resample filter must be interpolating**: Mitchell bleeds a row onto the colour
  key at vertical scale 1.0; Catrom is exact.
- **No art-side answer makes the cursors sharper.** `armada2.map` shows the engine
  allocates the cursor at `texW * [device+0x18]` by `texH * [device+0x1c]` and copies
  1:1, so more texels just draw a bigger cursor. **Under DXVK the cursor is not the
  hardware one**: `[device+0xe0]` is set and `RefreshDisplay` draws the sprite itself
  under the global 2D scale; `HUD.asi` fixes both paths (`hud/README.md`).
- The font ignores the canvas too: its quads scale by back-buffer / **1280x1024**, a
  hard-coded tier. `hud/ui-font-condense.py` squeezes glyph art and advance widths by
  `1.25 * H / W`; the vertical axis is already right and is not touched. `--check`
  verifies `.spr` and `.tga` still agree. Its backups are `.a2font-backup` (hard rule 1).
- **`--method runs` was built, measured, and rejected in game. Don't re-ship it.** It won
  on ink ratio, opaque-texel count and dropouts, and looked worse. Those metrics measure
  weight and structure, not legibility. **Never change the font's appearance on the
  strength of that table; put it in the game and look.**

### menus

- **The menus are a third UI, separate from the HUD and the textures, and need code.**
  Every menu is a Win32 dialog drawn with GDI from 800x600 BMPs; `Armada2.exe` imports
  `BitBlt` and **no `StretchBlt`**, and the shell never touches Direct3D — so no
  `gui_*.cfg`, config file or d3d8/DXVK setting can scale it. The engine asks for an
  **800x600 display mode for the front end**, so the shell was *filling a small screen*;
  `Menus.asi` raises that mode in memory and scales the shell into it.
- Every menu is a separate OS window unless **`Embed=1`**, which re-creates each as a
  `WS_CHILD` of the game window.
- **`Menus.asi` was `MenuScale.asi` before menus 2.0.0, and the two must never be
  installed together**: the ASI loader loads every `*.asi`, so both would hook
  everything. `menus/install.sh` removes the old names, `Menus.asi` stands down if it
  sees one, and `a2mod` still recognises `MenuScale.*` so a stale copy is set aside.
- **GetDC/ReleaseDC do not pair in this game**: `ShellButton::UpdateButton` has its
  `ReleaseDC` NOP'd out, so nothing in `menus.c` may assume they do.
- **Backdrops are composited per frame**, because the shell draws 1:1 into 800x600 and a
  bigger BMP cannot help: `stretch(frame) + w·(plate − stretch(stock))`. A weight, not a
  mask, because `MainBk_flare.bik` *is* the logo band with the background baked in.
  Plates come from `menus/backdrop.sh`; `field=`, `clone=`, `keep=`, `fade=`,
  `dehaze=` and `N.soften=` are documented in `menus/README.md`. The main menu's paid
  seeds are kept in `ai/`.

### cutscenes

`cutscenes/binkproxy/` is a replacement `binkw32.dll` that plays
`animations/<Name>.mp4` + `.wav` in place of `<Name>.bik` and raises
`PlayIntroMovie`'s hard-coded 640x480 mode, which a bigger `.bik` could never escape.
**H.264, VP9 and AAC do not decode under Proton outside Steam; AV1 does.** Movies live in
`cutscenes/movies/<Name>/`, laid out like a texture target: `ai/` is paid and never
overwritten, and **`ai-movie.sh` is the only script that spends**. Stock `.bik` files are
only ever read.

### msaa, postfx, platform

- **The game renders through DXVK/Vulkan, and did not until 2026-09-22** —
  `syswow64/d3d8.dll` was recorded as DXVK's on the strength of its byte count and was
  *Wine's builtin*. The working chain keeps DXVK's `d3d8.dll` **and** `d3d9.dll` in the
  **game directory** (the prefix is not durable — Proton restores it from symlinks),
  `d3d9=n,b` in the overrides, and the Wine virtual desktop **off**. The virtual desktop
  is **superseded**: it was once the fix for menus opening as separate windows, which
  `Menus.asi`'s `Embed=1` now solves inside the game window. Never turn it back on
  to fix a window problem — fix it in `menus.c`.
  `platform/d3d8-chain.py --status` identifies every link **by hash; never identify one
  by size.** "Heroic redeploys DXVK" is `autoInstallDxvk` working as designed.
- **Bloom** is vkBasalt, possible only because the chain is Vulkan. `postfx/postfx.py`
  refuses any effect `fxcheck` cannot compile.
- **MSAA is an ASI hook** because nothing in DXVK or `dxcfg.ini` can turn it on. The
  minimap is the one thing that could break (`msaa/README.md`).
