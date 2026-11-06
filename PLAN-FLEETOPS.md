# Plan: Fleet Operations: Roots

How the refit comes to Fleet Operations: Roots, the total conversion of Armada II, in
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
| Submods | `Data/Mods/`: "Fleet Ops 4.0" and "STA2 Classic Mod", each with **its own `ArmadaL.exe`**, and a data-only third | — |
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
| platform | the ASI loader as `Data/winmm.dll`; DXVK left as Heroic installs it | — |

## Work packages

Estimates in ROADMAP units: agent hours, bench passes, sign-off round. FO-0 and FO-1
can start now; FO-3 onward waits on phase 8's format and generator (8b).

| WP | What | Depends on | Agent time · bench · round |
|---|---|---|---|
| FO-0 | **Roots as it ships, under Proton**: the user launches it once from Heroic (it creates the prefix and `Settings.xml`), plays the front end and a skirmish. Note anything Roots' launcher or first run asks | nothing | none · none · a user check |
| FO-1 | **A game profile for the tooling**: `a2env` learns a profile (`A2_PROFILE=fo-roots`, or `--game` on each tool) that sets the game directory, the data directory (`Data/`), the executable (`ArmadaL.exe`), the prefix and the Heroic entry. The bench gets a game adapter: launch `ArmadaL.exe` directly in `Data/` (not the launcher), process matching by its name, a `Settings.xml` adapter in place of the `ARMADA.PRF` one, Roots' log files. `a2mod` and `./install` take the profile and offer only the layers above. A stock clone of Roots for `--install`. Scenarios `fo-launch` (to the main menu) and `fo-skirmish` (a map loaded, a tick logged). The profile mechanism is built once for every game: Armada 1 adds its own profile to it (`PLAN-ARMADA1.md`, A1-1). **The mechanism is done** (AR-2 of PLAN-ARCHITECTURE.md): a `games/<id>.toml` descriptor every tool reads ([`games/README.md`](games/README.md)), `A2_PROFILE` and `--game` on `a2env`, `a2mod`, `./install`, `a2test` and `tools/layers.py`, and a stub `games/fo-roots.toml` (not offered) whose paths resolve, with `Data/ArmadaL.exe` as its executable; the resolver names the executable by the matched build or the module. Left for FO-1: the descriptor's facts checked against FO-0's install (the folder name is assumed), the `Settings.xml` adapter, Roots' log files, the layers on offer, the stock clone and the two scenarios | FO-0 | 8–16 h · 5–8 · none |
| FO-2 | **Measure Roots at run time**, in bench clones only. Display8 has no forward mode and no Direct3D 9 any more: it owns the device or bails out to the system's `d3d8.dll`, so Roots is measured twice, as it ships (DxWrapper, then DXVK: the reference) and with Display8 in DxWrapper's place, tracing (`Trace=1`). (1) What `FleetOpsHook.dll` rewrites of `ArmadaL.exe` in memory: `testbench/memdiff/` (`MemDiff.asi`, through the ASI loader as `Data/winmm.dll`, in the clone only) compares the image's sections with the file at its load and at fixed times after, and logs each changed range with the module its jump leads into; the ranges are named by function offline, against the hook set the profiles have today, and the load order of `FleetOpsHook` against the ASI loader is read off the same log. (2) Whether `FleetOpsHook` makes a Direct3D 9 device of its own or hooks the d3d8 device (it imports `d3d9`, `D3DX81ab` and `d3dx9_43`): observed, from the modules loaded, DXVK's own log, and the callers in Display8's inventory. (3) Roots' Direct3D 8 call inventory on Display8 against stock's (`display-inventory`): block-compressed textures, D3DX8's use of the device, the 1200x900 shell mode, any call stock never makes. **Done 2026-11-06**: "FleetOpsHook.dll at run time", above | FO-1; ROADMAP 8a (the identity log) | 6–10 h · 3–5 · none |
| FO-3 | **The Roots profile**: (game, build) in the format, inheritance with overrides, the "changed by Roots" mark and its stand-down rule; the marked list from the static diff and FO-2's run-time diff; `a2test syms --game fo-roots`; the negative test (the Roots profile on stock and the reverse, both standing down cleanly). **Begun 2026-11-06**: `profiles/fo-roots-5.0Z11/` inherits gog-1.1 (timestamp, image size `0x405000`, section fingerprint) and marks the 30 functions of the file diff and 80 more of the run-time diff that our hooks use (49 of 103 hooks stand down, 54 ready; the rules plugin off); five callees FleetOpsHook takes over drop their entry check; `tools/engine/negative.sh` (e) tests it both ways | ROADMAP 8b; FO-2 | 6–12 h · 2 · none |
| FO-4 | **Display8 on Roots**: stub and passthrough on Vulkan with whatever FO-2 found (block-compressed textures sampled natively, the D3DX8 calls, the 1200x900 shell mode, Roots' `FindDisplayMode` change). If `FleetOpsHook` draws through its own Direct3D 9, that is solved here or Display8 is not installed on Roots (Roots keeps DxWrapper). Installed by `./install --game fo-roots` with DxWrapper's `d3d8.dll` backed up | FO-2, ROADMAP 1 signed off | 10–25 h · 10–20 with A/B against Roots as it ships (DxWrapper, then DXVK) · **round FO-A**: Roots looks as Roots on our device |
| FO-5 | **render3d on Roots**: the seam, then each module, verified beside Roots' changes; the Roots content profile; `a2fmt` reads DDS and Roots' FPQ archive, or render3d reads ODFs and textures through the engine's own file system (decided in this package by which is simpler and safer) | FO-3, FO-4; ROADMAP 7 for the per-module hooks | 20–40 h · 10–20 · **round FO-B**: a skirmish with the refit's look |
| FO-6 | **UI on Roots**: the Refit screen and settings over Roots' Options dialog, the loading screen; then the menu host's specs for the 1200x900 shell and HUD panels against Roots' components, each its own row, as wanted. The button bar is not stock's: its command area can be far larger (several weapons of one classlabel, each a button; FO-7), so the UI's bar and its labels (`ui/HUD.md`, "The button bar") are measured on Roots before they are drawn there | FO-3, FO-4; ROADMAP 6 | 10–30 h · 8–15 · with FO-B, then its own round if panels are taken |
| FO-7 | **QOL on Roots**: pan speed and camera first; each of the other features measured beside Roots' own and switched on only where both agree. A Fleet Operations developer noted (2026-11-06) that Roots' command button area can be far larger than stock's (several weapons of the same classlabel each get a button), so `BarKeys` and the slot plan (`qol/README.md`, QOL-1) cannot assume stock's bar | FO-3 | 4–10 h · 3–6 gameplay passes · with FO-B |
| FO-8 | **Multiplayer**: two bench games of Roots with the refit's visual plugins, on Microsoft's DirectPlay in the clones (`online/reference-dplay.sh`), to show the CRC check passes; then whether `Online.asi`'s peer carries a Roots game | FO-4; ROADMAP 9 (the rules plugin as its own layer) | 6–12 h · 4–8 two-game passes · round FO-C if Online is taken |
| FO-9 | **binkproxy** on Roots' intros, and the release package's Roots support (the GUI installer finding Roots, if wanted) | FO-4 | 4–10 h · 2–3 · with the release |

Summed: about 75–165 agent hours, three sign-off rounds of its own (FO-A, FO-B, FO-C).

**Where it sits in ROADMAP**: FO-0 and FO-1 now, beside rounds C and D. FO-2 needs only
8a. FO-3 is phase 8's second profile and the test of its design, so it lands with or just
after 8b–8d. FO-4 needs passthrough signed off (phase 1). FO-5 is best after phase 7, as
8e is. FO-8 after phase 9. The legacy plugins phase 11 removes are never installed on
Roots, so phase 11 changes nothing here.

## Acceptance

| Scenario | Shows |
|---|---|
| `fo-launch`, `fo-skirmish` | the bench runs Roots: front end, a map, a tick, with nothing of ours installed |
| `fo-syms` | "symbols: N/N resolved" with Roots' marked symbols listed as standing down; the negative test both ways |
| `fo-passthrough` | Roots with Display8 alone on Vulkan matches Roots as it ships (DxWrapper, then DXVK) at a paused tick, in the front end and in a mission, within the tolerance stock's passthrough references are judged by |
| `fo-render3d-*` | per module, Roots' scenes with the module on, and with it off the passthrough look |
| `fo-ui-refit` | the Refit screen in Roots' Options, a row toggled and kept after a relaunch |
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
4. **Roots updates.** A new Roots release is a new build with a new profile; an unknown
   build runs only what resolves by scan, as the profile design says. The three submods'
   own `ArmadaL.exe` files are further builds, not in scope until asked.
5. **Fairness and the Fleet Operations team.** A visual plugin in a competitive mod's
   network games is a question of the community's norms as well as of CRCs.

## Decisions taken by default (FO-4)

- `./install --game fo-roots` installs the `display` layer only, into `Data/`; DxWrapper's
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

**Decided by the user, 2026-11-06 (not a default):** Roots support stays in code only: never
in the GUI installer, the release packages and manifest, `PLAYING.md` or the release notes
until the user says otherwise. `tools/layers.py --check` and `publish/package.sh` enforce it
(`games/README.md`, "Not offered, not released").

## Open decisions

Decided by the user, 2026-11-06:

1. **The Fleet Operations team** approved the work; Roots' own developer wants to tinker
   with it once it works.
2. **Scope**: the renderer only on Roots first, Display8 and render3d (FO-2 to FO-5).
   The UI, QOL and Online on Roots come later, if ever (FO-6 to FO-8 wait).
3. **Roots' art** is at most a local experiment, never published: that is the Roots
   creator's call. The texture and movie pipelines are not pointed at it for any release.

Still open:

4. **The release**: ship Roots support in the same package and GUI installer, or as a
   separate download.
5. **The submods**: whether "Fleet Ops 4.0" and the classic mods (their own executables)
   are wanted at all.
