# Roadmap: Display8 on Vulkan and Direct3D 12, modular plugins, feature toggles

Where the project goes from 13.34.0, and in which order. It is the plan; the layer
READMEs stay the reference for how each part works, and the changelogs for what has
shipped. Update this file as phases land: tick the phase, link the merge, move what
was learned into the layer's README.

## Where we are

- **Display8** (`display/`) is the game's `d3d8.dll`. In stub mode (`Backend=auto`) it
  owns the device on Vulkan, then Direct3D 12, then Direct3D 9, and **drops every draw
  the engine makes**. render3d and the UI draw the whole frame instead.
- **Forward mode** passes every call to the old chain: d3d8to9 as `d3d8_real.dll`, then
  DXVK's `d3d9.dll`. The release package still ships that chain and the legacy plugins
  (`HUD.asi`, `GridLayout.asi`, `Menus.asi`) as its fallback. Display8 is not in the
  release package yet.
- **render3d** and **UI** draw through the `gpu` crate, which has Direct3D 9, Vulkan and
  Direct3D 12 backends. Shaders build to d3dbc (vkd3d), SPIR-V and DXIL (dxc).
- **QOL**, **QOLRules**, **Online** and **binkproxy** are C plugins with their own ini
  files, read once at load.

What the architecture review found (four read-only reviews, 2026-10-23):

| Problem | Effect |
|---|---|
| Stub mode drops the engine's draws but never checks that something draws instead | Without `Render3D.asi`, the 3D view is black. A module or UI part switched off draws nothing, not the stock look, under the shipped default. "A key at 0 gives the part back to the engine" is false today |
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
Display8 asks it every frame. A category nobody claims is drawn by passthrough (or,
until passthrough exists, makes Display8 fall back to forward mode). This replaces
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

## Phases

Each phase is one or more merges into `main`, each with its own root version and
"Finishing work" in CLAUDE.md: confirmed in game and signed off by the user, then
merged. A bench pass is evidence, not sign-off. Estimates are working days of agent
time, rough.

### Phase 1 — Remove the legacy plugins

**Goal:** the superseded plugins leave the repository; old installs are still cleaned.

| | |
|---|---|
| Steps | Delete `hud/` and `grid/`, and the plugin half of `menus/` (`menus.c`, `probe.c`, its build and ini). Move the backdrop pipeline (`backdrop.sh`, `backdrops/`, `BACKDROPS.md`, `install-plates.sh`) under `ui/backdrops/`, and `stop-game.sh`/`run-wine.sh` to `platform/`. Fold `hud/revert-files.sh` and the three scripts' `--revert` paths into one `retire/` script that `./install` and `a2mod` call, with today's retire lists (the twelve old render3d plugins, HUD, Grid, Menus, MenuScale, vkBasalt). Remove the UI's and Display8's stand-downs for the legacy plugins, `ui/bench-legacy.sh`, `bench-asi.sh --no-grid`, the six scenarios built on them, `testbench/d3d9probe` and `testbench/d3dtrace`. `a2mod` keeps recognising the old layers under a "retired" row so an old snapshot still restores |
| Keeps | The release package's legacy fallback stays until Phase 9 |
| Depends on | nothing |
| Parallel with | Phases 2 and 3 |
| Acceptance | `./a2test run no-assets`; a clone with the legacy plugins installed comes out clean through `--install .`; the remaining `ui-*` scenarios |
| Versions | hud, grid, menus: final MAJOR "Removed" entries; ui MAJOR (bench flags removed); testbench MAJOR; root MAJOR (`a2mod` layer set) |
| Estimate | 2 days |
| Risks | `GridLayout.ini [Cells]` has no UI counterpart; note the gap in `ui/README.md` rather than keep the plugin |

### Phase 2 — Shared libraries and the toolchain

**Goal:** one copy of the runtime, the formats, the shader toolchain and the C helpers.

