# Working in this directory

Tooling for remastering Star Trek: Armada II — the HUD, the menus, anti-aliasing and
bloom as run-time plugins; the texture pipeline (`a2tex`), the widened loading screen,
the cutscene proxy and the movie and menu-backdrop pipelines; and `a2mod`, which
switches the whole game between stock and refit.

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
alone). `./a2mod status` names the layers. Every folder has:

- a **`README.md`** — how the layer works and *why*: derivations, measurements, traps;
- a **`CHANGELOG.md`** — *what* changed, when, at which version, and whether it has been
  seen in game. **Read it before assuming the state of a layer**, and update it with
  every change (see "Changelogs and versions").

The root `CHANGELOG.md` is the project's: the modpack version and the layer versions
it bundles.

| Folder | What it is | Start with |
|---|---|---|
| `hud/` | `HUD.asi` — the in-game HUD layout, font and cursors at any aspect | `hud/README.md` |
| `menus/` | `Menus.asi` — the shell menus — and `backdrop.sh`, which builds the widescreen plates it composites | `menus/README.md`, `menus/BACKDROPS.md` |
| `msaa/` | `MSAA.asi` | `msaa/README.md` |
| `qol/` | `QOL.asi` — gameplay quality of life compatible with stock players (right-drag pan speed, Shift+number adds to a group, selections and groups beyond 16, stations in groups with one build menu for several), and the plan for the rest, split by whether other players need it | `qol/README.md` |
| `grid/` | `GridLayout.asi` — the button bar as a 5×3 grid of position keys (QOL-1), placed between the minimap and the info panel where there is room | `grid/README.md` |
| `lighting/` | `Lighting.asi` — ships and stations on the engine's own GPU vertex-buffer path, scene lights (warm key, dim blue fill, a faint sky light) in place of each map's own, planets with a night side, and light sources: nebulae, planets' day sides, explosions, torpedoes | `lighting/README.md` |
| `postfx/` | two layers: renderer (`dxvk.conf`) and bloom (vkBasalt) | `postfx/README.md` |
| `textures/` | the texture pipeline: `lib/`, `tools/`, and 84 `targets/` — recipes only | `textures/README.md` |
| `models/` | 3D geometry — the widened loading screen (`SOD`), `Planets.asi` (the planets' tessellation) and the smoothed dilithium moons (`moon-sod.py`) | `models/README.md` |
| `cutscenes/` | `binkproxy/`, a `binkw32.dll` that plays AV1 replacements full screen, and the `movies/` recipes | `cutscenes/binkproxy/README.md` |
| `platform/` | what `a2mod` never switches: DXVK, the ASI loader, Heroic/Proton; `vendor/` holds the third-party binaries we may redistribute | `platform/README.md` |
| `testbench/` | `./a2test`: the game headless at any resolution, scenarios, reports. Installs nothing | `testbench/README.md` |
| `online/` | `Online.asi` — online multiplayer, in progress: the *Internet – Online* menu entry, our own `IDirectPlay8Peer` over UDP (`peer.c`), and `server/a2online-server.py` for join codes, hole punching and the relay, public at `c20e.de:2399` (the default `Server=`; the bench blanks it). Not in `./install` or `a2mod` yet | `online/README.md` |
| `publish/` | what may be published and the check that enforces it. Installs nothing; versioned by the root | `publish/README.md` |

`./a2mod` is the entry point and stays at the root, with `./a2test` beside it for
testing, `./a2tex` for textures, `./install`, which installs every layer, and
`a2env.sh` / `a2env.py`, which say where the game and the assets are. `./a2tex` exports
`ROOT=textures/`, so everything under `textures/` addresses `$ROOT/lib`,
`$ROOT/targets` and `$ROOT/tools`.

**Never hard-code a path to the game, its prefix or Proton.** Source `a2env.sh` (shell)
or `import a2env` (Python, with the repository root on `sys.path`): they resolve
`A2_GAME`, `A2_PREFIX`, `A2_PROTON` and `A2_DATA` (where assets live, default
`~/.local/share/armada2-refit`; never a working tree) from the environment, then
`~/.config/armada2-refit.conf`, then the default location.

**To see a change working in the game without taking over the user's screen, use the
test bench**: `./a2test session start --res 21:9`, then `./a2test drive --session ID shot`
/ `click` / `key` / `step "…"`, and read the screenshots. It runs on a reflink clone of
the install and a headless display. **Keep the ID `session start` prints, pass
`--session ID` on every `drive`, and end with `./a2test session stop ID`** (the ID is
required). Other jobs run sessions on the same bench: stop only the sessions you
started, never one from `session list` you don't recognise (`testbench/README.md`,
"Which session a command acts on"). Write regression scenarios in `testbench/scenarios/` for fixes
worth keeping fixed. A bench pass is evidence for the user, not their sign-off
("Finishing work" still applies).
**To test a checkout as installed**, `./a2test session start --install <checkout>` (or
`a2test run ... --install ...`) starts the clone from stock and runs that checkout's
`./install` into it; the report records its commit. The builds come from `$A2_DATA`,
so every checkout and worktree installs the same ones. `A2_DATA=<empty dir>` shows the
game as someone without a texture pack gets it.

