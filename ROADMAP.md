# Roadmap: Display8 on Vulkan and Direct3D 12, modular plugins, feature toggles

Where the project goes from 13.36.1, and in which order. It is the plan for the display
work, and governs it: it carries the open items of the earlier display plan, whose
phases 0 to 6 are done. The layer READMEs stay the reference for how each part works,
and the changelogs for what has shipped. Update this file as phases land: tick the
phase, link the merge, move what was learned into the layer's README.

**Order, decided 2026-10-29:** passthrough comes first and is signed off in game before
anything else is built on it. The architecture phases (shared libraries, ABI and claims,
settings, the Refit screen, the render3d split, symbol profiles) wait for that sign-off.
The legacy plugins stay in the repository, and the release package keeps its fallback,
until the end, beside the removal of Direct3D 9.

## Where we are

Main is at `dbb106c`, 13.36.1. Phases 0 to 6 of the earlier display plan are done there:
the tracer and inventory, the `gpu` crate, Display8's forward mode, the stub, the Vulkan
backend and the Direct3D 12 backend, and `Backend=auto` with `Mode=stub` as the shipped
default.

- **Display8** (`display/`) is the game's `d3d8.dll`, in the release package since
  13.35.0. In stub mode (`Backend=auto`) it owns the device on Vulkan, then Direct3D 12,
  then Direct3D 9, and **drops every draw the engine makes**. render3d and the UI draw
  the whole frame instead. On macOS the package ships the same `Display8.ini`
  (`Backend=auto`, Vulkan on MoltenVK first) since 13.36.1.
- **Forward mode** passes every call to the old chain: d3d8to9 as `d3d8_real.dll`, then
  DXVK's `d3d9.dll`. The release package still ships that chain and the legacy plugins
  (`HUD.asi`, `GridLayout.asi`, `Menus.asi`) as its fallback, and Display8 stands down
  from the stub while those plugins are loaded. Both stay until Phase 11.
- **render3d** and **UI** draw through the `gpu` crate, which has Direct3D 9, Vulkan and
  Direct3D 12 backends. Shaders build to d3dbc (vkd3d), SPIR-V and DXIL (dxc).
- **QOL**, **QOLRules**, **Online** and **binkproxy** are C plugins with their own ini
  files, read once at load.
- **In progress:** passthrough (Phase 1); CI that rebuilds every shader and fails on a
  stale stamp, and the plugins' move from polling `d3d8_device()` to
  `display8_device_epoch` (both in Phase 2).

What the architecture review found (four read-only reviews, 2026-10-23):

