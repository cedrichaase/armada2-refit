# Display8 — the game's `d3d8.dll`, ours

`Armada2.exe` imports `Direct3DCreate8` from `d3d8.dll` and does all its drawing through
the Direct3D 8 device it gets back. Display8 is a `d3d8.dll` of our own, in `no_std`
Rust like render3d and the UI. The aim (the plan in the user's `plans/`) is that the
engine ends up drawing nothing: Display8 gives it a device that accepts everything, and
`Render3D.asi` and `UI.asi` draw the whole frame on Vulkan or Direct3D 12 directly.

    Armada2.exe -> d3d8.dll (Display8, draws nothing) -> Display8's own device: vulkan-1.dll or d3d12.dll
    the bail-out: Armada2.exe -> d3d8.dll (Display8) -> the system's own d3d8.dll (Windows', or Wine's builtin)

Display8 has no Direct3D 9 and passes no call to a chain of Direct3D 8 wrappers (until
display 1.0.0 it could do both). When it cannot draw the game itself it hands the game
the system's own Direct3D 8 and the plugins stand down ("The bail-out").

## Status

**One mode, the stub (since display 1.0.0).** `./install` puts Display8 in
(`display/install`) and `a2mod` switches it as the `display` layer. The engine gets
devices of ours that draw nothing, and render3d and the UI draw the whole frame on a
device of Display8's own: the first that starts of Vulkan and Direct3D 12
(`Backend=auto`, shipped; "Choosing a backend"), or the one `Backend=` names
("Backends"). When neither can start, or with `Backend=none`, the game draws through the
system's Direct3D 8 and the plugins stand down ("The bail-out"). The user confirmed
Vulkan in game on 2026-10-23 and made the stub the default (display 0.8.0). Forward
mode (every call passed to a chain behind Display8), `Mode=` and `Real=`, and the
Direct3D 9 backend (`Backend=d3d9`) were removed in 1.0.0; a player's ini that still
has them is read past and says so in the log ("Choosing a backend"). On the test bench
the tracer is on (`Trace=auto`: on in an `a2test` clone). The release package carries it
too, from 13.35.0 ("Installing", below).

    ./a2test session start --res 1280x720 --install .

## Installing

`display/install` (and `--remove`):

| File | What |
|---|---|
| `d3d8.dll` | Display8 |
| `d3d8.dll.a2display-backup` | the `d3d8.dll` that was there before, if any (made the first time a `d3d8.dll` that is not ours is found): what `a2mod stock` and `--remove` copy back into `d3d8.dll` |
| `Display8.ini` | the settings; a key the player's copy lacks is added (`tools/ini-merge.py`), and a value that still holds an older shipped default is moved to the new one (below) |

Nothing stands behind Display8 any more: the installer moves no chain to `d3d8_real.dll`
(until display 1.0.0 it did, and Display8 loaded it). A `d3d8_real.dll` an older install
left is not read; the retire step of `./install` takes it out, and `--remove` puts it
back as `d3d8.dll` if no backup is there and retire has not run.

**When the shipped defaults change**, an install must not keep the old ones by
accident nor undo a player's choice. `Defaults=` numbers the set a file holds (as
`UI.ini`'s does), and `tools/ini-merge.py` brings a copy with an older number (or none: 1)
up to the new one: each key the newer set changed moves to its new value only where it
still holds the old default. Defaults 2 (display 0.8.0) is `Mode=stub`,
`Backend=auto`, where 1 was `forward` and `d3d9`. Defaults 3 (display 1.0.0) drops
`Mode` and `Real` and the `d3d9` token from `Backend=` (an emptied list becomes the
shipped `auto`), and carries the sampler keys of a `dxvk.conf` that bears
`renderer-config.sh`'s marker into `[Samplers]` where the player set none and the value
differs from the shipped one ("Samplers"); Display8 reads no `dxvk.conf` after that.
The merge prints each key it moves. The code's defaults are the shipped file's
(`Backend=auto`, `Passthrough=1`), so a `Display8.ini` without those keys, or none at
all, runs as shipped.

With no `d3d8.dll` in the game directory (the system's is the one), Display8 goes in
alone; its bail-out loads the system's own (`GetSystemDirectory`: SysWOW64 on 64-bit
Windows, Wine's builtin under Wine). Run again, the installer replaces only `d3d8.dll`.
`d3d8-chain.py` knows Display8 by the panic message every build carries, not by a hash
(ours changes with every build).

Wine loads the game directory's `d3d8.dll` because of the `d3d8=n,b` override
(`platform/README.md`).

### On Fleet Operations: Roots

`./install --game fo-roots` (and `./a2mod --game fo-roots stock|refit|status`) work on
Roots for the bench and the maintainer's own Linux install; the game is `offered = false`
and in code only (`games/README.md`, "Not offered, not released"). The installer works in
the executable's folder, Roots' `Data/` (`exe.data_dir`), not the install folder. The
`d3d8.dll` there is DxWrapper's stub (it loads `dxwrapper.dll`, which turns Direct3D 8
into 9 for Roots' DXVK `d3d9.dll`); it is not ours, so it is copied to
`d3d8.dll.a2display-backup` as in any install, and `--remove` and `a2mod stock` put it
back byte for byte. Nothing else of Roots' chain is touched (`dxwrapper.dll`,
`dxwrapper.ini`, `d3d9.dll`, ReShade, `FleetOpsHook.dll`), nor `Textures/` or
`ODF.fpq`; `retire/` does not run there and no `platform` layer is installed (Roots has no
ASI loader of ours). Display8 is told from DxWrapper's stub by its panic message, as ever.
Heroic's `fleetopsroots50` entry already sets `WINEDLLOVERRIDES=winmm=n,b;d3d8=n,b;d3d9=n,b`,
so Wine loads the game folder's `d3d8.dll`; nothing in Heroic's config is edited.

What happens (FO-A, `fo-display8`, 2026-11-06): Display8 reads the build as
`armada2/fo-roots-5.0Z11` and **draws the game on its own device**, every draw through the
passthrough, since no plugin of ours runs there yet. `display.modes` is verified beside
Roots' changes (below) and ready; `display.native` and `display.drop` stand down, so the
game's mode is the one in Roots' `Settings.xml` and nothing is dropped. On the bench
(1280x960, Vulkan on the GPU) the shell draws in its windowed 1200x900 device, the
skirmish's fullscreen `CreateDevice` on another window is followed ("A second window"),
and the third, back to the shell when the game is left, too. D3DX8 is told DXT1, DXT3 and
DXT5 are available, so Roots' DDS art is kept block-compressed and sampled as BC1 to BC3.
FleetOpsHook's bump-mapped hulls (`SetVertexShader`, on nearly every Roots hull) use the
stock Borg bump shader's token stream, so they draw through the passthrough's port of it
("The engine's one vertex shader", under "Passthrough"); the skirmish looks as Roots does
as it ships on DXVK.
`Display8.log` is beside `ArmadaL.exe` in `Data/`.

**`display.modes` on Roots.** Its six call sites and `FindDisplayMode`'s checked first 13
bytes are stock's in `ArmadaL.exe` and still stock's in memory once `FleetOpsHook.dll` has
patched (MemDiff, FO-2): none of its writes falls on them. What Roots changes beside them:
`FindDisplayMode`'s width and height tests are turned from "equal" into "not smaller"
(`jne` to `jl`), so the lookup answers the first listed mode at least as large, at the
depth asked; the shell's request in `SetActiveDisplay_Internal` is 1200x900 in the file
and FleetOpsHook raises its depth to 32 bits (the byte before the call at `0x62c4e0`);
FleetOpsHook raises the intro's two requests in `Program::PlayIntroMovie` to 1200x900 and
re-points `SetActiveDisplay`'s call of `SetActiveDisplay_Internal` at a wrapper that sets
the windowed flag and calls the stock body; and it re-points the engine constructor's call
in `InitializeGraphicsSystem` (`+0x64`), away from the two mode calls. Display8's lookup
calls Roots' `FindDisplayMode` first and falls back to the one mode only when it answers
nothing, which on an output smaller than 1200x900 is what keeps the shell from the null
read FO-1 saw. `profiles/fo-roots-5.0Z11/display.toml` gives the shell's and the intro's
requests as Roots makes them, so the log names them and `RaiseIntro` knows the intro's.

**The release package** (`publish/README.md`, "Release packages") carries Display8 in
`display/` with the shipped `Display8.ini`, and its installers (`install.sh`,
`install.ps1`) put it in the same way, with one difference: they make no
`d3d8.dll.a2display-backup`, since there is no `a2mod` in a release. They merge
`Display8.ini` without Python, in awk and in PowerShell, by `ini-merge.py`'s rules and
upgrade table. On macOS too `Display8.ini` goes in as shipped, on `Backend=auto`: Vulkan
on MoltenVK under a 32-bit process works there ("Will Vulkan work on a Mac?"), and if it
cannot start the game draws through the system's Direct3D 8 ("The bail-out").
Nothing in Display8 is Wine-specific (`platform()` only names the system in the log), so
on Windows the stub runs on the GPU vendor's Vulkan or Direct3D 12; it has not been seen
there yet. `--uninstall` puts the backed-up `d3d8.dll` back and takes out what
`display/install --remove` takes out (on Windows also `Display8.pipelines` and
`Display8.d3d12pipelines`).

## What the plugins can ask of it

Plain C exports, found with `GetProcAddress(GetModuleHandleA("d3d8.dll"), name)`
(`src/api.rs`); a plugin that does not find them is on another `d3d8.dll` and works
as before:

| Export | What |
|---|---|
| `display8_version()` | the version, a NUL-terminated string |
| `display8_device_epoch()` | moves on at every `CreateDevice`, every `Reset` that succeeds, and when the device goes: whatever a plugin made at another epoch must be made again |
| `display8_device()` | the engine's device now (ours, not referenced), or null |
| `display8_frame()` | frames presented so far |
| `display8_abi(caller_major, caller_minor, caller, &major, &minor)` | the contract's handshake (since 0.12.0): Display8's contract version, the plugin's logged beside it ("Versions of the contract" below) |
| `display8v1_register(&plugin)`, `display8v1_claims(except)` | the claims (since 0.13.0, contract 1.1): a plugin registers at load with a callback saying what it draws now, and asks what the others claim ("Claims" below) |
| `display8v1_backend()` | what the plugins draw on: Vulkan, Direct3D 12 or none (`gpu/src/display.rs`, `BACKEND_*`). None until the device is made, and for good after the bail-out, when the plugins stand down ("The bail-out") |
| `display8v1_vk*`, `display8v1_dx12*`, `display8v1_engine_viewport`, `display8v1_engine_transform` | stub mode on Vulkan and Direct3D 12: the device, the frame, the upload ring, deferred frees, the engine's targets and mirrors, a flush, and the engine's state the plugins read. The contract is `gpu/src/display.rs` (its doc comment is the specification); `src/vk/exports.rs` and `src/dx12/exports.rs` implement it |
| `display8v1_vk_pass(event, &pass)`, `display8v1_dx12_pass(event, &pass)` | a plugin's `Cmd` begins or ends, or ends with its pass published (render3d's scene pass), for the passthrough to draw into; on Direct3D 12 since 0.19.0 (contract 1.4, below) |