Required reading, by task:

- **Anything renderer-adjacent: `platform/README.md`** — DLL overrides, the widescreen
  patch, the d3d8 chain, window management under Hyprland, the save format.
- **Making textures or packs, as a user would: `textures/PACKS.md`**, the short path
  through the pipeline. Keep it true when the commands change.
- **Any texture work, or anything that touches a texture file** (the font atlases, the
  cursors): `textures/README.md`. It holds the engine reference — TGA format, mip
  chains, what counts as an interface sprite, the two nebula systems. Do not re-derive
  any of it; it was established by measurement and several wrong turns.
- **Any texture that is not a nebula: `textures/REMASTERING.md`** — over half the set
  has a real alpha channel that these scripts would destroy.
- **"What is left": `textures/INVENTORY.md`**, generated by
  `textures/tools/inventory.py --md` from the game directory and this repo. Do not
  hand-edit it and do not re-derive its numbers in prose; re-run the tool.

## Changelogs and versions

Every layer folder in the table above has a `CHANGELOG.md` with its own semver version.
The root `CHANGELOG.md` versions the project as a whole. **A commit that changes a layer
updates that layer's changelog and the root one in the same commit**, with the version
bumped. A changelog that is updated "later" gets forgotten.

**What gets an entry:** anything that changes what a layer installs into the game, how
it builds, or its interface (CLI flags, `target.conf`/`.ini`/`.conf` keys, backup
names). This includes tuning changes like a new blend, seed or threshold, and a revert
of shipped work. **No entry:** changes to docs and notes only, and scratch work that
installs nothing.

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
2. **Merge into `main`** from the main checkout, with
   `git merge --no-ff <branch> -m "Merge <branch>: <what>"`. Resolve version clashes as
   "Parallel worktrees" says. Don't push unless the user asks: a push to `main` that
   bumps the root version publishes a GitHub Release (`publish/README.md`).
3. **Clean up.** Remove the worktree (`git worktree remove`) and delete its branch
   (`git branch -d`). Then do the same for any other worktree whose branch
   `git branch --merged main` lists. Use only the safe forms, never `--force` or
   `-D`. If one refuses because of uncommitted changes or unmerged commits, leave it
   and tell the user.

Until the sign-off, the work stays on its branch and in its worktree.

## Environment