| Problem | Effect |
|---|---|
| Stub mode drops the engine's draws but never checks that something draws instead | Without `Render3D.asi`, the 3D view is black. A module or UI part switched off draws nothing, not the stock look, under the shipped default. "A key at 0 gives the part back to the engine" is false today. Passthrough (Phase 1) is the answer |
| No version handshake between `d3d8.dll` and the plugins | `d3d8.dll`, `Render3D.asi` and `UI.asi` must ship in lock-step |
| Shared code is copied: `rt/` three times in Rust, string, log, patch and signature helpers in every C plugin, the shader toolchain inside `render3d/`, two ini-merge tools | A modder taking `gpu` alone drags in render3d; fixes land in one copy |
| Concerns sit in the wrong plugin: the grid bar keys (gameplay) and `KeepClip` (platform) in the UI; the draw distance (visual) in QOL; the rules plugin installed by the stock-compatible QOL layer | Nobody can take "visuals only" or "stock-compatible only" |
| Plugins find each other by DLL name, and the UI tells render3d about selection bubbles by patching an engine byte | Renaming a file silently breaks a pairing |
| render3d's `hulls` module is a hub; generic helpers live in `effects`, `ordnance` and `particles`; nebula and beam behaviour is built into the light clustering; engine addresses are spread over ~33 files of content | Modules cannot be taken separately; the light clustering cannot be reused |
| No list of settings anywhere; defaults disagree between code and shipped inis (Display8's); the QOL and Online installers overwrite the player's file | A settings screen has nothing to enumerate |
| No check of which executable is running; data addresses and struct offsets are never checked | On a modified executable a hook can pass its call-site check and read the wrong fields: a crash, not a stand-down |

## End state

1. **Display8 is our own Direct3D 8**, on Vulkan and Direct3D 12 only. Every engine draw
   that render3d or the UI does not claim is drawn by Display8 the way the stock engine
   would draw it (**passthrough**). Switching a feature off, or leaving a plugin out,
   gives the stock look on every backend.
2. **Nothing superseded remains**: no legacy HUD, Grid or Menus plugins, no forward mode,
   no Direct3D 9 backend in Display8 or `gpu`, no d3d8to9 or DXVK in the chain, no
   `dxvk.conf`, no d3dbc build. Old installs are still cleaned up by retire lists.
3. **Modular by contract**: renderer core, visual content and gameplay are separate
   units with explicit interfaces, so a modder can take Display8 alone, `gpu` for a
   plugin of their own, render3d with some modules, or QOL alone.
4. **Players switch whole features** from the game's own Options menu. Tuning stays in
   the ini files.
5. **Engine knowledge is data per executable build**, so another build (Fleet
   Operations, later) is a new profile, not a rewrite.

## Contracts

Five interfaces carry the decoupling. Each is one file both sides build against, and
changes on both sides together.

### Claims

Each plugin exports `<Plugin>_Claims()`: the draw categories it draws now (sky, hulls,
world, shields, sprites and ordnance, fog of war, HUD 2D, loading, credits, cursor).
Display8 asks it every frame. A category nobody claims is drawn by passthrough; until the
claims exist (Phase 4), Display8 routes by its own category switches. This replaces
every DLL-name probe and the engine-byte signal between UI and render3d.

### ABI handshake

`display8_abi()` returns a major and minor version. Every `#[repr(C)]` struct in
`gpu/src/display.rs` starts with its size. A major mismatch makes the plugin stand down
and log both versions. Minor versions only append fields, and `gpu` gates new fields on
the minor it was handed. `gpu` gets a real version and a changelog section in its
consumers' entries.

### Settings descriptors and the player overlay

Each plugin exports `<Plugin>_Settings(&count)`: a static table of descriptors with
file, section, key, label, help, kind (bool or enum), default, apply class (live,
next mission, restart) and scope (local, or must match in a network game). An optional
`<Plugin>_SettingsChanged()` applies live rows.

Player choices go in **one overlay file that installers never write**, generalised from
`UIPlayer.ini` (whose `BarHidden` migrates into it). A key is read overlay first, then
the plugin's ini, then the default. Only values that differ from the default are
stored, and setting a row back to its default deletes the key, so an overlay never pins
an old default against an installer's upgrade. The overlay's writer is our own and
keeps the file's lines; we don't depend on Wine's `WritePrivateProfileString`.

The menu rule extends: *a menu of ours never writes a game setting (`ARMADA.PRF`,
`RTS_CFG.h`); those go through the hidden stock controls. Settings of ours are written
only to the overlay, never to a shipped ini.*

### Engine symbol profiles

One data file per executable build names every symbol a plugin uses: functions, call
sites, vtable slots, globals and field offsets, each with its address or offset, an
optional short check signature, an optional scan pattern, and the hook that uses it
(`used_by`). A generator writes a C header and a Rust module from it. At start-up a
plugin identifies the executable from its PE header (timestamp and image size; the
bench also records its hash), picks the profile, resolves each symbol, and leaves a
missing one at zero. A hook whose symbols are not all resolved stands down and logs
which one is missing. An unknown build runs only hooks that resolved by scan and read no
offsets.

`publish/check.sh` lints the profiles: every entry names its hook, signatures stay
short. That keeps hard rule 9 checkable: addresses and short signatures are allowed,
listings are not, and every finding belongs to a hook.

A **content profile** does the same for game data: ODF and model names, the race lists,
the sky, surface and ordnance recipes. Stock is the only profile for now; a missing
recipe means stock behaviour, as a missing asset does today.

### Shared libraries

| Unit | What | Used by |
|---|---|---|
| `crates/a2rt` | the `no_std` runtime: memory, Win32 declarations, `Ini` with the overlay lookup, logging, `sfmt`, hook helpers, dumps, the clock | display, render3d, ui |
| `crates/a2fmt` | the SOD and texture readers, moved out of `render3d/` | render3d, ui |
| `crates/a2engine` | the symbol resolver and the generated tables | display, render3d, ui |
| `gpu` | as now, minus Direct3D 9 at the end | display, render3d, ui, a modder's plugin |
| `tools/shaderc` | `hlsl-rs.sh`, `dxil-spaces.py`, `fetch-dxc.sh` | every crate with shaders |
| `csdk/a2asi.h` | header-only C helpers: strings, log lines, patching, signature checks, the overlay lookup, one set of `.def` files | qol, rules, online, binkproxy |
| `tools/ini-merge.py` | the one ini merge, used by every installer | every installer |

## Pacing

**Writing the code is not what takes the time.** The display work landed in one day.
Author dates from `git log --format='%ad %s' --date=iso -- display gpu`, all on
2026-10-23. An agent commits a piece when it is finished, so a date marks when the
piece landed, not when it was begun:

| Landed | What | Versions |
|---|---|---|
| 00:31–01:03 | Shaders in modern HLSL, one source for d3dbc and SPIR-V; `gpu`, one graphics interface for render3d and the UI, on Direct3D 9 | root 13.3.0–13.4.0 |
| 00:40 | Display8 in forward mode, the tracer and the inventory | display 0.1.0 |
| 01:16 | Display8 in `./install` and `a2mod`, drop switches by category | display 0.2.0 |
| 01:26–02:03 | Every render3d module drawn through `gpu` | — |
| 01:45 | The stub on Display8's own Direct3D 9 device; the `port-*` and `display-stub` scenarios | display 0.3.0 |
| 02:02–02:47 | The stub on Vulkan; `gpu`'s Vulkan backend, the UI's first light on lavapipe | — |
| 08:33 | The stub on Direct3D 12, every shader also built as signed DXIL | display 0.4.0 |
| 09:27 | render3d and the UI on Vulkan | display 0.5.0 |
| 11:22 | render3d and the UI on Direct3D 12 | display 0.6.0 |
| 12:46 | The stub on Vulkan confirmed in game | — |
| 13:08 | `Backend=auto` with a fallback; the stub on `auto` is the shipped default | display 0.8.0, root 13.23.0 |
| 19:35 | The in-game confirmation recorded for the whole branch | — |

From the first Display8 commit to the shipped default took about twelve and a half
hours, with a six-hour gap overnight. Over the same day the root version went from
13.3.0 to 13.34.0. These are the real bottlenecks:

1. **The user's in-game sign-off rounds.** Each one waits for the user to play.
   Rounds are the critical path, so the phases below are grouped to need as few as
   possible.
2. **Bench capacity.** There is one GPU and at most three bench games at once, shared by
   every agent. A start takes about 80 s and a scenario a few minutes, so the bench runs
   a few dozen games an hour at best. A `Timing=1` run needs the bench to itself. Vulkan
   on the GPU (`A2TEST_VULKAN_GPU=1`) runs at 1280x720, and a session is stopped as soon
   as it has done its job.
3. **Pixel A/B iterations.** Each one is a change, a rebuild and a `baseline compare`.
   Wherever pixel exactness decides a phase (passthrough above all), the number of
   iterations sets the pace, not the code.
4. **The calendar.** Access to a Windows machine and a Mac (Phase 10), and two releases
   with `auto` as the default before Direct3D 9 and the legacy plugins go (Phase 11).

Each phase's **Estimate** is in three parts: *agent time* (hours of agent work, which
parallel worktrees overlap), *bench passes* (runs of the phase's acceptance set, plus
A/B iterations where pixels decide), and the *sign-off round* that confirms it in game.

