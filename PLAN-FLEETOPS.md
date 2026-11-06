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
| dialogs | `dialogs.reads_screens` | down | all 30 dialog procedures: `ShellBitmapImp` in each (the shell's bitmaps FleetOpsHook's, scaled with `StretchBlt`), twelve with Roots' 1200x900 constants; **nine `do_*` screens open through FleetOpsHook's own `DialogBoxParamA` import**, past `ArmadaL.exe`'s import table where the host sits; **eight never run: the main menu, Options (the in-mission menu too), Yes/No, Manual IP, Multiplayer Connection, Map Select, Graphics and Advanced Options are FleetOpsHook's Delphi forms** (step 7) | stays Roots' (decisions 68, 140) |
| shell | `shell.screens` | down | `DispatchDialog`, `SetActiveDisplay`'s wrapper, `Show_Credits`, and `SetCurrentBackground` replaced whole | stays Roots' (decision 140) |
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
  every screen; the main menu and seven other screens Delphi forms of its own (step 7);
  nine screens opened through its own `DialogBoxParamA`; its own Win32
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
| 4 | **The panels Roots changes least**, each its own row and off on Roots until verified: messages and objectives, the pause and replay boxes, overlay text, chat and trade, the top-right buttons (Roots' five), the resources (Roots' amounts, names from the set), the selection overlay, and the engine's last draws (`fade`, `frame_rate`, `order_ring`, `footprints`; the drag box stays the engine's). Sounds measured under FMOD first. **Built 2026-11-06** (branch `fo-6-step4`, on `fo-6-step5`): `hud.panels`, `hud.order_ring`, `hud.frame_rate`, `panels.presses`, `panels.chat` and `panels.trade` verified; seven panels on by default on Roots (the resources, the top-right buttons, the pause box, the chat, the selection overlay, the frame-rate panel, the 3D view's order ring, height posts and footprint), each with its `fo-ui-<panel>` scenario; messages, objectives, replay, overlay text, fade and the team texts stay off for want of a trigger in a skirmish; the UI plays no button sound on Roots; decisions 120–127, "Step 4: what the bench showed" below | profile marks, a panel at a time | 12–20 · 5–8 |
| 5 | **Measure the bar, draw nothing.** Log FleetOpsHook's bar on the bench: the buttons per selection (a ship with several weapons of one `classLabel`, the construction ship's build list, a station), the mode in use and its columns and rows, which button the range ring follows and through which of the stock bar's states. Decide from it whether a bar of ours can keep the rings without reading FleetOpsHook's memory (decision 65). **Measured 2026-11-06** (branch `fo-6-step5`): `bar: measure` and `bar: hover` log lines from the engine's own objects (`engine/palette.rs`, `log_measure`), scenario `fo-ui-bar-measure`; the answer is decisions 94 and 95, "Step 5: the bar measured" below | bench and logs only | 4–8 · 2–3 |
| 6 | *Optional, when asked*: **the bar**, laid out by the mode's columns (no 5x3 grid; position keys for the first cells only, QOL's in FO-7), then the info panel and the minimap with FleetOpsHook's own objects, the command info bar left to Roots | step 5; Roots developer's view on the look | 25–45 · 10–15 · **the bar part built 2026-11-06** (branch `fo-6-step6`): a skin over the stock buttons, off on Roots by default; decisions 128–131, "Step 6: the bar as a skin" below; the info panel, the minimap, the floating mode and several weapons of one `classLabel` are not done |
| 7 | *Optional, when asked*: **the menus**: the host's design space taken from the profile's shell (1200x900, not a fixed 800x600), the generic mirror first, specs after; the nine screens that open through FleetOpsHook's import stay Roots' (decision 68). **Stopped after the offline survey 2026-11-06** (branch `fo-6-step7`): the main menu is not a dialog on Roots, so the host cannot reach it; nothing built, nothing benched; decision 140, "Step 7: why the menus stay Roots'" below. **Option C planned 2026-11-06** (branch `fo-6-optionC-plan`): our menus over FleetOpsHook's forms, probe first; decisions 141–152, "Step 7, option C" below. **Option C taken 2026-11-06** (branch `fo-6-optionC`): the probe passed ("Step 0: what the probe showed"); decisions 170 onwards | step 3 | 2 (survey) · 3 (probe) |
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


### Step 6: the bar as a skin (2026-11-06)

Branch `fo-6-step6`, on `origin/fo-b`. With `[HUD] Bar=1` on a build whose bar is behind a pointer
(`palette_slots_indirect`), `ui/src/hud/barskin.rs` draws each shown button of the stock bar in the
theme (`widgets/theme.rs`) over its own rectangle: the palette's origin (its rectangle's top left, in
canvas units, scaled per axis) plus the button's local rectangle, buttons walked in slot order, a
two-pixel gap inside each 88-unit cell. The icon is found as on stock: the HUD's sprite database by
the command's or weapon's `ButtonName`, drawn as the UI's flat icon where it has one for the name
and otherwise the game's own art, read from the player's install at run time (the build list's
building icons, the weapon and trade icons); a button with no sprite shows its name without Roots'
`npc_` or `all_`. A button's look follows the engine's own bytes: hover (`+0x38`), pressed (`+0x30`),
disabled and active (`+0x34`). Nothing is claimed (no `interact`, no panel behind the row) and nothing
is pressed: the stock bar owns the pointer, so the click, the rollover and the tooltip are the
engine's. `engine::hud::set_bar_skin` makes `RenderAll`'s drawing of the palette skip (the panel
switch's `taken`) while `RefreshOwner`'s mouse call is left alone for it (`cursor_over`); the stock
bar's path (21 inline buttons, the 5x3 plan, QOL's keys) is not touched (`palette::slots_indirect`
chooses). `bar::shows` is false for a skin, so the bottom row's room is not taken.

- **Decision 94's open check: the ring and the tooltip survive.** Interactive bench (Roots, 1280x960,
  Vulkan with validation, Federation, `HUD.Bar=1`): with the construction ship selected, the pointer
  on the Tractor Beam button (slot 7) gives the red range circle on the map (radius 400, centred on
  the ship) and Roots' tooltip beside the pointer ("Tractor Beam (Hotkey: B), 400 Range, Locks on to a
  friendly or neutral target..."), the skin's button drawn lit. On the Repair button the circle is gone and
  its tooltip shows. A click on Build opens the build list (11 buttons: nine buildings with Roots'
  own icons, the unaffordable ones dimmed, cancel, back), and a click on back returns to the eight
  commands (`bar: measure menu=0` again in the log). The hover line in `UI.log` follows each move
  (`hover slot 7 Weapon npc_tractorbeam ... pointer owner: the bar`).
- **What the skin looks like.** Round lavender buttons of the Federation theme in one row, 70 px
  apart at 1280x960 (one per shown button, so the row's length is the selection's: six for an escort,
  eight for a construction ship, ten for a Starbase, eleven in a build list), over the stock
  buttons' rectangles, above the info strip and beside the minimap where Roots puts them.
