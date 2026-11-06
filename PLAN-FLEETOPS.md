# Plan: Fleet Operations: Roots

How the refit comes to Fleet Operations: Roots, a submod built on Fleet Operations (itself
a total conversion of Armada II), in
**this** repository. The first target is Roots 5.0Z11 (23 October 2026), as the user has
it installed through Heroic. Decided 2026-11-02: Roots comes first among other
executables, before Armada 1 (`PLAN-ARMADA1.md`), and its profile work rides on
`ROADMAP.md` phase 8.

It rests on a read-only survey of 2026-11-02: the install's files, its Heroic entry,
and `Data/ArmadaL.exe` compared byte for byte with stock `Armada2.exe` 1.1 at every
address our plugins use. Nothing was launched. What only a running game can show is
listed as measurement in the work packages, not assumed.

## What Roots is, measured

### The executable

`Data/ArmadaL.exe` is **stock 1.1's image, patched**, not a rebuild.

| | Roots `ArmadaL.exe` | stock `Armada2.exe` 1.1 |
|---|---|---|
| PE timestamp | `0x3c4c76bd` | `0x3c4c76bd` |
| Image size | `0x405000` | `0x3fa000` |
| Entry point, image base | the same | — |
| `.text`, `.rdata`, `.data`, `.idata`, `.rsrc` | the same addresses and sizes | — |
| Extra section | `.test` at `0x7fa000`, about 43 KB, executable: Fleet Operations' code | — |
| Imports | identical: the same 20 DLLs and functions (among them `d3d8`, `binkw32`, `NetworkManager`, `WINMM`, `Win2kDisableTaskSwitch`) | — |
| `.text` differences | 4,235 bytes in 1,012 ranges over 210 functions; the largest range is 129 bytes. 154 ranges are a jump or call into `.test` (detours), 764 are edits of four bytes or fewer (constants, flipped branches), 94 other | — |
| `.rdata`, `.data` differences | 90 and 410 bytes, small strings and tables; **none at an address our plugins read** | — |

So about 99% of a stock symbol profile resolves to the same address with the same
bytes. What differs, by our plugins:

| Plugin | Checked here | Same in Roots | Where Roots differs |
|---|---|---|---|
| qol (`qol.c`, `rules.c`) | 61 byte signatures; 118 code and 17 data addresses | **all 61 signatures**; 103 of 118 enclosing functions | Roots detours four functions our sites sit in: `mCheckGroupSelect` (11 of our sites), `cOverViewImp::mProcessKeyboardInput`, `ComputerVoice::Say` (the rules plugin's) and part of `cOverViewImp::Simulate`. `Producer::FinishBuild` and `CraftProcess::Handle_New_Command` differ elsewhere in the function |
| online | 9 code, 3 data | 7 | `InternetGameDlgProc`, `MultiplayerSetupDlgProc`: the dialog-size constants only |
| binkproxy | 3 | 3 | — |
| ui (`src/engine/`) | 228 code, 110 data | 212 | twelve dialog procedures (the shell's size constants), `Program::GameDoOne` (a three-byte edit), `PopupPaletteImp::mProcessKeyboardInput`, `SetActiveDisplay_Internal` |
| render3d | 167 code, 100 data | 154 | **the seam is untouched**: both `Armada_RenderAllOurStuff` calls, `GameObject_PreRenderAll`, `Ordnance::PreRenderAll`. Roots detours `GameObject::RenderInternal` (hulls), `Craft::Explode` and `BuildingEffect::Render`, and edits `ShieldHit::DrawShield`, `PlaceholderFeature::Render` and `sRenderInternal`, `BuildingEffect::SafeDestroyConstruction`, `Terrain_Geometry::DrawGrid`, `GetSensorScramblerWaveLight`, `ST3D_ChunkManager::CreateChunks` |
| display | 19 | 16 | `FindDisplayMode` (one branch), `SetActiveDisplay_Internal` (the shell mode) |

**The shell is 1200x900, not 800x600.** Nine places where stock asks for an 800x600
front end ask for 1200x900 in Roots, including `SetActiveDisplay_Internal` and the
dialog procedures above, and Roots ships its shell art at 1200x900 to match. Everything
in our code that assumes the 800x600 shell (the UI's menu host and its specs, the mode
raise that `Menus.asi` and the UI do, the backdrop composite) is wrong for Roots.

That is the static picture. **`FleetOpsHook.dll` patches more at run time**: it is a
3 MB Delphi DLL, loaded through `Win2kDisableTaskSwitch.dll`, which `ArmadaL.exe` imports
(Roots' own build of that DLL), and its unit names (shipped in `FleetOpsHook.map`)
say it enhances `Craft`, `Producer`, `Shipyard`, `PopupPalette`, `cOverViewImp`, the
file system, the chat and the version check. It also imports `d3d9`, `D3DX81ab` and
`d3dx9_43`. FO-2 measured what it does at run time (below).

### FleetOpsHook.dll at run time

Measured on the bench on 2026-11-06 (FO-2), in clones only: Roots as it ships, and Roots
with Display8 in DxWrapper's place, each with `testbench/memdiff/` (`MemDiff.asi`)
comparing `ArmadaL.exe`'s image with its file. A private static reading of the DLL, done
alongside, agrees with what was seen; neither is published beyond names and counts (hard
rule 9).

| | Measured |
|---|---|
| When it patches | All at once, as it loads: `Win2kDisableTaskSwitch.dll`'s attach loads it (from the install's root folder, beside `FleetOps.exe`), before `WinMain` and before any `.asi` or Display8 starts. At an ASI's first look every change is already there; nothing in code or read-only data changes after the first second, over 4 minutes, a skirmish included |
| How much | About 1,260 changed ranges, 7.2 KB, in `.text` and `.rdata` (plus a few in `.data`, which the game also writes): about 550 `call` and 80 `jmp` into `FleetOpsHook.dll`, about 50 vtable or pointer slots pointing into it, the rest constants, nops and bytes. None in Roots' own `.test` section. All but one range match the static reading of the DLL; the same set under DxWrapper and under Display8, so it does not depend on the device |
| Our hooks it touches | Through the functions they use: 85 functions beyond the file diff's, among them `Armada_RenderAllOurStuff` and `Armada_RenderFrame` (render3d's seam: FO edits the object loop, not the two seam calls), the five height-post sites and `Craft::Explode`'s fragment call (render3d re-points the same calls), `GameObject::RenderInternal`, `ShieldHit::DrawShield`, the selection's `cOverViewImp` sites (FO moved the selection as QOL's `MaxSelection` does), every dialog procedure, and the entries of `operator new`, `cPrjID::GetOdfName`, `cPrjID::GetName`, `GameObject::Render` and `Craft::RenderInternal`, which it takes over (a jump, or a trampoline). The Roots profile marks the first kind and drops the entry check of the second where our hooks only call them |
| Direct3D | On the default path it makes **no Direct3D 9 device**: no `d3d9` device was created in either run (its `d3d9.dll` import is loaded, unused; the intro movies, which it plays through a `d3d9` device of its own by the static reading, did not play on the bench). It works **on the engine's Direct3D 8 device**: it calls `IDirect3D8::CreateDevice` itself (in place of the engine's call), `GetAdapterMonitor`, and D3DX8 creates and fills the textures it loads (`CheckDeviceFormat`, `CreateTexture`, `GetSurfaceLevel`, `LockRect`). It draws Roots' bump-mapped meshes itself, about 170 indexed draws a frame in a skirmish beside the engine's 115, with `SetVertexShader`, `SetVertexShaderConstant`, `SetStreamSource` and `SetIndices` |
| The ASI loader | The Ultimate ASI Loader as `Data/winmm.dll` beside DxWrapper kills Roots at start (`c0000409`) with no plugin loaded; beside Display8 it loads `.asi` files normally, after `FleetOpsHook` has patched. DxWrapper loads `.asi` files itself (`LoadPlugins=1`), which is how `MemDiff.asi` ran on Roots as it ships |

Roots' Direct3D 8 calls on Display8, against stock's (`display-inventory`): new are
`CreateVertexShader` and `SetVertexShaderConstant` (FO's bump meshes), `DrawPrimitiveUP`,
`GetTransform`, `GetDirect3D`, `GetDeviceCaps` and `GetDisplayMode` on the device (D3DX8)
and `GetAdapterDisplayMode`; stock calls `GetBackBuffer`, `GetRenderTarget`, `GetViewport`
and the cursor's `SetCursor`, which Roots does not. D3DX8 asks `CheckDeviceFormat` for
DXT1 to DXT5 and Display8's stub answers not available, so D3DX8 decompresses Roots' DDS
art into uncompressed textures: it works, at four to eight times the memory. The shell's
1200x900 request gets Display8's one mode. The game window, the shell and a skirmish
run; entering the skirmish shows a white screen, the same as stock's current
device-re-creation bug, and was not chased here. Neither `dxwrapper.dll` nor its
`Magnification_Roots.dll` is loaded under Display8.

### The install

| | Roots | stock |
|---|---|---|
| Layout | the game in `Data/` under a Delphi launcher, `FleetOps.exe`, which only starts `Data\ArmadaL.exe` | the game at the root |
| Heroic | sideloaded app `fleetopsroots50`; Proton-CachyOS; prefix `~/Games/Heroic/Prefixes/Star Trek Armada II Fleet Operations Roots`, never run yet; `WINEDLLOVERRIDES=winmm=n,b;d3d8=n,b;d3d9=n,b`, `PROTON_USE_WOW64=1`; DXVK auto-install on. Heroic is the Flatpak here (`~/.var/app/com.heroicgameslauncher.hgl/config/heroic`) | GOG app `1174788223` |
| Direct3D chain | `d3d8.dll` is DxWrapper's stub, `dxwrapper.dll` runs its d3d8-to-9 conversion (`D3d8to9=1`), then `d3d9.dll`: DXVK as installed (byte-identical to the one in the stock game), with ReShade and a plain wrapper shipped as alternatives | Display8, then d3d8to9, then DXVK |
| ASI loader | none; `winmm.dll` in `Data/` would be loaded, as stock's | Ultimate ASI Loader as `winmm.dll` |
| Textures | `Textures/` (no `RGB/`): 911 TGA and **548 DDS** (DXT1, DXT3, DXT5 and uncompressed) | `Textures/RGB/`, 2115 TGA |
| ODFs | **packed in `ODF.fpq`**, Fleet Operations' own archive, read through `FleetOpsHook`'s file-system hooks; `odf/` is empty | 1101 loose files |
| HUD layout | `misc/gui_*.cfg` with aspect variants (4:3, 5:4, 16:10, 16:9; the 16:9 canvas is 2134x1200) | one 1600x1200 canvas |
| Movies | eight Bink files under `bitmaps/single/`, no `animations/` | `animations/` |
| Settings | `settings_saves_logs/Settings.xml` (`displayWidth`, `displayHeight`, `multisample`, `WindowMode`, ...); no `ARMADA.PRF` yet | `ARMADA.PRF` |
| Submods | `Data/Mods/`: "Fleet Ops 4.0", with **its own `ArmadaL.exe`**; "STA2 Classic Mod", `standalone=1` but with no executable of its own (it runs on Roots' `ArmadaL.exe` with `/mod "STA2 Classic Mod"`; corrected in FO-5 step 6); and "STA2 Classic Mod Enhanced Gameplay", data only, its `ParentMod` the STA2 Classic mod | — |
| `RTS_CFG.h` | 568 lines against 511, with keys of its own (`unitcap_*`, `sr_*`); `cfgSCREEN_WIDTH` still 1600 | — |

An older Fleet Operations install (non-Roots, its game named `Armada2.exe`, with its
own prefix) is also on this machine. It is out of scope.

### Multiplayer

Roots keeps stock's lobby layer (`NetworkManager.dll`, DirectPlay 8 and GameSpy) and
adds Tunngle. Its file check is its own: the engine's "files do not match" messages
remain, but `FleetOpsHook` computes the CRC over its own file system (the FPQ archive
included) with a list of allowed executable CRCs, and Roots' readme says `RTS_CFG.h` is
in it. No scan of the game directory for foreign DLLs was found.

What follows for us:

- **Visual plugins are invisible to peers**, as on stock: Display8, render3d and the UI
  write nothing the simulation reads and touch no checked file. To verify in a two-game
  bench match (FO-8).
- **The rules plugin must not run in a Roots game**: Roots has its own balance, and
  `PayOnQueue` changes what every node simulates. It stands down under Roots.
- **QOL's local features collide with Roots' own** where Roots detours the same
  functions (group selection, the keyboard handler). Those stand down until each is
  measured beside Roots' version.
- **Under Proton, Roots' multiplayer has the stock problem**: Wine's `dpnet` stubs
  `Host` and `EnumHosts`. `Online.asi`'s own peer could carry Roots games too, since it
  replaces the transport and leaves the game's messages (Roots' CRC packets among them)
  as they are. That is a measurement, not a promise.

## The approach

**Roots is an Armada II build with a content mod on top.** The profile key is
(game, build) from the start, so Armada 1 (`game = armada1`, in this repository's `armada1/` folder) does not change the format again:
`game = armada2`, `build = fo-roots-5.0Z11`, identified by timestamp **and image size**
(the timestamp alone is stock's), with the hash recorded by the bench. The Roots profile
**inherits** the stock profile and overrides only what differs: the shell mode, and
each symbol in a function Roots changes, marked as such.

The rule for a marked symbol: **a hook that uses one stands down under Roots until it
has been verified beside Roots' change**, and the profile entry then records that it
was. That turns the 210 changed functions, and whatever `FleetOpsHook` changes at run
time, into a list worked through one hook at a time, rather than a hope that the
signatures catch it. Signatures alone would not: all 61 of QOL's pass in Roots, yet four
of its functions are detoured.

**A content profile for Roots** carries what render3d and the UI know about the game's
data: ODF names, the race lists, the sky, surface and ordnance recipes. A missing recipe
means the engine's own look through passthrough, as everywhere.

**Roots' own files are Roots' and stay untouched**: `FleetOpsHook.dll`, DxWrapper,
ReShade, the FPQ archive and Roots' art. Everything we install goes beside them with a
backup, as on stock. Hard rules 8 and 9 apply to Roots as to Armada II: nothing derived
from its files is committed, and what we learn of `ArmadaL.exe` or `FleetOpsHook.dll` is
written per hook. Observing the running game is preferred over reading
`FleetOpsHook.dll`; its map stays where it is.

## What runs on Roots, and what stands down

| Layer | Under Roots | Why |
|---|---|---|
| display (Display8) | **yes**, in place of DxWrapper's `d3d8.dll` (backed up). Display8 owns the device on Vulkan (or Direct3D 12); when neither starts it bails out to the system's own `d3d8.dll`, as on stock, not to DxWrapper. Roots as it ships (DxWrapper, then DXVK) is the reference it is compared with, in a clone | Display8 is the seam everything else draws through. Roots' draws go through passthrough |
| render3d | **yes**, module by module | The seam is untouched. Modules whose sites Roots detours (hulls, effects' construction and explosions, shields) are verified one at a time |
| ui | the Refit screen, settings and the loading screen first; **HUD panels and the menu host off** at first | Roots changed its HUD (its own sprites and canvases) and its shell (1200x900, the dialog procedures); each part comes back when its specs are written for Roots |
| qol | pan speed and the camera; **group, keyboard and station features off** until measured | Roots detours the same functions |
| qol rules | **off** | must-match, and Roots has its own rules |
| online | not at first; measured in FO-8 | Roots' lobby and CRC ride on top of the transport |
| cutscenes (binkproxy) | yes, if the measurement passes | `FleetOpsHook` imports `binkw32` too; the proxy must forward every export it uses |
| hud, grid, menus (legacy) | **no** | Roots has its own widescreen HUD layouts and shell; the legacy plugins assume stock's |
| textures, models, menus backdrops, movies | **no** for now | the recipes are stock-hashed (`a2tex stock` refuses Roots' files, so nothing is installed by accident); Roots' art is mixed TGA and DDS |
| platform | not installed on Roots; the ASI loader is the `asiloader` layer (`Data/winmm.dll`, with Display8 and only with it: beside DxWrapper it stops Roots at start); DXVK left as Heroic installs it | FO-5 step 3: nothing loaded our `.asi` files on Roots before it (Roots ships no loader; DxWrapper loads them only with `LoadPlugins=1`, and is not loaded once Display8 is `d3d8.dll`) |

## Work packages

Estimates in ROADMAP units: agent hours, bench passes, sign-off round. FO-0 and FO-1
can start now; FO-3 onward waits on phase 8's format and generator (8b).

| WP | What | Depends on | Agent time · bench · round |
|---|---|---|---|
| FO-0 | **Roots as it ships, under Proton**: the user launches it once from Heroic (it creates the prefix and `Settings.xml`), plays the front end and a skirmish. Note anything Roots' launcher or first run asks | nothing | none · none · a user check |
| FO-1 | **A game profile for the tooling**: `a2env` learns a profile (`A2_PROFILE=fo-roots`, or `--game` on each tool) that sets the game directory, the data directory (`Data/`), the executable (`ArmadaL.exe`), the prefix and the Heroic entry. The bench gets a game adapter: launch `ArmadaL.exe` directly in `Data/` (not the launcher), process matching by its name, a `Settings.xml` adapter in place of the `ARMADA.PRF` one, Roots' log files. `a2mod` and `./install` take the profile and offer only the layers above. A stock clone of Roots for `--install`. Scenarios `fo-launch` (to the main menu) and `fo-skirmish` (a map loaded, a tick logged). The profile mechanism is built once for every game: Armada 1 adds its own profile to it (`PLAN-ARMADA1.md`, A1-1). **The mechanism is done** (AR-2 of PLAN-ARCHITECTURE.md): a `games/<id>.toml` descriptor every tool reads ([`games/README.md`](games/README.md)), `A2_PROFILE` and `--game` on `a2env`, `a2mod`, `./install`, `a2test` and `tools/layers.py`, and a stub `games/fo-roots.toml` (not offered) whose paths resolve, with `Data/ArmadaL.exe` as its executable; the resolver names the executable by the matched build or the module. Left for FO-1: the descriptor's facts checked against FO-0's install (the folder name is assumed), the `Settings.xml` adapter, Roots' log files, the layers on offer, the stock clone and the two scenarios | FO-0 | 8–16 h · 5–8 · none |
| FO-2 | **Measure Roots at run time**, in bench clones only. Display8 has no forward mode and no Direct3D 9 any more: it owns the device or bails out to the system's `d3d8.dll`, so Roots is measured twice, as it ships (DxWrapper, then DXVK: the reference) and with Display8 in DxWrapper's place, tracing (`Trace=1`). (1) What `FleetOpsHook.dll` rewrites of `ArmadaL.exe` in memory: `testbench/memdiff/` (`MemDiff.asi`, through the ASI loader as `Data/winmm.dll`, in the clone only) compares the image's sections with the file at its load and at fixed times after, and logs each changed range with the module its jump leads into; the ranges are named by function offline, against the hook set the profiles have today, and the load order of `FleetOpsHook` against the ASI loader is read off the same log. (2) Whether `FleetOpsHook` makes a Direct3D 9 device of its own or hooks the d3d8 device (it imports `d3d9`, `D3DX81ab` and `d3dx9_43`): observed, from the modules loaded, DXVK's own log, and the callers in Display8's inventory. (3) Roots' Direct3D 8 call inventory on Display8 against stock's (`display-inventory`): block-compressed textures, D3DX8's use of the device, the 1200x900 shell mode, any call stock never makes. **Done 2026-11-06**: "FleetOpsHook.dll at run time", above | FO-1; ROADMAP 8a (the identity log) | 6–10 h · 3–5 · none |
| FO-3 | **The Roots profile**: (game, build) in the format, inheritance with overrides, the "changed by Roots" mark and its stand-down rule; the marked list from the static diff and FO-2's run-time diff; `a2test syms --game fo-roots`; the negative test (the Roots profile on stock and the reverse, both standing down cleanly). **Begun 2026-11-06**: `profiles/fo-roots-5.0Z11/` inherits gog-1.1 (timestamp, image size `0x405000`, section fingerprint) and marks the 30 functions of the file diff and 80 more of the run-time diff that our hooks use (49 of 103 hooks stand down, 54 ready; the rules plugin off); five callees FleetOpsHook takes over drop their entry check; `tools/engine/negative.sh` (e) tests it both ways | ROADMAP 8b; FO-2 | 6–12 h · 2 · none |
| FO-4 | **Display8 on Roots**: stub and passthrough on Vulkan with whatever FO-2 found (block-compressed textures sampled natively, the D3DX8 calls, the 1200x900 shell mode, Roots' `FindDisplayMode` change). If `FleetOpsHook` draws through its own Direct3D 9, that is solved here or Display8 is not installed on Roots (Roots keeps DxWrapper). Installed by `./install --game fo-roots` with DxWrapper's `d3d8.dll` backed up. **Ready for round FO-A 2026-11-06** (branch `fo-a-display`): `display.modes` verified beside Roots' changes, so Display8 starts on Vulkan instead of bailing out; on the bench (1280x960, the GPU, the validation layer on) the shell, the skirmish's second device (followed) and the return to the shell draw through the passthrough with no validation message and no crash, D3DX8 keeps Roots' DDS art as BC1 to BC3, and FleetOpsHook's bump-mapped hulls draw through the port of the Borg bump shader; the skirmish looks as Roots on DXVK. `display.native` and `display.drop` stand down (not needed: the mode is `Settings.xml`'s, nothing is dropped). Not yet seen: the intro (FleetOpsHook plays it through a Direct3D 9 device of its own; the bench sets `IntroVideo` off), Direct3D 12 (its second window is black: `Dx::rebind`), the player's own install | FO-2, ROADMAP 1 signed off | 10–25 h · 10–20 with A/B against Roots as it ships (DxWrapper, then DXVK) · **round FO-A**: Roots looks as Roots on our device |
| FO-5 | **render3d on Roots**: designed 2026-11-06, "FO-5: render3d on Roots, the design" below: render3d keeps the frame and its steps; the object step calls through Roots' own per-object draw; hulls claim at the instance's database draw, not at the object, so Fleet Operations' wrappers (range rings, overlay models, highlight, node and texture swaps) keep running; where FleetOpsHook owns a call site, render3d takes over at the callee instead; the Roots content set reads `ODF.fpq` through `a2fmt::vfs` | FO-3, FO-4 (a skirmish drawn on Display8); ROADMAP 7 | 45–75 h · 20–30 · **round FO-B**: a skirmish with the refit's look |
| FO-6 | **UI on Roots**: the Refit screen and settings over Roots' Options dialog, the loading screen; then the menu host's specs for the 1200x900 shell and HUD panels against Roots' components, each its own row, as wanted. The button bar is not stock's: its command area can be far larger (several weapons of one classlabel, each a button; FO-7), so the UI's bar and its labels (`ui/HUD.md`, "The button bar") are measured on Roots before they are drawn there. **Planned 2026-11-06**: "FO-6: the UI on Roots (plan)" below, which re-estimates it | FO-3, FO-4; ROADMAP 6 | 35–60 h · 15–23 for round one (80–145 h with the bar, info panel, minimap and menus) · with FO-B, then its own round if panels are taken |
| FO-7 | **QOL on Roots**: pan speed and camera first; each of the other features measured beside Roots' own and switched on only where both agree. TamsynnImogen (Blade, the author of A2FOExtensions) noted (2026-11-06) that Roots' command button area can be far larger than stock's (several weapons of the same classlabel each get a button), so `BarKeys` and the slot plan (`qol/README.md`, QOL-1) cannot assume stock's bar | FO-3 | 4–10 h · 3–6 gameplay passes · with FO-B |
| FO-8 | **Multiplayer**: two bench games of Roots with the refit's visual plugins, on Microsoft's DirectPlay in the clones (`online/reference-dplay.sh`), to show the CRC check passes; then whether `Online.asi`'s peer carries a Roots game | FO-4; ROADMAP 9 (the rules plugin as its own layer) | 6–12 h · 4–8 two-game passes · round FO-C if Online is taken |
| FO-9 | **binkproxy** on Roots' intros. Roots support is not packaged: no GUI installer or release package carries it (Open decisions, 4) | FO-4 | 2–5 h · 2–3 · none |

Summed: about 75–165 agent hours, three sign-off rounds of its own (FO-A, FO-B, FO-C).

**Where it sits in ROADMAP**: FO-0 and FO-1 now, beside rounds C and D. FO-2 needs only
8a. FO-3 is phase 8's second profile and the test of its design, so it lands with or just
after 8b–8d. FO-4 needs passthrough signed off (phase 1). FO-5 is best after phase 7, as
8e is. FO-8 after phase 9. The legacy plugins phase 11 removes are never installed on
Roots, so phase 11 changes nothing here.

## FO-5: render3d on Roots, the design

Designed 2026-11-06 (branch `fo-5-design`), from a reading of `FleetOpsHook.dll` named by
its shipped map, the file and run-time diffs of FO-2, Roots' ODFs read out of `ODF.fpq`
with `a2fmt::fpq` (PR #60), FO-2's Display8 inventory, and one bench session of Roots as it
ships. It is built for **`FleetOpsHook.dll` as it ships and as it will change**: nothing in
it waits on the Roots developer or the Fleet Operations team (user's decision, 2026-11-06; as far as the user can tell, the Fleet Operations team has abandoned the project, and Roots, updated as recently as 5.0Z11 of 2026-10-23, is the live party). The detail that would
read as FleetOpsHook's internals is in the private notes (`fo5-design-notes.md` beside
`fleetopshook.md` in `$A2_DATA/research/fo-roots/`); here each finding is stated per hook.

### Why render3d stands down on Roots today

render3d does not run the engine's `Armada_RenderAllOurStuff`: the seam re-points its two
calls (0x59886e, 0x4e60f6) and `frame/steps.rs` runs the body as render3d's own steps, the
object loop included. Both seam calls are untouched by Roots, but FleetOpsHook rewrites
the object loop **inside** the body (0x598295 to 0x5982a5: the stock "drawn unless culled,
then `Render`" becomes one call into FleetOpsHook per object). The profile marks the whole
function `runtime`, so `frame.seam` and `frame.steps` stand down, and every module with
them. Unmarking it would not be enough: render3d's own loop would silently drop what
Roots does per object, and render3d's hulls, by claiming an object, would skip Roots'
wrappers of `GameObject::Render` and `Craft::RenderInternal` as well.

### What Roots draws, and in which order

Per object, inside render3d's object step (stock order otherwise unchanged):

| Where | What Roots does there | Seen |
|---|---|---|
| the object loop (0x598297, FleetOpsHook's `Armada_RenderAllOurStuff_GameObjectRender_Enhancement`) | draws the object through its own `Render` (vtable +0x2c) unless the engine culled it for distance this view (the stock byte +0x183), **or** its class sets the ODF key `renderIgnoreCameraDistance`; then, for a craft, calls an optional per-craft hook that nothing in FleetOpsHook sets | code; neither feature is used by 5.0Z11's ODFs |
| `GameObject::Render` (trampoline, `Gameobject__Render_Callback`) | for the player's own objects of a class with `selectHighlight`, while selected or under the pointer, raises Storm3D's ambient to the team colour (x0.3, x0.2) around the stock call | code; key unused in 5.0Z11 |
| `GameObject::RenderInternal` (`CloakingDeviceImp`'s two calls; `.test` wave-light detours) | Roots' cloak decides the vertex-alpha path; the hull itself is still drawn by the one stock call of `Entity::Render` (0x4d5418) | code |
| `CraftInstance::RenderInternal+0x238` (`CraftInstance_RenderInternal_Enhancement`) | just before the instance's database draw: one node per craft class shown with a per-ship texture, or hidden (the node's render flags +0xbc, bits 0 and 1), written into the shared database as stock's own node logic is | code |
| `ST3D_Dot3_MeshVB::Render` (trampoline, Direct3D 8 path only) | **draws every bump-mapped mesh itself**: one pass per light, the base texture, the glow; the engine's Dot3 vertex shader, its buffers, raw `IDirect3DDevice8` calls. No environment or reflection pass | FO-2's inventory: 170 indexed draws a frame (34 meshes x 5), every one under `GameObjectInstance::RenderInternal` → `ST3D_Instance::RenderInternal`; Display8's passthrough recognises the shader as the engine's Borg bump shader |
| `Craft::RenderInternal` (trampoline, `Craft_RenderInternal_Callback`) | **after** the stock body: (1) the craft's `overlaySod` model, a second instance placed on the ship's placement (12 classes: `effect_sensorarray`; one 8472 class); (2) **range rings** while the HUD asks for them: a red ring for the weapon whose button is under the pointer, for each selected craft, and a green or cyan and red ring while the pointer is over a subsystem icon of the player's ship; drawn as engine lines (`DrawCircleXZ`) | code; **bench, 2026-11-06**: hovering a construction ship's Tractor Beam button draws a red ring of its 400 range round the ship; hovering its Weapon Control icon draws a cyan ring |
| `Craft::Explode+0x14c` (`Craft__Explode_ST3D_Instance__Fragment_Callback`) | calls `ST3D_Instance::Fragment` only when the class's `explosionChunks` (default on) allows | code; key unused in 5.0Z11 |
| the five `DrawHeightPost` calls (`DrawHeightPost_Callback`) | skips the posts while FleetOpsHook takes a clean screenshot, and when `RTS_CFG.h`'s `DISABLE_HEIGHT_CHANGE` is set (unless a profile flag forces them); otherwise calls the stock `DrawHeightPost` (0x593840) | code |
| `SelectionEffect::Render` (trampoline) | skipped during a clean screenshot | code |
| `ShieldHit::DrawShield` (14 writes) | colours the hit by damage when `SHIELD_EFFECT_DAMAGE_COLORED` (0 in Roots), and scales the shell by the instance's scale effect when `SHIELD_EFFECT_RESCALE` (1 in Roots); both `RTS_CFG.h` keys | code |

A third-party project's claim that FleetOpsHook's craft callback draws "shields,
reflections and special effects" after the base draw is **refuted as stated**: it draws
the `overlaySod` model and the range rings; shields are `ShieldHit::DrawShield`, and the
Dot3 replacement has no reflection pass. Its other leads, checked by us: the node hide
bits are real (above), and Roots' own file detours `CraftClass::SetBorgMeshTextures`
(into `.test`) in 5.0Z11 as well.

### The three models weighed

| | (a) render3d re-implements Roots' loop and wrappers | (b) render3d leaves the body to the engine and hooks per category | (c) render3d keeps the frame; Roots' wrappers keep running; hulls claim at the instance draw |
|---|---|---|---|
| How | render3d's object step reproduces the culling exception, the per-craft hook, the highlight, the overlay model, the rings, `explosionChunks`, the clean-shot gates | the seam calls the stock body (Roots-edited) and modules hook call sites inside it, as the retired per-part plugins did | the object step calls the per-object draw **Roots put in the loop** (read from the call at 0x598297, as `steps.rs` already calls through every other site); hulls draw at `GameObjectInstance::RenderInternal`'s call of `ST3D_Instance::RenderInternal` (0x4d5a11), after stock's and Roots' per-instance writes; where FleetOpsHook owns a call site, render3d takes over the **callee** instead |
| Correctness | only as good as our copy of FleetOpsHook's private records (the craft record, the class records, the ring flags), none of which has a symbol of ours | the engine's order, but render3d's steps (lights before the view, the scene layer, the translucent step, per-view state) are its design; a second render3d for one build | Roots' logic runs as Roots wrote it; render3d decides only *how* a hull, a chunk or a post looks |
| A Roots update | breaks silently: a moved record field draws wrong rings or none | each site re-verified, as for (c) | the profile re-marks what moved (`fo-scan.py`); a hook whose site changed stands down; FleetOpsHook's wrappers are called, never read |
| What the player gets | everything render3d does | a fraction: no ownership of order, no scene layer | everything render3d does, with Roots' rings, overlay models and swaps drawn by the engine through Display8's passthrough **into render3d's target**, so tone-mapped and bloomed with the rest |

**Recommended: (c).** It is the one model in which FleetOpsHook stays a black box: render3d
calls what Roots put in place and reads nothing of FleetOpsHook's own memory. Its cost
is one new claim point for hulls and a handful of call-site hooks moved one level down.

### The rules of (c)

1. **The frame is render3d's** on Roots as on stock: the seam (both calls untouched),
   the scene target, lights, the scene layer, bloom, the tone map, MSAA. Display8's
   passthrough draws whatever render3d does not claim into render3d's published pass.
2. **Each step calls through its site.** The object step on Roots calls the function at
   the loop's call site 0x598297 (a call into FleetOpsHook, accepted as `chain`), once per
   object that is not a background object; that function makes the cull decision and calls
   `Render`. On stock there is no call there and the step keeps its own loop, unchanged.
3. **Hulls claim at the instance's database draw** on Roots, not at the object:
   `registry::object` does not take a Roots object; the engine's `Render` runs with every
   wrapper; at 0x4d5a11 the hulls module draws the instance it posed instead of the
   database, reading the per-instance node flags and mesh textures that stock and Roots
   wrote just before. **The owner**: the instance holds no pointer to its object, so the
   object step's call at 0x598297 sets a "current object" around the chained draw (it is
   synchronous, one object at a time), and the claim accepts the instance only when it is
   that object's own (`object+0x4`, the entity's instance, equals the instance at the call);
   that skips planets (reached through the craft instance), the overlay model, holograms and
   other modules' objects. The race comes from the object's ODF as on stock. Construction
   rigs: FleetOpsHook gives them the common `GameObject::Render`, so the stock rig draw is
   never called from the loop and rigs reach the claim like any object; the core's rig read
   (`core.reads`) stands down on Roots with its `ConstructionRig::Render` mark. Holograms
   and the overlay model do not pass the call and are left to the engine. Everything the engine draws around it (the overlay model, the rings,
   holograms, sprite nodes, wave lights) is the engine's, through passthrough; hulls' own
   versions of those stand down on Roots. FleetOpsHook's Dot3 draws are never reached for
   a claimed hull; for one hulls does not take, Display8's passthrough draws them (its
   port of the engine's bump shader).
4. **Where FleetOpsHook owns a call site, render3d takes over at the callee**, armed only
   during the chained call, so Roots' gate decides *whether* and render3d *how*: the debris
   at `ST3D_Instance::Fragment` (0x62eb20, around `Craft::Explode+0x14c`), the height
   posts at `DrawHeightPost` (0x593840, around its five sites). Neither callee is touched
   by Roots.
5. **What render3d reads of Roots is content, not FleetOpsHook's memory**: ODF keys and
   `RTS_CFG.h` values through `a2fmt::vfs` (the archive, then `Data/`, then a mod), in a
   content set of its own.

### The modules under (c)

| Module | On Roots |
|---|---|
| core (seam, steps) | the seam as on stock; `frame.steps` verified beside Roots' loop edit with the object step calling through 0x598297; `site_seam_frame`'s function (`Armada_RenderFrame`, FleetOpsHook's +0x245 edit removes an edit-menu call, not at our sites) verified as is; `fn_new` called unchecked (FleetOpsHook's allocator, as profiled) |
| hulls | the instance-level claim (rule 3); a Roots look keyed by race, not by "bump-mapped": in Roots almost every hull is bump-mapped (34 of 34 meshes in a skirmish), so the stock rule "bump-mapped means Borg" would give every hull the Borg profile; the normal map from Roots' own height maps as for the Borg; the highlight read from Storm3D's ambient at the claim point; cloak from the stock fields, verified beside Roots' cloak |
| sky | the step as on stock; Roots' backgrounds are its own art, so no built-in recipe matches and the module draws the map's own faces or cube, stars and background planets itself (decision 40); `Starfield_Load_Background_Geometry` (nine bytes nopped at +0x16) verified (step 7) |
| particles | `site_billboard` sits in `GameObject::RenderInternal` (Roots' cloak and wave-light edits): verified (step 7); the step as on stock |
| effects | debris at the callee (rule 4); shields follow `SHIELD_EFFECT_RESCALE` (the instance's scale effect) and `SHIELD_EFFECT_DAMAGE_COLORED` from `RTS_CFG.h`; the four `CreateShieldHit` taps verified beside Roots' `CreateShieldHit` edit and `Beam::Simulate`'s node-particle call |
| ordnance | as on stock for the stock classes (`cPrjID::GetOdfName` called unchecked: Roots' name table); Roots' own ordnance classes (registered by FleetOpsHook before `WinMain`) are left to the engine until each is surveyed; `PhotonTorpedo`'s `Hit` slot is Roots', which ordnance does not use |
| specials | keyed by ODF as on stock, Roots' specials named in the content set; Roots' new special-weapon classes left to the engine until surveyed |
| world | planets, moons, asteroids, nebulae: the step as on stock, Roots' nebula classes (`LatinumNebula`, `UtritiumBall` are edited) verified and named in the content set |
| overlays | posts at the callee (rule 4), which also keeps Roots' clean screenshot; the selection drawn by render3d (Roots' clean-shot skip of `SelectionEffect::Render` is lost in screenshots only, accepted); the grid colour is Roots' own (`DrawGrid` reads it from FleetOpsHook), read at run time from the pointer the function holds |

### Identity

Roots' builds cannot be told apart by their headers. Our `ArmadaL.exe` (5.0Z11, menu
"5.0.0.Bob") has the same timestamp, image size (0x405000) and file size (4,816,251) as
another Roots build documented elsewhere, and different `.text`, `.rdata` and `.test`
bytes; our `FleetOpsHook.dll` has the same timestamp, image size and file size as two
other builds and different contents. The section-header fingerprint in `build.toml` is the
same for all of them. So:

- the build identity adds **CRC-32s of `.text` and `.test` read from the file on disk**
  (FleetOpsHook rewrites `.text` and `.rdata` in memory before any plugin of ours runs;
  `.test` it leaves alone), for builds that set them;
- the build names a **companion module**, `FleetOpsHook.dll`, by the CRC-32 of its own
  `.text`: the marks of `fo-roots-5.0Z11` describe that FleetOpsHook. With another one
  loaded (or none, or one that cannot be read), every hook marked `runtime` stands down, as an
  unknown build's would, and so does every hook that calls through the module (a chained
  call) or whose callee check the build drops: built, and tested with a foreign copy, in
  step 1.

### Work list

| Step | What | Agent h · bench |
|---|---|---|
| 1 | Identity: `[pe] file_crc` (on-disk `.text`, `.test`) and `[[module]]` (FleetOpsHook's `.text` CRC) in `a2engine`, `gen.py`, the C twin and `fo-scan.py`; the negative test with a foreign FleetOpsHook. **Done 2026-11-06** (branch `fo-5-identity`; crates/a2engine/README.md, "Identity beyond the header") | 4–6 · 1 |
| 2 | **Done 2026-11-06**, as profile data (`planned` hooks, README rows): Profile: `site_obj_draw` (0x598297, `call`, `chain`, Roots only), `site_hull_draw` (0x4d5a11, `call` to `ST3D_Instance::RenderInternal`, in `GameObjectInstance::RenderInternal`, which neither Roots' file nor FleetOpsHook changes); the callee hooks for `Fragment` and `DrawHeightPost`; `verified` on `Armada_RenderAllOurStuff` for `render3d.frame.*`; `fo-scan.py --compare` | 3–5 · 0 |
| 3 | Core: the object step calls through the loop's call site when it is one; stock's loop untouched (the stock set must not move a pixel). **Built 2026-11-06** (branch `fo-5-step3`): `frame.obj_draw` ready on Roots (logged chained to FleetOpsHook), the step calls it per object with the object current around it; the Roots marks that held the core, the seam and the steps (`ConstructionRig::Render`, `Armada_RenderFrame`, `Terrain_Geometry::Render`) verified; Render3D on Roots through the `asiloader` layer and `./install --game fo-roots` (display, asiloader, render3d) | 2–4 · 2 |
| 4 | Hulls: the instance-level claim, chosen by the profile (a `value`), snapshotting node flags and textures at the claim; hulls' own sprite nodes, holograms and wave lights off on Roots; the race-keyed look. **Built 2026-11-06** with step 3: `hulls.instance_draw` (`hl_claim_instance` = 1 on Roots, 0 on stock, which installs nothing), the race-keyed profile and normal-map bump textures in the `fleetops` set (hulls only), Roots' files (`Textures\` with DDS, `ODF.fpq`) in render3d's file system. **Finished in step 5**: at the claim the node flags and mesh textures that the engine and FleetOpsHook wrote into the shared database for the instance are honoured (a hidden node is hidden; Roots' `stlogo` node is drawn with the ship's own logo texture); the hulls' sprite nodes stay on and wave lights stay ours (decisions 19 and 20 below) | 8–14 · 4–6 |
| 5 | Effects and overlays: the callee hooks; shields from `RTS_CFG.h`; the grid colour. **Built 2026-11-06** (branch `fo-5-step5`): a callee hook is a new hook kind (`Hooks::install_entry`): the function's first whole instructions, a profile value within the bytes its check vouches for, moved to a trampoline, and the detour armed only while the caller's return address lies in the build's companion module (`FleetOpsHook.dll`, by `[[module]]`), so Roots' gate decides whether and render3d how. `effects.debris_callee` (`ST3D_Instance::Fragment`, `ef_debris_at_callee` = 1 on Roots) takes a dying craft's debris; the cancelled construction and the Evolver's cocoon, whose calls are the exe's, stay the engine's on Roots. `overlays.posts_callee` (`DrawHeightPost`, `ov_posts_at_callee` = 1) notes the posts FleetOpsHook's two callbacks draw (none in 5.0Z11's skirmish: `DISABLE_HEIGHT_CHANGE` = 1). Shields: `SHIELD_EFFECT_RESCALE` = 1 scales the shell x2 (`ef_shield_rescale_pct` = 200, the factor FleetOpsHook applies), `SHIELD_EFFECT_DAMAGE_COLORED` = 0 draws stock-coloured hits white, both read from `Data\RTS_CFG.h` (`rtscfg`). Grid: the colour pointer `DrawGrid`'s first push holds (`site_grid_colour`), read every frame. The Roots marks of `ShieldHit::DrawShield`, `ShieldEffect::CreateShieldHit`, `Beam::Simulate`, `Armada_RenderAllOurStuff` (effects) and `Terrain_Geometry::DrawGrid` (overlays) are verified, so effects and overlays run on Roots. Stock: values 0, nothing at either entry, `RTS_CFG.h` unread, the grid's pointer is stock's dark orange | 5–9 · 3–4 |
| 6 | Content set `fleetops`: an ODF reader over `a2fmt::vfs` (`ODF.fpq`, then `Data/`, then the active mod; Roots' inheritance key), races, ordnance and special ODF names, nebula and planet classes, the two `RTS_CFG.h` keys. **Built 2026-11-06** (branch `fo-5-step6`). *The mod order*: Roots' file system takes the active mod from the first `-mod <name>` or `/mod <name>` on the command line (Settings.xml has no mod key; FleetOpsHook's own mod dialog relaunches with the switch), a folder `Data\Mods\<name>\` with an `info.ini`, and finds each file by name in that mod, then its `ParentMod`'s up the chain, then `Data\` unless a mod in the chain is `standalone=1`; each folder's `ODF.fpq` and loose files. render3d mounts its whole file list once per folder of the chain, each a level above the one under it (`a2fmt::vfs` `mount_level`; `render3d/core/src/files/mods.rs`, `games/fo-roots.toml` `[files] mods`), indexes names source by source (a name is the top folder's, wherever it sits), and reads `RTS_CFG.h` through it (`data\RTS_CFG.h`, replacing decision 23). *Roots' inheritance key*: an ODF that includes its own name inherits its next version down (`a2fmt::odf` `load_layered`), as Roots' parser does. *The set*: `profiles/fo-roots-5.0Z11/content.toml` names `fleetops` for all of render3d (until now every module but the hulls read the **stock** set on Roots, inherited from gog-1.1's `content.toml`: decision 16's "none" was only build.toml's fallback); `effects.ini` keys shield factions by race (`[Races]`), `ordnance.ini` names which Roots ordnance ODF each stock recipe draws (`[Names]`, 15 ODFs and 2 waves; Roots' `ionstm`, a utritium beam, no longer takes stock's ion-storm-torpedo recipe), `world.ini` gives Roots' nebula classes the stock nebula recipe of the same `nebulaType` and hue and leaves `latinum` (Roots' asteroid fields) and every surface out, `particles.ini`, `overlays.ini`, `sky.ini`, `specials.ini` the names Roots keeps or none. Decisions 25–33 below; render3d/README.md, "Content sets" and "Files". *Bench* (Roots, Display8 on Vulkan with the validation layer, Render3D, 1280x960, Klingon Instant Action): `files: no active mod (no -mod or /mod on the command line): the game's own folder alone`, `files: ODF.fpq mounted at odf: 1799 files`, Roots' Mutara, Crystalid and Nucleon sets `drawn as gas` and its asteroid sets (`latinum`) left to the engine, `ordnance: all_mine_dilithiumo.odf (...): Recipe(23)` with `drawing special minedilithium (Beam)` (the miner on a dilithium moon, the beam in the shot) and `npc_tractorbeamo.odf (...): Recipe(3)` with `drawing special tracbmo (Cone)` (the construction ship's Tractor Beam), no validation message. With `-mod R3DTest` (a test mod of ours over the STA2 Classic mod, in the clone) the game showed the STA2 Classic mod's own shell, so it takes the chain as render3d reads it; it then exited at Instant Action before render3d had read any file (no crash file), so render3d's mod mounts are shown by the host tests against the install, not yet in game. Stock `render3d-live-hulls` (`--install .`): PASS, pixel-exact. Not yet seen: a shield hit (the faction by race, `RTS_CFG.h` through `data\`), the nebula gas on screen (the bench's map hid them under the shroud) | 6–12 · 1–2 |
| 7 | Sky, particles, ordnance, specials, world: each verified beside Roots' changes and marked `verified`. **Done 2026-11-06** (branch `fo-5-step7`, "FO-5 step 7" below): `sky.reads`, `particles.billboard`, `ordnance.warp`, `view.distance` and `overlays.paths` verified; the move paths read the overview through `g_pOverView` (FleetOpsHook builds its own); specials and world had no marks. Every render3d hook is ready on Roots but the two call-site hooks the callee hooks replace | 8–16 · 6–10 |
| 8 | Scenarios: `fo-render3d-seam` (a skirmish, render3d on, no stand-down in the log), `fo-render3d-rings` (the Tractor Beam hover ring present with hulls on, as in the bench shot), one per module with the module off for the passthrough look; the stock set unchanged. **Built 2026-11-06** (branch `fo-5-step8`): `fo-render3d-seam`, `-rings`, `-debris`, `-grid` pass; `-shields` fails and is the finding (Roots' hits carry a model that is none of the shield database's, so render3d draws none of them and `RTS_CFG.h` is never read); `-stlogo` is a close-up only (no node was seen drawn with a database texture); the fight is the Hard AI's (no scene harness), and Roots has warp off (`GAME_OPTIONS_WARP = 0`, `cfg_MIN_MOVE_WARP_DISTANCE` 999999) | 4–6 · 3 |

About 45–75 agent hours and 20–30 bench passes, after FO-4 (a skirmish drawn on Display8;
FO-2 saw a white screen on entering one). Roots has no scene harness, so its scenarios
stage through the front end (Instant Action) and OCR, as `fo-skirmish` does.

### What we would ask the Roots developer, if ever (optional)

As far as the user can tell, the Fleet Operations team has abandoned the project; Roots
and its developer are the live party, and the `FleetOpsHook.dll` that ships with Roots
changes with Roots' releases. Nothing above depends on it. Each would replace a hook of ours with a call of theirs:
an exported "draw this object's extras" (overlay model and rings) for a renderer that
draws the hull itself; an export naming FleetOpsHook's build (a version number) for the
identity; a note when they move the object-loop edit or the craft wrappers.

## FO-6: the UI on Roots (plan)

Planned 2026-11-06 (branch `fo-6-plan`), offline: `./a2test syms --game fo-roots` on
`fo-b`, `tools/engine/fo-scan.py --compare` against `profiles/fo-roots-5.0Z11/`, the
private patch and collision tables beside `fleetopshook.md`, MemDiff's log of FO-2,
Roots' `misc/gui_*.cfg` (read for rectangles and counts, never copied), and the bench
shots of FO-4 and FO-5 (Federation, Klingon, Borg and Species 8472 in Instant Action at
1280x960). No game was run for it. Findings are stated per hook; the detail stays in the
private notes.

**The short of it.** The loading screen and the mission frame are nearly free: their
call sites are stock's in the file and in memory, and only the marks on the functions
around them hold them down. The HUD is not: FleetOpsHook replaces three of the objects
the UI reads (the info panel's `ShipDisplay`, the bar's button array, the resource
components) and draws gameplay feedback of its own off the stock components' mouse state
(the range rings), so several hooks that `a2test syms` calls ready would read the wrong
memory. The menus are the most changed part of Roots, and the last.

### The hooks, by UI part

`a2test syms --game fo-roots`: **40 of 61 UI hooks ready, 21 stand down** (every plugin:
70 of 107). "Ready" means only that no function a hook's entries sit in is marked: the
marks are per function, and a global or an object FleetOpsHook replaces is reported, not
marked (decision 17). So the table has a third verdict, **ready, wrong**: the profile
lets it in and it would read what Roots no longer uses.

| Part | Hook | `syms` | What FleetOpsHook (or Roots' file) changes there | Verdict |
|---|---|---|---|---|
| core | `core.storm3d`, `core.device`, `core.reads_mouse`, `core.reads_msvc`, `core.reads_clock`, `core.cursor`, `core.cursor_probe` | ready | nothing at their sites | ready |
| core | `core.mission` | **ready** (step 3) | `ChatDisplay::Render` is trampolined at its entry (Roots' clean screenshot); `Armada_RenderFrame`'s one edit (+0x245) is away from the call at 0x598a16, which is stock's | verified (decision 85) |
| core | `core.input`, `core.input_callee` | ready | the four sites are stock's, but **the game window is FleetOpsHook's own form**, whose window procedure calls `ProcessKeyboardMessages` and `ProcessMouseMessages` itself: the two `WindowProc` sites never run on Roots (measured, step 3) | Roots takes the two functions' entries (`core.input_callee`, decision 87) |
| core | `core.window` | ready | Roots makes the shell's device and the game's on two windows (`display/README.md`, "A second window"), both FleetOpsHook's forms on the game's thread; `GetWindowHandle` names the shell's form until its device goes, then the game's (measured, step 3) | ready |
| core | ~~`core.reads_hud_hidden`~~ | gone | `SetInterfaceState` writes the hidden flag into FleetOpsHook's own `ShipDisplay`; the stock global `shipDisplay` is not the panel any more | read from the panel the render lists name (decision 88) |
| core | `core.cursor_square` | down | `SetCursor`'s texture is FleetOpsHook's (enlarged `ST3D_Texture`); Roots sets a Win32 cursor and loads race-specific cursor sprites | leave down (decision 69) |
| core | `core.intro_frames` | ready | never reached: FleetOpsHook plays the intro on a Direct3D 9 device of its own | harmless |
| loading | `loading.overlay` | down | Roots' new load bar and its text (`RenderLoopAssetLoadStatusUpdate` +0x1b, +0x88, +0x343..+0x35c); the UI's call at +0x3b3 is stock's in file and memory | verify (cheap) |
| loading | `loading.own` | down | `Program::GameDoOne`: Roots' four-byte edit at +0x19b and FleetOpsHook's audio and close calls (+0x17c..+0x30a); the splash call at +0xcd and `LoadBarNext`'s call at 0x6019eb are stock's | verify (cheap) |
| loading | `loading.inner_db`, `loading.inner_pixels` | down | `FindLogicalDatabase`, `FindVisibleDatabase` and `ST3D_Texture::DemandLoad` are replaced whole: the bodies these sites sit in never run | down by design |
| loading | `loading.reads_progress`, `loading.reads_pump` | ready | the fraction and the pump are stock's (Roots' `LoadBarStart` callback only adds to it) | ready |
| hud | `hud.panels` | down | `DisplayInterface::RenderAll` is trampolined (clean screenshot); the call at +0x47 runs behind it and is stock's | verify |
| hud | `hud.reads_components` | ready | `g_pPopupPalette` and `mapRadar` hold Roots' objects; **`shipDisplay` does not** (FleetOpsHook constructs its own, larger) | **ready, wrong** for the info panel |
| hud | `hud.reads_bar` | ready | **the bar holds up to 64 buttons, not 21**: the inline array at +0x28 becomes a pointer to one of FleetOpsHook's, and the count in `PostLoad`, `Cleanup`, `Update`, `Clear`, `Render` and `CursorOver` goes from 21 to 64 (MemDiff); `mUpdateButtonsAndPosition` and the focus step are FleetOpsHook's, special weapons are laid out by its own function (one button per weapon, so several of one `classLabel` each get one); `cPrjID::GetName` is called unchecked (profiled) | **ready, wrong** |
| hud | `hud.reads_overlay` | ready | `SelectionDisplay`'s `PostLoad`, `Render` and `Cleanup` trampolined, 43 bytes of `PreRender` nopped, its tooltips and group colours FleetOpsHook's | verify |
| hud | `hud.tooltips`, `hud.reads_calls`, `hud.reads_popups`, `hud.reads_overlay_text`, `hud.bubbles`, `hud.fade`, `hud.height_posts` | ready | nothing at their sites (the tooltip box's one constant changed) | verify in game |
| hud | `hud.drag_box` | down | **direct clash**: FleetOpsHook owns the call at `mNormalModeRender+0x1ca` | stays the engine's (decision 74) |
| hud | `hud.order_ring`, `hud.footprints` | down | `mRenderWhere`'s post is FleetOpsHook's (+0x9d), and `DemandLoad`, which both only call, is replaced at its entry | drop the callee check, then verify |
| hud | `hud.video` | down | `CinematicView::mUpdateVideo`: FleetOpsHook's audio calls, away from the blits | verify |
| hud | `hud.frame_rate` | down | `Armada_RenderFrame`'s edit at +0x245, away from the prints | verify |
| info | `hud.info` | down | **the most changed panel**: its own object, the display type decided by FleetOpsHook (`DetermineSelectedDisplayType+0x99`), its own health bars, ship icons and wireframe icons, 26 to 30 bytes nopped in every single-object draw and simulate | stays Roots' (decision 66) |
| info | `hud.closeup` | down | the action camera's `Armada_RenderAllOurStuff` (verified for render3d only), and `operator new` with no override in `ui.info.toml` (`fo-scan --compare` found the gap) | with the info panel |
| info | `hud.unitview` | ready | **Roots has no unit view** (`cinematicPanelArea` is off in its layouts) | off (decision 67) |
| radar | `hud.radar`, `hud.radar_cursor` | down | `ActionMode::GetAction`, which they only call, carries an observer fix (+0x11e); `RadarComponent::Render` calls FleetOpsHook's map-object draw; team colours are Roots' | stays Roots' (decision 66) |
| panels | `panels.buttons`, `panels.toggle_press`, `panels.reads_selection`, `panels.reads_warp`, `panels.reads_race` | ready | the overview is FleetOpsHook's (read through `g_pOverView`, verified for render3d) | ready |
| panels | `panels.presses` | down | `DisplayInputProcess` changed elsewhere; the site at +0x1e9 runs as stock | verify |
| panels | `panels.resources`, `panels.topbar` | ready | `ResourceComponent` is larger and drawn by FleetOpsHook (its `Render` slot), `ResourcePanel::PostLoad` is FleetOpsHook's; Roots shows one to three resources and five top-right buttons, not stock's eight | verify, names in the set |
| panels | `panels.chat`, `panels.trade` | down | `Armada_RenderFrame` (+0x245 only); the chat's text input is FleetOpsHook's (`GameKeysProcessInput+0x23`) | verify |
| panels | `panels.sound` | ready | `operator new` unchecked (profiled), but **every `Audio*` class is FleetOpsHook's FMOD** | measure |
| panels | `panels.comm` | ready | `CommDisplay`'s constructor and `PostLoad` trampolined, its unit-transfer rule FleetOpsHook's | verify (multiplayer only) |
| dialogs | `dialogs.reads_screens` | down | all 30 dialog procedures: `ShellBitmapImp` in each (the shell's bitmaps FleetOpsHook's, scaled with `StretchBlt`), twelve with Roots' 1200x900 constants; **nine `do_*` screens open through FleetOpsHook's own `DialogBoxParamA` import**, past `ArmadaL.exe`'s import table where the host sits; `do_mainMenu` and `DispatchDialog` trampolined | stays Roots' (decision 68) |
| shell | `shell.screens` | down | `DispatchDialog`, `SetActiveDisplay`'s wrapper, `Show_Credits`, and `SetCurrentBackground` replaced whole | stays Roots' |
| shell | `shell.reads_*`, `shell.cursor`, `shell.widgets` | ready | fields and the `WM_SETCURSOR` call in a `WindowProc` that may not be the window's | with the menus |

### What Roots changes in the HUD and the shell

Seen on the bench shots (1280x960, 4:3) and read from Roots' layouts and FleetOpsHook's
patch table:

- **The layout.** The minimap bottom left, the bar a row of large square buttons above
  the info panel's left end, the info panel a wide strip along the bottom (one unit, or a
  builder's ten-slot queue), the resources top left (one to three, by race), five
  top-right buttons, and **new elements**: a command info bar at the top centre (the
  selected unit's name, hull, shields, crew, energy), an avatar box, a build box and
  multi-segment health bars. Each race has its own art. No unit view.
- **The canvas.** FleetOpsHook picks the layout file by the display's aspect
  (`gui_<race><aspect>.cfg`: 1600x1200 at 4:3, 1600x1280 at 5:4, 1920x1200 at 16:10,
  2134x1200 at 16:9); the engine still converts every rectangle into its fixed 1600x1200
  space and scales per axis, so the UI's reading of a component's rectangle is unchanged.
  The fonts are scaled by FleetOpsHook's own `FontNewScreenWidth`. What 21:9 picks is not
  known.
- **The bar.** 64 buttons behind a pointer; the static (bottom) mode 14 columns by 20
  rows, the floating one stock's 3 by 7; the slot plan and the special weapons laid out by
  FleetOpsHook (each weapon its own button), the hotkey tables edited (QOL's concern,
  FO-7). **Its range rings follow the button under the pointer** (FO-5: hovering the
  Tractor Beam button draws its range): a bar of ours, which never lets the stock bar own
  the mouse, would lose them.
- **The info panel.** FleetOpsHook's own `ShipDisplay` (enlarged), its single and builder
  faces drawn by its callbacks, its subsystem icons, whose hover draws Roots' cyan ring.
- **The minimap.** Map objects drawn by FleetOpsHook's callback, Roots' team colours, an
  observer fix in the action under the pointer.
- **The shell.** 1200x900 on its own windowed device; FleetOpsHook's bitmap object in
  every screen; nine screens opened through its own `DialogBoxParamA`; its own Win32
  dialogs (the mod check, before `WinMain`) outside the shell's code, which the host
  refuses by construction.
- **Elsewhere.** A clean-screenshot mode that hides the HUD and chat (FleetOpsHook's
  trampolines); FMOD in place of DirectSound; its own chat text input.

### Step list

| Step | What | Needs | Agent h · bench |
|---|---|---|---|
| 1 | **Groundwork, offline.** `profiles/fo-roots-5.0Z11/content.toml` names a `fleetops` set for the UI (`ui/content/sets/fleetops/ui.ini`: Roots' race names and their looks, resources, no campaigns, no tour; decision 70); `fo-scan.toml` drops the entry check of the callees the UI only calls (`ST3D_Texture::DemandLoad`, `ActionMode::GetAction`, and `operator new` for `hud.closeup`, the gap `--compare` found); the component reads find the info panel by the render list, not the `shipDisplay` global, on every build (decision 64); the bar's button count and indirection become profile values (21 inline on stock, 64 behind a pointer on Roots); `games/fo-roots.toml` adds `ui` with Roots' defaults (decision 63); the stock set must not move (`ui/hosttest`, `tools/engine/negative.sh`). **Built 2026-11-06** (branch `fo-6`): the set, the callee checks (`callees_whole` in `fo-scan.toml`), the info panel found by name, the bar's two profile values and the `ui` layer with Roots' defaults are in; decisions 76–81 | profile, code in `engine/hud.rs` and `engine/palette.rs`, a set | 4–8 · 0 |
| 2 | **The loading screen.** `loading.overlay` and `loading.own` verified beside Roots' load bar and `GameDoOne` edits; `inner_db`, `inner_pixels` down by design; the map and players named through `shell.reads_setup` | profile marks; bench A/B with the UI off. **Built 2026-11-06** (branch `fo-6`): `loading.overlay` and `loading.own` verified (the bytes at our sites are stock's in the file and in memory); on the bench (Roots, 1280x960, Vulkan with validation) the screen is ours edge to edge with Roots' load bar and logo scene under it, and `fo-ui-loading` / `fo-ui-loading-off` are the A/B; decisions 82–84 | 3–6 · 2–4 |
| 3 | **The mission frame, input, the Refit screen.** `core.mission` verified; the window procedure of both windows measured (class, `GWL_WNDPROC`, which one `GetWindowHandle` gives in the shell and in a mission); if Roots' form calls the two message functions itself, record and capture move to their entries on Roots (decision 71); the hidden flag read from Roots' panel; **the Refit screen as a modal in a mission**, opened by a key of the UI's (decision 62), with the clean-screenshot mode left alone (decision 72). **Built 2026-11-06** (branch `fo-6-step3`): the mission frame verified, input at the two functions' entries on Roots, the hidden flag from the named panel, the render lists logged, the Refit screen by Ctrl+F12, Roots' textures mounted for the loading screen's sky; decisions 85–93, "Step 3: what the bench showed" below | profile, `input.rs`, `engine/ingame.rs`, `menus/refit.rs` outside the host | 8–14 · 3–5 |
| 4 | **The panels Roots changes least**, each its own row and off on Roots until verified: messages and objectives, the pause and replay boxes, overlay text, chat and trade, the top-right buttons (Roots' five), the resources (Roots' amounts, names from the set), the selection overlay, and the engine's last draws (`fade`, `frame_rate`, `order_ring`, `footprints`; the drag box stays the engine's). Sounds measured under FMOD first | profile marks, a panel at a time | 12–20 · 5–8 |
| 5 | **Measure the bar, draw nothing.** Log FleetOpsHook's bar on the bench: the buttons per selection (a ship with several weapons of one `classLabel`, the construction ship's build list, a station), the mode in use and its columns and rows, which button the range ring follows and through which of the stock bar's states. Decide from it whether a bar of ours can keep the rings without reading FleetOpsHook's memory (decision 65). **Measured 2026-11-06** (branch `fo-6-step5`): `bar: measure` and `bar: hover` log lines from the engine's own objects (`engine/palette.rs`, `log_measure`), scenario `fo-ui-bar-measure`; the answer is decisions 94 and 95, "Step 5: the bar measured" below | bench and logs only | 4–8 · 2–3 |
| 6 | *Optional, when asked*: **the bar**, laid out by the mode's columns (no 5x3 grid; position keys for the first cells only, QOL's in FO-7), then the info panel and the minimap with FleetOpsHook's own objects, the command info bar left to Roots | step 5; Roots developer's view on the look | 25–45 · 10–15 |
| 7 | *Optional, when asked*: **the menus**: the host's design space taken from the profile's shell (1200x900, not a fixed 800x600), the generic mirror first, specs after; the nine screens that open through FleetOpsHook's import stay Roots' (decision 68) | step 3 | 20–40 · 8–15 |
| 8 | **Scenarios**: `fo-ui-loading` (a load with `[Loading] Hold`), `fo-ui-mission` (UI frames logged, a click on a panel of ours not reaching the engine), `fo-ui-refit` (a row toggled in the mission modal and kept after a relaunch: the Acceptance row, moved from Options for now), `fo-ui-standdown` (every UI part at 0: Roots as with Display8 and render3d alone, pixel-exact), one `fo-ui-<panel>` per panel of step 4; the stock set unchanged (`ui-runtime`, `ui-hud-bar`, `ui-whole-game` at 1280x720, `--install .`) | step 2 onward | 4–6 · 3 |

**Round one** (steps 1 to 5 and 8): about 35–60 agent hours and 15–23 bench passes, in
round FO-B or a round of its own after it; with steps 6 and 7, about 80–145 hours. This
replaces the 10–30 hours of the work-package table, which assumed the HUD's reads held.
Bench as for FO-5: Roots at 1280x960 (the shell needs 1200x900), Vulkan with the
validation layer, staged through Instant Action, one Roots game at a time.

### Steps 1 and 2: what the bench and the files showed (2026-11-06)

`./a2test syms --game fo-roots` is 73 of 108 hooks ready (70 before; stock 106 of 108, unchanged);
`hud.footprints`, `hud.radar` and `hud.radar_cursor` came ready by the callee rule,
`loading.overlay` and `loading.own` by verification. On the bench (Roots, Display8 on Vulkan
with validation, Render3D and the UI, 1280x960, Federation against Klingon) `fo-ui-loading`
passes every log check: the screen is ours edge to edge, in the Federation's theme, titled
"The Very First Map" (Roots' map `first`), the players panel names "steamuser, Federation"
and "AI Hard, Klingon" through `shell.reads_setup`, 700 frames of ours to the engine's 18
(longest wait 690 ms, with the 20 s hold), no frame inside a step, then Roots' own HUD with
no panel of the UI's over it. `fo-ui-loading-off` shows Roots' own loading screen (a planet,
a tip, its red bar) with the hook installed and nothing of ours drawn. Findings for step 3:

- **The mission hook (`core.mission`) is the next one down**: it logs `mission: standing
  down` because of `ChatDisplay::Render`, which is trampolined; the HUD part says "Enable=0,
  the HUD is stock".
- **Generated tables carry the marks**: a `verified` edit in `build.toml` is only seen after
  `tools/engine/gen.py` and a rebuild (`gen.py --check` fails CI otherwise); the first bench
  run of step 2 stood the loading hooks down for that reason.
- **The info panel's name on Roots is unmeasured**: `hud.reads_components` is ready but the
  component list is only logged by `log_components`, which runs from the HUD part; with the
  HUD off nothing logs it. Step 3 should log the render lists once per mission (the names
  and rectangles) on Roots to settle decision 77, and to read the window procedure question.
- **`core.reads_hud_hidden` still reads the stock global** (`shipDisplay+0x16`, ready,
  wrong): it should go through `hud::ship_display()` like the rest.
- **The Refit screen's defaults** (`ui/settings.toml`, `shipped = "ui/UI.ini"`) show stock's
  shipped values; on Roots they differ (`ui/defaults/fo-roots.ini`).
- **Roots has no `Data/Menus`**, no UI log of its own and its logs sit in the prefix's
  `AppData/Local/armada2-refit/logs/` (`"UI.log"` names it in a scenario).
- **A map's sky is TGA only** in `loading.rs`; Roots' faces may be DDS (the screen fell back
  to the menu backdrop with `sky none`).
- **The stock `ui-runtime` scenario fails at step 33** (`Context: Mission`, a load still at
  35%) on this machine, before this branch as after (a run of 2026-11-06 00:07 failed alike):
  its 8 s wait for the load is too short; `ui-loading` passes.

### Step 3: what the bench showed (2026-11-06)

`./a2test syms --game fo-roots`: 76 of 108 hooks ready (73 before); stock 106 of 108, unchanged.
`core.input_callee` is new and `core.reads_hud_hidden` is gone. On the bench (Roots, Display8
on Vulkan with validation, Render3D and the UI, 1280x960, Federation against Klingon),
`fo-ui-mission` passes: the UI draws about 62 frames a second over Roots' own HUD, with
Roots' cursor. Ctrl+F12 opens the Refit screen over the running mission. A click on its
Interface tab is kept from the engine (`input: a left button down kept from the engine`), and
Esc closes the modal without opening Roots' in-game menu. The rest of the bench round
(`fo-ui-refit`, `fo-ui-standdown`, `fo-ui-loading`, and stock's `ui-hud-bar` and
`ui-loading`) is listed under "Acceptance" with its results. What the first runs showed:

- **The window and its messages.** Every window of Roots' game thread is FleetOpsHook's
  (VCL): both device windows are its form class, and their window procedures are thunks in
  heap memory, in no module. The engine's `WindowProc` never runs. FleetOpsHook's form calls
  `ProcessMouseMessages` and `ProcessKeyboardMessages` itself, from two call sites in its
  window procedure. `GetWindowHandle` names the shell's form until the shell's device is
  released, then a new form for the mission, on the same thread.
- **The render lists.** Six views on Roots (stock's walk read nine). Views 0, 1, 3 and 4
  each hold 21 components, among them Roots' `AvatarBox`, `BuildBox` and `CommandInfoBar`.
  Views 2 and 5 are the cinematic views, with 7 components each. The info panel answers
  `ShipDisplay`, an object of FleetOpsHook's with an empty rectangle (decision 77 holds).
  The `ButtonPanel` and `ResourcePanel` rectangles are Roots' own.
- **The first mission frame stopped Roots twice.** The cause was the UI's own read past the
  views a build has: the slots after view 5 hold other memory, which led the walk to a
  "component" at address 8, and reading its rectangle faulted. FleetOpsHook's madExcept
  caught the fault and parked the game's thread in its report dialog, so it looked like a
  hang. A watchdog thread (bench only, since removed) read the parked thread's registers and
  stack, and madExcept's own report named the faulting function. The walk now rejects a list
  holding anything that is not a pointer, on every build (decision 92).
- **The sky.** Step 2's `sky none` was the mount, not DDS: the UI read `Textures\RGB`, which
  Roots does not have. All 181 prefix skies of Roots' maps are TGA, and 10 maps name a cube
  model. With Roots' `Textures\` mounted, `first` shows `env_dark_dust` (decision 91).

**The owed runs (2026-11-06, `fo-6-step3`).** `fo-ui-loading` at 1280x960: every log check
passes, step 42 included (`loading: standing down` is gone since the loading hooks were
verified in step 2); the screen is the UI's over Roots' load bar, then Roots' own HUD, no
Display8 validation message, no crash. Stock `ui-hud-bar` at 1280x720: PASS. Stock
`ui-loading` at 1280x720: the log checks pass; the vision judge failed step 22 for the
sky ("a starry nebula sky": Warzone's face, `mbgred`, is a dark field with a bright point
and radial star streaks, the same as in the run that preceded the Roots work), so the
scenario's wording now accepts the map's own streaked sky and the step passes. The
ring-of-dots step 25 stays `INCONCLUSIVE` in both: the judge is given one shot, and the
two shots' crops of the ring differ (23 and 29 pixels of 1600), so it turns.

### Step 5: the bar measured (2026-11-06)

Branch `fo-6-step5`, on `fo-6-step3`. With the HUD part off (Roots' defaults), `engine::palette::log_measure`
runs in the mission frame on a build whose bar array is behind a pointer (`palette_slots_indirect`) and
logs what the engine's own objects say, writing nothing: `bar: measure` (mode, menu, the columns and rows in
use and of the floating and the two static modes, the hidden flag, how many of the 64 buttons are shown, the
palette and the render list's "PopupPalette"), `bar:  slots` (each shown button as slot:kind:name, with a
command's preferred cell and its state) and `bar: hover` (the button whose hover state is set, its pressed
state, its rectangle and who owns the pointer). One line per change of the layout (cap 40) or of the hover
(cap 120), by a signature of the buttons' identities (a command by its name's bytes, a mode by kind and the
class, weapon or fixed index), not of the objects, which the bar makes anew at each update.
`fo-ui-bar-measure` (Roots, 1280x960, Vulkan with validation) passes: box selections of the start fleet, the
bar's buttons hovered and pressed, no panel of ours drawn.

- **The bar object.** `g_pPopupPalette` and the render list's "PopupPalette" are the same object, so decision 64's
  path and the global agree on Roots. Mode 1, columns and rows 14 by 20 in use and in both static modes.
- **One row.** The shown buttons sit in slots 0 to n-1 in the order the bar built them and are drawn left to
  right; a rectangle is local to the palette (`[88k, 0, 88k+87, 87]`). Nothing uses `preferredPosition`.
- **Per selection** (the start fleet, Federation): the Starbase 10 buttons (recrew, order, transport, build,
  rally, aiMenu, and four weapons: bastion and the three trades), a construction ship 8 (repair, stop, order,
  transport, build, form, aiMenu, tractor beam), the Starbase with ships 12 to 13, an escort without a special
  weapon 6; the construction ship's build list is 9 buildings, cancel and back (11), the Starbase's 4 ships,
  cancel and back (6), the AI menu 9 commands and back. "Weapon" (kind 2) is FleetOpsHook's whole special
  list: the station's trades are weapons too.
- **The rings.** The red circle of the Tractor Beam (range 400) is on screen exactly while the pointer is on
  that button, and gone on repair, build and the other menus, and on the station's bastion and trades. The
  button's hover state (`+0x38`, set by the engine's `StandardButton::Simulate` from the pointer and the button's
  rectangle, whoever owns the pointer) follows the pointer button by button, and so does the pressed state
  (`+0x30`). The pointer's owner (`CursorInterface`) reads 0 at the mission frame's hook even while a button is
  hovered, so the owner cannot tell the bar has the pointer there.
- **Not measured**: a ship with several weapons of one `classLabel`; the floating mode (X is not its toggle on
  Roots, the third top-right button opens Comms and Alliances, which stops the scenario); a menu over 14.
- **For step 4** (the panels Roots changes least): each is its own row, off on Roots until verified; the bar
  stays Roots' while step 6 is not asked. **For step 6** (a bar of ours): a skin per decision 94, laid out by
  decision 95, never the panel switch's mouse skip for the palette; its first bench draws the skin and
  confirms the ring and the tooltip stay.


### Defaults taken

| # | Default | Why | What would change it |
|---|---|---|---|
| 60 | **Roots' art stays Roots'**: the UI draws its own look (`ui/STYLE.md`) and never copies, converts or ships Roots' HUD or shell art; the information art the UI keeps on stock (unit and build icons, wireframes, portraits, cursor shapes) is read from the player's install at run time, as on stock | hard rule 8 and decision 7; the Roots developer decides Roots' art style | the Roots developer |
| 61 | **Order**: loading screen, mission frame and input, the Refit screen, the least-changed panels, a measurement of the bar; the bar, info panel, minimap and menus only when asked | lowest risk first, and the user's direction (renderer first, the UI after) | the user |
| 62 | **The Refit screen comes to Roots as a modal in a mission**, opened by a key of the UI's, not over Options | the host stands down on all 30 dialog procedures and nine screens open past it; the mission frame needs two hooks | the host working on Roots' Options (step 7) |
| 63 | **Roots gets UI defaults of its own** (the loading screen, the mission frame and the Refit modal on; every HUD panel, the menus, the Credits, the menu scene and the cursor off), merged at install for `--game fo-roots`; a panel is switched on there only once verified | Roots' HUD is Roots' gameplay until each panel is shown to keep it | each panel's verification, and the user's or the Roots developer's wish |
| 64 | **A component FleetOpsHook replaces with an object of its own is found through the engine's lists** (the render list, by its name), on every build, never through the static global and never at an address of FleetOpsHook's | as decision 42 for the overview: one path, any build that constructs it elsewhere is followed | a build whose list does not hold it |
| 65 | **The bar stays Roots'** (`HUD.Bar=0`, `BarLabels=0`, QOL's `BarKeys` off) until step 5; when taken, it reads the count and the array from the profile and lays out by the mode's columns, never the 5x3 grid | 64 buttons, FleetOpsHook's slot logic, and the range rings on hover, which a bar of ours would lose | a way to keep the rings without reading FleetOpsHook's memory |
| 66 | **The info panel, the minimap and Roots' new elements stay Roots'** in round one | most changed, drawn from FleetOpsHook's private records, and they carry its gameplay feedback (the subsystem rings) | the user asking, after step 5 |
| 67 | **No unit view on Roots** (`HUD.UnitView=0`) | Roots' layouts switch it off | a Roots layout with one |
| 68 | **The menus stay Roots'** (`Menus.Enable=0`, `Credits=0`, `Scene=0`), and nothing is patched in `FleetOpsHook.dll`'s own import table | the 1200x900 shell, FleetOpsHook's bitmaps and the nine screens past the host; FleetOpsHook stays a black box (decision 3) | step 7, asked for |
| 69 | **The engine's cursor on Roots** (`[UI] Cursor=0`) | Roots sets a Win32 cursor and race-specific sprites of its own; not measured | a measurement in step 3 or 4 |
| 70 | **The UI's `fleetops` set holds names only**: Roots' races with the six looks (Free Borg as the Borg's), the Dominion and the other races in the shell's look, no new look invented; Roots' resources; no campaign cards, credits tour or cursor sprites | decision 7; a race look for the Dominion is the Roots developer's call | the Roots developer |
| 71 | **Input on Roots is recorded at the two message functions' entries** if the window procedure is Roots' form, chosen per build by a profile value; stock keeps its call sites | it covers both callers and reads nothing of FleetOpsHook's; render3d's callee hooks are the precedent | the measurement in step 3 showing the engine's `WindowProc` still runs |
| 72 | **Roots' clean screenshot does not hide the UI** | its state is FleetOpsHook's memory (decision 5) | the user using Roots' clean screenshots |
| 73 | **The loading screen of ours covers Roots' whole** (its load bar and text included), and animates only between load steps | it is the UI's own look over no Roots art; Roots replaces the functions the in-step frames hooked | the Roots developer preferring Roots' loading screen |
| 74 | Callees FleetOpsHook takes over and the UI only calls **drop their entry check** (`fo-scan.toml`, as decision 15); **the drag box stays the engine's** (FleetOpsHook owns the call; no callee hook in FO-6) | Roots' replacement is what Roots means there; the box is minor | the drag box wanted in the theme |
| 75 | `games/fo-roots.toml` **keeps `hud_canvas = [1600, 1200]`**, the space the engine converts every rectangle into; Roots' per-aspect layouts are not read by the UI | the UI reads converted rectangles, never Roots' layout files | a UI part that needs the declared canvas |
| 76 | **`callees_whole`** in `fo-scan.toml` (`ST3D_Texture::DemandLoad`, `ActionMode::GetAction`): a function our hooks only call that Roots changes anywhere in the body drops its entry check; with a hook of ours also inside it (`loading.inner_pixels` patches a call in `DemandLoad`) it stays marked and only the hooks that merely call it (`hud.order_ring`, `hud.footprints`) are written `verified` by the scan. `operator new` for `hud.closeup` was already a callee; the scan drops its check | the plan's list of three callees did not fit the scan's rule (a callee with something of ours inside, or changed past its entry, was marked); the rule is extended, not the marks hand-edited | a callee whose Roots change alters what the UI reads from it |
| 77 | **The info panel is found by `GetName` == "ShipDisplay" in the render lists of every view**, kept while a list holds it; the panel switch takes it by name (`INFO_PANEL_TAKEN`, learned from the components `RenderAll` and `RefreshOwner` call). `hud_ship_display` is gone from the profile; `shipDisplay` stays only as `hud.info`'s gate. Outside a mission the answer is 0 | decision 64 on every build, stock the same object; all views, so the cinematic view answers as before. Roots' panel answering to the same name is unmeasured (step 3) | the bench showing another name on Roots |
| 78 | The bar's **count and indirection are profile values** (`palette_n_slots` 21 and `palette_slots_indirect` 0 on stock; 64 and 1 in `profiles/fo-roots-5.0Z11/ui.hud.toml`, a hand-written entry beside the scan's); `HUD.Bar=0` stays | decision 65; the values come from MemDiff's count and pointer, not yet read in game | step 5's measurement |
| 79 | **Roots' `UI.ini` defaults are an overlay**, `ui/defaults/fo-roots.ini`, applied over `ui/UI.ini` by `ui/apply-defaults.py` in `ui/install.sh` (a key `UI.ini` lacks fails the install): the HUD's `Enable` and every panel key, `Menus.Enable`, `Credits`, `Scene`, `CampaignLocks`, `[UI] Cursor` and `Movies` at 0; the loading screen on. `Movies=0` is added to the plan's list (Roots plays its intro on a Direct3D 9 device of its own). The Refit screen's "shipped default" still reads `ui/UI.ini` (step 3) | decision 63; one list per game, not a second whole file to keep in step | a part verified |
| 80 | The `fleetops` set: Federation, Klingon, Romulan, Cardassian, Borg, Species 8472, **Free Borg** (the Borg's look), **Dominion** (no look: the shell's); each race's aliases include Roots' ODF names (`unitedfederation`, `klingonempire`, `romulanempire`; `observer` and `federation` for the Federation); the same six resources; no campaigns, scenes, files or tour; the gallery lists the factions only | decision 70; the order is the plan's | the Roots developer |
| 81 | **No menu plates on Roots**: `ui/backdrops/install-plates.sh` neither installs nor removes anything for another game (Roots' `Data/` has no `Menus/` today; `layers.toml`'s `Menus/*.bmp` pattern would claim one if a Roots update shipped it, for `a2mod`) | decision 60 and 68 | a Roots with its own `Menus/` |
| 82 | **`loading.overlay` and `loading.own` are verified** by reading the bytes: the entry and the `LoadBarNext` call into `RenderLoopAssetLoadStatusUpdate`, its `DeleteRegisteredLights` call (+0x3b3) and `EndScene` label (+0x400), the splash call in `GameDoOne` (+0xcd) and `RenderLoop_Show_Splash_Screen`'s entry are the same in the file (compared with stock) and in memory after FleetOpsHook (MemDiff's ranges: Roots' writes are +0x1b, +0x88, +0x343..+0x35c and, in `GameDoOne`, +0x17c..+0x30a and the file's +0x19b); `fo-scan.toml` `[verified]` keeps them. `inner_db` and `inner_pixels` stay down | the plan's "verify (cheap)" | a Roots update changing the sites |
| 83 | **The screen of ours covers Roots' whole** (decision 73), frames between the load's steps only (0 of 4 inner sites); a map's sky is read as TGA only, so Roots' DDS faces give "the menu backdrop" (no sky) there | the A/B shows the cover; DDS sky faces are a later change of `a2fmt` use | the Roots developer, the user |
| 84 | **Bench: `ui/bench-asi.sh` follows the executable's folder** (`Data/` on Roots), keeps the clone's own `UI.ini` (Roots' defaults) as the base for `S.K=V` keys, and leaves `retire/` to Armada II; `display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh ...` is the Setup line. Scenarios `fo-ui-loading` (the B side, `Hold=20`) and `fo-ui-loading-off` (`Loading.Enable=0`, the A side) | the bench's scripts wrote `$work/game/` | |
| 85 | **`core.mission` is verified** by reading the bytes. The call of `ChatDisplay::Render` at 0x598a16 is the same in the file (compared with stock) and in memory after FleetOpsHook (MemDiff: nothing there). FleetOpsHook's writes are a trampoline at `ChatDisplay::Render`'s entry, which the UI only calls, so Roots' version runs whole, and +0x245 in `Armada_RenderFrame`. Both functions carry `ui.core.mission` in `[verified]` | the plan's "verify (cheap)" | a Roots update changing the call |
| 86 | **The Refit screen over a mission opens with Ctrl and `[UI] RefitKey`**, a virtual-key code: 123 (Ctrl+F12) in Roots' defaults, 0 (none) in the shipped `UI.ini`, so stock does not change. The key is kept from the engine. It opens the same modal (`refit::mission_frame`), drawn last in the mission frame; Esc, Done or the key again closes it. The game is not paused, and the rows count as "over a mission" for the network locks and the "next mission" badges | decision 62. F12 is a build-menu slot in stock (QOL-1), and Ctrl+F12 is bound nowhere we know of. A key of the UI's is the smallest addition, and needs neither Options nor the menu host | Roots' players, or the host reaching Roots' Options (step 7) |
| 87 | **Input on Roots is taken at the two message functions' entries** (decision 71 carried out). A new hook, `core.input_callee`, is chosen by the profile value `core_input_at_callee` (0 on stock, 1 in `profiles/fo-roots-5.0Z11/ui.core.toml`). The six entry bytes, checked, move to a trampoline (`core_input_moved`). A naked thunk notes the caller, records the message and keeps what capture takes; the rest runs through the trampoline. The drains are stock's. Stock keeps `WindowProc`'s calls | the bench: the engine's `WindowProc` never runs on Roots, and FleetOpsHook's form calls both functions | a Roots build that calls them elsewhere |
| 88 | **Whether the HUD is hidden is read from the panel the render lists name** (`hud::flag_hidden(hud::ship_display())`) on every build. The hook `core.reads_hud_hidden` and its `shipDisplay` symbol are gone | decision 64 | a build whose lists do not hold the panel |
| 89 | **The render lists are logged whenever they change** (in practice once per mission, at most 8 times a game), whichever HUD part is on. The thread's windows are logged with their classes, threads and window procedures at the first device release and the first loading frame, the engine's window at the first mission frame, and each message function's callers once | step 2's finding: nothing logged the lists with the HUD off | |
| 90 | **The Refit screen's defaults on Roots are the installed `UI.ini`'s.** The install puts `ui/defaults/fo-roots.ini` over it, and the screen's base for a row is the plugin ini's value; the compiled default applies only to a key `UI.ini` lacks, and the install never leaves one out. So there is no per-game defaults table. The two loading-screen hooks that draw inside a step (`inner_db`, `inner_pixels`) no longer name the `Loading.Enable` row: on Roots they stand down by design, and the screen's note claimed the whole screen stood down | the screen read Roots' values in the bench run; the note was misleading there | a row whose key is not in `UI.ini` |
| 91 | **The UI's file system follows the game** (`ui/src/files.rs`, `GAMES` by the executable's name, as render3d's). Roots' layout mounts `Textures\` with `.tga`, then `.dds`. A map's sky face is read with `a2fmt::image`, either format. Roots' mods (`Data\Mods\<name>`) are not mounted for the UI | step 2's `sky none` was the mount; DDS costs nothing with `a2fmt` | a UI part reading a mod's files |
| 92 | **A walk over the engine's lists follows only pointers.** Each list and its head is checked with `VirtualQuery`; each node, entry, component and vtable must be an aligned user-range address; a list holding anything else counts as empty. The UI sends no window message from inside a frame (no `GetWindowTextA`) | the first mission frame on Roots faulted in the walk over views 6 to 8 | |
| 93 | **`fo-ui-standdown` is compared with a reference run.** `fo-ui-standdown-ref` (Display8 and Render3D, no `UI.asi`) records the main menu and the Instant Action setup screen as the baseline `fo-ui-standdown`. The UI run, with every part at 0 (`Loading.Enable=0` and `UI.RefitKey=0` over Roots' defaults), must match with at most 0.05% of pixels more than 4/255 off (the first run differed only at the pointer: 102 and 152 pixels, 0.008% and 0.012%); the match is judged by eye, since a skirmish is not pixel-repeatable | the plan's "pixel-exact" holds where the game itself is repeatable | |
| 94 | **A bar of ours on Roots keeps the range rings only as a skin of the stock bar**: drawn over the stock buttons' own rectangles (the palette's origin plus each button's rectangle, both the engine's objects), with the stock bar left to simulate and to own the pointer, so the stock button's hover state, which the ring follows, is set by the engine itself and nothing of FleetOpsHook's memory is read. A bar laid out differently (the 5x3 grid, any cell off the stock rectangle) or a palette taken by the panel switch's mouse skip loses the rings. The skin's drawing skip is `RenderAll`'s alone; `RefreshOwner` is not skipped for the palette | the ring is on screen exactly while the pointer is on a weapon button's rectangle and gone on any other button (step 5, `fo-ui-bar-measure`); the hover byte is the engine's `StandardButton::Simulate` result and is set whatever the cursor owner is; the bar of ours was not yet drawn, so the claim is confirmed only for the hover state | the first bench of step 6 (ring with the skin drawn), or Roots changing its ring condition |
| 95 | **Roots' bar is one row in the mode in use** (mode 1, 14 columns by 20 rows in `use`, `staticA` and `staticB`; the floating 3 by 7 is only the stock field), the shown buttons packed in slot order (`preferredPosition` places nothing), 88 canvas units apart, each rectangle local to the palette's origin. A bar of ours reads the shown buttons in slot order and wraps at `use` columns; the 5x3 plan of the stock bar does not apply. The longest list seen is 11 (a construction ship's buildings, cancel and back); several weapons of one `classLabel`, the floating mode and a list over 14 are **not measured** (no start unit has more than one special weapon, no control for the floating mode was found) | step 5's per-selection table (private notes `fo6-step5.md`); a build list over 14 wraps by `use` columns in the engine, not seen | a ship with several special weapons, or a way to the floating mode, on the bench |

### Risks and unknowns

1. **"Ready" is not "right".** Globals and objects FleetOpsHook replaces are reported,
   not marked, so `syms` lets in hooks that would read stock's dead `shipDisplay` or a
   21-button array that is now a pointer: wrong pictures at best, a crash at worst. Step 1
   closes the three found; every `reads_` hook is checked in game before its panel is on.
2. **The window and its messages.** Roots' own form, two devices on two windows, and
   FleetOpsHook's activation handling: input capture may miss the first message of a
   burst, or read the pointer against the wrong window. Measured first in step 3.
3. **Roots' feedback lives on the stock components' mouse state** (the range rings of the
   bar's and the info panel's hover): any panel of ours that takes the mouse from them
   takes that feedback away. Known for two panels, possibly more.
4. **FMOD.** The UI makes its click and rollover sounds with the engine's
   `AudioSound2D`, whose class FleetOpsHook replaced: silent, or worse. Measured before
   any panel with sounds is on.
5. **Roots updates** move the bar's count, the replaced objects or the dialog path; the
   profile values and `fo-scan.py` catch the code, not the meaning, so each Roots release
   re-runs the UI scenarios.
6. **The look.** The UI's own style may not suit Roots, which has strong per-race art;
   the agreement is to try it, and the Roots developer decides.
7. **Unknown**: whether FleetOpsHook's `DialogBoxParamA` ever reaches `ArmadaL.exe`'s
   import table; what 21:9 and wider pick for a layout; where Roots' cursor is drawn on
   our device; how many buttons a real Roots fleet puts in the bar.

## Acceptance

| Scenario | Shows |
|---|---|
| `fo-launch`, `fo-skirmish` | the bench runs Roots: front end, a map, a tick, with nothing of ours installed |
| `fo-syms` | "symbols: N/N resolved" with Roots' marked symbols listed as standing down; the negative test both ways |
| `fo-passthrough` | Roots with Display8 alone on Vulkan matches Roots as it ships (DxWrapper, then DXVK) at a paused tick, in the front end and in a mission, within the tolerance stock's passthrough references are judged by |
| `fo-render3d-*` | per module, Roots' scenes with the module on, and with it off the passthrough look |
| `fo-ui-mission` | the UI's mission frame on Roots, input at the message functions' entries, a click on the Refit screen kept from the engine (passes, 2026-11-06) |
| `fo-ui-refit` | the Refit screen over a mission by Ctrl+F12 (decision 86), a row toggled and kept after a relaunch (passes, 2026-11-06) |
| `fo-ui-standdown-ref`, `fo-ui-standdown` | every UI part at 0: the shell pixel for pixel as with Display8 and Render3D alone (passes, 0 pixels differ, 2026-11-06) |
| `fo-ui-loading` | the loading screen of ours over Roots' own, 1280x960 (passes, 2026-11-06: `loading: standing down` not in the log, the hold, then Roots' HUD with no panel of ours; the ring-of-dots step is left to a person, the judge sees one shot, a pixel diff of the ring's crop between the two shots shows it turning) |
| stock `ui-hud-bar`, `ui-loading` | 1280x720, `--install .` on the step 3 branch: `ui-hud-bar` passes (70 steps); `ui-loading` passes but for the ring step, which the judge cannot see (review; 2026-11-06) |
| `fo-multiplayer` | two Roots games with the refit's visual plugins join and play past the file check |
| the stock set | unchanged on stock 1.1: the Roots work must not move a single stock pixel |

## What the user signs off

1. **FO-0**: Roots runs under Proton from Heroic as it ships.
2. **Round FO-A**: Roots on our device looks and plays as Roots: the front end at
   1200x900, a skirmish, the intro.
3. **Round FO-B**: render3d on Roots, the Refit screen, QOL's pan speed.
4. **Round FO-C**, if taken: a network game of Roots players with the refit.

## Risks

1. **`FleetOpsHook` at run time.** Its changes are invisible to the file diff. If it
   rewrites a function a hook of ours patches, whichever runs second wins. FO-2 measures
   it before any hook is let loose; the stand-down rule keeps the rest safe.
2. **Its own Direct3D use.** If the hook makes a Direct3D 9 device or relies on DxWrapper
   specifics, Display8 on Vulkan has nothing to give it. There is no forward mode to fall
   back to (phase 11 removed it): the fallback is Roots keeping DxWrapper, with none of our
   renderer, so the decision is made in FO-4 on FO-2's measurement.
3. **Struct layouts.** The survey found no data address that moved, but Roots edits
   classes (`Evolver`, `LatinumNebula`, `UtritiumBall`, `Producer`, `cOverViewImp`).
   A field a hook reads may mean something else. Each verified hook checks the fields it
   reads in game.
4. **Roots updates.** Updates come from Roots, not from Fleet Operations (whose team, as far
   as the user can tell, has abandoned the project), and Roots is actively updated (5.0Z11,
   2026-10-23), its `ArmadaL.exe` and `FleetOpsHook.dll` with it. A new Roots release is a
   new build with a new profile (`fo-scan.py`); an unknown
   build runs only what resolves by scan, as the profile design says. The three submods'
   own `ArmadaL.exe` files are further builds, not in scope until asked.
5. **Fairness and the Fleet Operations community.** A visual plugin in a competitive mod's
   network games is a question of the community's norms as well as of CRCs.

## Decisions taken by default

Taken 2026-11-06 where a choice would otherwise have waited on the user or the Fleet
Operations team. Each holds until what is named in its last column happens.

| # | Default | Why | What would change it |
|---|---|---|---|
| 1 | **No contact for now** with the Roots developer, the Fleet Operations team or TamsynnImogen (the author of the third-party A2FOExtensions); the design is for `FleetOpsHook.dll` as it ships and for its updates, with nothing asked of them | the user's decision, 2026-11-06 | the user opening contact; "What we would ask", under FO-5, lists what would then help |
| 2 | A2FOExtensions (no licence) is used only for **hypotheses**, each checked by us and written in our words; nothing of it is copied | it has no licence, so its text and tables are not ours to take | a licence from its author |
| 3 | render3d on Roots follows **model (c)** (FO-5 design): the frame is ours, Roots' wrappers keep running, hulls claim at the instance draw, callee hooks where FleetOpsHook owns a site | the only model that reads nothing of FleetOpsHook's memory, so a Roots update cannot silently break it | an FO export that draws a ship's extras, or a Roots build that moves the hull draw out of the instance path |
| 4 | Roots' **range rings and overlay models are drawn by the engine** through passthrough, in its own look (tone-mapped and bloomed with the scene) | they are Roots' gameplay feedback; drawing them ourselves would mean reading Roots' private records | the user wanting them in render3d's style: the rings could be taken at the engine's circle draw, as the posts are |
| 5 | Roots' **clean screenshot** keeps hiding the posts (callee hook) but not render3d's selection circles | the selection skip is FleetOpsHook's wrapper of a function render3d does not call | the user using Roots' clean screenshots |
| 6 | The Roots **hull look is keyed by race**; the Borg profile is for the Borg only, not for every bump-mapped mesh. The mapping is data: the `fleetops` set's `hulls.ini` (`ProfileBy = race`, `ProfileRaces = borg, freeborg`: the Borg profile; every other race the hull profile), `Render3D.ini` `[Hulls] BorgRaces=` replaces the list; the race is the ODF's `race` read through its includes. Roots' `*_bump` textures are read as the normal maps they are (`BumpMaps = normal`), and a flat one (`all_bump`) as none | nearly every Roots hull is bump-mapped, for the engine's GPU path, not for a look; the two races are the ones whose hulls are Borg art | the user's verdict in round FO-B |
| 7 | The `fleetops` content set holds **names and keys only** (ODFs, races, classes, `RTS_CFG.h` keys); a look fitted to Roots' own art (a sky recipe, a hull grade) stays local, in the game directory's `Sky\` and similar files, and is not committed | Roots' art is never published (decided 2026-11-06), and numbers fitted to it are close enough to it to wait for the creator | the user, after asking the Roots creator |
| 7b | **Reuse**: everything Roots-specific lives in data, not code: the build profile (`profiles/fo-roots-*`), the game descriptor (`games/fo-roots.toml`) and the `fleetops` content set; the code knows only "a build whose FleetOpsHook owns these sites" (`chain` entries, callee hooks, the companion-module identity) | so another Fleet Operations-based mod can be added by its creator as a descriptor, a profile and a content set of its own, without code of ours changing | a mod whose FleetOpsHook draws differently enough to need code |
| 8 | The Roots build identity adds **on-disk section CRC-32s** (`.text`, `.test`) and **FleetOpsHook's `.text` CRC** as a companion module | three Roots builds of each file share every header value we read | none expected; a Roots release that changes only data needs no new profile |
| 9 | The **submods** ("Fleet Ops 4.0", the STA2 classic mods) are not supported: their own `ArmadaL.exe` files are other builds, refused by the identity | scope is Roots itself | the user asking for one (it is then a profile of its own) |
| 11 | No Ghidra project for FO-5: the questions were answered by the map-named disassembly of a dozen functions and by the bench | cheaper and lighter on memory | a question the map names cannot settle |

FO-5 step 5 (branch `fo-5-step5`) took these defaults:

| # | Default | Why | What would change it |
|---|---|---|---|
| 18 | A callee hook is **armed by its caller's module**: the detour acts only when the return address lies in the build's companion module (`FleetOpsHook.dll`); every other caller, render3d's own calls included, gets the function as it was through the trampoline | FleetOpsHook calls `Fragment` only from its explode callback and `DrawHeightPost` only from its two post callbacks, so "called from the companion" is "during the chained call", with no address of FleetOpsHook's and none of its memory read; no FleetOpsHook site is written | a FleetOpsHook that calls either function from somewhere else, for another purpose |
| 19 | On Roots the **hulls draw their own sprite nodes** (running lights), honouring the node flags in the database at the claim | the claim replaces the database draw that drew them, so nothing else would | an engine draw of the sprite nodes outside the database draw |
| 20 | On Roots the **wave lights stay the hulls'** (`GetSensorScramblerWaveLight` called as the engine calls it, Roots' edit included) | the engine's wave light lights only the engine's draws, and a claimed hull is not one | the user's verdict in round FO-B |
| 21 | At the claim the **database's node flags and mesh textures are read for every node of the SOD** (`ST3D_Database::FindElement` on the node list), not only Roots' `stlogo` | generic: whatever the engine or a companion writes per instance before the draw is honoured, and no node name of Roots' is in the code | the cost showing in a timing run (the lookups could then be cached per database) |
| 22 | `SHIELD_EFFECT_RESCALE`'s **factor (x2) is the profile's** (`ef_shield_rescale_pct` = 200), applied to render3d's shield ellipsoid about its centre; `SHIELD_EFFECT_DAMAGE_COLORED` = 0 makes the stock colour mode white and stops the faction colours shifting with the shields' level | the factor is FleetOpsHook's code, not content; the key's meaning is "the hit is not coloured by damage" | the user's verdict in round FO-B on the shell's size |
| 23 | `RTS_CFG.h` is read from the **game directory** (Roots' `Data\`), not from an active mod | step 6 brings the mod order (`ODF.fpq`, `Data\`, the active mod) | step 6: **replaced by decision 25** |
| 24 | The **grid colour is read from the push in `DrawGrid`** whatever points there (Roots' file points it at its own colour, FleetOpsHook at `memGridColor` over that), checked as a `push` and as readable memory; stock's dark orange otherwise | it is what the engine itself draws the grid in, and needs no address of FleetOpsHook's | a build whose grid is not drawn by that push |

FO-5 step 6 (branch `fo-5-step6`) took these defaults:

| # | Default | Why | What would change it |
|---|---|---|---|
| 25 | **The mod order is the game's**: the active mod from the first `-mod`/`/mod` on the command line, its chain from each `info.ini` (`ParentMod`, `standalone`), and every file render3d reads (textures, SODs, sprites, ODFs, `RTS_CFG.h`) from the top folder of the chain that has it, `Data\` last unless the chain is standalone. Replaces decision 23 | it is how Roots' file system finds the file the engine reads (read off FleetOpsHook's file system and mod list, and its mods' `info.ini`); with no switch it is `Data\` alone, as before | a Roots that chooses its mod another way (a settings key), or a `-mod` the bench passes and the log shows the game did not follow |
| 26 | Within one folder, **loose files over the packed** `ODF.fpq`, render3d's order on Armada II | Roots' `odf\` folders are empty in 5.0Z11 (and the STA2 Classic mod's), so the order is not yet observable; Roots' file system adds the pack first, and which it then prefers was not read | a Roots install with a loose ODF beside a packed one of the same name |
| 27 | A name is found in the **top folder that has it, wherever it sits** under the virtual folder (a mod's `odf\weapons\x.odf` over `Data\`'s packed `x.odf`), and an ODF's include of **its own name** is its next version down (each such include in the tree the version below the last one taken) | Roots' file system keys files by name, and its parser treats a self-include so; stock has no versions, where the same include stays a loop and is cut | a mod whose files differ by folder rather than by name |
| 28 | **Any mod** under Roots' own executable is read through its chain, the STA2 Classic mod included (it runs on Roots' `ArmadaL.exe`), but the content set stays `fleetops`: a mod's names that are not Roots' simply match nothing (its look is the engine's there) | the content set is per build, not per mod, and the submods are not in scope (decision 9); reading their files right costs nothing and keeps every name that does match correct | the user wanting a submod supported: a set per mod, chosen by the active mod's name |
| 29 | Roots' ordnance: a built-in recipe draws **only the ODFs `[Names]` lists**, each paired with the recipe of the stock weapon of the same `classLabel` and use (read off `ODF.fpq`); `ionstm` (Roots' utritium beam) is left to the engine, the miners' tritanium beams take the Borg metal beam's recipe, the Species 8472 biomatter beam and the SOD-drawn bees and nanites none; the `[Model.*]` sections stay by their own name (only `amine` matches Roots) | a stock name Roots shares may be another weapon; a pairing by class and use reuses our looks without fitting anything to Roots' art | the user's verdict in round FO-B; a survey line for a shot that should have a recipe |
| 30 | Shield factions on Roots **by race** (`[Races]`): the six stock factions for their Roots races (Free Borg as Borg), and the Dominion, Ferengi and neutral races the "other" colour | Roots' ODF names do not start with stock's initials (Dominion `d`/`n`, Free Borg `f`/`s`); a Dominion colour would be a new look | the user wanting a Dominion colour |
| 31 | Roots' nebulae: the stock recipe of the stock nebula of the **same `nebulaType` and glow hue**, Tetryon (blue-violet, of a red stock type) none; no surface recipe; `latinum` not named | the type is what the nebula does in the game and the hue what its ODF tints it; Roots' `latinum` class is its asteroid sets and debris fields, which a latinum gas recipe would fill | the user's verdict in round FO-B, or a Roots-fitted recipe kept local (decision 7) |
| 32 | `particles.ini` names **no vent sprites** (Roots' venting emitters draw `empty`) and stock's chain-pulsar and Manheim emitters (Roots keeps them); `sky.ini` none; `overlays.ini` stock's `select` | each is what Roots' sprite tables and SODs hold in 5.0Z11 | a Roots release that brings venting sprites back |
| 33 | The **UI** keeps the set it inherits from gog-1.1 (`stock`) | the UI is not installed on Roots (FO-6) | FO-6 |

### FO-5 step 7

Branch `fo-5-step7`, 2026-11-06. Each render3d hook that still stood down on Roots was
compared with gog-1.1 at the bytes it checks and uses, in Roots' file and beside
FleetOpsHook's run-time patches, and checked for collisions; the findings are in
`render3d/README.md` beside each hook:

| Hook | Roots' change | Verdict |
|---|---|---|
| `render3d.sky.reads` | FleetOpsHook nops the free of the previous background database at `Starfield_Load_Background_Geometry+0x16` (and in `Background_Close`) | verified: entry, name, database and `Background_Render` are stock's; the module calls the function only on the bench |
| `render3d.particles.billboard` | Roots' wave-light detours and FleetOpsHook's cloak tests in `GameObject::RenderInternal` | verified: the call at +0x19c, its arguments and callee are stock's; FleetOpsHook's code starts on the instruction after it |
| `render3d.ordnance.warp` | FleetOpsHook's object-loop edit in `Armada_RenderAllOurStuff` (+0x185..+0x1a4) | verified: the warp call (+0x313) and its callees are untouched |
| `render3d.view.distance` | FleetOpsHook's call at `RTS_Configure+0x1d9d` (its own keys) | verified: the load at +0xebf and its 21 checked bytes are stock's; FleetOpsHook names neither value |
| `render3d.overlays.paths` | FleetOpsHook's three edits in `cOverViewImp::mNormalModeRender` (the cursor's post, the drag box); `GetSelectList`'s offset; an overview of its own | **adapted**: the paths' call (+0x207) and fields are stock's; the overview is now read through `g_pOverView` on every build, since FleetOpsHook constructs its own in place of the static one; `GetSelectList` verified (called through the vtable) |
| `render3d.effects.debris`, `render3d.overlays.posts` | FleetOpsHook owns the call sites | stay down by design: `effects.debris_callee` and `overlays.posts_callee` (step 5) replace them |
| `render3d.frame.obj_draw` | written by FleetOpsHook at run time | `a2test syms` reads the file (stock's call there) and says it stands down; in the game it is ready (step 3) |

Bench (Roots, Display8 on Vulkan with the validation layer, Render3D, 1280x960, three
skirmishes): every hook above installed or patched, `view: ART_CFG.h parsed: far plane now
1000000`, the sky drawn by the module from the map's own sky, the overview read as
FleetOpsHook's and the selection and the craft process's path read as on stock; 0 VUIDs, no
`device removed`. Not yet seen on Roots: a warp flash, a billboard effect and a sketched
move path (the bench could not stage them). Stock (`--install .`): `render3d-live-hulls`
PASS, `render3d-move-paths` PASS (a first run failed only at its last step, the game not
quitting through its menus; the rerun passed), `tools/engine/negative.sh` ok.

It took these defaults:

| # | Default | Why | What would change it |
|---|---|---|---|
| 40 | On Roots the **sky module draws the map's own sky** (its faces or cube, the starfield, the background planets) when no recipe matches, rather than handing the step to Display8's passthrough | it is what the module does on any stock map without a recipe, and it puts the sky on render3d's target, tone-mapped with the rest; a Roots recipe stays local (decision 7) | the user's verdict in round FO-B, or a Roots sky recipe made locally |
| 41 | **`View.Distance` applies on Roots as on stock** (default 1000000): the far plane and the culling distance are raised at Roots' parse of `ART_CFG.h` | both values are only read to draw, FleetOpsHook reads neither, and fog and shroud still decide what is seen; Roots' `renderIgnoreCameraDistance` exception then has nothing left to except | the user, or the Roots developer, judging the wide view unfair in Roots' network games (risk 5); `[View] Distance=0` gives the engine's limits |
| 42 | The **overview is read through `g_pOverView` on every build**, not through a Roots-only value | stock's pointer holds the static object, so stock reads the same address as before; one code path, and any build that constructs the overview elsewhere is followed with no profile value | a build whose pointer is not set by the constructor |
| 43 | On Roots the **particles module draws every object's billboard effect** and skips the engine's call in our views, as on stock | the call site is whole and runs for every object Roots' loop draws, so the trade is stock's | a Roots ODF that uses `renderIgnoreCameraDistance` (objects the engine culls but Roots draws), or a billboard seen missing or doubled in FO-B |
| 44 | `ordnance.warp` and `overlays.paths` are **marked verified from the code and the reads observed**, though the warp flash and a sketched path were not drawn on the bench | the sites, callees and fields are stock's, and the reads were seen right in game; only staging failed (a held key, a warp) | either seen wrong on Roots; step 8's scenarios are to stage them |

### FO-B: Roots' shield hits, the logo node

Branch `fo-b`, 2026-11-06. Step 8's `fo-render3d-shields` failed because no Roots hit
carried the shield database's models: FleetOpsHook replaces `CreateShieldHit`'s pick of
the model (`m_logical_db` of the hit's type) with a call of its own (+0xea), which picks
per ship class from the ODF (`shieldHit`, `shieldHitCritical`, `shieldDown`, `ShieldAdd`,
`ShieldSub`, `ShieldSpecial1..4`; Roots names sized sets such as `shield_fed_hitL`) and
loads each once by name. The effects module now chains that call
(`render3d.effects.shield_models`) and knows a model by the type it was picked for
(render3d/README.md, "Shields on Roots"). The shell's size is the class's shield
transform on both sides (the engine divides the hit's frame by the model's radius), so
render3d's own x2 (decision 22) is still needed and is not applied twice. Step 5's logo
node cannot fire on 5.0Z11: FleetOpsHook's `CraftClass_InitLogo` looks for a node named
`stlogo`, which none of the install's 626 SODs has, and the `logo_*` textures the ODFs'
`logoFileNames` list are not in the install. A plain move of one ship draws no path on Roots (bench, run
20261106-065649: the ship selected and sent, the overview read, no path counted), as on
stock: without QOL the overlays module draws the engine's path only when it was sketched
with the waypoint key, and the bench cannot hold a key on Roots (no scene harness), so the
sketched path is still unseen there.

| # | Default | Why | What would change it |
|---|---|---|---|
| 50 | A Roots hit's **type is learnt at FleetOpsHook's pick**: its call in `CreateShieldHit` is chained (a `call` site, `chain`, Roots only), and the model it set on the hit is noted against the type it was asked for; stock's models are still known from `m_logical_db` | the type is the call's argument and the model the hit's own field, so nothing of FleetOpsHook's memory is read (rule 5); reading the ODF's keys instead would need each database's name and FleetOpsHook's per-class cache, and calling `FindLogicalDatabase` loads a second copy | a FleetOpsHook whose pick takes other arguments (its `.text` CRC then differs and the hook stands down, as every runtime mark does) |
| 51 | The pick is chained **although FleetOpsHook owns the site** (rule 4 says to take the callee) | the callees (`SetLogicalDatabase`, `SetVisibleDatabase`) are called from everywhere and do not carry the type; the chained call is FleetOpsHook's own function, called on with its arguments unchanged, so Roots still decides the model | a callee that carries the type |
| 52 | The **logo node stays generic** (decision 21) and the scenario checks the lookup only (`N of N shown nodes found by name`) | Roots 5.0Z11 has no `stlogo` node, so there is nothing to honour; the code needs no change to honour one when it comes | a Roots release or mod with an `stlogo` node (then the scenario stages that ship and expects the `drawn with` line) |

The Roots profile tools (`tools/engine/fo-scan.py`, PR #61) took these defaults:

| # | Default | Why | What would change it |
|---|---|---|---|
| 12 | The build is named `fo-roots-<version>` from `Roots_Download_Version.txt`; `--build-name` overrides | the install says which release it is | a Roots release without that file |
| 13 | **One Roots variant** is assumed, the install as shipped with DxWrapper; `--display8` swaps Display8 in for the MemDiff run | that is what Heroic installs | a second variant wanted (the submods, decision 9) |
| 14 | The reference is GOG 1.1's `Armada2.exe` with its map; a stock executable of another identity is refused | the map names only that build | another stock build profiled |
| 15 | The callees our hooks only call and FleetOpsHook takes over (no stock entry check on Roots) are the five profiled; a new one needs a line in `tools/engine/fo-scan.toml` | a callee's check is dropped only where we know who replaced it | a new FleetOpsHook taking over another callee |
| 16 | The rules plugin is disabled and the content set is `none` on Roots until FO-5 supplies `fleetops` | Roots has its own rules; stock's names are not Roots' | FO-5 step 6 |
| 17 | Changed globals, vtables and slots are **reported, not marked**: marks are per function | the stand-down rule is per function | a hook of ours reading a global FleetOpsHook rewrites |

The installer (`./install --game fo-roots`, FO-4, PR #63, and FO-5 step 3) took these defaults:

- `./install --game fo-roots` installs `display`, `asiloader` and `render3d` (FO-5 step 3; at first `display` only), into `Data/`; DxWrapper's
  `d3d8.dll` is the backup (`.a2display-backup`, the usual suffix); no platform layer and no
  `retire/` run on Roots.
- `games/fo-roots.toml` gets `installable = true` (optional): `./install` and `a2mod`
  work on a game that is not `offered`; `offered` stays false.
- A requirement a game does not have is not a requirement (`display` requires `platform`,
  which is Armada II's).
- `a2mod stock` on a game with nothing modded says so and exits 0 (the bench's clone of a
  game as shipped).
- `fo-display8` records the bail-out as it is today; its bail-out lines change when the
  display hooks are verified.

**Decided by the user, 2026-11-06 (not a default):** the agreement came from the Roots developer (Roots is a submod of Fleet Operations), not the FO team, and there is no contact with anyone for now. Roots support stays in code only, installed only by `./install --game fo-roots`: never
in the GUI installer, the release packages and manifest, `PLAYING.md` or the release notes
until the user says otherwise. `tools/layers.py --check` and `publish/package.sh` enforce it
(`games/README.md`, "Not offered, not released").

## Open decisions

Decided by the user, 2026-11-06:

1. **What the Roots developer agreed to** (2026-11-06, the user's message to them; the earlier
   approval of the work, the same day, was also the Roots developer's, not the Fleet
   Operations team's):
   the user tries to get the refit's UI and renderer working on Roots, to see the effort
   and whether it suits Roots' art, and to make the code reusable for other Fleet
   Operations-based mods if their creators want it; the code is released only with a
   bash script for installation on the user's own Linux setup, **not advertised or
   announced**. The Roots developer wants to tinker with it once it works.
2. **Scope**: the renderer first on Roots, Display8 and render3d (FO-2 to FO-5); **the UI
   is within the agreed scope** and comes after it (FO-6 stays planned). QOL and Online on
   Roots come later, if ever (FO-7, FO-8 wait).
3. **Roots' art** is at most a local experiment, never published: that is the Roots
   creator's call. The texture and movie pipelines are not pointed at it for any release.
4. **The release** (closed): Roots support stays **in code only**, installed by
   `./install --game fo-roots` (bash) on the user's own setup. It is never in the GUI
   installer, the release packages, `PLAYING.md` or the release notes, and is not
   advertised, until the user says otherwise. Roots' layers stay out of what
   `publish/package.sh` and the GUI installer offer.
5. **Reuse**: Roots-specific knowledge is kept in the profile, the game descriptor and
   the content set (decision 7b), so other Fleet Operations-based mods could add their own.

Still open:

6. **The submods**: whether "Fleet Ops 4.0" and the classic mods (their own executables)
   are wanted at all (not by default: decision 9).