### Sign-off rounds

| Round | Phases | What the user checks in game |
|---|---|---|
| A | 1 passthrough on Vulkan, 2 open items | A render3d module and a UI part switched off each show the stock look on Vulkan, and so does the game with Display8 alone. The architecture phases wait for this round |
| B | 3 shared libraries, 4 ABI and claims | Nothing has changed: the HUD, the bar, the menus and a mission on the stub. One plugin left out shows its stock look through passthrough |
| C | 5 settings, 6 Refit screen | The Refit screen: switch rows in the shell and in a mission, relaunch, and the choices are kept. A hand edit in an ini still works |
| D | 7 render3d split, 8 symbol tables | Nothing has changed (both are pixel-identical refactors); one mission played through. Folds into E if both are ready by then |
| E | 9 owners and package, 10 Direct3D 12 on Linux | The pointer and the bar keys on the user's own desktop (Wine and Xwayland input), the release package with Display8 owning the window, `Backend=d3d12` |
| F | 11 remove Direct3D 9 and the legacy plugins, 12 docs | The game with no Direct3D 9 and no legacy plugin anywhere: the front end, a mission, the bail-out |

That makes five or six rounds. The native Windows and Mac runs (Phase 10) happen
outside the rounds, on whatever machines are available, and the two releases with
`auto` as the default come before round F. Agents work in parallel worktrees, and
whichever merges second renumbers its versions. Between rounds the bench is what
limits how many phases can be in flight at once.

## Phases

Each phase is one or more merges into `main`, each with its own root version and
"Finishing work" in CLAUDE.md: confirmed in game and signed off by the user, then
merged. A bench pass is evidence, not sign-off. A phase whose round has not come yet
stays on its branch. Phase 1 comes first and gates Phases 3 to 8. Estimates follow "Pacing" above.

### Phase 1 — Passthrough on Vulkan (in progress)

**Goal:** Display8 draws every unclaimed engine draw on Vulkan the way stock does. It is the chosen answer for render3d's hand-backs (hologram classes, effect classes, construction, the bees) and for every module or UI part switched off: the stock look, not nothing drawn and logged.

What the engine uses, from the stock traces (2026-10-24): 58 methods; four vertex
formats, nearly all pre-transformed and pre-lit (the bulk is position, colour and one
texture coordinate); one programmable vertex shader (the Borg bump); triangle fans from
one vertex buffer, indexed lists with 16-bit indices, and line strips; blending always
on (additive, alpha, opaque, modulate); depth test with write on or off; a rare alpha
test; cull none or clockwise; stage 0 as modulate, select-texture or dot3, stage 1 only
for the dot3 passes; specular add on the CPU-lit hulls; fixed-function lighting only
for asteroid fields; **fog never enabled**; textures only in 32-bit ARGB and XRGB, no
compressed or paletted formats; locks on the back buffer (movies), the scan-grid
textures and the vertex buffers; one gamma ramp. Never seen: `Reset`, `CopyRects`,
`UpdateTexture`, pixel shaders, stencil.

With the full refit on the stub the engine draws nothing outside the loading screens,
so routing is simple: **draw every engine draw**, and keep one drop for what the UI's
loading screen overdraws.