| | |
|---|---|
| Steps | Extract `crates/a2rt` (behaviour identical) and move `a2fmt`; move the shader toolchain to `tools/shaderc`; one `engine/hook.rs` in `a2rt`. Write `csdk/a2asi.h` and move qol, rules, online and binkproxy onto it. Fold `render3d/install.sh`'s inline ini merge into `tools/ini-merge.py`; the QOL and Online installers merge instead of overwrite |
| Depends on | nothing |
| Parallel with | Phases 1 and 3 |
| Acceptance | Rebuilt plugins byte-compare or behave identically: the regression set (`display-stub`, `render3d-*` pixel compares under `--clock bench`, `qol-station-groups`, `multiplayer-online-match`); a player edit in `QOL.ini` survives `./install` |
| Versions | PATCH on render3d, ui, display (build only); qol and online MINOR (installers now merge); root MINOR |
| Estimate | 4 days |
| Risks | `alloc::format!` linking in the shared crate; a C signature check regressing in the rewrite |

### Phase 3 — ABI handshake and stub-mode safety

**Goal:** mismatched files fail safe, and stub mode never leaves part of the frame
undrawn while passthrough does not exist yet.

| | |
|---|---|
| Steps | `display8_abi()`, sizes in the contract structs, `gpu` checks and logs both versions. The claims export in render3d and UI. Display8's stub falls back to forward mode at device creation unless every category is claimed; a module set to 0, or a UI part at 0, counts as unclaimed. render3d logs a warning on a hand-back under stub. The Display8 code default matches the shipped ini (`stub`, `auto`) |
| Depends on | nothing; easier after Phase 2 |
| Parallel with | Phases 1 and 2 |
| Acceptance | New `display-claims` scenario: Display8 alone, Display8 + UI, Display8 + render3d with `[Modules] sky=0` all draw a frame (forward); full refit stays on stub; an old `Render3D.asi` against a new `d3d8.dll` stands down cleanly |
| Versions | display MINOR, render3d MINOR, ui MINOR, root MINOR |
| Estimate | 3 days |
| Risks | Plugins load after `Direct3DCreate8`; the decision must be at `CreateDevice`, when the ASI loader has run |

### Phase 4 — Passthrough on Vulkan

**Goal:** Display8 draws every unclaimed engine draw on Vulkan the way stock does.

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

| Step | What | Estimate |
|---|---|---|
| 4a | Dispatch in place of the counting macro; a mirror for every engine texture, made at first draw; vertex and index upload per draw into the frame ring; a pipeline cache keyed by blend, depth, cull, topology, vertex layout, alpha test and the target's formats and samples; fans to lists; consecutive draws with identical state merged, never reordered | 4–5 |
| 4b | Fixed-function shaders through `compat.hlsli`: the pre-transformed path with D3D9 pixel centres, and a pixel shader driven by a small constant block for the recorded stage ops, texture factor, specular add and alpha test. An op outside the recorded set is logged once | 2–3 |
| 4c | The fixed-function lit path for asteroid fields: directional and point lights, material, ambient | 2 |
| 4d | The Borg vertex shader, ported by hand and matched by a hash of its tokens; an unknown shader is refused and logged | 2 |
| 4e | Drawing into Display8's own engine colour and depth images, the UI on top (no render3d) | 1 |
| 4f | Drawing into render3d's scene pass when a module is off: a contract change in `gpu/src/display.rs` publishes the pass now (images, formats, samples, viewport) and keeps render3d's targets in attachment layouts between recorders; Display8 opens a compatible pass on them, so the stock draw is tone-mapped and bloomed with the rest | 3–5 |
| 4g | Claims become the routing: unclaimed categories draw instead of making Display8 fall back to forward mode | 1 |

| | |
|---|---|
| Depends on | Phase 3 (claims, ABI) |
| Parallel with | Phases 5 and 6 |
| Acceptance | First, record the stock baseline twice and check that it repeats at a paused tick (`--clock bench` governs only render3d). Then per category: `port-showcase`, `port-nebula`, `port-firing` with `[Modules] <x>=0` or a UI key at 0, forward mode on the GPU against stub + passthrough with `A2TEST_VULKAN_GPU=1`, `baseline record`/`compare`, `PixelCentres=d3d9`. `Mod: stock` scene variants (asteroids, planet, shields, fog) with Display8 alone. New scenario `display-passthrough`; regressions `display-stub`, `display-movie`, `display-drop`, `ui-loading`, `render3d-mode-switch`. User sign-off in game with a module switched off |
| Versions | display MINOR per merged step (4a–4e, then 4f with render3d MINOR), root MINOR each |
| Estimate | 3–4 weeks |
| Risks | The invalid anisotropic mip filter and the half-pixel 2D decide pixel exactness; the cursor is assumed to be a sprite, to verify; whether weapon point lights reach the asteroid path is untraced; merging must never reorder the CPU-sorted alpha draws |

