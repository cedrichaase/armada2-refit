# Plan: Star Trek: Armada on the refit stack

How Star Trek: Armada (1), the GOG release at patch 1.2, gets what Armada II has from
this project: a renderer of our own on Vulkan and Direct3D 12, the HUD and the menus
redrawn, quality of life, online play, the texture, movie and backdrop pipelines, a
stock/refit switch and a test bench. It is a plan, not a commitment of dates. The work
happens in a **new repository**, not in this one. It starts after `ROADMAP.md` phase 8
and after Fleet Operations: Roots (`PLAN-FLEETOPS.md`), both decided 2026-11-02.

The ground it stands on is a read-only survey of 2026-11-02 (scratch notes, not in the
repository): the GOG install, its executable's run-time type information compared with
Armada II's, and one launch on the bench mechanism.

## Decided

| Decision | Date | What it means here |
|---|---|---|
| Armada 1 after Fleet Operations: Roots and after ROADMAP phase 8 | 2026-11-02 | The symbol profile format, `a2engine` and the per-game bench exist before the first Armada 1 profile is written. Roots proves the (game, build) design on the same engine first |
| A new repository with this one's structure | 2026-11-02 | Layer folders, each with a README and a CHANGELOG; a stock/refit switch; a test bench; the same publish rules |
| Full parity with today's Armada II refit | 2026-11-02 | Every layer this repository ships has an Armada 1 counterpart, where Armada 1 has the feature at all (below, "Parity") |
| Vulkan and Direct3D 12 directly | 2026-11-02 | A "Display7" `ddraw.dll` on the `display8v1_*` contract. No Direct3D 9 path of ours, ever: no forward mode that ships, no `gpu` Direct3D 9 backend in the build |

## What Armada 1 is, measured

| | Armada 1 (GOG, 1.2) | Armada II 1.1 |
|---|---|---|
| Executable | `Armada.exe`, PE32, MSVC 6, version 1.2.0.0, linked 2001-02-02 | `Armada2.exe` |
| Symbols | **none**: no map, no PDB (`Gamekey.map`, `Input.map`, `label.map` are key maps) | `armada2.map`, `Armada2.pdb` |
| Graphics | **DirectDraw 7 and Direct3D 7 immediate mode**, loaded at run time (`DirectDrawCreateEx`, no ddraw import); a software device exists | Direct3D 8 |
| GOG's chain | GOG's own `ddraw.dll`, a DirectDraw-to-Direct3D 9 wrapper configured by `dxcfg.ini`; not redistributable | the stock `d3d8.dll` |
| Under Proton | `Armada.exe` → GOG `ddraw.dll` → `d3d9` → DXVK → Vulkan; the Bink intros played at 1280x720 on the bench mechanism. Menus, a mission and the 3D view were not reached in that run | — |
| Imports worth knowing | `binkw32.dll` (static), `DSOUND.dll`, `WINMM.dll`, `WonInterface.dll`; GOG ships `dpwsockx.dll` and DirectPlay registry files | `binkw32.dll`, `dpnet` |
| Engine kinship | **the same codebase**: 408 of its 460 RTTI classes (89%) exist in Armada II, 64 of 78 assert source paths match, 42% of distinctive strings are verbatim, the same Storm3D layer (`ST3D_DeviceDirectX7` against `ST3D_DeviceDirectX8`) | — |
| Models | SOD `Storm3D_SW` 1.6–1.8, inside `a2fmt`'s 1.4–1.93; no self-illumination or bump materials | up to 1.93 |
| Textures | `Textures/Index8` (1078 files, 8-bit paletted, `Armada.pal`), `Textures/RGB` (994, 24- and 32-bit), `Compressed` | `Textures/RGB` only |
| Game data | ODFs packed in `.zfs` archives (`ZFSF` version 1) | loose ODFs |
| Front end | Win32 dialogs and GDI on **640x480** BMPs | 800x600 |
| HUD | `misc/gui_*.cfg` without a canvas declaration: a fixed pixel layout; a bridge UI (`BridgeDisplay`, `CrewDisplay`, `OfficerDisplay`, `DilithiumBar`) Armada II does not have | a declared 1600x1200 canvas |
| Multiplayer | DirectPlay up to version 7, and WON | DirectPlay 8 (`dpnet`) |
| Movies | 69 Bink files, tiny (280x190, 200x200) | larger |
| Heroic | GOG id 1582325353, Proton-CachyOS, DXVK auto-install on; the prefix `~/Games/Heroic/Prefixes/Star Trek Armada` exists since 2026-11-02 | — |