| Step | What | Agent time |
|---|---|---|
| 1a | Dispatch in place of the counting macro; a mirror for every engine texture, made at first draw; vertex and index upload per draw into the frame ring; a pipeline cache keyed by blend, depth, cull, topology, vertex layout, alpha test and the target's formats and samples; fans to lists; consecutive draws with identical state merged, never reordered | 4–6 h |
| 1b | Fixed-function shaders through `compat.hlsli`: the pre-transformed path with D3D9 pixel centres, and a pixel shader driven by a small constant block for the recorded stage ops, texture factor, specular add and alpha test. An op outside the recorded set is logged once | 2–4 h |
| 1c | The fixed-function lit path for asteroid fields: directional and point lights, material, ambient | 1–2 h |
| 1d | The Borg vertex shader, ported by hand and matched by a hash of its tokens; an unknown shader is refused and logged | 1–2 h |
| 1e | Drawing into Display8's own engine colour and depth images, the UI on top (no render3d) | 1 h |
| 1f | Drawing into render3d's scene pass when a module is off: a contract change in `gpu/src/display.rs` publishes the pass now (images, formats, samples, viewport) and keeps render3d's targets in attachment layouts between recorders; Display8 opens a compatible pass on them, so the stock draw is tone-mapped and bloomed with the rest | 3–6 h |
| 1g | Routing: whatever render3d and the UI do not draw, because the module or part is off or the plugin absent, the engine's draw goes through. Until the claims contract exists (Phase 4) the routing is Display8's own category switches, set from what is loaded and enabled; Phase 4 replaces them with the claims | 1 h |

| | |
|---|---|
| Depends on | nothing; it needs neither the claims nor the ABI handshake |
| Parallel with | Phase 2 |
| Acceptance | First, record the stock baseline twice and check that it repeats at a paused tick (`--clock bench` governs only render3d). Then per category: `port-showcase`, `port-nebula`, `port-firing` with `[Modules] <x>=0` or a UI key at 0, forward mode on the GPU against stub + passthrough with `A2TEST_VULKAN_GPU=1`, `baseline record`/`compare`, `PixelCentres=d3d9`. `Mod: stock` scene variants (asteroids, planet, shields, fog) with Display8 alone, and the stock front end without `UI.asi` (Phase 11, "Risks"; it runs here first). New scenario `display-passthrough`; regressions `display-stub`, `display-movie`, `display-drop`, `ui-loading`, `render3d-mode-switch`. User sign-off in game with a module switched off |
| Versions | display MINOR per merged step (1a–1e, then 1f with render3d MINOR), root MINOR each |
| Estimate | Agent 12–24 h over the steps. The bench sets the pace: the baseline is recorded twice, then each category is A/B-compared against forward mode on the GPU until the pixels agree. That is realistically 10–20 bench passes and several dozen A/B games. Round A, with a second round only if 1f's tone-mapped look is disputed |
| Risks | The invalid anisotropic mip filter and the half-pixel 2D decide pixel exactness; the cursor is assumed to be a sprite, to verify; whether weapon point lights reach the asteroid path is untraced; merging must never reorder the CPU-sorted alpha draws |

### Phase 2 — Open items of the earlier plan

**Goal:** the loose ends of the earlier display plan, each small, none waiting on the
architecture phases.

| | |
|---|---|
| In progress | **CI rebuilds every shader and fails on a stale stamp**: `hlsl-rs.sh` does it for d3dbc today; add dxc to SPIR-V checked with `spirv-val`, and dxc to DXIL. **Plugins use `display8_device_epoch`** (Display8 tells them when the device is made or reset) instead of polling `d3d8_device()`; this is earlier than Phase 11, which only removes the last polling fallback with Direct3D 9 |
| After passthrough | **HUD leftovers the UI has not taken**: the damaged systems' icons over a pointed unit, the shot director's rings, the action cursor over the minimap, and the frame-rate and team-data text `Armada_RenderFrame` draws. Each goes through the panel switch or a skip at its call site, as `ui/HUD.md` does for the panels, and reads the engine's state. With passthrough these draw as stock until the UI takes them, so they are polish, not a blocker |
| Open | **Record and replay of a `gpu` frame**: the interface records a frame's calls and resources to a file, and a small 32-bit replayer runs it under Wine on lavapipe, with no game and no user GPU, on each backend. **Logs out of `Program Files`**: `Render3D.log`, `UI.log` and `Display8.log` are written beside the game, which GOG installs into `Program Files` by default, where an executable with no manifest is redirected to VirtualStore; move them (and the Vulkan pipeline cache, when there is one) to `%LOCALAPPDATA%\armada2-refit\` |
| Depends on | nothing; the HUD leftovers on Phase 1 |
| Acceptance | CI fails on a deliberately stale stamp; `display-stub` and `no-assets` still pass with the plugins on the epoch; each leftover has a bench check that the engine draws nothing in its context; a replayed frame matches its recording on lavapipe |
| Versions | display, render3d, ui PATCH or MINOR as touched; testbench MINOR; root MINOR |
| Estimate | Agent 1–3 h each; 1 bench pass each; round A for the HUD leftovers, none for the rest |