### Phase 5 — Settings infrastructure

**Goal:** settings are enumerable, persist in the overlay, and some apply live.

| | |
|---|---|
| Steps | Overlay-first lookup in `a2rt::Ini` and `a2asi.h`; a log line whenever the overlay overrides the plugin's own ini. The overlay file and its writer; migrate `BarHidden`. Descriptor tables and `_Settings` exports, starting with QOL, render3d `[Render3D]`, `[Overlays]` and `[World]`, then UI, Online, binkproxy. A lint that checks shipped inis against the tables. Live paths: a per-module enabled flag in render3d's dispatch and `Render3D_SettingsChanged`; a writable UI `Config` for the per-frame keys; binkproxy re-reads `ReplaceMovies` at each open; QOL keeps its patches installed and gates them on a flag (`MaxSelection` stays restart). The bench empties the overlay in every clone and scenarios set toggles through it. The GUI installer lists the overlay in its "settings changed" report |
| Depends on | Phase 2 |
| Parallel with | Phases 4 and 6 |
| Acceptance | A player value set in the overlay survives `./install`; the lint passes; `ui-hud-bar-after-cinematic` and `no-assets` still pass; a live flip of hull surfaces mid-mission changes the frame at the same tick |
| Versions | MINOR on each plugin touched, root MINOR |
| Estimate | 1 week |
| Risks | The overlay silently winning over a hand edit (the log line and the screen show it); three copies of `rt` if Phase 2 has not landed |

### Phase 6 — The Refit screen

**Goal:** players switch whole features from the Options menu.

| | |
|---|---|
| Steps | A "Refit" entry in the Options column, in the shell and in a mission. It opens a modal of ours over the hosted Options dialog, with tabs Graphics, Interface, Gameplay, Multiplayer and Movies. Rows come from the loaded plugins' descriptor tables; a row whose plugin is absent is hidden; restart rows say so. Must-match rows (Pay on queue) are locked in network games and the multiplayer setup screens. Until Phase 4g, render3d module rows are disabled under stub mode with a note |
| Rows | Graphics: anti-aliasing, HDR and bloom, ships, sky, particles, effects, weapons, special weapons, planets, asteroids, nebulae, lit nebula gas, hull surfaces, fog of war, grid, selection, ghosts, height posts, renderer (advanced). Interface: refit HUD, minimap, info panel, selection overlay, unit view, tooltips, command bar, grid keys, key labels, icons, gauges, info camera, cursor, theme, redrawn menus, menu backdrop, credits, loading screen. Gameplay: pan speed, Shift adds to group, station groups, long moves at warp, view distance, big selections. Multiplayer and Movies: Pay on queue, the Online entry, replacement movies |
| Depends on | Phase 5; module rows on stub need Phase 4g |
| Acceptance | New `ui-menus-refit` scenario with its clicks in `testbench/operating/`: toggle a row, check the overlay and the plugin's log after a relaunch; a lock check in `multiplayer-two-players`. User sign-off in game |
| Versions | ui MINOR, root MINOR |
| Estimate | 1 week |
| Risks | Input capture of a modal over a hosted dialog in a mission is untested; our labels are English only |

### Phase 7 — render3d core and content

**Goal:** render3d's modules can be taken separately, and the core holds no content.