- **Scenarios.** `fo-ui-bar` (the skin: `bar: ours as a skin`, the layout lines, the Tractor Beam ring and
  tooltip judged, no ring on Repair, the build list and back by clicks, no VUID, no crash) passed
  (20261106-171404), as did `fo-ui-bar-off` (the same steps with `HUD.Bar=0`, Roots' own bar; its
  ring and tooltip checks pass, its last judge step's wording was loosened); stock `ui-hud-bar` at 1280x720 with `--install .`
  passed (20261106-171734, its 5x3 grid, QOL's keys and labels, the placement footprint unchanged).
  The agent selects the construction ship (a box, never a click) because the fleet's ships change
  places, and ends with a step that puts the pointer where the bench believes it is: a pointer moved
  by another process parted the bench's aim in both the skin's and the A side's runs (the
  cursor drawn about 100 px off, then Roots' edge scroll); every later hover and click starts from
  the same point above the bar.
- **What it cannot do.** It does not move or re-flow the row (decision 95's re-laid-out bar would lose
  the ring: the skin never lays out), so the Refit row for the bar changes the look only.
- **Not measured**: a ship with several weapons of one `classLabel` (a button each, each its own hover),
  the floating mode, a list over 14 buttons (the skin reads rectangles, so it would follow the engine
  wherever it wraps, but it was not seen), the info panel and the minimap (still Roots').

### Step 4: what the bench showed (2026-11-06)

Branch `fo-6-step4`, on `fo-6-step5`. `./a2test syms --game fo-roots`: 82 of 108 hooks ready (76 before step 4; stock
106 of 108, unchanged). Verified, each against the MemDiff log of FO-2 (no changed range touches its sites) and the
file: `hud.panels` (`DisplayInterface::RenderAll`'s entry is FleetOpsHook's trampoline and the Render call behind it and
`CursorInterface::RefreshOwner`'s CursorOver call are stock's), `hud.order_ring` (`mRenderWhere`'s edits are FleetOpsHook's
own calls, away from the two DrawQuad calls the hook takes), `hud.frame_rate`, `panels.chat` and `panels.trade`
(`Armada_RenderFrame`'s one edit, +0x245, is away from their call sites), `panels.presses` (`Program::DisplayInputProcess`
is trampolined at its entry; the ResetVisibility call the hook takes is stock's); `hud.footprints` was ready since step 1.
Bench: Roots at 1280x960, Federation against Klingon in Instant Action, Vulkan with validation, one scenario per panel and
`fo-ui-hud` for the seven together.

| Panel (row) | Verdict | Evidence |
|---|---|---|
| Resources (`HUD.Resources`) | **on** | `fo-ui-resources`: three cells for the Federation (dilithium, metal, biomatter: the components found through the panel's array, the resource id checked against the slot), Roots' own strip hidden by the panel switch; Roots' components answer a tooltip that is a string-table token (a control character), so the name comes from the `fleetops` set when the tooltip's first line is not a word (decision 124) |
| Top-right buttons (`HUD.Buttons`) | **on** | `fo-ui-topbar`: Roots' panel holds the stock buttons (`buttons: panel` logs each field's rectangle: idle, view, comm, objectives and the "more" toggle laid out, the menu behind the toggle, help and palette unplaced), all recognised by their press functions; ours draws menu, comm, idle, view and objectives, a click presses Roots' own button (`panels: click idle` selected the idle ship and moved the camera, `click view` swapped the view), the tooltip is the engine's text |
| Pause box (`HUD.Pause`) | **on** | `fo-ui-pause`: the pause key shows the box, the Continue button presses the stock unpause (`unpaused the game` follows in the chat); Roots' string table answers both keys with an empty string, so the box has words of its own (decision 124) |
| Chat (`HUD.Chat`) | **on** | `fo-ui-chat`: the entry follows each key (`chat: entry active "steamuser: " "hello roots"`), FleetOpsHook's text input still receives them, the lines are ours (`steamuser paused the game.`), Roots' own entry and lines are not drawn twice |
| Selection overlay (`HUD.Selection`) | **on, one caveat** | `fo-ui-selection` against `fo-ui-selection-off`: ours draws corner brackets and the gauges (health green, shield blue) where Roots draws only the gauges; Roots' unit tooltip, bar, info strip and minimap are unchanged. FleetOpsHook's `SelectionDisplay` callbacks (its group colours and tooltips) are skipped while ours is on: not seen in the skirmish, so unjudged (decision 127) |
| Frame rate (`HUD.FrameRate`) | **on** | `fo-ui-frame-rate` (`--rts FRAMERATE_DISPLAY=1`): the engine's lines become one panel of ours; it takes the mouse where it is drawn, as on stock |
| Order ring, height posts, footprints (`HUD.Overview`) | **on** | `fo-ui-overview`: the move point is marked at the click, the placement of a building shows ours (the footprint hatched red where it is blocked, amber tiles for the cells taken; `leftovers: 4 of the engine's last draws are ours`). The drag box is the engine's (decision 74), and its stray end marker at the pointer is as without the UI. The log's `order ring at 520,744` four times over names one degenerate quad of the stock call; Roots draws its own marker through FleetOpsHook's call in `mRenderWhere`, so the ring seen at the click is probably Roots' (unconfirmed: a bench with the row off, a right-click and a screenshot would settle it) |
| Messages, objectives (`HUD.Messages`), overlay text (`HUD.Overlay`), replay (`HUD.Replay`), fade (`HUD.Fade`), team texts (`HUD.TeamTexts`) | **off** | claimed and read without error in the probe, but a skirmish raises none of their boxes (no script message, no objectives, no replay, no fade), so no scenario can show them; none is on for want of one. The objectives button is dimmed in a skirmish |
| Comm, drop box, load wait | **off** | multiplayer only (two games; not asked in step 4) |
| Drag box | engine's | decision 74 |
| `panels.sound` | **off** | decision 126 |

**Sound.** Every call site that makes a button sound in Roots (`StandardButton::Simulate`, `ShellButton::SetButtonMode`,
the bar's key handler, the audio queue) has both its `operator new` call and its constructor call re-pointed to
FleetOpsHook's own FMOD classes, whose object is not the stock 0x1c bytes; the UI's own sound is a stock-size object made
with the game's `operator new` and the stock constructor, which would sit under FleetOpsHook's vtable. It was not played to
find out: a build value (`panels_sound_ours`, 0 on Roots) turns the UI's button sounds off, the HUD part no longer requires
them, and the panels are silent on Roots. The first attempt, marking `AudioSound2D::AudioSound2D` as changed, stood the HUD
down whole (the HUD part required the sound hook) and also Render3D's reads, which share the symbol; a value is the cleaner
switch.

What the bench rounds showed and fixed:

- **A mark is by function, across plugins.** Marking a function changed stands down every hook of every plugin whose
  entries use it; `verified` names the hooks the mark does not apply to. A thing one plugin must switch off goes in a
  profile value.
- **The HUD part stood down on "an engine function is not stock"** because `api::sound::functions_stock()` was required for
  the whole HUD; sound is optional now.
- **Roots' string table gives "" for `GUI_POP_PAUSE_TEXT` and `GUI_POP_PAUSE_CONTINUE`** (stock gives words), and its
  resource tooltips are tokens; both read as a blank box or `?` before the fallbacks.
- **The frame-rate panel sits over the top left** (to y 340): a drag that starts there is the panel's, not the map's.
  Scenarios start their boxes below it.
- **`fo-ui-topbar` failed once on `_fo-sides`' "Klingon is visible"** (OCR of the setup screen), and stock `ui-hud-bar`
  once on a selection that did not land (`| Nexus |` not logged); both passed on the rerun, neither on a step of this
  work.

**Acceptance (2026-11-06, `fo-6-step4`).** Roots, 1280x960: `fo-ui-resources`, `-topbar`, `-pause`, `-chat`, `-selection`,
`-frame-rate`, `-overview` and `-hud` pass every log check (the screens left to the judge are in the reports); the earlier
`fo-ui-mission`, `-refit`, `-standdown`, `-loading`, `-loading-off` and `-bar-measure` carry `HUD.Enable=0` so that they
still test what they did (not re-run in this step except through `-hud`). Stock at 1280x720 with `--install .`:
`ui-hud-bar` (a flake on its first run, a pass on the second) and `ui-hud-panels` ran, the log checks pass.

### Step 4a: the resources' names and place on Roots (2026-11-06)

The user's report from the Federation economy tutorial at 3440x1440: "metal should be tritanium, biomatter should
be supplies", and the strip of ours "painted over HUD drawn by Fleet Ops" (branch `fo-ui-resources-fix`, on `fo-b`).
**Names.** The `fleetops` set carried stock's words (Metal, Biomatter). Roots renames them in its string table:
the long tooltips of the resource components (`GUI_RD_*_VTOOLTIP`) read Dilithium, Tritanium, Supply
(`GUI_RD_BIOMATTER_*`), Biomatter for the Species 8472 (the alternative keys `*_BIOMA_ALT_*`, chosen by the race's
ODF) and Connections Nodes for the Borg's latinum, while the short tooltips are one icon glyph each (the
"unresolved token" of step 4: a font glyph, not a key). The panel now reads the name from the long tooltip when
the short one holds no word, so a race or a build that renames is followed; the set's `Names` (Tritanium,
Supply, Connection Nodes) remain the fallback. Measured on every race (`fo-ui-resources-<race>`): Federation,
Klingon, Romulan, Cardassian and Dominion read Dilithium, Tritanium, Supply (Borg and Free Borg:
Connections Nodes for the latinum, Roots' spelling), the Species 8472 Biomatter alone.
**Place.** At the user's aspect the canvas file is the 16:9 one (2134x1200 in the layout, converted to the 1600x1200
space); Roots' own `ResourcePanel` sits at [75,8,426,55] there, top left, and its top centre belongs to its
command info bar offset, messages and build areas. Ours was centred at the top, as on stock. The strip now sits at
the top left corner of the game's component rectangle (`[Resources] Placement = component`; stock keeps the top
centre). On the bench at 2150x900 (2.39, the user's aspect; the shell needs 900 high) the tutorial with the
Enterprise selected shows the strip over Roots' empty resource spot, clear of the info strip, the bar, the
minimap and the avatar box's column; no overlap with Roots' art was reproduced at the top centre while the
strip was there, so the cause is read as the top-centre placement meeting Roots' top-centre elements (the
command info bar and messages, hidden in the tutorial's first minutes), not a draw order.

### Step 4b: Roots' tooltips through the UI (2026-11-06)

Branch `fo-tooltip-fix`, on `fo-b`. The user's report on their Roots install (`fo-b` 8d8270fa, the Federation
economy tutorial): "almost all tooltips have some weird symbols, question marks, etc."

**Whose they are: the UI's** (`HUD.Tooltips`, "The tooltips at the pointer" in `ui/HUD.md`). Roots ships the row at
0 (decision 125); the player's `RefitPlayer.ini` turned it on with the bar, the info panel and the minimap, and their
`UI.log` says `tooltips: ours` and logs each tooltip it drew, every one holding bytes 0x7f..0xa0 and letters such as
`¤ ª © ¨ £ ¥ × ¾ ø`. The A side, `fo-ui-tooltips-off` (Roots' defaults, Display8, render3d and the UI installed),
shows Roots' own drawing of the same text right: red figures, resource icons, class names as yellow words. With the
row on, the stock `Render` draws no tooltip, so Display8's passthrough, render3d and QOL are not in the path; and it
is not Roots' own bug at the player's resolution (below).

**The cause.** Roots' tooltips are written in its fonts' own codes. Its sprite fonts (`Sprites\FontFinal4_*.spr`,
`FontMedium.spr` ...) give some codes pictures of their own: ten red figures (0x7f is 0, 0x81 1, 0x8d 2, ... 0xd7 9),
the resource icons (0x85 dilithium, 0x86 tritanium, 0x87 supply, 0x89 crew, 0x8c biomatter, 0x99 connection nodes,
0x80 a clock), and the unit classes as plates of words (0xa4 Scout, 0xaa Construction Ship, 0xf8 Station, ...); a
cost line sets its icons apart with `$` (`350$<dilithium>$/$175$<tritanium>$/...`), and tabs lay the text out beside
a picture. The UI read every byte as Latin-1 and drew it in Noto: the C1 codes and tabs as `?`, the Latin-1 letters as
themselves. Stock's own bullet (0x95, Windows-1252) drew as `?` too. Decision 124's "string-table token" for the
resource tooltips was this: the short tooltip is the resource's icon alone. Roots' small font tiers (`FontFinal4_10`,
`_13`, used at 1280x960) draw `$` and 0x80 as `$` and `€`, so Roots' own cost lines read `350$◆$/$...$60$€` there;
the larger tiers a wide screen uses make `$` a spacer and 0x80 a clock (decision 162).

**The fix** (ui): `text::game` reads the engine's text as Windows-1252 and writes the content set's `[Glyphs]`
(`ui/content/sets/fleetops/ui.ini`) as digits and words, where the engine's text enters for display (the string
streams behind every tooltip, `buttons::clean`); the layout draws a tab as a space and other control characters as
nothing; the font bakes the rest of Windows-1252. A Roots tooltip now reads "350 Dilithium / 175 Tritanium / 8 Supply
/ 150 Crew / 50 s", "1500 Hitpoints", "Construction Ship", "Weak vs: Artillery, Berserker, Battleship, Dreadnought".

**Evidence.** `fo-ui-tooltips` (Roots, 1280x960, Vulkan with validation): the Tractor Beam, the starbase, Activate
Bastion and Buy Dilithium tooltips logged and drawn as plain text, `\u{` nowhere in `UI.log`, no VUID; the resource
strip reads Tritanium and Supply. `render3d/hosttest` (`ui_fleetops_glyphs`) holds the cost line, the hitpoints, the
classes and the bullet from the player's log. The build list's cost line was not reached on the bench (a box round
the station alone does not select it reliably); it is held by the host test. Stock at 1280x720 with `--install .`:
`ui-hud-bar` and `display-stub` (below, in the PR).

### Step 7: why the menus stay Roots' (2026-11-06)

Offline, as the step asked first: every `DialogBoxParamA` and `CreateDialogParamA` call in
`ArmadaL.exe`, the function it sits in, FleetOpsHook's patch there (the private patch table)
and what FleetOpsHook's replacement does, read from its shipped map and its Delphi form
resources. No game was run. The per-procedure table is in the private notes
(`fo6-step7.md`); in short, of the 30 dialog procedures `dialogs.reads_screens` names:

| On Roots | Procedures | The host |
|---|---|---|
| **Never run**: FleetOpsHook shows a Delphi form of its own instead (`ShowModal` on a `TForm` placed in its 1200x900 window), and does not call the stock function | the main menu (`do_mainMenu`'s detour never calls its trampoline), Options and the in-mission menu (`do_escapeMenu`'s two calls in `DispatchDialog`), Yes/No, Manual IP, Multiplayer Connection, Map Select (from Game Setup), Graphics Options, Advanced Options: **8** | nothing to host: no dialog, no dialog procedure; the forms' buttons are windowless graphic controls |
| **Behind FleetOpsHook's import**: the stock procedure runs, behind a dialog procedure of FleetOpsHook's that places the dialog in its form, disables the form and forwards every message | Single Player, Game Setup (Instant Action and every multiplayer setup), Game Settings, Load Game, Save Game, the licence, the Technology Tree, the Admiral's Log, Internet: **9** (decision 68) | past the host; even a hook in user32 would meet FleetOpsHook's procedure first and have to read its record for the stock one |
| **Through `ArmadaL.exe`'s import** | Sound Settings, opened by FleetOpsHook's forms with the engine's window as owner; the GameSpy and lobby dialogs, Map Transfer, Net Waiting, Password, Game Name, Help, One Button, opened from a screen of the two rows above; Mission Select (campaigns, which Roots has none of): **13** | only Sound Settings would be hosted; the rest are refused by owner (their owner is a window the UI does not host), unreached or GameSpy's |

So the main menu, the first screen of the front end, is out of the host's reach, and so are
Instant Action's setup and the in-mission menu: the bench passes the step asked for (the main
menu and the Instant Action setup through our host) cannot be had without hooking
FleetOpsHook's memory, which the step ruled out. Nothing was built and nothing benched; the
host's design space from the profile's shell size was not started, since no screen would use
it. What was weighed (detail in the private notes):

- **The front end stays Roots'** (taken, decision 140). The UI on Roots keeps the loading
  screen, the mission frame, the seven panels and the Refit modal on Ctrl+F12 (decisions 62,
  86), and nothing of FleetOpsHook's is hooked.
- Host the nine through a hook in user32 (inline or a CBT hook), reading FleetOpsHook's
  record to find the stock procedure and working against its placement and its
  `EnableWindow`: reads FleetOpsHook's memory and changes its behaviour by proxy, and the main
  menu, Options, Map Select and the rest stay Roots' forms, so the shell would alternate
  between two looks. Not taken.
- Draw over the Delphi forms: their controls have no windows, so a mirror would walk Delphi
  objects in FleetOpsHook's memory. Excluded.
- Host Sound Settings alone, the one screen within reach: one screen of ours among Roots'
  forms. Not taken.
- **When contact opens** (decision 1): a switch in FleetOpsHook that lets the shell's screens
  run as the executable's dialogs, or an interface that names a form's buttons, is the one
  clean path to a front end of ours on Roots.

`[Menus]` stays at 0 in `ui/defaults/fo-roots.ini` (`Enable`, `Credits`, `Scene`,
`CampaignLocks`), as it was; stock is untouched (no code changed), so no stock run was due.

### Step 7, option C: our menus over FleetOpsHook's forms (plan)

Planned 2026-11-06 (branch `fo-6-optionC-plan`), offline: FleetOpsHook's Delphi form
resources, its shipped map and its classes' run-time type information, and the UI's stock
menu host, read without running the game or building anything. On that day the user allowed
reading FleetOpsHook's memory for this: "read nothing of FleetOpsHook's memory" was model
(c)'s default (decision 3), not a rule of the user's. So option C of the list above is no
longer excluded, and this section plans it. Class, component and handler names, object
offsets and the per-screen dispatch are in the private notes (`fo6-optionC.md`). Decision
140 stands until the user takes this plan up.

**What the forms are.** Each of the eight screens is a Delphi form of FleetOpsHook's,
created when the screen opens, shown modal and freed when it closes: a top-level pop-up
owned by the engine's device window (itself a form of FleetOpsHook's), not a child of it,
with screen-sized forms pinned to the device window's top-left at 1200x900. The buttons are
graphic controls with no window of their own. A button's size comes from its bitmap at run
time, the Options form stacks its buttons in code each time it opens, and every caption is
set from the game's language table as the form is created, so no position or caption in the
form resources can be trusted: everything is read from the live objects. Which buttons show
depends on the context (the in-mission menu and the shell's Options are one form), and two
buttons are bound to their action only when the form opens (the Options form's Back, and
Multiplayer Connection's Internet). A press sets the shell's mode and closes the form, so
the next screen is again the engine's choice. Three screens write settings in their own
handlers: Manual IP saves FleetOpsHook's settings file, the graphics screen writes the
profile as a control changes and saves it on Back. A button fires on a left press and
release over an opaque pixel of its bitmap, its centre included, with no hover needed.
Text fields, lists and the map grid are ordinary windows. While a form is modal the thread
waits in the form's own loop: nothing renders or presents a frame.

**How Roots' releases differ.** FleetOpsHook is one build edited in place across Roots
4.0.93, Roots 5.0Z11 and Tactical Maneuvers 5.2.3: same size, stamp and layout, 1 to 3 KB of
byte edits between any two, mostly in game logic; every symbol of the one shipped map keeps
its address, no class changes its size or published fields, and the form resources are
identical but for one screen's control positions. Fleet Operations 3.2.7 is a different
build (addresses moved, forms laid out at 800x600). `ArmadaL.exe` differs per release.

**The mechanism.**

1. **Finding the form.** A thread CBT hook of the UI's sees each window the game thread
   creates or activates. A form of FleetOpsHook's is told by its window class (the Delphi
   class name) and its owner (the device window); its object comes from the property the
   VCL itself sets on each of its windows, the property's name rebuilt from FleetOpsHook's
   module handle and the thread's id. The object counts only when its class pointer lies in
   FleetOpsHook's image and the class name there is the one expected. No global of
   FleetOpsHook's is read.
2. **Reading the controls.** By name, through the class's own table of published fields
   (how the VCL streams a form), each control's class checked by name; rectangle,
   visibility and the published events through the class's property information. The few
   private fields (caption, enabled, the control lists, the form's state) are profile
   entries, checked per build by the tool of decision 147. Every read is guarded: on the
   game thread, only while the form is the modal one and not being destroyed, every pointer
   aligned and in user memory, nothing kept past a dispatched message.
3. **Hiding Roots' drawing.** The form gets an empty window region. That one call clips its
   paint, the buttons' direct repaints (they draw through the form's DC on every state
   change, not only in a paint message) and its hit-testing, writes nothing of
   FleetOpsHook's, and FleetOpsHook never sets a region on these forms. Fallbacks, in this
   order: a nearly transparent layered window; a subclass with an empty paint, moved off
   screen. A patch of FleetOpsHook's paint methods is rejected.
4. **Drawing ours.** A thread timer of the UI's (about 60 Hz) is dispatched by
   FleetOpsHook's own modal loop like any message; in it the stock host's `frame()` draws
   the backdrop or scene and the UI's widgets and presents on the engine's shell device, in
   Display8's one mode. The design space is the form's, 1200x900 (the profile's shell
   size), fitted to the back buffer as the host fits 800x600. Nothing is written into
   FleetOpsHook's application object. The UI's model (`menus::model`) gets a second source
   beside its Win32 walk: a form's controls as `Control`s, with rectangles, captions, states
   and an action each.
5. **Acting.** A press is posted to the form's window: left button down and up at the
   centre of the live rectangle, in the form's client coordinates. It runs through
   FleetOpsHook's loop and its button code, so the action is the one Roots binds at that
   moment. Text fields, lists and combo boxes are driven through their windows as the stock
   host drives a dialog's. Nothing calls a handler or `Click` directly (that would run a
   nested modal form on the UI's stack), and nothing writes a setting: FleetOpsHook's
   handlers write what they write, as on Roots today (MENUS.md, "Writing a screen spec").
6. **Input.** With the region empty the real pointer falls to the device window, which one
   screen (Yes/No) leaves disabled. So the pump polls the pointer and the buttons itself,
   and keys are taken from a thread `WH_GETMESSAGE` hook, which can keep a key from the form
   while a field of ours has the keyboard.
7. **A picture of the form** for the mirror mode (`Generic=mirror`): the form printed into a
   memory DC (the VCL answers the print message, graphic buttons included, whatever the
   region). It gives every screen a mirrored first step with no widgets, the map grid
   included.
8. **The nine dialogs behind FleetOpsHook's import** (Single Player, Game Setup, Game
   Settings, Load, Save, the licence, the Technology Tree, the Admiral's Log, Internet) are
   a second phase, by **late attach**: the stock dialog is found as it is created and left
   in FleetOpsHook's hands (its placement, its procedure, its owner's enable state). The
   record FleetOpsHook keeps on the window names the stock procedure, which the UI checks
   against the profile's shell range and uses to name the screen; the stock host's model
   (which reads the shell's widgets as on stock) then runs on that window, hidden by the
   same empty region and printed for the mirror. Until then these nine stay Roots', so a
   player with the forms on sees two looks in turn.

**Identity and safety.** The forms are keyed to **FleetOpsHook alone**: a list of measured
identities (the three family members today); any other FleetOpsHook stands the forms down,
logged once, whichever `ArmadaL.exe` build runs. Everything that reads the executable stays
keyed to its own build profile, as now. No address of FleetOpsHook's is in the profile or
the code. The shipped map is not read at run time (it describes the unpatched build: good
for names and addresses, not for bytes); a tool reads it once per new FleetOpsHook
(decision 147).

**Hard rule 9.** The repository says only what a hook needs, in our words: that the screens
are VCL forms, how a form, a control and a press are found and made, and which VCL
mechanisms are used. FleetOpsHook's class, component and handler names, its addresses and
its form resources stay in the private notes; a screen spec names the controls it uses by
the names the forms give them, which are symbol names the rule allows.

**The look** (decision 60). Ours replaces Roots' front end only when the player turns it
on: the forms part ships off on Roots (decision 149), and with it off nothing of this runs.
Stock is untouched: the new source, hooks and timer exist only where the profile says a
build has forms.

**First bench probe** (step 0; the questions are numbered in the private notes): does an
empty region hide a VCL pop-up under Wine on XWayland/Hyprland, and does the form keep focus
and its loop; does a present on the shell device from the timer reach the screen while a
form is modal; does the timer fire inside the form's loop and inside the nine dialogs'
loops; does the window property give the object, and is the window class the Delphi class;
does a posted press on Options' Back act with no hover; does a printed form include its
buttons; which window gets the real pointer, and does polling read it right under Wine's
scaling; where is the cursor drawn while a form is modal; does a combo box changed through
its window fire the form's change handler.

**Work list.**

| Step | What | Agent h · bench |
|---|---|---|
| 0 | **Probe**: a bench-only switch in the UI that finds the main menu's form, empties its region, runs the timer pump with a plain frame, logs the controls it reads and posts one press; answers the nine questions. Go or stop here **Done 2026-11-06** (branch `fo-6-optionC`): go, "Step 0: what the probe showed"; the region failed, the off-screen fallback works (decision 170) | 4–6 · 1–2 |
| 1 | **Identity**: the tool of decision 147, the companion list of decision 148 (three identities), a hook-level "requires the companion" flag in `a2engine` and its C twin (about 30 lines each; no module-relative addresses needed), negative tests **Done 2026-11-06**: `FAMILY` in the UI by `a2engine::companion_crc`, `tools/engine/fo-forms.py`; the hook-level flag and its C twin not built (decision 172) | 6–10 · 0 |
| 2 | **The VCL reader** (`ui/src/engine/forms.rs`): window to object, class checks, published fields and properties by name, the guarded walk; host tests on a recorded object image **Done 2026-11-06**: part `ui.forms` (the VCL's fields in the profile), the owned-component walk, class checks, the guarded reads; no host tests (decision 173) | 10–16 · 1–2 |
| 3 | **The forms host**: the CBT and message hooks, the timer pump, the empty region and its fallback, input by polling, the model's second source and an action back end (`menus::model` and `actions` get a source-neutral seam), the claim and Display8's mode **Done 2026-11-06**: the CBT hook, the timer pump, off screen, the button polled under a disabled owner (decision 183), no message hook (decision 175); a host of its own beside the dialog host's model rather than a source-neutral seam in `menus::model`; no Display8 claim needed (the shell's device is already the desktop's size) | 16–26 · 4–6 |
| 4 | **Generic screens**: the mirror (printed form, forwarded presses) and the widgets mode over a form's controls, at 1200x900 **Done 2026-11-06**: `menus/roots.rs`, widgets and mirror (decisions 174, 184) | 8–14 · 2–3 |
| 5 | **Specs** for the main menu, Options (shell and mission), Yes/No, Manual IP, Multiplayer Connection, Graphics, Advanced Options; Map Select stays mirrored (decision 152) **Done in part 2026-11-06**: the main menu and Options (shell and mission) as a column, Yes/No over its picture; the rest generic (decision 176) | 18–30 · 5–8 |
| 6 | *Phase two, optional*: **the nine dialogs** by late attach, generic first **Not done** (decision 178) | 20–35 · 6–10 |
| 7 | **Scenarios and stock**: `fo-ui-forms-main`, `-options` (shell and mission), `-yesno`, `-manualip` (the address typed, the setting written by Roots' own handler and kept after a relaunch), `-graphics`, `-standdown` (forms on with an unlisted FleetOpsHook: Roots' own look, compared as decision 93); the bench's design-to-screen mapping for 1200x900; stock's `ui-menus-*` set unchanged at 1280x720 **Done 2026-11-06**: `fo-ui-forms-main`, `-options` (in a mission, with Yes/No), `-graphics`, `-manualip` (typed and cancelled: decision 181), `-standdown` (the hook forced down: decision 180), `fo-ui-forms-probe`, all passing on Roots 5.0Z11; the clicks by caption (decision 179); stock's `ui-menus-*` below | 6–10 · 3–4 |

Without phase two about **68–112 agent hours and 16–25 bench passes**; with it about
**88–147 hours and 22–35 passes**. Roots at 1280x960 on Vulkan, one Roots game at a time.

**Against the other options.**

| Option | Cost | What the player gets |
|---|---|---|
| A, the front end stays Roots' (taken) | 0 | Roots' menus; the Refit modal on Ctrl+F12 |
| B, host the nine through user32 | about 25–40 h | nine screens of ours among eight of Roots': two looks in turn, the main menu Roots' |
| C, this plan | 68–112 h (eight forms), 88–147 h (all seventeen) | the whole front end in our style with phase two, opt-in |
| D, ask the Roots developer for a switch or an interface | ours small once it exists (10–20 h); waits on contact (decision 1) | the clean path; not available now |

**Recommendation: no-go on the build for now; go on step 0 when the user wants Roots' menus
in our style.** C is feasible, and better than B: it covers the main menu, reads only what
the VCL publishes plus a few checked fields, writes nothing of FleetOpsHook's, and the
family's stability makes one table enough. But it is the largest UI piece on Roots for a part
that ships off, the most used path (Instant Action's setup) keeps Roots' look without phase
two, and its whole case rests on the probe (an empty region, a timer frame and a posted press
under Wine). So: the probe (step 0, 4–6 hours, one or two passes), then the user's go for
steps 1 to 5 and 7, and phase two only after the eight forms are seen in game.

**Risks.**

1. **Wine and the compositor.** An empty region or a layered window on a pop-up may behave
   differently under XWayland; focus or the modal loop may break. The probe answers it
   first; neither fallback is free.
2. **Lifetime.** The forms are freed after every screen, and a press starts a nested modal
   form: a pointer kept a message too long is a use-after-free. The guarded walk and "read
   inside the pump, never across a dispatch" are the defence; step 2's host tests pin them.
3. **Unmeasured FleetOpsHook releases** (Roots 5.0, Z1 to Z10, two identities named by
   A2FOExtensions). The family edits bytes in place, so any release may change a form's code
   without moving it. The identity list refuses everything unmeasured; the tool makes a new
   member a re-run plus the step 7 scenarios.
4. **Run-time bindings.** Back, Internet and the per-context visibility are set when the
   form opens; a spec that assumed a button's meaning from its name would act wrongly. Specs
   read visibility and the bound action's name live, and fall back to the generic widgets
   when a control they expect is missing.
5. **Two looks** until phase two: the shell alternates between ours and Roots' at Single
   Player, Game Setup, Load and Save.
6. **Settings written by Roots' handlers** (Manual IP's settings file, the graphics
   profile) are Roots' behaviour; a press of ours makes the same writes, and no row of the
   Refit screen maps onto them.
7. **The cursor and the modal device.** Where Roots draws the pointer during a form, and the
   shell device's state while nothing presents, are unmeasured (probe).

*A later extension, not planned:* since FleetOpsHook is one build across Roots 4.0.93, Roots
5.0Z11 and Tactical Maneuvers 5.2.3, the other two could become game profiles cheaply; only
their `ArmadaL.exe` would need a profile of its own.


### Step 0: what the probe showed (2026-11-06)

The user gave the go on 2026-11-06 ("please go ahead"). The probe is `[Menus] Forms=probe`
(`ui/src/engine/forms.rs`, the bench only), scenario `fo-ui-forms-probe`, three passes on
Roots 5.0Z11 (Display8 on Vulkan with validation, 1280x960). It walks by itself: the main
menu's Options, Options' Graphics, Graphics' Back, Options' Back, the main menu's Instant
Action. The nine answers, in short (detail in the private notes, `fo6-optionC.md`, section 8):

| # | Question | Answer |
|---|---|---|
| 1 | An empty region hides a form | **No, on Wine under XWayland**: Roots' art goes, but the pop-up still covers its rectangle in black over our frame, and a layered window at alpha 1 does the same. **The listed fallback works: the form moved off screen** keeps the activation, the focus and its modal loop, and our frame shows whole; FleetOpsHook never moved one back during the walk (decision 170) |
| 2 | A present from the timer reaches the screen | yes: over 7,000 frames on the shell device while forms were modal, no validation message |
| 3 | The timer fires inside the loops | yes: about 62 a second inside a form's modal loop, about 53 inside the Instant Action setup dialog's (one of the nine) |
| 4 | The window property gives the object; the class is the window's | yes: the VCL's property names FleetOpsHook's module and the game thread (the one the UI's `DllMain` runs on); the object's class lies in FleetOpsHook's image and its name is the window class |
| 5 | A posted press acts with no hover | yes, on five buttons of three forms (the main menu's Options and Instant Action, Options' Graphics and Back, Graphics' Back); the real mouse needs a hover first on Roots' shell, a posted press does not |
| 6 | A printed form has its picture | yes: 80 to 96% of the pixels drawn with the region set (the buttons in it not checked by eye) |
| 7 | Who gets the real pointer | the device window, enabled under these forms, in client coordinates equal to the back buffer's; the UI's own poll reads it right and its widgets hover |
| 8 | Where the cursor comes from | a Win32 cursor Roots set before the form; once the form is out from under the pointer nothing sets one (the device window eats `WM_SETCURSOR`), so the pump sets the arrow (decision 171). The bench's shots do not show the pointer |
| 9 | A combo changed through its window fires the form's handler | yes: the device combo's change handler ran (it refilled the resolution list) on a `CB_SETCURSEL` and the parent's `CBN_SELCHANGE` |

**Go.** Every question passes, the first through its fallback; decision 140 gives way to the
work list below, with the forms off by default on Roots (decision 149).

### Steps 1 to 5 and 7: what the bench showed (2026-11-06)

Built on branch `fo-6-optionC` (the work list above has each step's status; decisions
170–184). On Roots 5.0Z11 at 1280x960, Vulkan with validation, `Menus.Forms=1`:
`fo-ui-forms-main` (our main menu, Options from it and back, Instant Action into Roots' own
setup dialog), `fo-ui-forms-options` (Esc in a skirmish: our Options over the mission's last
frame, End Mission, our Yes/No over it, No, Return to Game), `fo-ui-forms-graphics` (the
generic Graphics screen: dropdowns of ours with the entries read through the combos'
windows, Roots' check boxes and sliders as crops of its picture, Previous Menu),
`fo-ui-forms-manualip` (Multiplayer, Manual IP over it, an address typed into Roots' own
field through ours, Cancel), `fo-ui-forms-standdown` (the hook forced down: Roots' own main
menu matches the `fo-ui-standdown` baseline) and `fo-ui-forms-probe` pass; no `PANIC`, no
validation message. Stock Armada II at 1280x720 (`--install .`): `ui-menus-dialogs`,
`-multiplayer`, `-options`, `-setup`, `-slider-max` and `-widgets` pass, `-scene` asks for
review; `-locked` and `-scene-abort` fail a judged look of render3d's menu scene (the Borg
cube seen face on), `-refit` an OCR check ("Movies" matched by "Move"), `-single` two
judged looks (an empty save list, the Admiral's Log table) and `-success` a 300 s wait for a
briefing. None of them runs the forms (`Forms=0`: the stock UI.log has only the `forms`
table's summary line) and none was re-run on `fo-b` for comparison; `ui-menus-match` (two
players) was not run.

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
| 68 | **The menus stay Roots'** (`Menus.Enable=0`, `Credits=0`, `Scene=0`), and nothing is patched in `FleetOpsHook.dll`'s own import table | the 1200x900 shell, FleetOpsHook's bitmaps and the nine screens past the host; FleetOpsHook stays a black box (decision 3) | step 7, asked for: **widened by decision 140** (step 7 found the whole shell out of reach) |
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
| 120 | **`hud.panels` is verified on Roots**: `DisplayInterface::RenderAll` is FleetOpsHook's trampoline at its entry and the stock body runs behind it, so the panel switch's two sites (the Render call in `RenderAll`, `RefreshOwner`'s CursorOver call) are stock's in the file and in memory | MemDiff, no changed range touches either; the resource strip is hidden where ours draws and the mouse works | a Roots build that changes either call |
| 121 | **`hud.order_ring` is verified** (`cOverViewImp::mRenderWhere`'s edits are FleetOpsHook's own calls, away from the two DrawQuad calls the hook takes), `hud.footprints` through the callee rule; `HUD.Overview` is on, the drag box stays the engine's | the placement footprint is ours in the bench, the move marker is drawn at the click; no crash, no VUID | a Roots build with another marker |
| 122 | **`panels.presses` is verified** (`Program::DisplayInputProcess`'s entry is a trampoline, the ResetVisibility call the hook takes is stock's): a press of ours is made at the engine's simulate pass | `panels: click idle`, `click view` and `click Continue` took effect on Roots' own buttons | a build that moves the call |
| 123 | **`hud.frame_rate`, `panels.chat` and `panels.trade` are verified** against `Armada_RenderFrame`'s one edit (+0x245) | their call sites are stock's in memory; the chat's entry follows FleetOpsHook's text input | an edit near them |
| 124 | **Where Roots' string table gives no word, the UI uses its own**: the pause box ("Game paused", "Continue") when `GUI_POP_PAUSE_*` come back empty, and a resource's name from the content set when its tooltip's first line is not a word | the first bench showed a blank box and `?` for two resources; the tooltips are string-table tokens | Roots' table carrying the keys |
| 125 | **Roots' defaults** (`ui/defaults/fo-roots.ini`): `HUD.Enable=1` with `Resources`, `Buttons`, `Pause`, `Chat`, `Selection`, `FrameRate` and `Overview` on; the info panel, bar, minimap, messages, objectives, replay, overlay text, fade, team texts, tooltips, comm and the rest off | each is on only with a passing Roots scenario (`fo-ui-<panel>`); the others have no trigger in a skirmish or are decisions 65 and 66 | a scenario for each of the rest |
| 126 | **The UI plays no button sound on Roots**: a profile value `panels_sound_ours` (1 on stock, 0 on Roots) gates `panels.sound`, and the HUD no longer requires it | every audio call site is FleetOpsHook's FMOD (`operator new` and the constructor re-pointed at each site), its object size unknown; a stock-size object under its vtable could corrupt the heap, so it was not played | the size and constructor measured, or the Roots developer's word |
| 127 | **The selection overlay is ours on Roots**, replacing FleetOpsHook's `SelectionDisplay` callbacks while it is on (its group colours and tooltips are not drawn) | a passing scenario with Roots' unit tooltip, bar and info strip unchanged; the group colours were not seen in a skirmish | a Roots game showing a loss, then `HUD.Selection=0` |
| 128 | **The bar of ours on Roots is a skin** (`HUD.Bar=1`): the stock buttons drawn in the theme over their own rectangles (the palette's origin plus each local rectangle), hover and pressed read from the buttons' own bytes (`+0x38`, `+0x30`), the stock bar left to simulate, own the pointer, show its tooltip and press; the panel switch skips `RenderAll`'s drawing of the palette only (`set_bar_skin`: `RefreshOwner`'s mouse call still reaches it) | decision 94; the ring and the tooltip are the engine's and stay | a Roots build that draws the ring from the palette's `Render` |
| 129 | **Roots' default stays `HUD.Bar=0`** (`ui/defaults/fo-roots.ini`, `BarLabels=0`, QOL's `BarKeys` off, decision 103); the player turns the bar on in the Refit screen to judge the look. The skin takes no room in the bottom row (`bar::shows` is false), draws nothing while the palette or the HUD is hidden, and is not drawn without the UI's device (the stock bar draws then) | the look is the Roots developer's and the user's call (decision 60); a skin cannot lose the gameplay feedback, but the user asked to judge it first | the user's sign-off, then the default line goes |
| 130 | **Decision 94's open check is closed**: the Tractor Beam ring and tooltip show with the skin drawn; the build list and its back button work through the stock bar's own click | `fo-ui-bar` and the interactive run above | Roots changing its ring condition |
| 131 | **The skin reads icons from the player's install at run time and copies nothing**: the sprite database's art by name, the UI's own flat icon where it has one, a weapon's name without its `npc_`/`all_` prefix where neither exists; no Roots icon is in the repository or converted | decision 60 and hard rule 8 | a Roots weapon with no sprite of its own and a wanted look |
| 180 | **`fo-ui-overview`'s failures were staging, not a bug of ours; the height posts are no longer judged.** (1) The box `300,560 to 520,745` took the second construction ship (at about 515,680) only now and then; with both selected the bar's fifth button is not the build button, so the click at 600,788 did nothing and the pointer rested on the starbase's tooltip (step 49). The box is now `375,560 to 465,650`, the first Mediterranean class ship alone, and the bar buttons are hovered before they are clicked, as `fo-ui-bar` does. (2) Posts: `DrawHeightPost` is the pointed unit's, a placement's and the editor's, never a selected ship's, and on Roots the pointed unit's is FleetOpsHook's own edit of `mNormalModeRender` and the move point's its `mRenderWhere`; ships and the ground lie in one plane, so a post would be a point. The runs log no `leftovers: height post` on Roots, hovering the starbase included. The judged step asks for the ring and the dotted move line instead; the footprint is judged as hatched red with amber tiles (the brackets are for a clear place, and the judge was unsure what the starbase's amber tiles were). The placement wait is a plain `Expect` after two seconds: the wait on a log line only matches lines written after the last action | the three passes `20261106-191527`, `-191741`, `-192002`; failing before: `-182038`, `-183823`, `-184615` | a Roots build whose ships hover above the plane |
| 140 | **The front end stays Roots' on Roots**: `[Menus] Enable`, `Credits`, `Scene` and `CampaignLocks` stay 0 in `ui/defaults/fo-roots.ini`; no dialog host, mirror or spec on Roots, no host design space from the profile's shell size, and nothing of `FleetOpsHook.dll` hooked, in user32 or in its memory. Decision 68 widened from nine screens to the whole shell | step 7's survey: the main menu, Options (the in-mission menu too) and six more screens are FleetOpsHook's Delphi forms and never run the stock dialog; nine more run behind its own import and dialog procedure; only Sound Settings passes the host | the Roots developer offering a way for the shell's screens to run as the executable's dialogs (decision 1), or a Roots update that does it |
| 141 | **Option C is taken** (the user's go, 2026-11-06, "please go ahead"): our menus over FleetOpsHook's forms, reading its objects (the user's allowance of 2026-11-06), read-only, on the game thread, only while a form is the modal one; nothing of FleetOpsHook's memory is written and none of its code patched. Decision 140 holds for what the forms part does not cover (the nine dialogs until phase two, and every screen while `Forms=0`) | "Step 7, option C" above; reading nothing was a default of model (c), not a rule; the probe passed ("Step 0") | |
| 142 | **A form is found by its window** (a thread CBT hook; the Delphi class name as window class, owned by the device window), its object through the VCL's own window property, checked by its class pointer lying in FleetOpsHook's image and its class name; no global of FleetOpsHook's | the forms are created and freed per screen and no global holds them | a FleetOpsHook that subclasses its forms or renames the property |
| 143 | **Controls by name and class** through the class's published field table, rectangles and visibility through its property information, from the live objects; captions from the control; nothing from the form resources | sizes come from bitmaps, Options lays out in code, captions are set at run time | none expected |
| 144 | **The frame pump is a thread timer of the UI's**, dispatched by FleetOpsHook's own modal loop; the stock host's `frame()` draws and presents on the engine's shell device; the design space is the profile's shell size (1200x900) | nothing renders while a form is modal, and a timer needs no write into FleetOpsHook's application object | probe questions 2 or 3 failing |
| 145 | **A press is posted** (left down and up at the centre of the live rectangle, to the form's window); windowed controls through their windows as the stock host does; no handler or `Click` called directly; no setting written by the UI | the press runs through Roots' own button code and loop, with Roots' binding of the moment; a direct call would nest a modal form on our stack | probe question 5 failing |
| 146 | **Roots' drawing is hidden by an empty window region**; input is polled in the pump and keys come from a thread message hook; fallbacks a nearly transparent layered window, then an empty paint off screen; never a patch of FleetOpsHook's code | one call clips paint, the buttons' direct repaints and hit-testing | probe question 1 |
| 147 | **A tool reads FleetOpsHook and its shipped map once per new build** (`tools/engine/`): checks the map against the DLL, checks the few private VCL offsets against the accessors the map names, lists the forms and the controls each spec needs, and prints the identity lines to add; the map is not read at run time and nothing is generated into the game | a new family member is a re-run and the scenarios, not hand work; the map describes the unpatched build, so it is trusted for names and addresses, not bytes | a FleetOpsHook shipped without its map |
| 148 | **The forms are keyed to FleetOpsHook alone**: a list of measured identities (Roots 4.0.93, Roots 5.0Z11, Tactical Maneuvers 5.2.3); anything else refused and logged once; what reads `ArmadaL.exe` stays keyed to its own build | the three share every address, class layout and form but one screen's positions; the exe differs per release | a measured release outside the family |
| 149 | **The forms ship off on Roots**: a new `[Menus] Forms` key, 0 in `ui/UI.ini` and in `ui/defaults/fo-roots.ini`, a Refit row only once step 5 passes; stock ignores it (no build there has forms) | decision 60: ours replaces Roots' look only when the player turns it on | the user or the Roots developer |
| 150 | **The nine dialogs behind FleetOpsHook's import are phase two**, by late attach: found as created, left in FleetOpsHook's placement and procedure, named through the record FleetOpsHook keeps on the window, read with the stock host's model | the eight forms are the larger and simpler half; the nine need the stock procedure's identity | the eight seen in game |
| 151 | **Generic first** (mirror, then widgets over a form's controls), **specs after**, one per screen at 1200x900 | as on stock; specs need the live bindings seen | |
| 152 | **Out of scope in this plan**: the map editor's forms, the Internet button's Tunngle and GameSpy menus, hidden debug controls, and Map Select's grid as widgets (mirrored only) | Roots-only tools, dead services, and a grid whose rows are a third-party control's private data | the user asking |
| 153 | **The resources' names come from Roots' own table at run time, the set's only a fallback**: the long tooltip's first line when the short one is a glyph; the `fleetops` set's `Names` are Roots' defaults (Tritanium, Supply, Connection Nodes). Supersedes the stock words of decision 124 | Roots renames them, per race (the Species 8472 read Biomatter); reading its table follows any race or build | a Roots table with no long tooltip (the set answers) |
| 154 | **The resources strip sits over the game's own `ResourcePanel` rectangle on Roots** (`[Resources] Placement = component`, new content-set key; stock stays top centre), top left corner, natural size | Roots draws its command, message and build areas at the top centre; its own resources were top left | an aspect where Roots moves the panel (the rectangle is read each frame) |
| 155 | **`Resources` stays on in Roots' defaults**: no clean reason to give the panel back | the names and the place are fixed and every race measured | the user seeing the overlap again |
| 160 | **The game's text is read as Windows-1252, and a game's own font codes are the content set's `[Glyphs]`**: Roots' red figures as digits, its resource icons as the resources' names in its own table (Dilithium, Tritanium, Supply, Crew, Biomatter, Connection Nodes; 0x80, a clock after a cost's crew, as "s"), its unit classes drawn as pictures of words as those words (Construction Ship, Mining Ship and the rest, as the larger fonts name them), `$` as a space; applied where the engine's text enters for display (`text::game`: the string streams of every tooltip and `buttons::clean`) | the user's report (2026-11-06): with `HUD.Tooltips=1` every Roots tooltip showed `?` and stray letters; the UI read Roots' codes as Latin-1. A/B: `fo-ui-tooltips-off` (Roots' own drawing, right) against `fo-ui-tooltips` (ours, now plain text); Display8, render3d and QOL are not in the path (with the row on, the stock `Render` draws no tooltip) | a Roots build whose fonts move a code (the set's table is one edit) |
| 161 | **Words, never Roots' pictures**: the icons and the class plates are written as words in the UI's font, not drawn from Roots' font atlas | decision 60 and hard rule 8; a word reads at any size, the plates are a 13 px atlas cell | the user wanting the icons inline (the atlas read at run time, as decision 131 allows) |
| 162 | **The meaning of a code is the one Roots' larger fonts give it** (`FontFinal4_16`, `FontMedium`, `FontLarge`: `$` a 2-3 px spacer, 0x80 a clock, 0xaa Construction Ship, 0xae Mining Ship), not the small tiers' (`FontFinal4_10` and `_13` draw `$` and `€`, and label 0xaa "Civil Vessel"); a player's wide screen uses the larger | Roots' own tooltip at 1280x960 shows `350$◆$/$175$...$60$€` (bench, interactive); the user's 3440x1440 uses a larger tier; the text is the same at every tier, so one meaning is right | the Roots developer |
| 163 | **The resource strip keeps decision 153's names**: a short tooltip that is a glyph alone (`Content::is_glyph`) still sends the strip to the long tooltip, so a race's own word (the Species 8472's Biomatter) wins over the glyph's word | the two fixes met on `fo-b` (#93): the glyph's word would otherwise read as a name | |
| 170 | **A form of ours is hidden by moving it off screen** (`SetWindowPos` far above and left of every output, its size kept), its region emptied as well; checked every tick and moved again if anything brings it back. Decision 146's order is reversed: the region alone and a layered window at alpha 1 leave a black rectangle on Wine under XWayland | step 0, question 1: the region clipped Roots' paint but not the pop-up's window; off screen showed our frame whole with the form's focus and loop intact | a Wine or compositor where the region works (then nothing need move), or FleetOpsHook moving its forms back |
| 171 | **While a form is ours the pump sets the arrow** each tick (`SetCursor`), since nothing else does once the form is out from under the pointer | step 0, question 8 | Roots' race cursors wanted in our menus |
| 172 | **The forms' identity is a list in the UI** (`FAMILY` in `ui/src/engine/forms.rs`: the CRC-32 of FleetOpsHook's file `.text` for Roots 4.0.93, Roots 5.0Z11 and Tactical Maneuvers 5.2.3), read through `a2engine::companion_crc` (new: the `[[module]]` rule as a function). The hook-level "requires the companion" flag in `a2engine` and its C twin is **not built**: the forms are the one part keyed to the companion alone, and no C plugin reads them | the list is about FleetOpsHook, not the executable, so it does not belong to an exe build's profile; a flag in the generator would carry one user | a second part keyed to FleetOpsHook alone, or a C plugin that needs it |
| 173 | **The VCL's fields are a UI part of their own**, `ui.forms` (`profiles/gog-1.1/ui.forms.toml`, hook `ui.forms.vcl`): the eleven field offsets and three values (`forms_present` 0, the design size 0x0) in the base profile, which every build inherits; Roots 5.0Z11 sets `forms_present` 1 and the design 1200x900. A control is found by the **form's owned-component list**, not the class's published field table: one walk gives every control with its name; the published field table is read only by the tool (`fo-forms.py`). **Host tests on a recorded object image are not built**: the bench scenarios carry the reader | the profile's field mechanism already checks a literal offset out of the sources (lint rule 1); the component list is the VCL's own owner list and needs no class table | a form whose controls are owned by another component |
| 174 | **Roots' defaults take `[Menus] Generic=widgets`**: a form without a spec is drawn as our widgets (buttons, labels, text fields and combo boxes), and the controls the UI has no widget for (check boxes, sliders, grids) as crops of the form's printed picture with the mouse forwarded to the window under the point; `mirror` draws the whole picture | ours replaces Roots' look where the UI knows the control; the picture keeps the rest usable without reading a third-party control's private state | widgets for the check boxes and sliders (their state read from the VCL's published properties) |
| 175 | **No keyboard hook**: keys go to Roots' form and its focused control as they would (Esc, Enter and Alt+F4 are Roots' handlers); a click on a text field of ours gives Roots' real field the keyboard with its text selected, the typing goes there, and ours shows its text each tick | the form keeps the activation and the focus off screen (step 0); a `WH_GETMESSAGE` hook is needed only for a field of ours, which no screen has | a screen with a field of ours |
| 176 | **Specs: the main menu and Options as a column** (the form's visible buttons in its order, Back at the window's foot, the version line bottom right) **and Yes/No** (the form's own picture, where Roots draws the question, under our two buttons); **every other form is generic**: Manual IP, Multiplayer Connection, Graphics, Advanced Options, Map Select, Sound, Mods and Advanced Settings. The plan's step 5 named specs for seven screens | the generic widgets lay a form out at its own rectangles, which for Roots' remaining screens is already a sensible layout; a spec for them would only move the same controls | the user's look at the generic screens |
| 177 | **The forms the UI takes**: the main menu, Options, Yes/No, Manual IP, Multiplayer Connection, Map Select, Graphics, Advanced Options, Sound, Mods and Advanced Settings; the map editor's forms and Tunngle's wait stay Roots' (decision 152). While one of the nine dialogs FleetOpsHook opens through its import is up over a form of ours, no frame of ours is presented: Roots' dialog stands over our last frame | the eight of the plan and the three settings screens Options opens, which have the same buttons-and-controls shape | |
| 178 | **Step 6 (the nine dialogs by late attach, phase two) is not done** | optional in the plan and in the user's go; the eight forms first, seen in game | the user, after the forms are seen in game |
| 179 | **The bench clicks Roots' forms of ours by their captions** (OCR on our buttons, which carry Roots' own captions); the design-to-screen line is logged (`menus: forms 1200x900 @0,0 -> WxH @X,Y`) but the scenarios' `at forms X,Y` space is not added | captions are what a player reads and survive a layout change | a step that must click a control without a caption |
| 180 | **`fo-ui-forms-standdown` forces the forms' hook down** (`A2EngineTest.ini`, `ui.forms.vcl`) rather than loading a FleetOpsHook outside the family: Roots' own main menu, matched against the `fo-ui-standdown` baseline. The family check is the same stand-down, logged (`forms: FleetOpsHook.dll .text crc ... is not a measured build`) | the bench cannot load another FleetOpsHook without changing Roots itself | |
| 181 | **Manual IP is checked up to its form, not through Join**: the bench opens it from Multiplayer and reads its controls; Join writes the address into Roots' settings file through Roots' own handler and then starts a connection that Wine's DirectPlay cannot make, so it is left to the user's check in game | Online's notes: DirectPlay does not work under Proton | a bench with Microsoft's DirectPlay in the clone |
| 182 | **`[Menus] Forms` stays 0 in Roots' defaults** although the main menu, Options and Yes/No pass on the bench: decision 149 (ours replaces Roots' look only when the player turns it on) and the user's in-game check come first; a Refit row is not added yet | decisions 60 and 149 | the user's look in game |
| 183 | **Under a form that leaves the device window disabled** (Yes/No, Manual IP: Roots re-enables it only for its full-screen screens) **the pump reads the left button itself** (`GetAsyncKeyState`) and records its changes for the UI; under the others the device window's own procedure hands the clicks to the engine's message functions, where the UI records them as in a mission | the first bench of Yes/No: the pointer was read, the click never arrived; the plan's mechanism 6 named the polling | a form Roots leaves enabled whose clicks still do not arrive |
| 184 | **A small generic form** (one not the design space's size: Manual IP, Map Select, Advanced Options) **stands on its own printed picture** under our widgets, as Yes/No does: Roots paints those forms' titles and field labels into their backgrounds, not as controls. A full-screen form draws no picture behind our widgets, so its painted labels (Multiplayer Connection's "enter your name") are not shown; a full-screen form hides the forms under it | the first bench of Manual IP: an address field with no word saying what it is | labels of ours per screen (a spec) |
| 190 | **`fo-ui-forms-options`'s failures were staging, and Yes/No is drawn as designed.** (1) The Yes/No on screen is ours: our panel frames the form's printed picture (the form is moved off screen, nothing of Roots' own window shows) under our two buttons. The black ground, the "Warning" title, the icon and the question are Roots' own paint: `TYesNoForm.FormCreate` writes them into its background (title and message text from the executable's `yesNo*Text` strings) before our print. (2) The question is cut at "end the miss" at the form's right edge **in Roots' bitmap**: the text starts about 157 of the form's 657 px in and runs past its edge at this font, so the print holds it whole and the crop is not ours. We do not redraw the text (that would read the executable's string for a cosmetic); the judged step now describes the picture and allows the cut. (3) The earlier step 26-37 failures were `_fo-sides`: its OCR reads of Federation/Klingon in Roots' own setup dialog, whose clicks Roots takes only after a hover. The scenario no longer picks sides (it waits on render3d's log, not the race) and hovers LAUNCH in two moves with a pause before the click | `20261106-214410` and the three in a row `-215337`, `-215520`, `-215659`; `fo-ui-forms-main` `-215851`; failing: `-213007` (the judge), `-214744` (a lost LAUNCH click with one hover) | a Roots whose Yes/No lays its question out to fit |

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

### FO-B: the formation move's path

Branch `fo-formation-path`, 2026-11-06, designed from the files only (no build, no bench
run yet). The gap: on Roots a move of several ships draws no path; a step-7 bench run
logged the first selected ship of such a move with the command 11 and a path of two points
whose last lay near the map's origin.

**What it is.** Command 11 is not Roots': it is stock's FOLLOW (the engine's own table of
command names says so, the same in both files), and stock uses it for every move of
several ships. A move sent to more than one ship goes out as one group order and reaches
`FormationManager::Send_Ships_In_Formation`, which gives the move to the group's leader
and FOLLOW to every other ship, with the leader and the ship's place in the formation
relative to it. With `cfgDEFAULT_TO_FORMATION_MOVE = 1` (stock's and Roots' `RTS_CFG.h`)
and no Ctrl, the followers keep that place for the whole move; with Ctrl each one soon
turns its FOLLOW into a move of its own along a copy of the leader's path, shifted by its
place. A following ship's process keeps the leader's handle and the place
(`+0x9c`, `+0xf0`); its own path field holds the order's start and the place, not a
route, which is the two-point "path" the bench saw. A path sketched with the waypoint key
never takes this road: it is sent as a copy of the path for each ship.

**Why nothing is drawn on Roots.** Two filters in the overlays module's engine fallback
(`paths.rs`), both stock's by design, not anything of Roots': (1) it reads a path only
from a ship whose command is GO or PATROL, so a follower is skipped; (2) without QOL's
queue it draws only paths that came from a sketch, and an ordinary move, of one ship or
of a group, is not one. On stock the player has QOL, whose queue draws every move, groups
included (their own formation spread); on Roots QOL is not installed, so (2) alone hides
every unsketched move, the one-ship move too (as the FO-B note above found on the bench),
and (1) would hide the followers even with (2) lifted. FleetOpsHook draws no path of its
own (it references neither `mDrawPath` nor the engine's line draw) and leaves the order's
road alone: the FOLLOW case of `CraftProcess::Handle_New_Command`, `Send_Ships_In_Formation`,
`Do_Group_Move`, `Copy_Offset_Path`, the group order and the sketched order are stock's in
Roots' file and in memory. It changes `CraftProcess::Follow` only in a computer player's
cloaking, in how the leader is looked up and for a ship that would follow itself, and runs the user formation's fleet through
`AIFleet`'s plain `ProcessFleetAI` and `ReplaceLeader`, none of which the paths read.
Confidence: high for the mechanism (read in both files and matched to the bench's log
line); the drawing itself is unseen.

**The fix, under model (c)** (code written, unbuilt): the engine's own order data only,
nothing of FleetOpsHook's memory, no new hook.

- `hooks::ship_follow` (render3d overlays): a selected ship of the player's team whose
  command is FOLLOW is drawn by its leader, found through `Entity::Get` of the handle; the
  leader must itself be on a path (GO or PATROL, read by the existing `ship_path`). The
  group's path is the leader's, drawn once (the module's chain already draws a shared
  segment or point once a frame, and a new list of legs does the same for the leader's
  leg); each follower adds a leg from itself to its place at the point the leader flies to,
  and a small dot at its place at each point after, the drawing QOL's groups already have
  on stock.
- `EngineMoves` in the content set's `overlays.ini`: `sketched` in the stock set (stock
  without QOL unchanged), `all` in the fleetops set, so every move of a selected ship
  draws on Roots, ordinary and formation moves alike. QOL's queue still wins when a plugin
  exports one.
- Profile (`profiles/gog-1.1/render3d.overlays.toml`, inherited by Roots):
  `cmd_follow` (value 11), `proc_follow_target` (`CraftProcess+0x9c`, i32),
  `proc_follow_offset` (`CraftProcess+0xf0`, v3), all `used_by = ["overlays.paths"]`; no
  Roots entry (the values are stock's there, and no `[[changed]]` function holds them);
  the overlays' symbol count goes from 107 to 110 (`engine-syms`).
- What stock keeps doing: with the stock set nothing new is drawn (a follower's leader
  is on an unsketched path, which `sketched` still drops), QOL's queue is untouched, and
  `mDrawPath`'s re-pointing is unchanged.

**To confirm** (needs a build and a bench run; neither was done): `cargo` builds of
render3d and its `hosttest` (`overlays_fleetops_draws_every_engine_move`,
`overlays_stock_is_the_old_table`); on Roots (Vulkan, 1280x960), a one-ship move and a
move of three ships, each followed by `overlays: move paths: N selected ship(s) on a path,
M point(s) ahead (engine)` with N the ships selected, and shots of the leader's path with a
leg and a dot per follower; the same with Ctrl (the followers then fly copies, drawn as
moves of their own); on stock with QOL, `render3d-move-paths` and `engine-syms`
(overlays 110/110) unchanged; on stock without QOL, a group move still draws nothing.

| # | Default | Why | What would change it |
|---|---|---|---|
| 53 | A **follower draws its leader's path** (once for the group), a leg to its own place at the leader's next point and a small dot at its place at each point after | the leader's path is the route the group flies, and the place is where the follower will be; it is the drawing QOL's groups have on stock, so the two look alike | the user's verdict in game; a follower seen away from its dots |
| 54 | A follower's place is **turned by the heading of each leg of the leader's path** (ahead along the leg, right level with the map), as the engine turns it when it copies a path, not by the leader's live heading | it is fixed on the map, so the dots stay put as the ships fly (the module's rule for every dot), and it is where the engine itself would put the ship on a copied path | dots seen to swing off the ships' tracks on long turns |
| 55 | On Roots **every move draws** (`EngineMoves = all` in the fleetops set); stock's set stays `sketched` | Roots has no QOL, so the plain and the formation move would draw nothing; stock without QOL keeps its look | QOL installed on Roots (its queue then draws, as on stock); the user preferring only sketched paths on Roots |
| 56 | The switch is a **content-set key**, not a profile value or a player's `[Overlays]` key | it is per build, as `ProfileBy` (decision 6) and the rest of the set (decision 7b); a mod with QOL of its own sets `sketched` | the user wanting to choose it in the Refit screen (then an `[Overlays]` key with the set's value as default) |
| 57 | The leader is looked up with **`Entity::Get`** of its handle, not the engine's craft-unless-destroyed lookup FleetOpsHook wraps in `Follow` | it is the module's one lookup and reads nothing of FleetOpsHook's; a leader that is gone draws nothing, as the engine then stops the follower | a follower seen without a path while its leader still flies (a leader FleetOpsHook's lookup resolves another way) |

**Bench result (2026-11-06, Roots, Vulkan with validation, 1280x960).** A one-ship move and a
two-ship group move each logged `overlays: move paths: N selected ship(s) on a path, ... (engine)`
(N = 1, then 2) and drew a fine dotted line from the leader to the point and one from the follower to
its place beside it; N went to 0 on arrival with no line left, no stray line to the origin, no `VUID`.
Not staged: three ships (a drag selects only one class) and Ctrl (the bench cannot hold a key), so a
follower's real end position against its dot is unchecked. Stock: `render3d-move-paths` passes and the
overlays resolve 110/110. `fo-render3d-paths` is the scenario.

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

## FO-7: QOL on Roots

Round one (branch `fo-7`, 2026-11-06): the right-drag pan speed runs on Roots, every other
QOL feature is off by default. QOL.asi is installed by `./install --game fo-roots` into
`Data/` with Roots' own defaults (`qol/defaults/fo-roots.ini`, applied over `qol/QOL.ini` by
`qol/install.sh` with `ui/apply-defaults.py`, the mechanism FO-6 uses for `UI.ini`). The
scenario is `testbench/scenarios/fo-qol-camera.md`. Which hook is ready: `./a2test syms
--game fo-roots` (QOL): `qol.pan_speed` ready and `qol.rig_self` ready (off by decision 102);
`shift_group`, `selection`, `special_select`, `stations`, `warp`, `move_paths` and `bar_keys`
stand down (their functions are changed by Roots and not yet verified), `qolrules.*` are
disabled by the profile.

| # | Default | Why | What would change it |
|---|---|---|---|
| 100 | **QOL on Roots is the `qol` layer** of `games/fo-roots.toml` and `layers.toml` (`games = ['armada2', 'fo-roots']`), installed into the executable's folder (`dirname $A2_EXE`, `Data/`), with a defaults overlay `qol/defaults/fo-roots.ini` that leaves `PanSpeed=2.5` and sets every other key off; the overlay applies to a new file and to keys an existing `QOL.ini` lacks (the player's values are kept) | one mechanism per game for a plugin's shipped defaults, the same as FO-6's; a feature is switched on in the overlay once its row here says it was measured beside Roots' behaviour | a feature verified: its overlay line is deleted |
| 101 | **`qol.pan_speed` is verified on Roots** (`RTS_Configure` is marked `verified = ["render3d.view.distance", "qol.pan_speed"]`): the whole function is byte-identical to stock's in `ArmadaL.exe`, the 25 bytes at the site (+0xa36, the store over the parsed `FASTSCROLL_COEFFICIENT`) and the global's initial value are stock's, and in memory after FleetOpsHook has patched (MemDiff log, FO-2) its one write inside the function is a call at +0x1d9e, 0x1367 bytes past the site; Roots' `RTS_CFG.h` carries `FASTSCROLL_COEFFICIENT = 0.005`, stock's value, on its own line (`qol/rts-cfg-check.py` treats it as stock's) | the same method as FO-5 and FO-6: bytes in the file, bytes in memory, then the bench's log line (`RTS_CFG.h parsed: right-drag FASTSCROLL_COEFFICIENT now 0.0125`) | a Roots build whose parser moves the site or whose `RTS_CFG.h` value is not 0.005 (the file check and the profile identity both fail first) |
| 102 | **Every other QOL key is 0 on Roots, including those whose hook already stands down, and `RigIgnoresSelf`, whose hook is ready**: `ShiftAddsToGroup`, `StationGroups`, `WarpDistance`, `PathStartsAtOnce`, `RigIgnoresSelf`, `MaxSelection=16`, `BarKeys` | a stood-down hook is the profile's judgement today; the key is the user's: it keeps a feature off after its hook is verified and before it is measured beside Roots' own rules (FleetOpsHook edits the construction ship's render and the bar; the placement check of the ghost is unmeasured) | each feature's measurement round |
| 103 | **`BarKeys` stays off, and is not a candidate until FO-6 has measured the bar**: the bar is FleetOpsHook's (it adds buttons and edits the hotkey table), and TamsynnImogen's note (2026-11-06) is that Roots' command area can reach 64 buttons with several per weapon classlabel; QOL-1's position keys assume stock's slot plan (`row * width + col`, `qol/README.md`) | a key that presses the wrong button is worse than no key | FO-6's measurement of the bar's slot plan on Roots, then a `BarKeys` row of its own |
| 104 | **QOL reads the overview through `g_pOverView`** (`overview_ptr`, 0x768e3c, in the profile): the selection array patch and the move queue's minimap entry go through it, not through the static object at 0x768e40; before the constructor has run (NULL) the static is used on stock's build only, and any other build gets 0, so the hook stands down | Roots constructs an enlarged overview of its own (FO-5 step 7, decision 42); the 31 selection sites are rewritten to `g_sel` minus the object's address, so a wrong object corrupts the selection. The change is behaviour-preserving on Armada II | the selection round: whether `cOverViewImp`'s fields are at stock's offsets in the enlarged object (the profile's `field` entries) |
| 105 | **`QOLRules.asi` is not installed on Roots** (`qolrules.*` stay disabled by `[[disable]]`); QOL.asi keeps to changes that stay on this player's machine | Roots has its own rules and network code; multiplayer is FO-8 | FO-8 |
| 106 | **The next features are measured in this order, each its own row:** `ShiftAddsToGroup` (key handling only), `MaxSelection` (the overview object, decision 104), `StationGroups`, `WarpDistance`, `RigIgnoresSelf`, `PathStartsAtOnce` last (its orders interact with FleetOpsHook's formation path, "FO-B: the formation move's path") | from least to most coupled to what FleetOpsHook rewrites | the user's priorities |
| 107 | **`ui/apply-defaults.py` is shared** with FO-6 (the same file, `qol/install.sh` calls it from there) | one tool, no second copy to keep in step | a layer-neutral home for it |
| 108 | **`ShiftAddsToGroup` is verified and on on Roots** (`qol.shift_group`; the overlay line is deleted, `QOL.ini`'s 1 stands). `cOverViewImp::mCheckGroupSelect` is marked `verified = ["qol.shift_group"]` and so is `mBindGroup`: in the file Roots changes the first function in one place only, a jump at +0xde (before the loop over the ten group keys) into its own code, which takes the Alt+number case whole and returns, and with Alt not held comes back to the stock instruction; every site the hook uses (the Control test at +0x18a, the centre test, the three continue points, the globals) is on the stock path, with stock's bytes in the file and in memory after FleetOpsHook (MemDiff: its writes in the two functions are the selection offset, 0x3d0, at five sites (two in `mCheckGroupSelect`, three in `mBindGroup`) and the group cap in `mBindGroup`, none at a site of ours); `mBindGroup` is byte for byte stock's in the file, FleetOpsHook rewrites four operands of it, and `Input.map` has stock's group keys. Roots has **no Shift+number behaviour of its own**. Its Alt+number is **not** QOL's "select the ship group and centre": it empties the selection and selects the *station* group of that number (nothing for a ship group), with the double-tap centre, so on Roots Shift's old meaning (select and centre at once) is gone, and the log line and the Refit row, which say "Alt+N selects and centres", do not hold there | the three-way test the user asked for: no Roots Shift+number, hook sites stock, the bench shows the add and Roots' Alt+number unchanged (`fo-qol-groups`) | a Roots build that takes Shift+number itself |
| 109 | **`MaxSelection` stays 16 on Roots, and `qol.selection` and `qol.special_select` are disabled by the profile** (`[[disable]]`, with the reason), not only "not yet verified". FleetOpsHook has already done this work in its own way: it rewrites the selection-array offset to 0x3d0 at 29 sites of `cOverViewImp` (the object it enlarges), replaces `Select` with a function of its own that refuses a 31st object (count at +0xb8, cap 0x1e), lifts `mBindGroup`'s group cap to 30, and replaces the special-weapon button's gathering (`mQueueSpecialWeaponCommand` jump-replaced, both `mSetupSpecialWeapons` calls re-pointed) with functions that keep larger frames of their own. The readers of that array are FleetOpsHook's Delphi code, which QOL cannot re-point; QOL's own array (all 31 `+0xbc` sites, which Roots has already moved) would sit beside it and the two would disagree about what is selected, and a cap above 30 would run past FleetOpsHook's array. Roots' cap is 30, not 40 or 120, and that is Roots' own to change | the static reading (`fleetopshook.md` of the private notes, round two) shows the arrays are Roots' own; a bench of 17 and 30 special-weapon ships in combat would test FleetOpsHook's code, not ours, and Roots has no scene harness to stage it | a Roots build that leaves `+0xbc` alone, or a FleetOpsHook that exposes its cap |
| 110 | **The next features (`StationGroups`, `WarpDistance`, `RigIgnoresSelf`, `PathStartsAtOnce`, `BarKeys`) stay as decisions 102 and 106 left them**: off, in that order; `StationGroups` needs a measurement beside Roots' own station groups, which round two found: the Alt+number recall of the station group (decision 108) and `Select`'s own handling, both FleetOpsHook's | one feature per round, least coupled first | each feature's round |
| 111 | **`StationGroups` is off on Roots and `qol.stations` is disabled by the profile** (`[[disable]]`, with the reason). Two of QOL-6's seven sites sit in code that Roots no longer runs. `cOverViewImp::Select` (0x51f600) is overwritten at its entry with a jump to FleetOpsHook's `Select_Callback`, which calls its own port `Select_Enhancement` (cap 30, array at +0x3d0) and never the original: nothing reads the trampoline's variable, so `stn_sel`, the site that lets a second station of one kind be selected, is in a body that never runs, and the port keeps the stock rule that selecting anything after a lone producer station empties the selection. Both callers of `PopupPaletteImp::mUpdateButtonsAndPosition` (in `Update` and `Show`) are re-pointed at FleetOpsHook's port of it (64 buttons behind a pointer; it does not call the original), so `stn_enable`, the build button's enable site, is dead too. The other five sites (`stn_focus`, `stn_route`, `stn_cancel`, `stn_qsize`, `stn_built`) are stock's bytes in the file and in memory (`CheckCanExecute`, `mFocusCameraOnShipGroup` and `Producer::FinishBuild` after its entry trampoline), and `mBindGroup` is verified (decision 108), but without the multi-station selection and the one build menu there is nothing for them to serve; Roots has station groups of its own (Alt+number recalls the station group, one station selected: decision 108) | the user asked for stock's sites to be verified in the file and in memory, and for the feature to be enabled only with a passing Roots scenario: two sites fail the first, so there is no scenario to write; `fo-qol-rig` expects the stand-down line | a Roots build whose `Select` or bar update runs the original, or a feature of QOL that works through FleetOpsHook's own selection |
| 112 | **`WarpDistance` stays 0 on Roots**; `qol.warp` is verified (`cOverViewImp::mNormalModeInput` is marked `verified = ["qol.warp"]`: the whole function is byte for byte stock's in the file, the block that turns a GO into GO_WARP while Alt is held and its continue point are stock's in memory, and FleetOpsHook's one write in it is the call at +0xda, 0x62 bytes past the block; `GameSetup::isAllowWarpSpeed`, `Transport::getGameSetup` and `g_allowWarp` are stock's), so the stub is installed and runs on every right-click (the bench moves of `fo-qol-move-queue` go through it), but the feature has nothing to do: Roots' `RTS_CFG.h` has `GAME_OPTIONS_WARP = 0`, which is a key the exe reads and stock's file lacks, the Instant Action "Game Settings" dialog (Advanced) has no warp option (a screenshot of it: starting resources, cease fire, AI type, battleship limit, three tick boxes and six sliders), `cfg_MIN_MOVE_WARP_DISTANCE` is 999999 and the dewarp radii are 0. The hook's own test reads `isAllowWarpSpeed` first, so with warp off a far move is never made a GO_WARP whatever `WarpDistance` says. The multiplayer lobby was not looked at | the user's note that warp is off in Roots' `RTS_CFG.h`, checked against the setup screen | a Roots game where `isAllowWarpSpeed` is true (a lobby option, a mission, an `RTS_CFG.h` of a mod): then `WarpDistance=1100` is a key to try |
| 113 | **`RigIgnoresSelf` stays 0 on Roots**; `qol.rig_self` is ready and works, and does nothing there. `ConstructionRig::GetAction` is byte for byte stock's in the file and in memory (the one write of FleetOpsHook near it ends 12 bytes before its entry, in the function before), `CraftClass::CanPlaceHere` is replaced at its entry by FleetOpsHook's port, which passes the object to ignore on to the placement check on both of its paths, and `CraftProcess::Build`'s arrival check still passes the rig (Roots' jump at 0x4359a5 only chooses the call's last argument from a flag of the class). The bench (`fo-qol-rig`, `fo-qol-rig-off`): with the key at 1 QOL.log says `the blueprint's spot is clear (the construction ship ignored)`, the Antares Type Shipyard's ghost over the ship is green and the click builds (the counters 6000 to 5600, a station frame beside the ship); **with the key at 0 the ghost is green too** (`clear`, without the parenthesis), and also over the other Mediterranean class ship; it is red only over the starbase. Roots' placement (`Is_Clear_For_Placement` has Roots' own edit at +0x2d and a FleetOpsHook call at +0x380 where it looks objects up) does not count ships as blockers, so the feature is Roots' own already; the reason was not read further | the user's note that Roots may have its own version of each feature | a Roots build whose ghost goes red over the ship |
| 114 | **`PathStartsAtOnce` (the move queue) is on on Roots; `qol.move_paths` is verified.** Every function the queue reads or hooks for orders is byte for byte stock's in the file, with no write of FleetOpsHook in it (the one in the `NetOrder` range is Pause's audio call): `mNormalModeRightClick`, `mProcessWaypoints`, `mAddPathPoint`, `GameObject::QueueCommand` (all overloads) and `DeQueueCommand`, `PreSimulateAll`, `SetCommand`, the `NetOrder*::Execute` family, `FormationManager::Broadcast_Go_Command` and `FormFormation`, `CaptainsVoice::Say`, `AiCommandToName`; the 82 sites of QOL-6, 8, 10 and 11 have no MemDiff row at them. Roots' edits in the three functions marked `verified = ["qol.move_paths"]` lie elsewhere: `mProcessKeyboardInput` (the key that selected the next ship now calls `mSelectOnscreen`, a jump at +0x3f, and two selection-array offsets), `Simulate` (two flag reads at its end, the camera-pan callback, one offset), neither at the keyboard step's call (0x51ef2e) or the waypoint key's handler. **Two things differ from stock.** `Craft::Say` is replaced whole by FleetOpsHook's port (`Craft_Say_New`), so the call of `CaptainsVoice::Say` that QOL re-points to keep its own sends quiet is in a body that never runs: on Roots the ships speak as they take each leg (the bench has no sound; the quiet-sends line, `acknowledgement left out`, never appears in the move queue's bench log). And a path is advanced earlier: the log's `passed point N` says about 300 from the ship's own target on the Mediterranean class where stock's destroyers say about 150 (the log marks it `short of it`, its `done` line counts it). **The paths drawn.** render3d's overlays take QOL's queue where a ship has one and the engine's path otherwise, but a ship QOL reported 0 points for was skipped, so with QOL on Roots a plain move drew nothing (seen on the bench); with `EngineMoves = all` that ship now falls through to the engine's path (`paths.rs`; stock's `sketched` is unchanged). The bench stages Tab with the middle mouse button (decision 115) | `fo-qol-move-queue` (below) | a Roots build that changes `NetOrder*::Execute` or `QueueCommand`; the voice (decision above) if FleetOpsHook gets a hook to chain |
| 115 | **The bench holds Roots' waypoint key as the middle mouse button.** A held key is let go at once under Xwayland, Roots has no scene harness to hold it, and a held mouse button is not let go: `testbench/bench/roots-waypoint.sh` (a `Setup:` script) binds `enter_waypoint` to the middle button in the clone's `Input.map` and the step `Right-click at X,Y holding the middle mouse button` presses it around one right click. Not two steps (hold, then click): the button's own camera drag took over when it was held for the length of a step, the view turned, and the right clicks went out as plain moves. Never an install's file: the script edits the clone's `Input.map` | the first attempts (a held Tab, a Tab repeated, the button held across steps) and what each did | a key hold that survives Xwayland |

Round three (branch `fo-7-round3`, 2026-11-06): `PathStartsAtOnce` is on (decision 114), `StationGroups`, `WarpDistance` and
`RigIgnoresSelf` stay off (decisions 111 to 113). Bench, `./a2test --game fo-roots run <scenario> --install .` (1280x960,
Vulkan): `fo-qol-move-queue` passes (the log names the build and `PathStartsAtOnce=1 -> move queues ... patched`, no hook
stands down; the two Mediterranean class ships are dragged into a selection, three points are queued with the middle button
held for each right click: a new queue with two ships in formation slots 0 and 1, one run sent per point, `queue 1 done, 2
ship(s)` with each ship 20 and 22 from its own target at the end, the dotted queue drawn `(qol)` mid-flight and nothing left
after; then a second queue and a plain right click with no key: `cleared for 2 ship(s): move` and the engine's path for the
plain move drawn `(engine)`), `fo-qol-rig` passes (decision 113: green ghost with the key at 1, the shipyard paid for and
rising), `fo-qol-rig-off` passes (green ghost with the key at 0, and over the other ship), and `fo-qol-camera` and `fo-qol-groups`
pass with the new defaults. Stock, `--install .`, 1280x720: `qol-move-queue`, `qol-warp` and `qol-rig-ignores-self` pass; `qol-station-groups` fails at its steps 51 to 56 (the queue sizes after five orders with three yards selected) and `render3d-move-paths` at its step 73 (the bar's Stop click, whose position follows the HUD's race, which is drawn at random), **both on an unmodified `origin/fo-b` too** (a `git archive` of it installed with `--install`, the same steps failing, `render3d-move-paths` after one pass on a Borg HUD): not this branch's. `fo-render3d-paths` (Roots, no QOL) has every check passing and ends REVIEW because the game did not leave through its menus. Not benched: a warp on Roots (there is none to
bench), stations (disabled), the queue in a network game (FO-8), and the voice of the ships (no sound on the bench).

Round two (branch `fo-7-round2`, 2026-11-06): `ShiftAddsToGroup` is on (decision 108), the selection
stays Roots' own (decision 109). Bench, `./a2test --game fo-roots run fo-qol-groups --install .`
(1280x960, Vulkan; `fo-qol-groups.md`, new): the log says `ShiftAddsToGroup=1 -> Shift+N adds to group N,
Alt+N selects and centres, patched` and no hook stands down for `qol.shift_group`; Ctrl+1 on one
Mediterranean class ship, Shift+1 on the Mandril miner, a click on the other Mediterranean ship and 1 selects
the first two and not the third (health bars and the group digit, judged); Alt+4 after Ctrl+4 on the starbase
empties the selection and selects the starbase (Roots' own, the same with `ShiftAddsToGroup=0`); Shift+2 on one
ship binds it, 2 recalls it; Ctrl+1 on the ship alone replaces the group; no `patch refused`, no crash. Not
benched: Ctrl+Shift+number (stock's own add), whose three-key chord from the bench leaves Shift logically held in the
game with `ShiftAddsToGroup` on or off (a later plain number then centres), and big selections on Roots (decision 109).
Stock, `--install .`, 1280x720: `qol-big-selection` (20 Galaxy class ships selected, a group bound and recalled,
`MaxSelection=40`) and `qol-settings-live` (Shift+1 and Shift+2 add, live off and on; the stock check of the
Shift-group hook) both pass; `fo-qol-camera` passes with its sixth check changed to the new log line.

Bench, 2026-11-06 (`./a2test run fo-qol-camera --game fo-roots --install .`, 1280x960, Vulkan):
the log names the build and the matched companion, `PanSpeed=2.5 -> x2.50, patched`, `RTS_CFG.h
parsed: ... now 0.0125 (file value x 2.50)` and the seven stood-down or off features; a right-drag
of 400 px moved the map about 260 px (the stock factor would be about 104). The stock plugin is
unchanged: `qol-big-selection` and `qol-move-queue` pass at 1280x720 with `--install .`
(`qol-move-queue` failed once on two waits for the "cleared" lines and passed on the
re-run and on `origin/fo-b`: a timing flake, not the overview change).

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