### Phase 3 — Shared libraries and the toolchain

**Goal:** one copy of the runtime, the formats, the shader toolchain and the C helpers.

| | |
|---|---|
| Steps | Extract `crates/a2rt` (behaviour identical) and move `a2fmt`; move the shader toolchain to `tools/shaderc`; one `engine/hook.rs` in `a2rt`. Write `csdk/a2asi.h` and move qol, rules, online and binkproxy onto it. Fold `render3d/install.sh`'s inline ini merge into `tools/ini-merge.py`; the QOL and Online installers merge instead of overwrite |
| Depends on | Phase 1 signed off in game |
| Parallel with | Phase 4 |
| Acceptance | Rebuilt plugins byte-compare or behave identically: the regression set (`display-stub`, `render3d-*` pixel compares under `--clock bench`, `qol-station-groups`, `multiplayer-online-match`); a player edit in `QOL.ini` survives `./install` |
| Versions | PATCH on render3d, ui, display (build only); qol and online MINOR (installers now merge); root MINOR |
| Estimate | Agent 4–8 h; 2–3 bench passes (the pixel compares under `--clock bench`); round B |
| Risks | `alloc::format!` linking in the shared crate; a C signature check regressing in the rewrite |

### Phase 4 — ABI handshake and claims

**Goal:** mismatched files fail safe, and the plugins' claims replace Display8's own
category switches as the routing of passthrough.

| | |
|---|---|
| Steps | `display8_abi()`, sizes in the contract structs, `gpu` checks and logs both versions. The claims export in render3d and UI. Display8 asks for them every frame and passes through every category nobody claims; a module set to 0, or a UI part at 0, counts as unclaimed. The Display8 code default matches the shipped ini (`stub`, `auto`) |
| Depends on | Phase 1 signed off in game; easier after Phase 3 |
| Parallel with | Phase 3 |
| Acceptance | New `display-claims` scenario: Display8 alone, Display8 + UI, Display8 + render3d with `[Modules] sky=0` all draw a frame on the stub through passthrough; full refit draws as before; an old `Render3D.asi` against a new `d3d8.dll` stands down cleanly |
| Versions | display MINOR, render3d MINOR, ui MINOR, root MINOR |
| Estimate | Agent 2–4 h; 2 bench passes; round B |
| Risks | Plugins load after `Direct3DCreate8`; the routing must be read at `CreateDevice` and every frame, when the ASI loader has run |

### Phase 5 — Settings infrastructure

**Goal:** settings are enumerable, persist in the overlay, and some apply live.

| | |
|---|---|
| Steps | Overlay-first lookup in `a2rt::Ini` and `a2asi.h`; a log line whenever the overlay overrides the plugin's own ini. The overlay file and its writer; migrate `BarHidden`. Descriptor tables and `_Settings` exports, starting with QOL, render3d `[Render3D]`, `[Overlays]` and `[World]`, then UI, Online, binkproxy. A lint that checks shipped inis against the tables. Live paths: a per-module enabled flag in render3d's dispatch and `Render3D_SettingsChanged`; a writable UI `Config` for the per-frame keys; binkproxy re-reads `ReplaceMovies` at each open; QOL keeps its patches installed and gates them on a flag (`MaxSelection` stays restart). The bench empties the overlay in every clone and scenarios set toggles through it. The GUI installer lists the overlay in its "settings changed" report |
| Depends on | Phases 1 (signed off) and 3 |
| Parallel with | Phase 6 |
| Acceptance | A player value set in the overlay survives `./install`; the lint passes; `ui-hud-bar-after-cinematic` and `no-assets` still pass; a live flip of hull surfaces mid-mission changes the frame at the same tick |
| Versions | MINOR on each plugin touched, root MINOR |
| Estimate | Agent 3–6 h; 1–2 bench passes; round C, with Phase 6 |
| Risks | The overlay silently winning over a hand edit (the log line and the screen show it); three copies of `rt` if Phase 3 has not landed |

### Phase 6 — The Refit screen

**Goal:** players switch whole features from the Options menu.