| | |
|---|---|
| Steps | Lift the shared helpers out of `hulls`, `effects`, `ordnance` and `particles` into the core (instance scale, posing, visibility, frustum tests, on-screen marking, animation values, the camera, the light pool, sprites, particle feeding as a core service). Replace modules asking about each other by name with capability queries, and give module order an explicit priority. Make light semantics data: the gas kinds and the beam constants become per-light parameters set by modules; the cluster pass gets a generic medium-response slot, with the shader falloff unchanged. Each module declares its own config section and parses it in one call; recipes come from a per-module data directory with the built-ins as fallback. Move each module's engine reads into a `hooks.rs` of its own, then split into crates, one per module, chosen by Cargo features in a thin `render3d` cdylib |
| Depends on | Phase 2; best after Phase 4f, which touches the same frame code |
| Parallel with | Phase 8's first stages |
| Acceptance | Each step pixel-identical under `--clock bench` (`render3d-*` scenarios); a build with one module left out links and runs |
| Versions | render3d PATCH per no-op step, MINOR when modules become crates; root accordingly |
| Estimate | 2–3 weeks |
| Risks | Statics and the `no_std` allocator linked once across crates; cdylib size |

### Phase 8 — Executable identity and symbol tables

**Goal:** engine knowledge is data per build; a modified executable stands down cleanly.

| | |
|---|---|
| Steps | 8a: every plugin logs the executable's PE timestamp and image size, and the bench records its hash; no behaviour change. 8b: the profile format, the generator and `crates/a2engine`; the stock profile reproduces today's values exactly. 8c: UI first (already centralised in `ui/src/engine/`). 8d: the C plugins: binkproxy, then online and qol/rules, with their assembly stubs jumping through resolved variables. 8e: render3d last, after Phase 7 moved its engine reads into each module's `hooks.rs`. 8f: the content profile (ODF and model names, the Borg model test, vent sprite names, race lists, recipe overlays). 8g: `a2test syms` resolves a profile against the player's executable offline, and a negative test runs a profile against the wrong executable expecting clean stand-downs |
| Depends on | Phase 2; 8e on Phase 7 |
| Acceptance | "symbols: N/N resolved" in every plugin's log on stock; the negative test; full regression set unchanged |
| Versions | MINOR per plugin as it moves, root MINOR |
| Estimate | 3–4 weeks |
| Risks | Register and stack bugs in the rewritten C stubs: a bench gameplay pass per stub |
| Fleet Operations | Not installed; no profile yet. A port needs its install, its executable's identity, and new findings tied to each hook |

### Phase 9 — Code to its owner, and the release package

**Goal:** each plugin holds one concern, and the release ships Display8.

| | |
|---|---|
| Steps | QOLRules becomes its own layer in `./install`, `a2mod`, the manifest and the GUI installer; decide its shipped default. `KeepClip` and the window and display-mode handling move to Display8; the bar keys to QOL, with the UI drawing the labels QOL exports; the view distance to render3d. Per-layer selection in `./install` and the release installer. The release package ships Display8 (Linux, then Windows) and stops shipping the legacy fallback |
| Depends on | Phases 1, 3; Display8 in the package needs Phase 4 confirmed |
| Acceptance | `ui-focus-pointer`, `ui-pointer-bar`, `ui-hud-bar`, `qol-*`, a subset matrix on the bench (each supported subset draws and plays); user in-game check of the pointer and the bar keys |
| Versions | qol MAJOR (layer split), ui MAJOR (bar keys leave), display MINOR, gui-installer MAJOR (package schema), root MAJOR |
| Estimate | 2 weeks |
| Risks | Input under Wine and Xwayland is fragile |

### Phase 10 — Direct3D 12 parity and the platform gates

**Goal:** passthrough on Direct3D 12, and proof that every platform has a backend.

| | |
|---|---|
| Steps | Port 4a–4g to Direct3D 12. Measure Direct3D 12 on native Windows (every measurement so far ran on vkd3d-proton). Run `vkprobe` on a Mac through MoltenVK and set the portability flags in the Vulkan start-up. Record new GPU-Vulkan baselines to replace the `pre-gpu-*` ones recorded on Direct3D 9 |
| Depends on | Phase 4 |
| Acceptance | The Phase 4 scenarios on `Backend=d3d12`; a native Windows run; a Mac run of the probe and the game |
| Versions | display MINOR, root MINOR |
| Estimate | 1–2 weeks, plus access to a Windows machine and a Mac |
| Risks | MoltenVK is unproven; until a Mac run passes, the Mac only works through d3d8to9 on wined3d |

### Phase 11 — Remove Direct3D 9