- Game: `$A2_GAME` (`./a2env.sh` prints it; here, Heroic's `~/Games/Heroic/Star Trek Armada II`) — GOG release, patch 1.1, run
  through Heroic with Proton-CachyOS.
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
  `menus/build.sh`. `objdump` also reads `pei-i386`, and the game ships
  `armada2.map` — a full symbol map — so `Armada2.exe` can be read rather than guessed at.
- Scratchpad for intermediates; this directory is the user's, keep it tidy.

## Hard rules

1. **Never write into `Textures/RGB/` directly.** `./a2tex build` writes to
   `$A2_DATA/textures/<NAME>/out/`; `./a2tex install` copies it across, making a `.a2neb-backup`
   on first touch. `./a2tex revert all` sweeps every `*.a2neb-backup` in `Textures/RGB` —
   the cursors included — and restores it.
   **The one sanctioned exception is `hud/ui-font-condense.py`**, which rewrites the
   twelve `FontFinal4_*` atlases in place because its edit is paired with the `.spr`
   metrics beside them in `Sprites/` and so cannot live in a target. It owns its own
   backups under `.a2font-backup` — a *different* suffix, precisely so `a2tex revert
   all` cannot restore a stock atlas under condensed metrics and garble every glyph.
   The UI configs in `misc/` also have `.a2neb-backup`s, but are reverted by
   `hud/ui-widescreen.py --revert`, not by `a2tex`.
   **The one file added rather than replaced is `a2flatbump.tga`**, from
   `models/hull-bump.py`: a flat height map no stock texture shares a name with, which
   the hull SODs it patches all name. It replaces nothing, so it has no backup;
   `--revert` deletes it and `a2mod` lists it as an added `models` file.
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
  before/after: textures, font, HUD layout, the menus (`Menus.asi`), MSAA, `QOL.asi`, `GridLayout.asi`, `Lighting.asi`, cutscenes, the loading-screen
  model, `dxvk.conf` and bloom at launch. DXVK, the ASI loader, the widescreen patch and
  the player's own options (`ARMADA.PRF`, `RTS_CFG.h`) stay as they are in both states. It **snapshots** rather than
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
  Plates are built by `menus/backdrop.sh` into `$A2_DATA/backdrops/<name>/` from the
  recipe `menus/backdrops/<name>.conf` (`menus/BACKDROPS.md` has `field=`, `clone=`,
  `keep=`, `fade=`, `dehaze=` and the paid seeds); `menus/install-plates.sh` (run by
  `./install`) copies each built `wide.bmp` to the game's `Menus/`, and
  `menus/install.sh` leaves them be. With no plate, `Menus.asi` draws black sides.
  `N.soften=` is a `Menus.ini` key, documented in `menus/README.md`.

### msaa, postfx, platform

- **The game renders through DXVK/Vulkan.** The chain keeps a `d3d8.dll` (crosire's
  d3d8to9 since platform 3.2.0, which `./install` puts there so plugins can reach
  Direct3D 9; DXVK's own d3d8 before) **and** DXVK's `d3d9.dll` in the **game directory** (the prefix is not durable — Proton restores it
  from symlinks), `d3d9=n,b` in the overrides, and the Wine virtual desktop **off**.
  The stock `syswow64/d3d8.dll` is *Wine's builtin*, and was once taken for DXVK's on
  the strength of its byte count. The virtual desktop also puts the menus in one window,
  but DXVK cannot set a display mode inside it; `Menus.asi`'s `Embed=1` does that job
  inside the game window. Never turn it on to fix a window problem — fix it in
  `menus.c`.
  `platform/d3d8-chain.py --status` identifies every link **by hash; never identify one
  by size.** "Heroic redeploys DXVK" is `autoInstallDxvk` working as designed.
- **Bloom** is vkBasalt, possible only because the chain is Vulkan. `postfx/postfx.py`
  refuses any effect `fxcheck` cannot compile.
- **MSAA is an ASI hook** because nothing in DXVK or `dxcfg.ini` can turn it on. The
  minimap is the one thing that could break (`msaa/README.md`).

### qol

- **Two plugins, by what other players need.** `QOL.asi` holds only changes that stay
  on this player's machine (camera, keys, which ordinary command a key sends): a player
  with it can play a player without it. Anything that changes what every node simulates
  (QOL-7's pay-on-enqueue) goes in a separate rules plugin that every player must run,
  checked when a network game is set up. Never put a rules change into `QOL.asi`. Which
  idea is which: `qol/README.md`.
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
- **Never edit `RTS_CFG.h` for a QOL change.** Network games compare a CRC of its bytes
  ("EXE / RTS_CFG.h files do not match node %d"), so an edited file locks the player
  out of games with stock players. Scale the value in memory after the parse, as
  `QOL.asi` does for `FASTSCROLL_COEFFICIENT` (right-drag only). `ARMADA.PRF` carries no
  such check.

### grid

- **A separate plugin on purpose**, so the grid can be switched on and off on its own:
  without `GridLayout.asi` the bar and all its keys are stock. It changes how orders are
  given, never what they are, so it is stock-compatible.
- **Never patch the bar's vtable.** `HUD.asi` recognises the bar (`PopupPaletteImp`) by
  the `PostLoad` in its vtable when it re-lays the HUD out; the grid wraps call sites
  inside `Update` and `ShipDisplay::PostLoad` and detours `Render`'s first instruction.
- **Don't touch the floating palette's width (+0x7c).** It is also the slot plan's width
  (`row * [+0x7c] + col`); changing it made buttons vanish from the bar.
- **Stock's bar keys are global per selection**, not per open menu (F5 builds from the
  top level). The grid empties the bar's hotkey tables around the stock handler and
  points the menu toggles at a zero; nothing outside the bar reads those controls.

### lighting

- **`Lighting.asi` is the engine's own GPU path, switched on.** Stock draws only asteroid
  fields through `ST3D_Standard_MeshVB`; the plugin enables it for every object type's
  model, as the asteroid field does. Don't write a renderer of our own before reading
  `lighting/README.md`.
- **On that path the SOD material lies.** Its diffuse colour (red, brick, purple
  leftovers) and ambient were never shown by the CPU path; the plugin replaces them. The
  engine's colour at +0x28 is not an ambient term (it turned every hull pink).
- **Planets already take the scene lights** (CPU path, `LightVertices_Lambert`). What
  flattened them is their material's constant half-white term, which `Planets=` replaces;
  judge a planet side on, since the Key comes from behind the usual camera.
- **A mesh under a mirrored node matrix lights inside out** on the GPU path (the Akira's
  distant mesh). `FixMirrored` reverses the lights for such a draw. Measuring normals
  against a mesh's centre does not find these meshes: most stock meshes read as
  "inward", and flipping them darkened the near hulls.
- **The map's `dlight` objects are the scene lights**, one to three per map; the plugin
  replaces them every frame. The axis convention is the engine's (`KeyAxis`).
- **Point lights never reach the vertex-buffer path on their own**: `PreRender` copies
  only directional lights. The plugin hands them over per draw (`pick_points`); any new
  light source is a point light in the engine's list, and nothing more. A light the
  CPU-lit planets must not take gets the plugin's no-op vtable, as the planet glow does.
- **A soft point light goes to Direct3D mirrored through the draw's origin**: stock
  normals point inward, and a light given where it is lights the far side of a hull.
- **A nebula lights only while it counts as on screen**, by its bounding sphere, which
  is smaller than its light's reach. `NebulaCull` widens the test, nothing else.
- **`Shaders=1` binds its shaders in a d3d8 `DrawIndexedPrimitive` hook, inside
  `VBRender` only**: the engine's `SetVertexShader(FVF)` becomes d3d9
  `SetVertexShader(NULL)` under d3d8to9, so anything bound earlier is lost. Every d3d9
  path must fall back when `d9_device()` is NULL (DXVK's d3d8). `platform/D3D9.md`.
- **Bump-mapped hulls (the Borg, `models/hull-bump.py`'s) go the dot3 way**, which
  takes precedence over the vertex buffers. `BumpShaders=1` wraps
  `ST3D_Dot3_MeshVB::Render` and draws them in the hull shaders under d3d8to9. Their
  normal is the SOD's vertex normal tilted by the map along S and T. Never use the
  engine's S x T as the normal: it is summed from UV slopes and drew dark streaks where
  UVs are mirrored.

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

The loading screen is a 3D model, not a sprite: six quads in `SOD/logo.SOD` carrying
`LOADING1..6`. The `LOADING` target (art, `models/loading-panel.sh`) and
`models/logo-sod.py` (quads, `panel=`) ship together — `a2tex install`/`revert` move
both; **never ship one without the other.**

**Federation hulls can be lit per pixel by the engine itself**: a SOD material spelled
the Borg way (type 6, a second texture with word `0x200`) goes through the dot3 path.
`models/hull-bump.py` does it with a *flat* height map; one derived from the hull art
was rejected in game. On the dot3 passes it is 18–21% darker, because they have no ambient term
(`models/README.md`, "Hull lighting").

**The planets' shape is not in their SOD.** `Planet_Database` builds its own sphere and
re-tessellates it per frame against a facet tolerance set for 640x480; a 5120-triangle
`PB_CLSS*.sod` drew pixel for pixel like stock. `Planets.asi` lowers that tolerance
(`Detail=`). Don't re-try new planet meshes: `models/README.md`.

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