| | |
|---|---|
| Steps | A "Refit" entry in the Options column, in the shell and in a mission. It opens a modal of ours over the hosted Options dialog, with tabs Graphics, Interface, Gameplay, Multiplayer and Movies. Rows come from the loaded plugins' descriptor tables; a row whose plugin is absent is hidden; restart rows say so. Must-match rows (Pay on queue) are locked in network games and the multiplayer setup screens. With passthrough (Phase 1) a module row off shows the stock look under stub mode |
| Rows | Graphics: anti-aliasing, HDR and bloom, ships, sky, particles, effects, weapons, special weapons, planets, asteroids, nebulae, lit nebula gas, hull surfaces, fog of war, grid, selection, ghosts, height posts, renderer (advanced). Interface: refit HUD, minimap, info panel, selection overlay, unit view, tooltips, command bar, grid keys, key labels, icons, gauges, info camera, cursor, theme, redrawn menus, menu backdrop, credits, loading screen. Gameplay: pan speed, Shift adds to group, station groups, long moves at warp, view distance, big selections. Multiplayer and Movies: Pay on queue, the Online entry, replacement movies |
| Depends on | Phase 5 |
| Acceptance | New `ui-menus-refit` scenario with its clicks in `testbench/operating/`: toggle a row, check the overlay and the plugin's log after a relaunch; a lock check in `multiplayer-two-players`. User sign-off in game |
| Versions | ui MINOR, root MINOR |
| Estimate | Agent 4–8 h; 3–5 bench passes (iterating the new scenario's clicks); round C |
| Risks | Input capture of a modal over a hosted dialog in a mission is untested; our labels are English only |

### Phase 7 — render3d core and content

**Goal:** render3d's modules can be taken separately, and the core holds no content.

| | |
|---|---|
| Steps | Lift the shared helpers out of `hulls`, `effects`, `ordnance` and `particles` into the core (instance scale, posing, visibility, frustum tests, on-screen marking, animation values, the camera, the light pool, sprites, particle feeding as a core service). Replace modules asking about each other by name with capability queries, and give module order an explicit priority. Make light semantics data: the gas kinds and the beam constants become per-light parameters set by modules; the cluster pass gets a generic medium-response slot, with the shader falloff unchanged. Each module declares its own config section and parses it in one call; recipes come from a per-module data directory with the built-ins as fallback. Move each module's engine reads into a `hooks.rs` of its own, then split into crates, one per module, chosen by Cargo features in a thin `render3d` cdylib |
| Depends on | Phases 1 (signed off) and 3; best after Phase 1f, which touches the same frame code |
| Parallel with | Phase 8's first stages |
| Acceptance | Each step pixel-identical under `--clock bench` (`render3d-*` scenarios); a build with one module left out links and runs |
| Versions | render3d PATCH per no-op step, MINOR when modules become crates; root accordingly |
| Estimate | Agent 8–16 h; one pixel-identical bench pass per step, about 8–10; round D (no visible change to judge) |
| Risks | Statics and the `no_std` allocator linked once across crates; cdylib size |

### Phase 8 — Executable identity and symbol tables

**Goal:** engine knowledge is data per build; a modified executable stands down cleanly.

| | |
|---|---|
| Steps | 8a: every plugin logs the executable's PE timestamp and image size, and the bench records its hash; no behaviour change. 8b: the profile format, the generator and `crates/a2engine`; the stock profile reproduces today's values exactly. 8c: UI first (already centralised in `ui/src/engine/`). 8d: the C plugins: binkproxy, then online and qol/rules, with their assembly stubs jumping through resolved variables. 8e: render3d last, after Phase 7 moved its engine reads into each module's `hooks.rs`. 8f: the content profile (ODF and model names, the Borg model test, vent sprite names, race lists, recipe overlays). 8g: `a2test syms` resolves a profile against the player's executable offline, and a negative test runs a profile against the wrong executable expecting clean stand-downs |
| Depends on | Phases 1 (signed off) and 3; 8e on Phase 7 |
| Acceptance | "symbols: N/N resolved" in every plugin's log on stock; the negative test; full regression set unchanged |
| Versions | MINOR per plugin as it moves, root MINOR |
| Estimate | Agent 8–16 h; one gameplay bench pass per rewritten C stub plus the negative test, about 6–10; round D |
| Risks | Register and stack bugs in the rewritten C stubs: a bench gameplay pass per stub |
| Fleet Operations | Not installed; no profile yet. A port needs its install, its executable's identity, and new findings tied to each hook |

### Phase 9 — Code to its owner, and the release package

**Goal:** each plugin holds one concern, and Display8 owns the window and the display mode.

| | |
|---|---|
| Steps | QOLRules becomes its own layer in `./install`, `a2mod`, the manifest and the GUI installer; decide its shipped default. `KeepClip` and the window and display-mode handling move to Display8; the bar keys to QOL, with the UI drawing the labels QOL exports; the view distance to render3d. Per-layer selection in `./install` and the release installer. The release package keeps its legacy fallback until Phase 11 |
| Depends on | Phases 1 (signed off) and 4 |
| Acceptance | `ui-focus-pointer`, `ui-pointer-bar`, `ui-hud-bar`, `qol-*`, a subset matrix on the bench (each supported subset draws and plays); user in-game check of the pointer and the bar keys |
| Versions | qol MAJOR (layer split), ui MAJOR (bar keys leave), display MINOR, gui-installer MAJOR (per-layer selection changes the package schema), root MAJOR |
| Estimate | Agent 4–8 h; 3–5 bench passes (mostly the subset matrix); round E, which needs the user's own desktop for input |
| Risks | Input under Wine and Xwayland is fragile |

### Phase 10 — Direct3D 12 parity and the platform gates

**Goal:** passthrough on Direct3D 12, and proof that every platform has a backend.

| | |
|---|---|
| Steps | Port 1a–1g to Direct3D 12. Measure Direct3D 12 on native Windows (every measurement so far ran on vkd3d-proton). Run `vkprobe` on a Mac through MoltenVK and set the portability flags in the Vulkan start-up. Record new GPU-Vulkan baselines to replace the `pre-gpu-*` ones recorded on Direct3D 9 |
| Depends on | Phase 1 |
| Acceptance | The Phase 1 scenarios on `Backend=d3d12`; a native Windows run; a Mac run of the probe and the game |
| Versions | display MINOR, root MINOR |
| Estimate | Agent 3–6 h for the port; 2–4 bench passes on `Backend=d3d12`; round E on Linux. The native Windows and Mac runs wait on access to those machines and sit outside the rounds |
| Risks | MoltenVK is unproven; until a Mac run passes, the Mac only works through d3d8to9 on wined3d |

### Phase 11 — Remove Direct3D 9 and the legacy plugins

**Goal:** the Direct3D 9 path and everything the UI superseded are gone; old installs are still cleaned.

| | |
|---|---|
| Gate | Phases 1, 9 and 10 confirmed; `Backend=auto` the default for two releases with no Direct3D 9 in bug reports |
| Steps | Forward mode becomes a **bail-out** of about fifty lines: when neither Vulkan nor Direct3D 12 starts, Display8 loads the old `d3d8.dll` and hands its interface back unwrapped, and the plugins stand down. Delete the forward wrappers, `Real=` forwarding, `Backend=d3d9`, Display8's Direct3D 9 backend and its d3dbc present shader; `gpu`'s Direct3D 9 backend and the vkd3d branches of `compat.hlsli`; the d3dbc shaders, `d3dbc-fix.py`, `d3dbc-slots.py` and the vkd3d build step; every plugin's `d9_device` fallback; `platform/d3d9`, `D3D9.md`, `dxvk-logging.py`; d3d8to9 and DXVK from the chain and from `platform/vendor/` once no platform needs them; `d3d9=n,b` from the overrides (Linux and bench); `renderer/` and `dxvk.conf` after the sampler keys move to `Display8.ini`. The last device polling in `gpu` goes (the plugins use the epoch since Phase 2). `ui/ini-merge.py` maps `Backend=d3d9` and `Mode=forward` to the new defaults. Retire lists recognise our `dxvk.conf` by its marker and our d3d8to9 and DXVK by hash, never by name |
| Legacy removal | Delete `hud/` and `grid/`, the plugin half of `menus/` (`menus.c`, `probe.c`, its build and ini), the `LOADING` target and `models/logo-sod.py` (the engine's widened loading scene, which the stub drops; they ship together and go together). Move the backdrop pipeline (`backdrop.sh`, `backdrops/`, `BACKDROPS.md`, `install-plates.sh`) under `ui/backdrops/`, and `stop-game.sh`/`run-wine.sh` to `platform/`. Fold `hud/revert-files.sh` and the three scripts' `--revert` paths into one `retire/` script that `./install` and `a2mod` call, with today's retire lists (the twelve old render3d plugins, HUD, Grid, Menus, MenuScale, vkBasalt). Remove the UI's and Display8's stand-downs for the legacy plugins, `ui/bench-legacy.sh`, `bench-asi.sh --no-grid`, the scenarios built on them, `testbench/d3d9probe` and `testbench/d3dtrace`. The release package stops shipping the legacy fallback. `a2mod` keeps recognising the old layers under a "retired" row so an old snapshot still restores |
| Findings first | Before any of `hud/`, `grid/` or the plugin half of `menus/` is deleted, go through their READMEs. Hard rule 9 says a finding belongs to the plugin whose code uses it, so each finding a remaining plugin still uses moves into that plugin's README, beside the hook that uses it, written in our own words. The shell's hard-coded front-end mode, the embedded dialogs (owner, z-order, input, focus, cursor), GetDC and ReleaseDC not pairing, the 2D canvas, the font and cursor references, and the bar's slot plan and hotkey tables go to `ui/README.md`, `ui/HUD.md` and `ui/MENUS.md`. Those files point at `menus/README.md` and `hud/README.md` for them today. The window and display-mode findings go to `display/README.md`, and the raised mode that `FindDisplayMode` has to list goes to `cutscenes/binkproxy/README.md`. A finding that no remaining plugin uses leaves the repository with its code. The backdrop pipeline's docs move with the backdrop scripts |
| Textures nothing reads | Last, once the list of locks is complete (the stock traces and the mirror table show what the engine reads back): stop keeping textures in Display8's memory that nothing samples or locks, to save address space |
| Acceptance | `./a2test run no-assets`; a clone with the legacy plugins installed comes out clean through `--install .`; no link in the repository points into a deleted README; the remaining `ui-*` scenarios; `display-fallback` (Vulkan, then Direct3D 12, then bail-out); the stock front-end check below; the full regression set on GPU Vulkan; `no-assets`; user sign-off |
| Versions | hud, grid, menus: final MAJOR "Removed" entries; display MAJOR, platform MAJOR, renderer final MAJOR "Removed", render3d and ui MAJOR (no longer run without Display8; the UI's bench flags go), testbench MAJOR, gui-installer MAJOR, root MAJOR. Several merges, each with its own root bump |
| Estimate | Agent 6–11 h, of which 1–2 h move findings; 2–3 full regression passes on GPU Vulkan; round F, after two releases with `auto` as the default |
| Risks | `GridLayout.ini [Cells]` has no UI counterpart; note the gap in `ui/README.md` rather than keep the plugin. **The stock front end over Vulkan, without `UI.asi`**, is a bench check, because most of it is already settled. The findings it reuses: (1) the shell asks for a fixed 800x600x16 mode, and Menus.asi, now the UI's menu host as well, raises that request to the desktop's size in memory, so the shell and the 3D display share one mode and the device is not re-created at each switch; (2) the stub offers the engine one mode, the desktop's, so no mode change happens on Vulkan at all, and the cursor and mouse need no translation; (3) `Embed=1`, and the UI's host after it, makes every dialog a child of the game window, so there is one OS window, with owner, z-order, input, focus and cursor each handled and measured; (4) while a GDI dialog is up the game thread sits in the dialog's modal loop and nothing presents, and only the intro movie and the Credits use the device in the shell; both already run on the stub (`display-stub`, intro to Exit; movies since display 0.6.1). **The check:** the stub on GPU Vulkan, with neither `UI.asi` nor `Menus.asi` loaded, through the main menu, Options and a nested screen, the Credits, a mission and its in-mission menu and back, with the cursor visible throughout. It runs with Phase 1's Display8-alone runs and again in this phase's acceptance. Stock's separate popup windows are stock behaviour and stay acceptable. If a stock dialog does not appear, Display8 takes over the shell-mode raise from the UI, which is code already written twice and a display concern anyway |

### Phase 12 — Documentation

Rewrite the CLAUDE.md sections this plan changes: the layout table (hud, grid, menus,
renderer rows), "renderer, platform", the render3d note on a missing Direct3D 9 device,
"display", and the bench's GPU policy (`A2TEST_VULKAN_GPU=1`; lavapipe stays for
validation). Each layer's README keeps its own derivations.

## Dependencies

```
P1 passthrough VK ── sign-off round A ──┬─ P3 shared libs ──┬─ P5 settings ── P6 Refit ──┐
P2 open items (beside P1)               │                   ├─ P7 render3d split ── P8e  │
                                        │                   └─ P8 symbols (a–d, f–g)     │
                                        ├─ P4 ABI + claims ── P9 owners + package        │
                                        └─ P10 D3D12 + gates                             │
                         P11 remove D3D9 and legacy (after P1, P9, P10) ── P12 docs ─────┘
```

| Phase | Depends on | Can run beside | Agent time · bench passes · round |
|---|---|---|---|
| 1 passthrough, Vulkan (in progress) | — | 2 | 12–24 h · 10–20 plus A/B · A |
| 2 open items | (1 for the HUD leftovers) | 1 | 1–3 h each · 1 each · A |
| 3 shared libraries | 1 signed off | 4 | 4–8 h · 2–3 · B |
| 4 ABI and claims | 1 signed off, (3) | 3 | 2–4 h · 2 · B |
| 5 settings | 1 signed off, 3 | 6, 7, 8 | 3–6 h · 1–2 · C |
| 6 Refit screen | 5 | 7, 8 | 4–8 h · 3–5 · C |
| 7 render3d split | 1 signed off, 3 (after 1f) | 8a–d | 8–16 h · 8–10 · D |
| 8 symbol tables | 1 signed off, 3; 8e on 7 | 5–7 | 8–16 h · 6–10 · D |
| 9 owners, package | 1 signed off, 4 | 10 | 4–8 h · 3–5 · E |
| 10 D3D12, gates | 1 | 9 | 3–6 h · 2–4 · E, plus the machines |
| 11 remove D3D9 and legacy | 1, 9, 10 + two releases | — | 6–11 h · 2–3 · F |
| 12 docs | 11 | — | 1–2 h · none · F |

Summed, the agent time is about 55–115 hours, much of it in parallel worktrees.
The wall-clock time is set by the five or six sign-off rounds and by the bench ("Pacing").

## Open decisions

1. **QOLRules' shipped default.** Once it is its own layer, should `PayOnQueue` ship on
   (as now) or off, so that installing the refit never changes single-player rules?
2. **Where the overlay lives and what it is called**: keep `UIPlayer.ini` or rename it
   (for example `RefitPlayer.ini`), in the game directory or the player's profile.
3. **Packaging for modders**: built DLLs with ini files, source crates, or both. The
   plan keeps code boundaries first and packaging granularity second.
4. **Windows and Mac access** for Phase 10: who runs the native Direct3D 12 and the
   MoltenVK checks, and on which machines.
5. **How long the Direct3D 9 path stays** after Phase 10: the plan's floor is two
   releases with `auto` as the default; the user may shorten or lengthen it.
6. **The renderer row in the Refit screen**: offer the backend choice to players
   (advanced tier), or leave it in `Display8.ini` only.
7. **Fleet Operations**: when an install is available, which version to target first.