The first five (`src/api.rs`) are unversioned: plain scalars, the same in every release.
render3d (`frame::begin`) and the UI (`gfx::attach`) tell a new device or a `Reset` by
`display8_device_epoch` (`gpu::Gpu::device_epoch`, resolved once).

### The pass on Direct3D 12 (contract 1.4)

Since 0.19.0 a plugin publishes its pass on Direct3D 12 as on Vulkan, through
`display8v1_dx12_pass(event, pass)` (`gpu/src/display.rs`, "Pass publishing on
Direct3D 12", is the specification):

- **Events** as `display8v1_vk_pass`'s: `PASS_ENTER` (0) when a plugin's outermost
  `Cmd` begins, `PASS_LEAVE` (1) when it ends, `PASS_PUBLISH` (2) when it ends with
  `pass` its pass now. `PASS_PUBLISH` is a leave with a pass, not a second event;
  `pass` is null for the other two. Only render3d publishes; the UI enters and leaves.
- **`Dx12Pass8`**, read within its `size`: `colour` and `depth` (`Dx12Image8`, a null
  `resource` is none; `format` the view's `DXGI_FORMAT`, `state` as below), `rtv` and
  `dsv` (the `D3D12_CPU_DESCRIPTOR_HANDLE`s a draw sets with `OMSetRenderTargets`, in
  the plugin's heaps that are not shader-visible; `rtv` is the published layer's view),
  `samples`, `width` and `height`.
- **Kept for the frame**: the pass published last, with the frame it came in
  (`src/dx12/exports.rs`, `published()`), and forgotten at the engine's `Present`, when
  the frame's index moves on. Whether a `Cmd` is alive is `cmd_alive()`: Display8
  records nothing into the list then. Each kind of pass (size, samples, depth) is logged once, the first eight:
  `dx12: pass published 1280x720 x4 with depth (frame 412)`.
- **States**: whenever no `Cmd` is alive the published colour is in `RENDER_TARGET` and
  the depth in `DEPTH_WRITE`. Display8 draws into them so and leaves them so; it never
  samples, copies or transitions them.
- **The rebind rule**: Display8 may set anything on the list between `Cmd`s but the
  descriptor heaps (it sets only its own two, at the frame's start and after a flush);
  `gpu` binds everything again at its next `Cmd`. While a `Cmd` is alive, nothing
  Display8 does in a call (the frame, the ring, descriptors, mirrors, targets) binds
  anything on the list; a mirror's update is a copy and its barriers.

The passthrough does not draw into it yet (phase 10 X1): with 0.19.0 the engine's
draws on Direct3D 12 are dropped, as before.

## Versions of the contract

The `display8v1_*` exports and their `#[repr(C)]` structs are a contract between this
DLL and the `gpu` crate linked into `Render3D.asi` and `UI.asi`. Up to display 0.11.0
it had no version: a d3d8.dll and plugins from two releases would have read each
other's structs at the wrong offsets. Since 0.12.0 (contract 1.0) it has a major and a
minor (`gpu::display::ABI_MAJOR`, `ABI_MINOR`), and the rule is the one in
`gpu/src/display.rs`, "Versions":

- **A minor only adds**: an export, a constant, or a field at the end of a struct.
  Every struct starts with `size`, so each side reads only what the other knows (a
  struct handed out says its size; a struct to be filled says its room, and the
  writer fills no more and sets `size` to what it wrote: `gpu::display::read`,
  `write`). A plugin uses what minor N added only on a Display8 of minor N or more. A
  struct inside another (`VkPass8`'s images) does not grow in a minor.
- **Anything else is a new major**: a field removed, moved, retyped or meaning
  something new; an export's signature or meaning; a rule between the recorders
  (layouts, states, heaps, who closes a render pass). The exports carry the major in
  their names (`display8v1_*`), so a plugin of another major finds none of them, and
  `display8_abi`, whose name and signature never change, says why.

What each side does with a mismatch, and logs:

| Files | What happens | The log |
|---|---|---|
| plugins and Display8 of one major | the handshake agrees; the plugins draw on the backend Display8 says | `Render3D.log`/`UI.log`: `gpu 1.0.0 (contract 1.0): Display8 0.12.0 (contract 1.0)`; `Display8.log`: `contract: UI.asi asks for 1.0; Display8's is 1.0: drawn through` |
| no Display8 (another `d3d8.dll`: crosire's d3d8to9, DXVK's, Wine's) | the plugins stand down and draw nothing (there is no Direct3D 9 to fall back to since gpu 2.0.0) | `gpu ...: no Display8 (the game's d3d8.dll is another chain's): this plugin stands down and draws nothing` |
| plugins newer than Display8 (a Display8 before 0.12.0) | `gpu` finds unversioned exports and no `display8_abi`: it stands down and draws nothing | `gpu ...: Display8 0.11.0 is too old for this plugin (no contract handshake ...)` |
| plugins older than Display8 (built before 0.12.0) | they look for `display8_backend` and find nothing: nothing drawn on Vulkan or Direct3D 12 | `Display8.log`, in stub mode 300 frames into the first mission (such a plugin calls nothing of Display8's, so it is told by not having asked): `contract: Render3D.asi is loaded and has not asked for the contract ...` |
| another major either way | as above, both sides | `gpu ...: Display8 ... has contract 2.0, another major ...`; `Display8.log`: `... another major: it finds none of this Display8's exports and stands down` |

On Vulkan the engine's own draws pass through where the plugins draw nothing
("Passthrough"), so the game still shows its stock look and can be played; on Direct3D
12, which passes nothing through yet, the 3D view stays empty. Display8 0.12.0,
render3d 0.31.0 and UI 0.26.0 were the last release that had to ship together; from
there a release of one may meet the others of the same major.
`testbench/scenarios/display-abi-mismatch.md` runs both directions against 13.42.4
(`bench-abi.sh`). Display8 0.11.0 beside the current plugins needs the retired `Menus.asi` back
(the shell's mode raise is Display8's since 0.18.0), which `bench-abi.sh` adds.

## Claims

Since 0.13.0 (contract 1.1; ROADMAP Phase 4, "Claims") the plugins say what they draw,
and passthrough draws what nobody does. `gpu::Gpu::register` (at the plugin's load, the
handshake made then) hands Display8 a callback through `display8v1_register`; Display8
asks every registered callback at each of the engine's `BeginScene`s in stub mode
(`src/claims.rs`, `claims::ask`) and logs a plugin's claims when they change:

    claims: UI.asi registers (id 1)
    claims: UI.asi claims selection hud loading credits (was not asked)
    claims: Render3D.asi claims hulls world particles effects ordnance specials fog selection (was none)

The categories are `gpu::display::CLAIM_*`. A plugin claims only what it draws now:

| Plugin | Claims | Not claimed |
|---|---|---|
| render3d | each module running (`sky`, `hulls`, `world`, `particles`, `effects`, `ordnance`, `specials`), `fog` and `selection` by `[Overlays] Fog` and `Selection` | a module at 0 (or kept for the menu scene only), one standing down beside its old plugin, everything before its first frame, and everything when its frame seam is not stock |
| UI | `menus` (the menu host runs: Display8 raises the shell's mode, "Window and display mode"; not a draw, so it holds when the UI cannot draw), `hud` (its panels are in), `selection` (`[HUD] SelectionBubbles=0`, its brackets marking the selection), `loading` (`[Loading] Enable=1`, the screen of ours), `credits` (`[Menus] Credits=1`), `cursor` (while it keeps the engine's cursor from drawing: at the present where its own is drawn) | a part at 0; everything while it cannot draw (`gfx::drawing`: 30 frames in a row with no device) |

A plugin that stood down because of the contract (another major, a Display8 too old)
never registers, and one built before 1.1 has nothing to register with: either claims
nothing, so what it would have drawn is the engine's again.

**How claims route.** Display8 drops the engine's draws of a claimed category only
where the engine draws them *anyway*, under the plugin's, and tells them by the scene:

| Category | The engine's scene | Dropped while claimed |
|---|---|---|
| `loading` | a `BeginScene` with `RenderLoopAssetLoadStatusUpdate` up its stack | yes: the UI's loading screen covers it |
| `credits` | a `BeginScene` from `Credits3D.obj` (0x591340..0x592a70) | yes (with the UI's Credits the engine's never run, so none come) |
| `cursor` | `RefreshDisplay`'s own scene (0x624630..0x624750, the immediate caller or the next) | yes (the UI empties the engine's cursor for that present, so it draws none) |
| the 3D categories, `hud`, `selection` | — | never: the engine issues these draws only when no plugin draws the part (render3d skips the engine's step; the UI skips a stock component's `Render`; the UI's `ret` in `RenderSelectionEffects`), so what it still issues, a hand-back, is drawn |

So `Passthrough=1` with every plugin on draws the same frame as before (the routing
was "every draw but the loading loop's while `UI.asi` is loaded"), and without the UI,
with `[Loading] Enable=0`, or with a UI that stood down, the engine's loading screen
comes through instead of a black screen.

**Between plugins.** `display8v1_claims(except)` is what every plugin but the caller
claims, each callback asked as it is called (`gpu::Gpu::claimed_by_others`). render3d
draws no selection bubbles while another plugin claims `selection`: that replaced its
reading of the `ret` the UI puts in `RenderSelectionEffects`, which it reads only on a
`d3d8.dll` without claims (`render3d/src/mods/overlays/selection.rs`). Nothing looks up
a plugin by its DLL name for routing any more.

`display-claims` checks three games on the stub: nothing claimed, the UI, and render3d
at `sky=0`.

**Display8 alone.** With no `UI.asi` and no `Render3D.asi` nothing registers, and in
stub mode passthrough draws the whole game as the engine issues it: the stock menus,
loading screen, HUD and 3D view ("Stub mode", "The engine's hard-coded modes", for
why that needs Display8 to answer the engine's own mode requests). `display-claims`'s
"alone" is that case.

## How it works

**An object of ours for every interface.** `IDirect3D8`, the device, the swap chain,
the three kinds of texture, surfaces, volumes, vertex and index buffers: each is a
vtable of ours (`src/stub/`) over memory of ours; `src/d3d8/` holds Direct3D 8's
structures and method names and `src/com.rs` the COM vocabulary they share (GUIDs, the
kinds of object, the roles). Every method sits in its slot with its exact `__stdcall`
argument list, from Wine's `include/d3d8.h`; one argument too many or too few unbalances
the caller's stack.

**`QueryInterface` answers an object's own interfaces and nothing else.** A plugin does
not reach Direct3D 9 through the engine's objects any more (until display 1.0.0 it did,
by `IID_IDirect3DDevice9` on the device and `IID_IDirect3DTexture9` on a texture): it
asks Display8 for its device and a texture's mirror through the exports of
`gpu/src/display.rs`.

**The plugins chain slots of our device's vtable** (`Reset`, `Present`: `ui/src/engine/device.rs`,
`render3d/src/frame/mod.rs`) with `VirtualProtect` and a write. The vtables are therefore
mutable statics, filled at start-up, and nothing in Display8 calls its own methods
through them.

**The bail-out is the exception to all of it**: the engine then holds the system's
objects, not ours ("The bail-out").

## Callers

Each method reads its own frame pointer (every function keeps one:
`force-frame-pointers` in `.cargo/config.toml`), so `[ebp+4]` is the exact return address
into whoever called it. The engine is built without frame pointers, so for the callers
beyond that one the stack is scanned for values that point just past a `call`
instruction in `Armada2.exe`'s code (`E8`, or `FF /2` of 2 to 7 bytes): up to five. The
first is exact; the rest are the usual heuristic, and a stale return address left in an
uninitialised local passes it too. State setters and getters, called thousands of
times a frame, keep only the exact one.

Addresses in the exe are written as they are and named offline through `armada2.map`;
an address in any other module is written `Name.dll+offset`. The map and the names
never go into this repository (hard rule 9).

## The inventory

The tracer counts the calls the engine makes on the stub's objects, as it counted them on
the real chain's until display 1.0.0 (`display-inventory`, `display-engine-draws` and
`display-drop` run on the stub). After a bail-out the engine holds the system's objects
and nothing is counted. With `Trace=1` in `Display8.ini`:

- **`Display8.inventory.tsv`**, beside the exe, rewritten every two seconds of
  presenting and at each new context: a row per context, interface, method and callers,
  with the calls, the frames they fell in, and how many came from a thread other than
  the device's. `#` lines at the end give each context's frames, engine draws and time
  spent in `Present`.
- **`Display8.log`** (in `%LOCALAPPDATA%\armada2-refit\logs\` since display 0.14.0, the game directory if that is unset or not writable; its first line says where): each distinct line once per context: the answers the engine
  branches on (`ANSWER`: caps, formats, modes, `ValidateDevice`), the present
  parameters of every `CreateDevice` and `Reset`, gamma ramps (`GAMMA`), every lock
  (`LOCK`) and read-back (`READBACK`), render targets, the queries for interfaces the
  object does not answer (`QI`), and the `user32`/`gdi32` calls that read the mode or move
  the mouse (`MODE`, `MOUSE`, `WINDOW`: `src/user32.rs`, hooked in the exe's import table).
- **Contexts** are labels the bench sets: `Mark the display context "mission"` writes
  `Display8.mark`, read at most four times a second at `Present` and at every
  `CreateDevice` and `Reset`, then deleted. Before the first mark the context is
  `start`.
- **`Expect the engine to draw nothing in "mission"`** reads the inventory: no `Draw*`
  on the device whose caller is in `Armada2.exe`, in that context.

## Hooks

What Display8 reads of `Armada2.exe` is in its symbol profile,
`profiles/<build>/display.toml` (`profiles/gog-1.1/` for GOG's 1.1), generated into
`src/engine_syms.rs` by `tools/engine/gen.py`; the format and the resolver are
`crates/a2engine/README.md`. At start Display8 logs the executable's identity, the
build it recognises and the summary:

    engine: Armada2.exe timestamp 0x3c4c76bd, image size 0x3fa000, sections 0xa1a8a321
    engine: build armada2/gog-1.1
    engine: symbols: 29/29 resolved; 0 hooks blocked

| Hook | Entries | Used by | Without it |
|---|---|---|---|
| `display.modes` | `ST3D_DisplayDevice::FindDisplayMode` (`fn`, its first 13 bytes checked), its six calls (`call`, each checked to call it), and the front end's 800x600x16 request (`value`) | `stub/modes.rs` ("The engine's hard-coded modes"); the stub's start in `lib.rs` | no stub: the bail-out |
| `display.native` | `GameConfiguration::InitializeGraphicsSystem` (`fn`) and its call in `WinMain` (`call`, checked), `UserProfile` (`global`), the profile's mode fields at +0x50, +0x54, +0x58 and the two stock modes (`value`) | `window.rs`: `[Window] Native` ("Window and display mode") | the profile's mode is as set |
| `display.onewindow` | `device_windows` (`value`): 1 where the game makes its device on a new window at each switch (Roots' profile), 0 on Armada II | `host.rs`: `[Window] OneWindow` ("One window") | the swap chain follows each new window ("A second window") |
| `display.keepclip` | the mouse driver's button bits and the flag that says the pointer is held for a pan (`global`; read only) | `window.rs`: `[Window] KeepClip` | the clip is put back regardless of a held button |
| `display.drop` | the code ranges of `RenderLoopAssetLoadStatusUpdate`, the Credits (`Credits3D.obj`, from its first function `SpriteWithLocation::Render`), `ST3D_DeviceDirectX8::RefreshDisplay` and the two workspaces' `Submit` (`fn` with `end`; nothing is written, so nothing is checked) | `drop.rs`: `[Drop]` ("Drop switches"), and the scene a `BeginScene` begins for the claims (`passthrough.rs`) and the contract's mission check (`api.rs`) | nothing is dropped and no scene of the engine's is claimed: the engine draws its own loading screen, Credits and cursor |

The tracer's caller names (`trace.rs`) are not a hook: they are addresses read off the
stack against the identity's image range, and mean the same on any build.

**On a build Display8 does not know** (`engine: unknown build`, or `display.modes`
standing down) the stub does not start: the game gets the system's Direct3D 8 and the
plugins stand down (`bail-out: engine: unknown build; ...`, "The bail-out"). Measured on
the bench 2026-11-03 with `A2EngineTest.ini` `Mismatch=display.modes` on the stock exe
and no `UI.asi` (what an unknown build leaves, since the UI stands down there too): the
stub started, the engine's own mode requests found no mode, and the game died of a stack
overflow right after `CheckDeviceType`, as before 0.15.1. So the stub with passthrough
is not an option without the mode hook. The bench's `A2EngineTest.ini`
(`[Test] Mismatch=<globs>`) makes chosen hooks stand down on the stock exe as a failed
check would; `./install` deletes it.

## Drop switches

`[Drop]` in `Display8.ini` (`src/drop.rs`): the engine's draws on the stub, accepted and not
passed on. Read again every 60 presents, so one session can shoot a frame with and without.

| Key | Drops |
|---|---|
| `Drop2D=1` | the engine's 2D: its batched quads (the trifan buffer), its lines, the fades |
| `Drop3D=1` | the meshes it still draws (render3d's hand-backs): every draw whose immediate caller is a workspace's `Submit` (`ST3D_WorkspaceDirectX8::Submit` 0x648af0..0x648c30, `ST3D_WorkspaceDirectX8NonVB::Submit` 0x648520..0x6485d0), which every mesh the engine draws goes through |
| `DropLoading=1` | everything drawn from the engine's loading loop: any draw with a return address in `RenderLoopAssetLoadStatusUpdate` (0x598ba0..0x598ff0) among the sixteen found up its stack, which is the function the loop calls to draw each step's frame (the logo scene, the load bar) |
| `Callers=0x..., 0x...` | any draw with one of these return addresses on its call chain |

A draw is the engine's when its immediate caller is in `Armada2.exe`; the plugins draw
through `gpu` and never pass here. With the refit installed and every category's
work taken over, the game must look the same with the switch on: that is how each
context is proved done before the stub drops everything.

## Naming a draw

- **`Display8.go`** (`src/detail.rs`): while it exists beside the exe (looked for every
  30 presents, then deleted), the next `[Trace] Frames=` frames (default 3) log a `DRAW`
  line for each draw from `Armada2.exe`: the blend and depth state, the texture, and for
  vertices that can be read, their box on the screen and their colours. The engine's 2D
  goes through one dynamic vertex buffer it writes by lock (its trifan buffer), so
  during a capture each `Unlock` copies what was written. This names a draw without a
  picture: the fade from black is a full-screen black quad whose alpha falls frame by
  frame, the drag box four white lines.

## Stub mode

The stub (`src/stub/`), the only mode since display 1.0.0: the engine gets an
`IDirect3D8` and devices of ours that accept everything and draw nothing, and Display8
makes a device of its own, on Vulkan or Direct3D 12 ("Backends"), on the game's window.
render3d and the UI find it through the exports of `gpu/src/display.rs` and draw the
whole frame on it. The answers the engine's `IDirect3D8` gives (caps, formats, modes) are
those DXVK behind crosire's d3d8to9 gave on the bench in phase 0 (the chain since removed) (`stub/d3d.rs`), so the
engine takes the code paths it took there.

- **One device for the game's life**, made at the engine's first `CreateDevice`. Its
  later `CreateDevice`s and `Reset`s get a new device object of ours on the same device,
  or nothing at all.
- **One display mode: the desktop's.** The engine is offered only that one (X8R8G8B8,
  the adapter's refresh), so it draws at the size the swap chain has and no mouse
  coordinate needs translating (phase 0 found the engine reads the mouse only through
  `GetCursorPos`, `ScreenToClient` and `GetClientRect`). The game's window becomes a
  borderless one covering the screen, and the swap chain is windowed on it.
- **The engine's hard-coded modes get the one mode** (`src/stub/modes.rs`, since
  0.15.1; below).
- **What the plugins read of the engine's state reaches the device**: the
  transforms (render3d reads the view and the projection), the viewport, `Clear` of the
  engine's own back buffer and depth, `BeginScene` and `EndScene`. Everything else is
  kept for the engine's own `Get*` and goes nowhere.
- **Resources are memory.** A texture's levels, a surface's pixels and a buffer's bytes
  live in Display8, and a lock hands out a pointer into them, as the managed pool's
  system copy did. Draws are counted (the inventory), and dropped, or drawn by the
  passthrough ("Passthrough").
- **The mirror table.** A texture a plugin samples (the radar's two scan-grid sprites,
  phase 0's only ones; the unit view's Bink frame) is made on the device once with every
  level (`display8v1_vk_mirror`, the Direct3D 12 twin) and brought up to date when the
  engine unlocks a level, at once on the device's thread, else at the next `BeginScene`
  or `Present` (`BinkThread` writes from its own).
- **The back buffer's pixels** (the movie path locks it and writes each frame) go into
  the frame when it is unlocked, as on a real device, so what is drawn after lands on
  top: the UI draws each movie frame at the `Present` that follows, and the cutscene
  proxy then writes nothing into the locked buffer. Copied at `Present` instead (until
  display 0.6.1), the untouched buffer covered the UI's frame, and every movie played
  black with its sound, on every backend. An unlock off the device's thread waits for
  `Present`. `display-movie` checks it.
- **The gamma ramp in the last pass** (`shaders/present.hlsl`, in each backend's
  present): a windowed swap chain ignores `SetGammaRamp`, and the ramp the engine sets at
  start-up lifts every frame (2 to 3, 80 to 90). So the back buffer is drawn into the
  swap chain through a 256x1 lookup of the ramp.
- **Does not start** (the bail-out, "The bail-out") when a stale `HUD.asi`, `GridLayout.asi` or
  `Menus.asi` (retired plugins an old install left; `retire/` takes them out) is loaded (each changes what the engine draws, which the stub drops;
  `GridLayout.asi` would leave no button bar at all, since the UI's stands down beside
  it), on a build or with a `display.modes` hook that is not stock, with `Backend=none`,
  and when no backend of the order can start. A device is made at the engine's first
  `CreateDevice`: if the backend cannot be made there, the next is tried, and when none
  starts the bail-out is taken there, late ("Choosing a backend").

### The engine's hard-coded modes

Storm3D keeps the modes the adapter lists on its display device and looks one up by
width, height and depth (`ST3D_DisplayDevice::FindDisplayMode`, `0x626bf0`), which
finds nothing without an exact match. Three of its callers ask for a mode of their own
rather than one from the list: `SetActiveDisplay_Internal` the shell's 800x600x16 (the
call at `0x62c4e0`; the same mode again at `0x62c643`, its fallback when a display does
not open), `GameConfiguration::InitializeGraphicsSystem` the profile's (`0x53e7c9`,
`0x53e7e7`; GOG's `ARMADA.PRF` holds 1024x768x32, no profile 800x600x16) and
`Program::PlayIntroMovie` the intro's 640x480 (`0x4861da`, `0x4861f2`). The stub lists
the desktop's mode alone, so with nothing raising those requests the shell's display
was opened on no mode, which failed, and the game died of a stack overflow before the
engine's first `CreateDevice` (seen on the bench 2026-10-30: Wine's log says
`stack overflow`, `Display8.log` ends at `CheckDeviceType`). Until 0.15.1 only the
UI's `raise_shell_mode` (`[Menus] Enable=1`) kept it from happening, by making the
shell's request the desktop's.

Display8 re-points those six calls at a lookup of its own
(`modes::find`): the engine's first, and when it finds nothing, the listed mode of the
stub's size. The six calls and `FindDisplayMode`'s first bytes are one hook, `display.modes`
("Hooks", below): all are checked before anything is written, and if one is not stock
none is written and the stub is not started (the bail-out). The lookup's other callers
keep the exact match: the adapter's enumeration, which uses it to skip a mode it has
listed already, and Graphics Settings, which offers what is listed (the one mode).
`Display8.log` says what was asked and given:

    stub: the engine's own mode requests (6 of 6 calls) get the one mode offered when theirs is not listed
    stub: the engine asks for 800x600x16, not listed: given the one mode, 1280x720x32

Since 0.18.0 the same six calls are also where the shell's and the intro's modes are
raised ("Window and display mode"); the UI no longer patches anything of the kind.

**What the stock shell looks like then** (Display8 alone, 1280x720 on the bench): the
game's window covers the screen from the first `CreateDevice`, and the GDI menus draw
at their stock 800x600, 1:1, in its top-left corner; the rest of the window is
unpainted (white on the bench). Every screen works with the mouse at those
coordinates (main menu, Single Player, the campaign's Select Mission), and a mission
then opens at the desktop's size, drawn by the engine through passthrough. Scaling the
shell into the window is the UI's (`ui/MENUS.md`).

### Window and display mode

Since 0.18.0 (contract 1.3; ROADMAP phase 9, C3) Display8 is the only plugin that
changes a display-mode request of the engine, the game's top-level window or the
pointer's clip. The keys are `[Window]` in `Display8.ini` (`settings.toml`; every row is
`NO_ROW`, none on the Refit screen), and the first three moved there from `UI.ini`'s
`[UI]` (`Native`, `KeepClip`) and the cutscene proxy's own raise:

- **The shell** (`SetActiveDisplay_Internal`'s 800x600x16, the calls at `0x62c4e0` and
  `0x62c643`) is raised to the desktop's size at 32 bits while a plugin claims
  `CLAIM_MENUS`, asked when the engine makes the request (`claims::menus`). The UI claims
  it while its menu host runs (`[Menus] Enable=1`, no stale `Menus.asi`); the host draws the
  shell's dialogs at the screen's size, so a shell at 800x600 would be wrong for it. The
  raise is done inside the lookup (`modes::find`: the request is replaced before the
  engine's own lookup is called, and the engine's request is the fallback). The UI used
  to patch the two pushes before these calls instead; this needed no byte patch because
  the engine goes on with the mode the lookup returned.
- **The intro** (`PlayIntroMovie`'s 640x480 at 32 bits and then 16, the calls at
  `0x4861da` and `0x4861f2`) is raised the same way under `RaiseIntro=1` (shipped 1). Display8's `RaiseIntro` is the
  intro's only owner: the cutscene proxy's own `RaiseIntroMode=` went in cutscenes 3.0.0,
  and its `Fit` only fits each frame to the screen it is given.
- **`Native`**: `display.native`, a stock profile mode (800x600 with no `ARMADA.PRF`,
  GOG's 1024x768) becomes the desktop's before `InitializeGraphicsSystem`; a mode chosen
  in Graphics Settings is kept.
- **`KeepClip`** (`display.keepclip`): under Wine, at each `Present` (every scene,
  not only the mission), Wine's clip is handed to `ClipCursor` again at most
  every 500 ms, on a frame where the pointer moved, while the game's window is the
  foreground and active one and the engine has no button down and is not holding the
  pointer for a pan.
- **`display8v1_mode(&w, &h)`** (contract 1.3) tells a plugin the mode the device was
  opened at (the stub's); false before. The UI reads it
  for its log line of what the shell runs at; a plugin falls back to measuring the
  desktop when the export is absent.

**One OS window.** The game is meant to be one window from launch to exit. Every shell
menu is a `WS_POPUP` dialog owned by the 3D window, which Wine turns into a second X11
window that Hyprland tiles and focuses like a new application (the menu is visible while
input stays with the other window); the UI's host makes each dialog a hidden child of the
3D window instead (`ui/MENUS.md`, "What the shell is, in the engine"). The Wine virtual
desktop would also give one window but a device of ours cannot set a display mode inside it (nor could DXVK), so it
stays off. A stub window that covers the screen at the desktop's mode needs neither.

`MoveWindow` and `SetWindowPos` in the exe's imports stay the UI's, for the hosted
dialogs (`user32.rs` leaves them alone). Without Display8 nothing raises the shell
(UI no longer does), so it stays at the stock 800x600: fail safe. The bench checks:
`display-window`, `ui-settings-moved`, `display-movie`, `ui-menus-refit`.

`display/bench-mode.sh stub` sets Display8's keys in a bench clone from a scenario's
`Setup:` (the word `forward` is still accepted, runs the stub and says so);
`display-stub` walks the whole game on the stub.

## Passthrough

`Passthrough=1` in `Display8.ini` (shipped, since the GPU runs below passed, and the
code's default too since 0.12.0; `0` drops every engine draw, as before): on Vulkan, and since 0.20.0 on Direct3D 12, the engine's own Direct3D 8 draws are
drawn after all, on Display8's device, the way Direct3D 8's fixed function draws them
(`src/passthrough/`, `src/vk/pass.rs`, `src/dx12/pass.rs`, `shaders/ff.hlsl`). It is ROADMAP phase 4's
first slice (4a, 4b, most of 4c, 4d, 4e and 4f on Vulkan). Without it, whatever render3d
or the UI does not draw itself vanishes: a module switched off
(`[Modules] hulls=0`), a SOD render3d hands back (one that does not load, a hologram's
class that is not a plain database), an effect class it does not know, specials with
no particle art, construction and the bees without the hulls module, a HUD part the UI
has not taken. With it, each shows as stock, as it did on the forward chain (DXVK, until display 1.0.0; history).

**Which draws.** Every draw on the stub's device is the engine's (the plugins draw
through `gpu`), so routing is simple: every one is drawn, except

- what `[Drop]` names (so a category can still be A/B'd: `Drop3D=1` drops the hand-backs
  again);
- the scenes a plugin claims to draw over ("Claims"): the loading loop's while the UI's
  loading screen is up (`CLAIM_LOADING`), the stock Credits' and the cursor's. Without
  the UI, with its part at 0 or with a UI that stood down, the engine's loading screen
  is the only one and is drawn;
- a draw into a render target of the engine's own other than its back buffer (none seen);
- a draw through a vertex shader other than the one it has a port of (below), logged
  once;
- a draw made while a plugin's `Cmd` is recording (a plugin calling the engine with a
  `Cmd` alive): logged once with its caller, never recorded into another recorder's
  render pass.

Since 0.13.0 the plugins' claims are this routing ("Claims"): no DLL is looked up by
name.

**Where it lands.** In the frame's command buffer, at the moment the engine makes it,
between the plugins' recorders: the order the engine's draws had on Direct3D 9 (until
display 1.0.0), where a plugin's targets stayed bound after its `Cmd` and the engine's
draws landed in them. On Vulkan a plugin's
pass ends with its `Cmd`, so render3d's `gpu` **publishes** its pass now at each of its
`Cmd`s' ends (`display8_vk_pass`, `PASS_PUBLISH`; `gpu::Gpu::with_display_publishing`),
every attachment put in its attachment layout first. While its body runs that is the
scene pass (the half-float, multisampled target and its depth): the hulls it hands back
are drawn into the scene, depth-tested against what render3d drew, then resolved,
bloomed and tone-mapped with the rest. After its post passes it is the engine's back
buffer, and the engine's targets are used (Display8's colour and depth). Without
render3d (or before it publishes in a frame) every draw goes to the engine's targets,
and the UI draws on top (4e). What was published is forgotten at `Present`: the views
in it are render3d's, which it may let go of. The UI's `gpu` publishes nothing; it, and
every other recorder, says only when a `Cmd` begins and ends (`PASS_ENTER`,
`PASS_LEAVE`), which is how a draw inside one is told.

**One render pass for a run of draws**, opened at an engine draw on the pass it goes to
and left open; it is closed at the next call into Display8 that records or hands the
frame over (`display8_vk_frame`, `display8_vk_mirror`, `display8_vk_flush`,
`display8_vk_pass`, an upload, a `Clear`, `Present`), so the contract's "no render pass
is open between recorders" still holds for every recorder. Draws are never merged or
reordered, so the engine's back-to-front translucent order stands.

**How Direct3D 8's state maps:**

| Direct3D 8 | Here |
|---|---|
| FVF (`XYZ`, `XYZRHW`, normal, diffuse, specular, 0 to 8 coordinate sets of 1 to 4 floats) | vertex attributes at their offsets, at the stream's stride; what the FVF lacks reads a defaults buffer at stride 0: white diffuse, black specular, zero normal and coordinates, as Direct3D 8 has them. Vertex blending (`XYZB*`) is drawn unblended |
| `XYZRHW` | screen pixels into clip space relative to the viewport, which Vulkan's viewport (the same rectangle) maps back, so the viewport clips them as Direct3D's does; w from 1/rhw, so coordinates are interpolated in perspective; Direct3D 9's pixel centres kept (DXVK's 0.5 - 1/128) |
| `XYZ` | world, view and projection multiplied on the CPU; the viewport as the engine set it; Direct3D 9's centres only with `PixelCentres=d3d9`, as render3d's scene |
| lighting (`LIGHTING`, the asteroid fields) | per vertex in world space: emissive, the ambient render state, each enabled light's (up to 8) ambient and diffuse, point and directional (a spot light as a point light), attenuation and range; material sources from `COLORVERTEX` and the four `*MATERIALSOURCE`s. No specular light |
| texture stages 0 and 1 | `ps_ff` runs each stage's colour and alpha operation (every `D3DTOP_*` up to `LERP` but the bump-map ones and `PREMODULATE`) on its three arguments with complement and alpha replicate, `TEMP` as a result, `TEXTUREFACTOR`; a stage with no texture reads white. A third stage, a texture transform or generated coordinates are logged once and left out |
| `SPECULARENABLE` | the specular colour added after the stages |
| alpha test | in the pixel shader: the alpha as 255ths, rounded, against `ALPHAREF` by `ALPHAFUNC` |
| blend | `ALPHABLENDENABLE`, `SRCBLEND`, `DESTBLEND` (`BOTH*` as Direct3D's), `BLENDOP`, `COLORWRITEENABLE`; on the engine's back buffer, which is X8R8G8B8, a destination alpha reads one |
| depth | `ZENABLE`, `ZWRITEENABLE`, `ZFUNC`; off without a depth surface or attachment. `ZBIAS` is logged once and left out |
| cull | `CULLMODE` as gpu has it (the vertex shader turns y over): `CW` is Vulkan's back face with a counter-clockwise front |
| samplers | per stage: address, filters, mip filter, `MAXANISOTROPY`, `MIPMAPLODBIAS`, `MAXMIPLEVEL`, made as DXVK made them under the game's `dxvk.conf` (the forced anisotropy and LOD bias), now `[Samplers]` ("Samplers") |
| fog | never enabled by the engine (the stock traces); logged once if it ever is |
| fans | lists (a portability device has no fans) |

Pipelines are made per target formats and samples, topology, vertex layout, blend,
depth and cull, in Display8's pipeline cache (kept on disk). Everything else goes in
two constant blocks in the upload ring, bound as dynamic uniform buffers of one set per
ring chunk; a block equal to the last draw's is not sent again. Textures are the mirror
table's: every engine texture a draw reads is mirrored at its first draw, with every
level, and kept up to date as the engine writes it by lock; one over 256 KB goes up
through a staging buffer of its own, let go of with the frame, so a mission's hull
textures arriving at once do not grow the upload ring, whose chunks live as long as the
device. A texture of another format than A8R8G8B8 and X8R8G8B8, a cube or a volume, is
drawn with white and logged once; the stub offers the engine no other texture format,
so none should arrive.

**Texture formats.** `CheckDeviceFormat` says yes to A8R8G8B8 and X8R8G8B8 alone as
textures (`stub/d3d.rs`, `texture_format_ok`), because a mirror holds no other (and, since
display 1.1.0, DXT1, DXT3 and DXT5 where the backend samples them: "Block-compressed
textures" below). The
engine chooses its texture formats in `ST3D_DeviceDirectX8::SelectTextureFormats`
(0x625f80) by asking in an order its graphics detail sets (`ST3D_GraphicsEngine_Configuration`
+0x40, the Graphics Detail slider, `ARMADA.PRF` line 5, field 6): at the top detail
X8R8G8B8 and A8R8G8B8 come first; below it R5G6B5, X1R5G5B5, A4R4G4B4 and A1R5G5B5 do.
Until display 1.0.2 the stub also said yes to the 16-bit formats (a list from when stub
mode drew on a Direct3D 9 device), so below the top detail every engine texture was
created 16-bit and drawn white: the nebulae left to the engine (`[World] Nebulae=0`)
were flat squares, and the fog's texture had no mirror for the overlays
(`render3d-nebula-detail`). Refused, the engine falls through to the two 32-bit formats
at every detail; the detail still sets the levels of detail, the starfield and the
first level of a texture's chain it loads (`ST3D_GraphicsEngine::GetLargestMipMapToUse`).

**Block-compressed textures** (for Fleet Operations: Roots, `PLAN-FLEETOPS.md` FO-4; the
stock engine never makes one). Roots loads about 550 DDS files through D3DX8, which asks
`CheckDeviceFormat` for DXT1 to DXT5 and, told no, decompresses each file itself (4 to 8
times the memory). `CheckDeviceFormat` now says yes to DXT1, DXT3 and DXT5 as 2D textures
when the backend can sample them (`stub::compressed_textures`: Direct3D 12 always, Vulkan
with `textureCompressionBC`, which Display8 enables when the physical device has it), and
to no other new format; cubes and volumes stay refused. The stub already sized them as
Direct3D 8 does (`stub/blocks.rs`, host-tested): a level's pitch is one row of 4x4 blocks
(8 bytes a block for DXT1, 16 for the rest), `GetLevelDesc`'s size is the level's bytes,
and a level under one block (the 2x2 and 1x1 ends of a chain) is one whole block, which is
what D3DX8's `LockRect` fills. The mirror holds them as BC1 (RGBA), BC2 and BC3 (DXT2 and
DXT4, the premultiplied spellings, as BC2 and BC3 too), uploaded by block rows and sampled
natively, with every level (`upload_blocks` on both backends; on Direct3D 12 a footprint
is the whole block and a base level not a multiple of 4 gets no mirror, as that API
requires). A plugin that samples such a mirror gets the format in `VkImage8.format` and
`Dx12Image8.format`: it is not B8G8R8A8, which the contract has always said to read off
the info. `display-compressed-textures` checks it: with `BenchTextures=N` (bench clones
only, `benchfo.rs`) Display8 itself makes a DXT1, a DXT3 and a DXT5 texture of 64x64 with
seven levels through its own device's vtable, fills them through `LockRect` and draws
them through the passthrough with `DrawPrimitiveUP`, over a paused mission; the checker
of blocks, the alpha ramps and the flat lower levels are in the screenshot on Vulkan and
on Direct3D 12. `CheckDeviceMultiSampleType` says yes to any multisample type (2 to 16)
for a back buffer or depth format, and `CreateDevice` accepts it and ignores it, as the
device's back buffer is single-sampled whatever is asked and render3d does its own MSAA.

**A second window** (Roots makes one device for its 1200x900 shell and another for the
game, on another window, and a skirmish was a white screen while the engine drew on).
The backend is made at the first `CreateDevice`, on its window, and a later `CreateDevice`
only looked the device up. When it names another window now (`stub::follow_window`) that
window gets the first one's treatment (a popup over the desktop) and the swap chain moves
to it: on Vulkan a surface of the new window, the old swap chain and surface destroyed and
a swap chain made again (`Vk::rebind`); the colour target, the pipelines and the mirrors
stay, so nothing a plugin holds changes. The stock engine reuses its one window at every
`CreateDevice`, so this never runs there. `display-window-follow` checks it (the bench-only
`BenchWindow=N` hook hands a popup to the same function at the Nth `Present`, and the first
window again at the 2Nth, with the popup left from taking the activation, which stops the
engine drawing). **Direct3D 12 has the same move (`Dx::rebind`) and it is not verified**:
on the bench the game goes on drawing after it but the new window stays black. Roots on
Direct3D 12 is for FO-4 to settle; `auto` tries Vulkan first.

**One window** (`[Window] OneWindow`, `host.rs`; PLAN-FLEETOPS.md decisions 314 to 319).
Following each new window still showed at every switch: Roots destroys its device window and
makes another between its menus, the loading screen and a mission, so the window manager saw
the game's window close and a new one open (focus and full screen moved, an animation on
Hyprland) and the new one was black until its first frame. Where the build's profile says
the game does this (`device_windows` 1: Roots), Display8 makes a top-level window of its own
at the first `CreateDevice`, the **host** (class `Display8Host`, the game's title), and puts
every device window inside it as a child covering it (`WS_CHILD` and `SetParent` before it
is shown, so it is never mapped as a window of its own); the swap chain is made on the host
and never moves. The device window keeps its procedure and gets the mouse (it covers the
host) and the keyboard (the host gives it the focus at every activation and forwards
`WM_ACTIVATEAPP` and `WM_CLOSE`); the host stays enabled when a modal form disables the
thread's top-level windows, so the device window's own enabled state is what counts, as it
was. A window's owner is always a top-level one, so the forms FleetOpsHook owns by the
device window are owned by the host: the UI finds the device window inside it. Armada II's
profile says 0 and the bench's `BenchWindow` still follows. The host is made only for a plugin that claims `CLAIM_WINDOW` (contract 1.5: the UI with Roots' forms its own, `[Menus] Forms=1`), decided at the first `CreateDevice`; with the game's own forms and dialogs, windows of their own over the game's, the host took the pointer from them (decision 405), so without the claim the swap chain follows each new window as before. The host is never hidden by the game (decision 404). Frames with nothing drawn are not shown right after a switch, and,
with the host made, a frame none of the plugins drew on is not shown while they are
drawing the game's frames (for at most 5 s after their last), so the picture holds
between two screens instead of going black (`host::hold`, `host::between_screens`;
PLAN-FLEETOPS.md decisions 315 and 403).

**A 1200x900 shell** needs nothing new: a mode the stub does not list is answered with the
desktop's (the same as the 800x600 and 1024x768 requests of the stock shell and profile),
`CreateDevice` is told it got what it asked and the window covers the desktop. The six
`FindDisplayMode` calls of `stub/modes.rs` are Armada II's addresses, and Roots keeps
them: the profile marks them verified (FO-A; "On Fleet Operations: Roots").

**The engine's one vertex shader** (ROADMAP 4d) is the Borg hulls' bump pass. Display8
keeps every vertex shader's declaration and function tokens at `CreateVertexShader`, the
vertex shader constants at `SetVertexShaderConstant` and streams 1 to 3, and knows the
shader by an FNV-1a hash of its function tokens (`passthrough::BORG_BUMP`); where its
declaration puts each input in stream 0 is read off the declaration. Its port by hand is
`vs_bump` in `shaders/ff.hlsl`: the position through the four rows the engine sets in
constants 2 to 5, and the light direction of constant 6 taken into the vertex's tangent
frame, normalised, offset and scaled by constants 0 and 1 into the diffuse colour, which
the texture stages dot with the normal map (`DOTPRODUCT3`). The engine draws a Borg hull
in four passes this way (two lights, the first written and the second added; the base
texture multiplied over them; then the base texture again, blended by its alpha, which
is the hull's glow), all of which the stage interpreter runs. Any other shader is
refused and logged once with its hash; on the test bench its tokens are logged too, for
porting.

**The engine leaves `LIGHTING` on for these passes** on a map with lights (the Borg
campaign's, not the showcase's `a2_borg01`). Direct3D ignores it under a vertex shader,
and so must the passthrough: its fixed-function lights go in the vertex block from
register 16, where `vs_bump`'s constants are, and until 0.15.2 they were written over
them, so every pass but the glow's had its transform thrown away and the hull vanished
around its lights (seen in game on *Strange Bedfellows* with every render3d module off;
`display-passthrough-all`). Display8 logs once that it saw the case. (The `PASSTHROUGH`
lines logged "once" are the tracer's: outside the bench, with `Trace=auto`, a player's
log does not have them.)

**On Direct3D 12** (0.20.0, phase 10 X1; `src/dx12/pass.rs`). The routing is the same
code (`src/passthrough/mod.rs`), and so is every translation of the engine's state to
what a pipeline and its constants hold: `src/passthrough/state.rs` names no API type,
and `display/hosttest` (`cargo test --release --target x86_64-unknown-linux-gnu
--manifest-path display/hosttest/Cargo.toml`) tests it on the host. Where Direct3D 8
and Direct3D 12 number a thing alike (blend factors and operations, comparison
functions, primitive topologies) the number is Direct3D's. What differs:

- **No render pass is open between draws.** A draw sets what differs from what is set
  (root signature, pipeline, targets, viewport and scissor, topology, root arguments,
  vertex and index buffers) and draws. What it assumes set is forgotten whenever someone
  else may have recorded: a plugin's `Cmd` begins or ends (`display8v1_dx12_pass`), a
  `display8v1_dx12_flush`, a new frame (`pass::forget`). Display8 never sets descriptor
  heaps (the frame's two are set when it begins; it takes descriptors from their ranges
  as the plugins do), never samples, copies or changes the state of the pass it draws
  into, and while a `Cmd` is alive records nothing (a mirror's update is copies and
  barriers, which bind nothing: `gpu/src/display.rs`, "The rebind rule").
- **Where it lands**: `exports::published()`, the pass a plugin published in this frame
  (render3d publishes two kinds each frame, its 4x scene pass with depth while its body
  runs, then a 1x pass with depth; the last published is the one), by its own `rtv` and
  `dsv`, its formats and sample count in the pipeline's key; a published colour that is
  Display8's own colour target is the engine's targets, as on Vulkan.
- **One root signature**: the vertex block as a root CBV at b0 in space 0 (`vs_ff`) and
  in space 1 (`vs_bump`; the DXIL's `CB_SPACES`), the pixel block as one at b1, a table
  of t0 and t1 and a table of s0 and s1. The blocks are written once per change into the
  frame's ring at 256 alignment (the pixel block at `PS_AT`).
- **Pipelines** are made on first use (no library yet), keyed by the target's formats
  and sample count, topology type, vertex layout, blend, depth and cull. The destination
  alpha of the engine's back buffer reads one, as on Vulkan; blend factors that read
  colour become the alpha ones for the alpha channel (Direct3D 12 refuses colour factors
  there, Vulkan reads the same component). Cull follows Direct3D 8's clockwise culling
  with a clockwise front face. What the vertices lack comes from a 48-byte defaults record
  in the frame's ring, in slot 1 as **instance data** read by the draw's one instance:
  a vertex stream of stride 0 is not asked for.
- **Samplers** are made in the frame's sampler heap, one pair per distinct pair of stage
  states in a frame, by `state::sampler_params` (DXVK's way, the values of
  `[Samplers]`: a linear filter anisotropic at the forced amount, the forced bias added
  where there are mips); **textures** are the stub's mirrors (a view copied into the
  frame's range, one table per distinct pair) or a 1x1 white one.

Measured on the bench (2026-11-03, 1280x720, the GPU): `display-passthrough-d3d12`
against Vulkan's frame RMSE 0.48 (`roundD-base-display-passthrough`; the plan's
tolerance is 1.0), the showcase with `sky=0` and `hulls=0` against the same on Vulkan
0.48, the nebula scene against the forward chain's baseline 0.9 (inside the scenario's
1.5), `display-passthrough-all` (every module off, a Borg mission with the bump
passes) and the stock front end through a campaign mission, save and load, Instant
Action and the Credits (`display-stub`) with no `device removed`. The Vulkan frames are
unchanged (RMSE 0 against the base).

**What is not done yet**: a pipeline library for these pipelines. Specular lighting, spot cones, texture transforms, a third stage and
`ZBIAS` are not drawn, each logged once when the engine uses it; none was seen in the
stock traces.

**Map nebulae** (reported 2026-10-30: with every render3d module off, the cerulean
nebulae "move a lot more than in stock"). The engine's map puffs are measured to draw
and move through the passthrough exactly as on the forward chain and in stock, bench
2026-10-30, 1280x720, scene `nebula-cerulean`, the bench clock, `Step=16`:

- the same tick against the forward chain: RMSE 0.93/255 (the last bit of the additive
  blend over the gas, the ship's edges); from four other camera positions, the 3D view
  within RMSE 0.5;
- frame-to-frame change over 2 ticks (mean absolute difference, 0..255): forward 1.24,
  stub 1.24; over 16 ticks 6.07 and 6.05. A paused frame stands still in both;
- against stock (`Mod: stock`, the HUD view): with the stock textures (`A2_DATA` empty)
  the gas is stock's to the pixel and changes 0.72 per 2 ticks to stock's 0.75; on the
  wall clock the motion per second is the same at 60, 300 and 700 frames a second, so
  it is not the frame rate either (`VSync=0`);
- with the refit's textures the same motion changes 1.01 per 2 ticks, 39% more: the
  upscaled `Mnebula5` (`textures/targets/Mnebula5`, `blend=35`) carries finer, sharper
  detail than stock's, and that detail is what is seen to swim. With render3d on, the
  cerulean class is its marched volume (`render3d/content/nebulae/mnebula10.ini`), not these
  puffs.

No unsupported state is used by those draws (no texture transform, no generated
coordinates). `display-passthrough-nebula` holds it, against
`display-passthrough-nebula-forward`.

`display-passthrough` checks it (render3d's hulls off on the stub; the ships drawn),
against `display-passthrough-forward`, the same scene through the forward chain.
`display-passthrough-all` checks it with every render3d module off, on a Borg mission
with lights entered through the menus, against `display-passthrough-all-forward`.

**Measured 2026-10-29** (1280x720, the showcase paused at its tick, bench clock): with
`[Modules] hulls=0`, stub and passthrough on lavapipe with the validation layer against
the forward chain (DXVK on the GPU). The Federation hulls are drawn into render3d's scene
in place and lit as on the forward chain: RMSE 4.3/255 over the Galaxy, 4.8 over the
Sovereign (a CPU rasteriser against a GPU's, as `gpu/README.md`'s "Measured"), no
validation message, no draw made inside a plugin's recording. The Borg cubes are
missing (their bump vertex shader, 4d), which makes the whole frame's RMSE 22.9. With
every module on, `port-showcase` on the stub draws the same frame with `Passthrough=1`
as with 0 (RMSE 0): the engine draws nothing there.

**On the GPU** (RX 5700, RADV, the validation layer on; `A2TEST_VULKAN_GPU=1`), the same
scene against the same forward frame, RMSE in 1/255:

| | Galaxy | Sovereign | Borg cube | whole frame |
|---|---|---|---|---|
| before the bump port | 3.54 | 3.96 | missing | 22.83 |
| with it | 3.54 | 3.96 | 18.78 | 6.81 |
| with it, `PixelCentres=d3d9` | 3.04 | 3.79 | 1.37 | 1.64 |

The cube's 18.8 is half a pixel: the engine's transformed draws keep Vulkan's pixel
centres, as render3d's own scene does, unless `PixelCentres=d3d9`, and the cube's fine
plating shows a half-pixel shift that much; with Direct3D 9's centres it is the forward
frame within 1.4. No validation message in any run. `display-stub` (the whole game:
intro to Exit, a campaign mission, save and load, an Instant Action match, the Credits)
and `display-drop` both passed on the GPU with `Passthrough=1` and the layer, with no
validation message; with every module on, the engine drew into the passthrough only
once, in the shell, into its own targets.

## VSync

The game has no frame limit of its own: the only cap is the vertical blank. The engine
makes its device with `D3DPRESENT_INTERVAL_DEFAULT` (`interval 0x0` in the log), which
every backend reads as "wait for the vertical blank", so the frame rate is held to the
display's refresh rate: Vulkan presents FIFO, Direct3D 12 at sync interval 1. (The two `Sleep`s in the engine's own loop are not
a limit: `Program::GameDoOne` sleeps once, and `Transport::DoGameCycleDelay` sleeps only
for a delay that is 0 in an ordinary game.)

**`VSync=0`** in `Display8.ini` gives the device `D3DPRESENT_INTERVAL_IMMEDIATE`
instead, at the engine's `CreateDevice`, so frames are presented as fast as they are
drawn: on Vulkan the surface's mailbox mode, else immediate (tearing); on Direct3D 12
sync interval 0. `Display8.log` says so at start, and the swap chain's line names the
present mode (Vulkan's 1 is mailbox, 0 immediate, 2 FIFO). After a bail-out the setting
is not read: the system's device keeps the engine's interval. The shipped value is
`VSync=1`.

## Backends

`Backend=` in `Display8.ini` says what the stub draws on: `vulkan` (`src/vk/`), `d3d12`
(`src/dx12/`), `auto` (below) or `none` (the bail-out, "The bail-out"). No `d3d9.dll` is
loaded: Display8 owns the device, the swap chain and the frame, and render3d's and the
UI's `gpu` record into that frame through the plain C exports of
`gpu/src/display.rs`, the contract both sides build against (its doc comment is the
specification). `gpu` has a backend for each (`gpu/README.md`, "The Vulkan backend",
"The Direct3D 12 backend"). What Display8 draws itself is the engine's `Clear`s, the
movie frames the engine writes into the back buffer by lock, and the last pass through
the gamma ramp. `Validate=1` asks for the Vulkan validation layer or Direct3D 12's
debug layer, where the system has one.

### Choosing a backend

`Backend=` is an order, probed at `Direct3DCreate8` and tried at the engine's first
`CreateDevice` (`stub::ensure_device`). When nothing in it starts, the bail-out follows
("The bail-out"): the game still runs, on the system's Direct3D 8, without the plugins.

| `Backend=` | Tried |
|---|---|
| `vulkan` | Vulkan, then the bail-out |
| `d3d12` | Direct3D 12, then the bail-out |
| `auto` (shipped) | Vulkan, then Direct3D 12, then the bail-out, on every system |
| a list, `d3d12,vulkan` | in that order, then the bail-out |
| `none` | the bail-out at once; also ends a list (`vulkan,none` tries Vulkan only) |

**Retired values.** `d3d9` was a backend until display 1.0.0. As a token in a list it is
skipped and logged once (`settings: Backend token d3d9 retired (Direct3D 9 removed)`); a
list left empty by that is `auto`. An unknown word is skipped with a line in the log, and
a value that names nothing is `auto`. `PixelCentres=d3d9` stays: it is a pixel-centre
convention for the plugins, not a backend.

`auto`'s order is the user's (2026-10-23), not the display plan's (phase 6 had Direct3D
12 first on Windows and none under Wine): Vulkan is the backend confirmed in game first.
Under Wine Direct3D 12 is vkd3d-proton over Vulkan, so it rarely starts where Vulkan
did not; on Windows it is the native API for a machine without a Vulkan driver. The
log names the system, told by Wine's `ntdll.dll` (`wine_get_host_version`: `Darwin` is
macOS). On macOS Vulkan is MoltenVK, which may not map memory where a 32-bit process
reaches it ("Will Vulkan work on a Mac?"); if the device or its mapped ring cannot be
made there, the next backend is what follows.

- **The engine sees one thing whatever is chosen**: the desktop's mode is offered at
  `Direct3DCreate8` from the system (`GetSystemMetrics`); the engine keeps the mode it
  was offered.
- **A backend that fails is taken down whole** before the next is tried
  (`Vk::destroy`, `Dx::destroy`, and the partial instance and surface before them): a
  Vulkan surface or swap chain left on the game's window keeps another from being made
  on it.
- **`display8v1_backend` answers only once the device is made** (`stub::made`), and none
  before: a plugin is never told a backend that a fallback then leaves. `gpu` asks
  again while the answer is none and opens its backend at the first frame, after the
  engine's `CreateDevice` has returned (`gpu/src/api.rs`). Until display 0.7.0 Vulkan
  and Direct3D 12 were said from `Direct3DCreate8` on; no plugin asked that early on the
  bench, and `gpu` could not have opened Vulkan then anyway (no device yet).
- **The log says each step**, in `Display8.log`:

      Backend=auto: on Linux: vulkan, then d3d12
      probe: vulkan can start
      probe: d3d12 can start
      stub mode on vulkan, then d3d12
      stub: trying Vulkan (1 of 2)
      stub: Vulkan could not start: <why>; falling back to Direct3D 12
      stub: trying Direct3D 12 (2 of 2)
      stub: the backend is Direct3D 12, after vulkan could not start

  and with nothing able to start, the `bail-out:` line ("The bail-out").
- **`FailBackend=` (the test bench only)** makes the named backends (`vulkan`, `d3d12`,
  a list) fail on purpose once everything is made, so that the teardown is
  exercised too, and the late bail-out when all of them are named. Honoured only in a bench clone (a game directory under
  `a2test\sessions\`) and ignored with a line in the log anywhere else; it is not in the
  shipped `Display8.ini`, and `display/bench-mode.sh` adds it to a clone's. The
  `display-fallback` scenario uses it.

### Vulkan

`Backend=vulkan` (`src/vk/`). Display8 opens `vulkan-1.dll` and
makes the instance and device (Vulkan 1.1, one queue family that draws and presents,
`VK_KHR_swapchain`; anisotropic filtering where the device has it), a swap chain on the
game's window, an offscreen `B8G8R8A8` colour image and a `D16` depth image standing for
the engine's back buffer and depth, and two frames in flight, each with a command
buffer, a fence, a descriptor pool and an upload ring. render3d and the UI draw through
`gpu`'s Vulkan backend (`gpu/README.md`, "The Vulkan backend") into the frame's command
buffer, reaching all of it through the exports of `gpu/src/display.rs` (the contract);
at the engine's `Present` Display8 draws the colour image through the gamma ramp into
the swap chain's image, submits and presents.

- **The backend is said once the device is made** ("Choosing a backend"):
  `display8v1_backend` answers Vulkan from the engine's first `CreateDevice` on. If
  Vulkan cannot start there, the next backend in `Backend=`'s order is tried.
- **The engine's targets** are in their attachment layouts between recorders, the
  mirrors in `SHADER_READ_ONLY`; whoever changes one puts it back. The depth image is
  sampled and copied too (render3d's depth resolve, the dumps).
- **`display8_vk_flush`** submits what the frame has recorded and waits, then begins the
  same command buffer again (its pool reset): a plugin's read-back (the frame dumps).
  A failed submit gives the slot a fresh signalled fence, as a failed frame does, and
  the command buffer is begun again all the same, so the frame stays open and the next
  `begin` on the slot does not wait for ever.
- **A swap chain made again** destroys the old images' views, framebuffers and
  semaphores as it takes them out of its lists, so a failure part way leaves the lists
  short, never holding a destroyed handle; `present` makes it again before it acquires
  an image while the lists do not cover every image.
- **Mirrors** are the engine textures a plugin samples (`display8_vk_mirror`), brought
  up to date when the frame is asked for (the stub's mirror table, "Stub mode").
  They can be copied from (`TRANSFER_SRC`), so a recorded frame (`gpu/README.md`,
  "Record and replay") reads back the mirrors it samples as they were.
- **The physical device** is chosen by type: discrete, then integrated, virtual, other,
  and a CPU (software) device last, among those with a queue that draws and presents;
  `Display8.log` lists every device and its rank and says which it `chose`.
  **`Device=`** takes only a device whose name has it (the test bench with
  `--vulkan-device lavapipe`: `llvmpipe`, Mesa's CPU Vulkan); **`Validate=1`** the Khronos validation layer, its
  warnings and errors in `Display8.log` (`vulkan 0x...:`); **`PixelCentres=d3d9`**
  Direct3D 9's pixel centres for the plugins (`gpu/README.md`, "Pixel centres"), for
  comparing with frames made on Direct3D 9 before display 1.0.0 pixel for pixel.

- **The pipeline cache is kept on disk**, in `%LOCALAPPDATA%\armada2-refit\Display8.pipelines`
  (`src/vk/cache.rs`), not the game directory: GOG installs into `Program Files`,
  which is not writable, and an exe with no manifest is silently redirected to
  VirtualStore there. It is read when the device is made, only if its header names this
  device and driver (vendor, device, cache UUID), and written again, through a
  temporary file and a rename, whenever it has grown (looked at every 600 frames).
- **Direct3D 12 keeps a pipeline library** the same way
  (`%LOCALAPPDATA%\armada2-refit\Display8.d3d12pipelines`, `src/dx12/cache.rs`, since
  0.15.0). Display8 makes one `ID3D12PipelineLibrary` (`ID3D12Device1`) when the device
  is made, from the file if its 64-byte header names the same file format, Display8
  version and adapter (vendor, device, subsystem, revision, the user-mode driver's
  version where DXGI says it) and the driver accepts the blob (a refusal starts empty);
  hands it to the plugins as `Dx12Device8::pipeline_library` (contract 1.2); keeps its own
  present pass in it; and writes it again, atomically, whenever its serialized size has
  grown (looked at every 120 frames). The file's bytes stay in memory for as long as the
  library lives, as D3D12 requires. A plugin stores the pipelines it makes in it and
  loads them from it by a name hashed from the whole description; with no library, or on
  any error, it creates them plainly. `gpu` logs `pipeline library: N loaded, M stored`
  (at each power of two), Display8 the file's size read and written.
- **Samplers** (`[Samplers]`, below).

On the bench every Vulkan game runs on the GPU (the one DXVK uses) with the layer on
(display 0.9.1): the game's environment gets `VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation`
(`testbench/bench/session.py`, `vulkan_env`, from the clone's `Display8.ini`) and
`display/bench-mode.sh` gives `Display8.ini` `Validate=1`, so the layer catches errors
before they can hang the GPU. `a2test ... --vulkan-device lavapipe`
(`A2TEST_VULKAN_DEVICE=lavapipe`) puts the game on lavapipe instead: `VK_DRIVER_FILES`
(and `VK_ICD_FILENAMES`) naming `/usr/share/vulkan/icd.d/lvp_icd.json` (umu passes the
bench's environment on; pressure-vessel honours both) and `Device=llvmpipe`, so even
an environment that lost the variables cannot reach the GPU; a frame takes seconds
there and its time means nothing. `--no-validate` (`A2TEST_VULKAN_VALIDATE=0`) drops
the layer, for timing runs. `Display8.log` names the device (`vulkan: chose ...`).

### Samplers

Under DXVK the game's `dxvk.conf` (the renderer layer) gave every linear sampler 16x
anisotropy and every sampler with mips a LOD bias of -0.5 (`d3d9.samplerAnisotropy`,
`d3d9.samplerLodBias`, `d3d9.clampNegativeLodBias`). Without the same on Vulkan every
hull read a level blurrier. Since 0.21.0 `Display8.ini`'s `[Samplers]` (`Anisotropy`,
`LodBias`, `ClampNegativeLodBias`; rows in `settings.toml`, no screen row, advanced)
names the values for the plugins' samplers on Vulkan and on Direct3D 12 and for the
passthrough's (`crate::dxvk_samplers` feeds all three; Display8 hands them to `gpu` as
`VkDevice8::sampler_anisotropy`, `sampler_lod_bias`). Since 1.0.0 `[Samplers]` is the
only source and `dxvk.conf` is not read: the shipped values are 16, -0.5 and 0, and
`auto` (or no key) is the same. `Display8.log` says where each came from
(`samplers: Anisotropy=16 (default)` or `(Display8.ini)`). A player's `dxvk.conf` values
carry over once, when the install brings `Display8.ini` to `Defaults=3`
("Installing"). Vulkan's cube maps are always seamless, as `d3d9.seamlessCubes = True`
made DXVK's.

### Direct3D 12

`src/dx12/`, the same stub as on Vulkan, by the same plan:

- `d3d12.dll` and `dxgi.dll` by `LoadLibraryA`; the device at feature level 11_0 on the
  default adapter, one direct queue. Every interface is called by vtable slot, declared
  by hand from Wine's IDLs (`src/dx12/raw.rs`, each structure's 32-bit size checked at
  compile time). A method that returns a structure (a descriptor heap's start) takes a
  hidden pointer after `this`, as Microsoft's compiler passes it.
- A flip-model swap chain (`FLIP_DISCARD`, two B8G8R8A8 buffers) on the game's window,
  with DXGI's Alt+Enter and window changes off; sync interval 1 unless the engine asks
  for `D3DPRESENT_INTERVAL_IMMEDIATE`.
- The engine's back buffer and depth are B8G8R8A8 and D16 resources of ours, in
  `RENDER_TARGET` and `DEPTH_WRITE` between recorders; `Clear` clears them by
  rectangles, clipped to the viewport.
- Two frames in flight, each with a command allocator and list, its fence value, an
  upload ring (upload-heap buffers mapped for good, 4 MB chunks added as a frame needs
  them) and the frees deferred to it.
- Descriptors: two shader-visible heaps, CBV/SRV/UAV (32768 a frame) and samplers (1024
  a frame, Direct3D 12's 2048 in all), set on the frame's list when it begins; the
  views of Display8's own textures live in a heap that is not shader-visible and are
  copied into a frame's range to be bound. A recorder takes its descriptors from those
  ranges and never sets heaps of its own (only one of each type can be set).
- Mirrors as on Vulkan: B8G8R8A8 textures with every level, X8R8G8B8 through a view
  whose alpha reads one, in `PIXEL_SHADER_RESOURCE | NON_PIXEL_SHADER_RESOURCE`.
- The last pass is `shaders/present.hlsl` as DXIL (`src/shaders/present_dxil.rs`), with
  a root signature of four root constants (b0) and a table of the frame and the ramp
  (t0, t1) under two static point samplers.
- A device that is removed is logged with its reason, and nothing is drawn after. A
  fence that is not reached in six waits of 5 s, or with the device removed, makes the
  device lost the same way: the slot's allocator, list and deferred frees are left as
  they are (the GPU may still be using them), no frame begins, and
  `display8v1_dx12_frame` answers null.
- **The backend is said once the device is made**, as on Vulkan: `display8v1_backend`
  answers Direct3D 12 from the engine's first `CreateDevice` on, and `gpu` opens its
  backend then (`display8_dx12` is null before).
- **`display8_dx12_flush`** closes the frame's list, submits it, waits for its fence and
  opens the same list again (its allocator reset, the heaps set again): a plugin's
  read-back (the frame dumps, render3d's nebula bake). The frame's ring, descriptors and
  deferred calls stay as they are.
- **`PixelCentres=d3d9`** and the samplers of `[Samplers]`,
  as on Vulkan: `Dx12Device8::features` (`FEATURE_D3D9_CENTRES`; anisotropy is always
  there), `sampler_anisotropy`, `sampler_lod_bias`.

On the bench a `Backend=d3d12` game runs on the GPU (vkd3d-proton has no CPU driver to
run on here); with `Validate=1` the bench loads the Khronos layer under vkd3d-proton
(`testbench/bench/session.py`, `vulkan_env`), which writes `logs/vvl.log`. Neither layer
reaches anyone there: vkd3d-proton has no Microsoft debug layer
(`D3D12GetDebugInterface` answers `DXGI_ERROR_SDK_COMPONENT_MISSING`, logged as "no
debug layer"), and its own check finds the Khronos layer's messages do not come
through the 32-bit winevulkan. vkd3d-proton keeps its pipeline cache in
`vkd3d-proton.cache` in the game directory.

**On the bench** (Proton-CachyOS), the 32-bit game gets Direct3D 12 with no setting:
the prefix's `syswow64/d3d12.dll` is vkd3d-proton and its `dxgi.dll` DXVK's, and
Proton's own launch makes both native (the bench's overrides name only `winmm`, `d3d8`
and `d3d9`). DXVK's DXGI logs that it ignores `MakeWindowAssociation`'s flags.
Measured 2026-10-23: the device comes up at 11_0 on the RX 5700, the intro movies play
through the back buffer, the gamma pass and the flip-model swap chain, and a mission
(`-nointro a2_borg01`, the plugins standing down, as they did before display 0.6.0)
presents past frame 1000 and opens the in-mission menu. With render3d and the UI
drawing on it: `gpu/README.md`, "The Direct3D 12 backend", "Measured".

## The bail-out

Display8 draws the game itself only when it can. When it cannot, the game gets the
system's own `d3d8.dll` and draws as stock, and `Render3D.asi` and `UI.asi` stand down
(`display8v1_backend` answers none, so `gpu` opens no backend). The game is not lost for
want of a Vulkan or Direct3D 12 driver; it loses the refit's look. The system's DLL is
found with `GetSystemDirectoryA` (SysWOW64 under WoW64): Windows' own, or Wine's builtin.
Display8 loads it by that full path and never a `d3d8_real.dll` (until display 1.0.0 it
loaded that and passed every call to it).

**Early**, at `Direct3DCreate8`, before the window exists: the engine gets the system's
`IDirect3D8` itself, unwrapped, and every later call goes straight to it. The reasons:

- `Backend=none`, in `RefitPlayer.ini` (`[Display8.Display8]`) or `Display8.ini`: the
  way to play with the stock renderer, or to test this path.
- An unknown build, or a `display.modes` hook that stands down ("Hooks": the stub could
  open no display on it). This is what used to forward to the old chain.
- A stale `HUD.asi`, `GridLayout.asi` or `Menus.asi` (retired; `retire/` takes them out) is loaded: each changes what the engine
  draws, which the stub drops.
- Both probes fail. A probe asks whether a backend can start without making the device.
  Vulkan (`vk::probe`): `vulkan-1.dll` loads, an instance with the surface extensions
  makes, and a physical device with a graphics queue exists, named as `Device=` says if it
  says. Direct3D 12 (`dx12::probe`): `d3d12.dll` and `dxgi.dll` load with their entry
  points, a DXGI factory makes, and `D3D12CreateDevice` at feature level 11_0, asked with
  a null out pointer, says it would succeed. A probe that fails drops its backend from the
  order, so with one failed the other is tried alone; the log has `probe: vulkan cannot
  start: <why>`. Whether the device presents on the window is the start's to find out,
  at the engine's first `CreateDevice`.

Where `display.modes` is ready the intro's mode raise ("Window and display mode") is
still installed, answered from the system's own list of modes. The shell's is not
raised after a bail-out, though the UI's menu host claims the menus from load: with no
device the host refuses every dialog, and a stock shell draws 800x600, so it keeps its
800x600 mode and fills the screen as stock does.

A second `Direct3DCreate8` never starts the stub again (starting it reinstalls the
stub's methods over the forward a late bail-out made, and begins a new stub over the
one whose device exists): after a late bail-out it gets the system's `IDirect3D8`, while
the stub runs another `IDirect3D8` of the stub's.

**Late**, at the engine's first `CreateDevice`, when every backend of the order failed
there (a swap chain the probe could not foresee, or `FailBackend=` on the bench): the
stub's `IDirect3D8` has made it too far to be taken back, so its 16 methods become a thin
forward to the system's `IDirect3D8`, and the engine gets the system's device itself.
The engine's own `CreateDevice` is the system's, with its parameters.

In both, the exports `ValidatePixelShader`, `ValidateVertexShader` and `DebugSetMute` go
to the system's `d3d8.dll`, and `Display8.log` has one line:

    bail-out: <why>; the game draws through C:\windows\syswow64\d3d8.dll; the plugins stand down

Nothing of Display8 is on the path after it: no tracer, no `[Drop]`, no `VSync=`, no
passthrough. If the system has no `d3d8.dll` to load, the log says `bail-out: cannot
load ...: no Direct3D for the game`.

To test it: `Backend=none` (`[Display8.Display8]` in `RefitPlayer.ini`, or `Display8.ini`)
runs the early path; the bench scenario `display-bail-out` checks it. `display-fallback`
(`FailBackend=vulkan,d3d12`) runs the late one. Once the plugins have stood down the game
shows stock art through the stock chain, so a picture of it is a stock picture.

## Will Vulkan work on a Mac?

`vkprobe/` is a small 32-bit program that does what the Vulkan backend does (a mapped
upload ring, dynamic uniform offsets, a texture upload, a present pass) and checks the
pixels: the question is whether winevulkan on MoltenVK maps host-visible memory where
a 32-bit process can reach it, in the new-style WoW64 that macOS Wine always is. How to
run it on a Mac, and what it showed here: `vkprobe/README.md`.

**Seen in game on a Mac (2026-10-28):** Apple M1, macOS 27.0.1, a Sikarugir wrapper on
`WS12WineSikarugir11.0_1` (Wine 11, new-style WoW64). Display8 on `Backend=vulkan` made
its device on MoltenVK (`vulkan: physical device Apple M1`, Vulkan 1.4), and the menu
scene drew with Render3D's nebulae baked at full size. On `Backend=d3d9` (since removed) the same scene
crashed there. The wrapper's D9VK, under d3d8to9, emulates the nebulae's RESZ depth
resolve, and Metal aborts on a pipeline with `MTLPixelFormatDepth16Unorm` as colour
target 0 (issue #19). That is why Direct3D 9 was only the fallback on a Mac, and why the
fallback is now the bail-out ("The bail-out"), where the plugins draw nothing.