What counts for the plan: the *knowledge* and the *infrastructure* carry over; very
little of the *hook code* does. About half of Armada II's hooks are re-findable by
RTTI and vtables, about a quarter by strings and call patterns, and the rest have no
counterpart (the planets' database, the button bar, the Bink shell backdrops).

## The shared code: one core repository, a submodule in both

The two repositories share every unit that holds **no engine knowledge**: `gpu`,
`crates/a2rt`, `crates/a2ini`, `crates/a2fmt`, the resolver half of `crates/a2engine`,
`csdk/a2asi.h` and its `.def` files, `tools/shaderc`, `tools/settings`,
`tools/ini-merge.py`, and the game-independent core of the test bench. After ROADMAP
phase 7 that also covers the drawing half of each render3d module and the UI's runtime
(2D renderer, text, widgets, theme, input capture), whose engine reads phase 7 and
phase 8 move into per-module `hooks.rs` files and the profile.

**Choice: a third public repository, `refit-core` (name open), with the shared units,
included in both game repositories as a git submodule at `core/`.**

| Option | For | Against |
|---|---|---|
| **Submodule of a core repository** (chosen) | One copy. Each game repository pins an exact core commit, so a core change reaches a game only when that game's repository moves the pointer, in a commit with its own changelog entry. The core has its own `CHANGELOG.md` and semver, which makes the contract versions (`display8v1`, `A2SET_ABI`) visible in one place. No engine finding can leak into it unnoticed, because it has no profile and no hook | Agents must run `git submodule update --init` in every new worktree; a core change is two commits in two repositories; CI checks out with submodules |
| Subtree in each repository | Plain directories; no extra step for a worktree | Two copies that drift: a fix made in Armada 1's copy has to be split and pushed back by hand. That is the problem the architecture review found with `rt/` copied three times |
| Vendored copy with a sync check | Simplest checkout | The same drift, caught only after the fact by CI |
| Armada 1 includes this whole repository | No extraction | Couples Armada 1 to Armada II's release cadence; pulls Armada II's engine findings and docs into Armada 1's checkout, where agents would read them as if they applied |

Rules that make it work:

- **Engine knowledge never enters the core.** Profiles, hooks, recipes and findings stay
  in the game repository whose plugin uses them (hard rule 9 in both). `publish/check.sh`
  in the core refuses a profile file and any address table.
- **The core is publishable on its own**: the same `publish/check.sh`, MIT, its own CI
  (the host tests of `a2fmt` and `a2ini`, `tools/shaderc/verify-generated.sh`, the
  settings selftest, `gpu`'s SPIR-V selftest).
- **A game repository's CI** checks out the submodule, builds against it, and fails if
  the pinned commit is not on the core's `main`, so nothing ships against an unmerged
  core branch.
- **Local development** uses the core clone beside the game repositories; nothing is
  pushed unless the user asks, as today. A core change made for one game is merged in
  the core first, then each game moves its pointer when it has run its own regression
  set.
- **Contract changes** (`gpu/src/display.rs`, the settings ABI) bump the core's MAJOR or
  MINOR and land in both games' pointers in the same round, because Display8 and
  Display7 and the plugins of each game must agree.

The extraction itself is work in **this** repository (work package C0 below): paths move
from `gpu/` to `core/gpu/` and so on, and every script, build and doc that names them
follows. It is done at a quiet moment, when few worktrees are open, after phase 8.

## The new repository

Name open (`armada1-refit` would match). MIT, as this one. Its layout mirrors this
repository's, by `a1mod` layer:

| Folder | What | Counterpart here |
|---|---|---|
| `core/` | the submodule | — |
| `display/` | `ddraw.dll`, "Display7" | `display/` (Display8) |
| `render3d/` | `Render3D.asi` for Armada 1: the core's modules with Armada 1's `hooks.rs`, Armada 1's recipes | `render3d/` |
| `ui/` | `UI.asi`: the core's runtime with Armada 1's engine hooks, HUD panels (the bridge UI among them) and menu specs | `ui/` |
| `qol/` | `QOL.asi` and the rules plugin | `qol/` |
| `online/` | `Online.asi`, our own peer in place of DirectPlay 4 | `online/` |
| `textures/`, `models/`, `cutscenes/`, `menus/` (backdrops only) | the asset pipelines, recipes only | the same |
| `platform/` | the Heroic and Proton notes, the ASI loader, what `a1mod` never switches | `platform/` |
| `testbench/` | Armada 1's game profile, scenarios, staging recipes; the bench itself is the core's | `testbench/` |
| `publish/` | the same rules, the same check | `publish/` |
| root | `a1env.sh`/`a1env.py` (`A1_GAME`, `A1_PREFIX`, `A1_PROTON`, `A1_DATA`, default `~/.local/share/armada1-refit`), `./a1mod`, `./a1test`, `./a1tex`, `./install`, `CLAUDE.md`, `ROADMAP.md`, root `CHANGELOG.md` | the same |

There is no `renderer/` (no DXVK) and no `hud/`, `grid/` or legacy `menus/` plugin:
Armada 1 starts where ROADMAP phase 11 ends. Core tools take paths as arguments and
never read `A2_*` or `A1_*` themselves; each repository's env script supplies them.

## Symbol recovery without a map

Armada 1 has no symbol map, so every symbol a hook uses is recovered, and written into
an Armada 1 profile in phase 8's format with its key extended from build to
**(game, build)**: `game = armada1`, `build = gog-1.2`, identified by PE timestamp and
image size, with the hash recorded by the bench. `PLAN-FLEETOPS.md` already adds the
`game` field, so the format does not change again here.

The tooling, in the Armada 1 repository's `tools/` (it reads both games' executables
from the player's installs and writes only into scratch):

1. **RTTI walk.** Type descriptors, complete object locators and vtables for the 460
   classes; each vtable slot becomes a candidate function, named by class and slot.
2. **Anchors.** Assert source paths and distinctive strings, with their referencing
   functions; the 42% of strings that are verbatim in Armada II name many functions
   directly.
3. **Matching against Armada II.** For each Armada II function a hook needs, by name from
   `armada2.map` on the developer's own install, the matcher looks for the Armada 1
   function with the same vtable slot, the same anchors, the same callees and similar
   size and shape, and reports candidates with a score. It needs Armada II installed on
   the developer's machine, never the player's: players get the finished profile.
4. **Confirmation on the bench.** A candidate becomes a profile entry when a hook built
   on it behaves as expected in game: a log line at the expected moment, a check
   signature that holds, a value that changes as the game state does.

The profile entries name the Armada II symbol they correspond to where there is one
(names are allowed). The findings are written per hook in the Armada 1 layer READMEs,
under hard rule 9; a function found on the way that no hook uses is not written down.
The hard part is the non-virtual functions: the render seam (`Armada_RenderAllOurStuff`'s
counterpart) and the `PreRenderAll` family are found from their callers and the
`ST3D_*` device calls, not from vtables.

## Display7

A `ddraw.dll` of our own in front of nothing: it implements `IDirectDraw7`,
`IDirectDrawSurface7`, `IDirectDrawPalette`, `IDirectDrawClipper`, `IDirect3D7`,
`IDirect3DDevice7` and `IDirect3DVertexBuffer7` (and the older interfaces the game or
Bink asks for through `QueryInterface`, which the tracer shows), every slot with its
exact `__stdcall` argument list from Wine's headers. It owns the device on Vulkan or
Direct3D 12 through `gpu` and hands it to the plugins through the same
`display8v1_*` C exports as Display8, so render3d and the UI of Armada 1 build against
the same contract. The exports keep their name; the contract is the device's, not
Direct3D 8's.

Its core is **passthrough from the start**, since there is no forward mode to fall back
to: every Direct3D 7 draw is drawn on Vulkan or Direct3D 12 the way the game's own
renderer would draw it, through a fixed-function translation shared with Display8. That
sharing is a work package of its own: after ROADMAP phases 1 and 10, Display8's pipeline
cache, texture mirrors, fixed-function shaders and lit path move into the core as one
crate both front ends call (D3D7's fixed function is a subset of D3D8's, without vertex
shaders).

What Display7 has that Display8 does not:

- **Surfaces**: the primary and back buffer, offscreen plain surfaces, `Blt` and
  `BltFast` with colour keys, `Lock` on the back buffer (movies, 2D), and `GetDC` on a
  surface for GDI text.
- **Palettes and 8-bit textures**: a paletted texture is mirrored as RGBA at upload, with
  the palette applied and the colour key turned into alpha.
- **Device enumeration**: one hardware device and one display mode, the desktop's, as
  Display8's stub offers; the software device is never offered.
- **A bail-out**, as phase 11 gives Display8: when neither Vulkan nor Direct3D 12
  starts, Display7 loads the player's own GOG `ddraw.dll` and hands its interface back
  unwrapped, and the plugins stand down. That is the player's file, never ours.

On the bench the tracer runs first, in front of GOG's wrapper in a clone only, to record
what Armada 1 calls and to record the stock baselines that passthrough is A/B-compared
against. That is the reference look, as stock under DXVK is for Armada II; it is not a
path we ship.

**Loading the plugins.** The Ultimate ASI Loader can stand in for `WINMM.dll` or
`DSOUND.dll`, both of which Armada 1 imports; which one is settled in A1-0. Display7
loads no plugins itself.

## Parity

What "full parity" means, layer by layer, where Armada 1 has the feature:

| Layer here | Armada 1 counterpart | Note |
|---|---|---|
| display (Display8) | Display7 | above |
| render3d: hulls, sky, particles, ordnance, effects, world (moons, asteroids, nebulae), fog and selection, MSAA, bloom, tone map, shadows | the same modules on Armada 1's seam | The planets are not `Planet_Database` in Armada 1; whether they are SOD models the hulls module can take is measured. The Borg bump profile has nothing to read (no bump materials). Self-illumination comes from the 32-bit `RGB` set where it has alpha |
| ui: HUD panels, menus hosted and drawn, loading screen, Refit screen, settings | the same, written against Armada 1's classes | The bridge UI is Armada 1's own and gets panels of its own. The menu host works as Armada II's: the stock 640x480 dialog runs hidden and a screen spec acts through its controls |
| qol: pan speed, Shift+number, big selections, station groups, long moves at warp | each where Armada 1 has the mechanic | The selection cap and its 16-entry assumptions are audited afresh, as `QOL.asi`'s 1.1.0 crash taught |
| qol rules (`PayOnQueue`) | the same, must match in a network game | Armada 1's network check is surveyed first |
| online | our own peer in place of `IDirectPlay4`, the same server protocol and the *Internet – Online* entry | First measure whether Wine's DirectPlay 4 works for LAN at all; WON is gone |
| textures (`a2tex`) | `a1tex` with the core's library; a new **Index8** class | Whether the engine reads `Textures/RGB` in place of `Index8` when both exist is measured before any paletted output is written. If not, an upscale must be written back as 8-bit against `Armada.pal`, which loses most of what it gains |
| models | SOD rewrites (smoothed moons and asteroids where they exist) | `a2fmt` reads the versions |
| cutscenes (binkproxy, movies) | the same proxy (static import), AV1 replacements | The sources are tiny: an 8x lift is where generative upscalers invent the most (rule 4 of the six) |
| menus backdrops | 640x480 plates | The same per-frame composite |
| testbench | the core's bench with Armada 1's profile | scenarios, staging recipes and the menu graph are Armada 1's own |
| `a2mod` | `a1mod` | the same snapshot design |
| GUI installer | the core's installer with a game choice, or a second build of it | open below |

Not carried: `HUD.asi`, `GridLayout.asi`, `Menus.asi` and their scripts (they are being
removed here in phase 11), DXVK and `dxvk.conf`, the widescreen patch (Display7 owns the
mode), the button-bar grid (Armada 1 has no `PopupPaletteImp`).

## Work packages

Effort in ROADMAP units: agent hours, bench passes, sign-off round. Numbers are rough;
the survey reached the intros only.

| WP | What | Where | Depends on | Agent time · bench · round |
|---|---|---|---|---|
| A1-0 | **Platform.** Launch from Heroic: menus, a skirmish and the 3D view under GOG's wrapper. Which ASI loader name works. `ARMADA.PRF` lines 3 and 6. Whether Wine's DirectPlay 4 hosts a LAN game | notes, then `platform/` | nothing | 2–4 h · 2 · one user check |
| C0 | **Core extraction**: `refit-core` with history (a filter of the shared paths), the submodule at `core/` here, every path, build, CI job and doc that names them | this repository, then the core | ROADMAP 8; FO-1 (the per-game bench) | 8–16 h · 1 full regression pass · none (nothing visible changes) |
| C1 | **Shared fixed-function translation**: Display8's passthrough pipeline as a core crate both front ends call | the core, Display8 here | ROADMAP 1 and 10 signed off; C0 | 6–12 h · pixel-identical A/B on Armada II, 2–3 · none |
| A1-1 | **Bootstrap**: the repository, `CLAUDE.md`, `publish/`, CI, `a1env`, `./install`, an `a1mod` with no layers, licence | new repository | C0 | 4–8 h · none · none |
| A1-2 | **Bench profile**: executable, arguments, overrides, `ARMADA.PRF` adapter, prefix bootstrap, logs (`Logs/logfile.txt`), the menu graph found visually | new repository on the core's bench | A1-1, FO-1 | 10–20 h · 5–10 · none |
| A1-3 | **Identity and recovery tooling**: the RTTI walk, the anchors, the matcher; the empty Armada 1 profile; `a1test syms` | new repository | ROADMAP 8, A1-2 | 20–40 h · none (offline) · none |
| A1-4 | **Display7 tracer and inventory**, in front of GOG's wrapper in a clone | new repository | A1-2 | 10–15 h · 5–10 · none |
| A1-5 | **Display7 on Vulkan, then Direct3D 12**, passthrough of everything the inventory shows, A/B against the stock baselines; the bail-out; Display7 in `./install` and `a1mod` | new repository | A1-4, C1 | 40–80 h · 20–40 with A/B · round 1: the game as stock, on our device |
| A1-6 | **First visible win**: render3d's seam on Armada 1 with the scene target, MSAA, bloom and tone map over passthrough; the desktop's resolution | new repository | A1-3, A1-5 | 20–40 h · 10–15 · round 2 |
| A1-7 | **render3d modules**, one at a time, each switchable, each falling back to passthrough; Armada 1 recipes (skies, surfaces, nebulae) fitted to stock by measurement | new repository | A1-6, ROADMAP 7 | 100–200 h · about 10 per module · a round per two or three modules |
| A1-8 | **UI**: the runtime's hooks into Armada 1's mission frame, front end and loading screen; HUD panels including the bridge; the menu host and screen specs for the 640x480 shell; the Refit screen and settings | new repository | A1-3, A1-5 | 60–120 h · 20–30 · two or three rounds |
| A1-9 | **QOL and rules** | new repository | A1-3 | 20–40 h · 5–10 gameplay passes · one round |
| A1-10 | **Online** | new repository | A1-0's DirectPlay check, A1-3 | 30–60 h · 5–10 two-game passes · one round |
| A1-11 | **Formats and assets**: the `.zfs` reader in `a2fmt` (core), paletted TGA reading, the content profile; `a1tex` with the Index8 class; binkproxy and the movie recipes; backdrop plates; SOD rewrites | core and new repository | A1-1; the RGB-versus-Index8 measurement first | 30–60 h of tooling · 5–10 · content rounds as targets are built |
| A1-12 | **Release**: the package, the GUI installer's game choice, the first public release | new repository, core | A1-5 at least; parity for a 1.0 | 10–20 h · 2–3 · a release round |

Summed: about 370–740 agent hours, against about 55–115 for the whole ROADMAP. As there,
the wall clock is set by the user's sign-off rounds and by the bench, which the two
repositories share (one GPU, the same slot limit).

## Dependencies

```
ROADMAP 8, FO-1 (per-game bench) ── C0 core ── A1-1 bootstrap ── A1-2 bench ──┬── A1-3 recovery
                                                                              └── A1-4 tracer
ROADMAP 1 and 10 signed off, C0 ── C1 shared fixed function
A1-4, C1 ── A1-5 Display7
A1-3, A1-5 ── A1-6 first win ── A1-7 render3d modules (and ROADMAP 7)
A1-3, A1-5 ── A1-8 UI;  A1-3 ── A1-9 QOL;  A1-0, A1-3 ── A1-10 online
A1-1 ── A1-11 formats and assets;  A1-5 and parity ── A1-12 release
Fleet Operations: Roots signed off ── Armada 1 starts
```

ROADMAP phase 11 is not a dependency: Armada 1 never builds the Direct3D 9 backend, so it
can start before phase 11 with `gpu`'s Direct3D 9 backend left out of its build. Phase 11
then removes it from the core for both.

## Risks

1. **No symbol map.** Everything Armada II work got from `armada2.map` must be recovered.
   The matcher ranks; the bench confirms. Budget for the non-virtual render seam to take
   longer than the rest of A1-3.
2. **Direct3D 7 is broader than what Armada II's Direct3D 8 used**: surfaces, blits,
   colour keys, palettes, GDI on surfaces. The tracer bounds it, as the stock traces
   bounded Display8's passthrough. A call outside the recorded set is logged once.
3. **Paletted textures** may halve what the texture pipeline can give, if the engine
   reads only `Index8` in the mode the player runs.
4. **The front end at 640x480** has less room for the menu host's redraw to look right
   than Armada II's 800x600, and the bridge UI is new design work, not a port.
5. **Two repositories, one bench, one user.** Rounds for Armada 1 compete with rounds for
   Armada II; the plan does not run them in parallel beyond what the bench allows.
6. **Core churn.** While ROADMAP phases 9 to 11 still change `gpu` and the contract,
   every core change has to be taken into both games. C0 after phase 8, not before, keeps
   most of that churn inside one repository.

## Open decisions

1. **Names**: the new repository (`armada1-refit`?) and the core (`refit-core`?).
2. **Licence**: MIT for both, as this repository — to confirm.
3. **The GUI installer**: one installer that offers both games (and Fleet Operations), or
   one build per game from the same source.
4. **Art direction for Armada 1**: whether it follows `ART-DIRECTION.md` as written for
   Armada II, or gets a direction of its own (its stock look is older and flatter).