**Goal:** the Direct3D 9 path is gone.

| | |
|---|---|
| Gate | Phases 4, 9 and 10 confirmed; `Backend=auto` the default for two releases with no Direct3D 9 in bug reports |
| Steps | Forward mode becomes a **bail-out** of about fifty lines: when neither Vulkan nor Direct3D 12 starts, Display8 loads the old `d3d8.dll` and hands its interface back unwrapped, and the plugins stand down. Delete the forward wrappers, `Real=` forwarding, `Backend=d3d9`, Display8's Direct3D 9 backend and its d3dbc present shader; `gpu`'s Direct3D 9 backend and the vkd3d branches of `compat.hlsli`; the d3dbc shaders, `d3dbc-fix.py`, `d3dbc-slots.py` and the vkd3d build step; every plugin's `d9_device` fallback; `platform/d3d9`, `D3D9.md`, `dxvk-logging.py`; d3d8to9 and DXVK from the chain and from `platform/vendor/` once no platform needs them; `d3d9=n,b` from the overrides (Linux and bench); `renderer/` and `dxvk.conf` after the sampler keys move to `Display8.ini`. `gpu`'s device polling becomes the device epoch. `ui/ini-merge.py` maps `Backend=d3d9` and `Mode=forward` to the new defaults. Retire lists recognise our `dxvk.conf` by its marker and our d3d8to9 and DXVK by hash, never by name |
| Acceptance | `display-fallback` (Vulkan, then Direct3D 12, then bail-out); the full regression set on GPU Vulkan; `no-assets`; user sign-off |
| Versions | display MAJOR, platform MAJOR, renderer final MAJOR "Removed", render3d and ui MAJOR (no longer run without Display8), testbench MAJOR, gui-installer MAJOR, root MAJOR. Several merges, each with its own root bump |
| Estimate | 1–2 weeks |
| Risks | **The front end over Vulkan.** Without the UI's menus the shell asks for an 800x600 mode and draws with GDI over a Vulkan swap chain; credits, the cursor and mode changes must be covered. This is the largest single risk of the plan, and it is not a rendering problem |

### Phase 12 — Documentation

Rewrite the CLAUDE.md sections this plan changes: the layout table (hud, grid, menus,
renderer rows), "renderer, platform", the render3d note on a missing Direct3D 9 device,
"display", and the bench's GPU policy (`A2TEST_VULKAN_GPU=1`; lavapipe stays for
validation). Each layer's README keeps its own derivations.

## Dependencies

```
P1 legacy removal ─────────────────────────────┐
P2 shared libs ──┬─ P5 settings ── P6 Refit ───┤
                 ├─ P7 render3d split ── P8e   │
                 └─ P8 symbols (a–d, f–g)      │
P3 ABI + claims ── P4 passthrough VK ──┬─ P9 owners + package
                     (4g enables P6     ├─ P10 D3D12 + gates
                      module rows)      └──────── P11 remove D3D9 ── P12 docs
```

| Phase | Depends on | Can run beside | Estimate |
|---|---|---|---|
| 1 legacy removal | — | 2, 3 | 2 d |
| 2 shared libraries | — | 1, 3 | 4 d |
| 3 ABI and claims | (2) | 1, 2 | 3 d |
| 4 passthrough, Vulkan | 3 | 5, 6, 8a–d | 3–4 wk |
| 5 settings | 2 | 4, 7, 8 | 1 wk |
| 6 Refit screen | 5 (4g for module rows) | 4, 7, 8 | 1 wk |
| 7 render3d split | 2 (after 4f) | 8a–d | 2–3 wk |
| 8 symbol tables | 2; 8e on 7 | 4–7 | 3–4 wk |
| 9 owners, package | 1, 3, 4 | 10 | 2 wk |
| 10 D3D12, gates | 4 | 9 | 1–2 wk + machines |
| 11 remove D3D9 | 4, 9, 10 + two releases | — | 1–2 wk |
| 12 docs | 11 | — | 2 d |

Agents work in parallel worktrees; whichever merges second renumbers its versions.
A timing measurement (`Timing=1`) runs alone on the bench; pixel comparisons may share it.

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
