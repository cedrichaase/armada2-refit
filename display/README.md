# Display8 — the game's `d3d8.dll`, ours

`Armada2.exe` imports `Direct3DCreate8` from `d3d8.dll` and does all its drawing through
the Direct3D 8 device it gets back. Display8 is a `d3d8.dll` of our own, in `no_std`
Rust like render3d and the UI. The aim (the plan in the user's `plans/`) is that the
engine ends up drawing nothing: Display8 gives it a device that accepts everything, and
`Render3D.asi` and `UI.asi` draw the whole frame on Vulkan or Direct3D 12 directly.

    Mode=forward  Armada2.exe -> d3d8.dll (Display8) -> d3d8_real.dll (crosire's d3d8to9) -> d3d9.dll (DXVK; Windows' own)
    Mode=stub     Armada2.exe -> d3d8.dll (Display8, draws nothing) -> Display8's own device: d3d9.dll, vulkan-1.dll or d3d12.dll

## Status

**Two modes, the stub by default (since display 0.8.0).** `./install` puts Display8 in
(`display/install`), `a2mod` switches it as the `display` layer, and
`platform/d3d8-chain.py` switches the chain behind it. In **stub mode** (`Mode=stub`,
shipped; "Stub mode" below) the real chain is never loaded, and render3d and the UI
draw the whole frame on a device of Display8's own: the first that starts of Vulkan,
Direct3D 12 and Direct3D 9 (`Backend=auto`, shipped; "Choosing a backend"), or the one
`Backend=` names ("Backends"). The user confirmed Vulkan in game on 2026-10-23 and made
this the default. In **forward mode** (`Mode=forward`, the default until display 0.7.0)
it stands in front of the real chain and passes every call through; on the test bench
it traces every call (`Trace=auto`: on in an `a2test` clone). A `Display8.ini` with
neither key falls back to forward and d3d9 in the code; the shipped file names both.
The release package carries it too, from 13.35.0 ("Installing", below).

    ./a2test session start --res 1280x720 --install .

## Installing

`display/install` (and `--remove`):

| File | What |
|---|---|
| `d3d8.dll` | Display8 |
| `d3d8_real.dll` | the `d3d8.dll` that was there: crosire's d3d8to9 after `./install`'s platform step. Display8 loads it (`Real=auto`) and passes every call to it |
| `d3d8.dll.a2display-backup` | the same file again: what `a2mod stock` copies back into `d3d8.dll`. Stock is the stock chain's binary, not ours passing through |
| `Display8.ini` | the settings; a key the player's copy lacks is added (`tools/ini-merge.py`), and a value that still holds an older shipped default is moved to the new one (below) |

**When the shipped defaults change**, an install must not keep the old ones by
accident nor undo a player's choice. `Defaults=` numbers the set a file holds (as
`UI.ini`'s does), and `tools/ini-merge.py` brings a copy with an older number (or none: 1)
up to the new one: each key the newer set changed moves to its new value only where it
still holds the old default. Defaults 2 (display 0.8.0) is `Mode=stub`,
`Backend=auto`, where 1 was `forward` and `d3d9`: a game installed with display 0.7.0 or
earlier comes to the stub on `auto`, and a game whose `Backend=vulkan` (or any value
other than d3d9) was set by hand keeps it, its `Mode=forward` moved to stub. A player
who had chosen `forward` or `d3d9` before their default changed is moved too; the merge
cannot tell that choice from the old default, and prints each key it moves.
Since 0.12.0 the code's defaults are the shipped file's (`Mode=stub`, `Backend=auto`,
`Passthrough=1`), so a `Display8.ini` without those keys, or none at all, runs as
shipped.

With no `d3d8.dll` in the game directory (the system's is the chain), Display8 goes in
alone and loads the system's own (`GetSystemDirectory`: SysWOW64 on 64-bit Windows,
Wine's builtin under Wine). Run again, the installer replaces only `d3d8.dll`.
`d3d8-chain.py` knows Display8 by the panic message every build carries, not by a hash
(ours changes with every build); behind it, the chain's slot is `d3d8_real.dll`, and
every switch there rewrites the backup with it, so `a2mod stock` restores the chain in
use.

Wine loads the game directory's `d3d8.dll` because of the `d3d8=n,b` override
(`platform/README.md`); `d3d8_real.dll` is loaded by full path, so it needs none.

**The release package** (`publish/README.md`, "Release packages") carries Display8 in
`display/` with the shipped `Display8.ini`, and its installers (`install.sh`,
`install.ps1`) put it in the same way, with two differences. They put it only in front
of crosire's d3d8to9 after their chain step (a chain left as the player's gets none),
and they make no `d3d8.dll.a2display-backup`, since there is no `a2mod` in a release.
They merge `Display8.ini` without Python, in awk and in PowerShell, by `ini-merge.py`'s
rules and upgrade table. On macOS too `Display8.ini` goes in as shipped, on
`Backend=auto`: Vulkan on MoltenVK under a 32-bit process works there ("Will Vulkan work
on a Mac?"), and Direct3D 9 (Wine's wined3d, or a wrapper's D9VK) comes last if it
cannot start.
Nothing in Display8 is Wine-specific (`platform()` only names the system in the log), so
on Windows the stub runs on the GPU vendor's Vulkan, Direct3D 12 or Windows' own
Direct3D 9; it has not been seen there yet. `--uninstall` puts d3d8to9 back as `d3d8.dll`
and takes out what `display/install --remove` takes out (on Windows also
`Display8.pipelines`).

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
| `display8v1_backend()` | what the plugins draw on: Direct3D 9, Vulkan, Direct3D 12 or none (`gpu/src/display.rs`, `BACKEND_*`) |
| `display8v1_vk*`, `display8v1_dx12*`, `display8v1_engine_viewport`, `display8v1_engine_transform` | stub mode on Vulkan and Direct3D 12: the device, the frame, the upload ring, deferred frees, the engine's targets and mirrors, a flush, and the engine's state the plugins read. The contract is `gpu/src/display.rs` (its doc comment is the specification); `src/vk/exports.rs` and `src/dx12/exports.rs` implement it |

The first five are the forward mode's (`src/api.rs`), unversioned: plain scalars, the
same in every release. render3d (`frame::begin`) and the
UI (`gfx::attach`) tell a new device or a `Reset` by `display8_device_epoch`
(`gpu::Gpu::device_epoch`, resolved once); on a `d3d8.dll` without it they compare the
Direct3D 9 device behind the engine's (`gpu::Gpu::engine_id`) as before, until the
Direct3D 9 backend goes (the display plan's last phase).

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
| no Display8 (another chain's `d3d8.dll`: crosire's d3d8to9, DXVK's, Wine's) | Direct3D 9, as ever | `gpu ...: no Display8 (the game's d3d8.dll is another chain's): Direct3D 9` |
| plugins newer than Display8 (a Display8 before 0.12.0) | `gpu` finds unversioned exports and no `display8_abi`: it stands down to Direct3D 9 only, which on Display8's own Vulkan or Direct3D 12 draws nothing | `gpu ...: Display8 0.11.0 is too old for this plugin (no contract handshake ...)` |
| plugins older than Display8 (built before 0.12.0) | they look for `display8_backend` and find nothing: Direct3D 9 only, nothing drawn on Vulkan or Direct3D 12 | `Display8.log`, in stub mode 300 frames into the first mission (such a plugin calls nothing of Display8's, so it is told by not having asked): `contract: Render3D.asi is loaded and has not asked for the contract ...` |
| another major either way | as above, both sides | `gpu ...: Display8 ... has contract 2.0, another major ...`; `Display8.log`: `... another major: it finds none of this Display8's exports and stands down` |

On Vulkan the engine's own draws pass through where the plugins draw nothing
("Passthrough"), so the game still shows its stock look and can be played; on Direct3D
12, which passes nothing through yet, the 3D view stays empty. Display8 0.12.0,
render3d 0.31.0 and UI 0.26.0 were the last release that had to ship together; from
there a release of one may meet the others of the same major.
`testbench/scenarios/display-abi-mismatch.md` runs both directions against 13.42.4
(`bench-abi.sh`).

## How it works

**An object of ours for every interface.** `IDirect3D8`, the device, the swap chain,
the three kinds of texture, surfaces, volumes, vertex and index buffers: each is a
vtable of ours (`src/d3d8/`) and the real object it stands for (`src/com.rs`). Every
method sits in its slot with its exact `__stdcall` argument list, from Wine's
`include/d3d8.h`; one argument too many or too few unbalances the caller's stack. A
method that takes an object swaps ours for the real one (`SetTexture`,
`SetRenderTarget`, `CopyRects`, ...), and one that hands an object back wraps it
(`CreateTexture`, `GetBackBuffer`, `GetSurfaceLevel`, `GetContainer`, ...).

**One object of ours per real object.** A registry maps each real pointer to ours, so
`GetTexture` hands back the very pointer the engine set. The reference count is the real
object's: `AddRef` and `Release` pass through, and ours is freed when the real one
reports 0. In Direct3D 9 a texture's surface shares its texture's count, so a surface
of ours may outlive its real one; a later real object at the same address and of the
same kind takes it over, which is sound because ours holds nothing but the pointer.

**`QueryInterface` for anything that is not Direct3D 8's goes to the real object.**
That is how the plugins reach Direct3D 9 through the engine's objects
(`IID_IDirect3DDevice9` on the device; `IID_IDirect3DTexture9` on the minimap's and the
fog's textures), as they did on d3d8to9 itself (`platform/D3D9.md`).

**The plugins chain slots of our device's vtable** (`Reset`, `Present`: `ui/src/engine/device.rs`,
`render3d/src/frame/mod.rs`) with `VirtualProtect` and a write, as they did on
d3d8to9's. The vtables are therefore mutable statics, filled at start-up, and nothing in
Display8 calls its own methods through them.

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

With `Trace=1` in `Display8.ini`:

- **`Display8.inventory.tsv`**, beside the exe, rewritten every two seconds of
  presenting and at each new context: a row per context, interface, method and callers,
  with the calls, the frames they fell in, and how many came from a thread other than
  the device's. `#` lines at the end give each context's frames, engine draws and time
  spent in the real `Present`.
- **`Display8.log`**: each distinct line once per context: the answers the engine
  branches on (`ANSWER`: caps, formats, modes, `ValidateDevice`), the present
  parameters of every `CreateDevice` and `Reset`, gamma ramps (`GAMMA`), every lock
  (`LOCK`) and read-back (`READBACK`), render targets, the plugins' queries for
  Direct3D 9 (`QI`), and the `user32`/`gdi32` calls that read the mode or move the mouse
  (`MODE`, `MOUSE`, `WINDOW`: `src/user32.rs`, hooked in the exe's import table).
- **Contexts** are labels the bench sets: `Mark the display context "mission"` writes
  `Display8.mark`, read at most four times a second at `Present` and at every
  `CreateDevice` and `Reset`, then deleted. Before the first mark the context is
  `start`.
- **`Expect the engine to draw nothing in "mission"`** reads the inventory: no `Draw*`
  on the device whose caller is in `Armada2.exe`, in that context.

## Drop switches

`[Drop]` in `Display8.ini` (`src/drop.rs`): the engine's draws, accepted and not passed
on. Read again every 60 presents, so one session can shoot a frame with and without.

| Key | Drops |
|---|---|
| `Drop2D=1` | the engine's 2D: its batched quads (the trifan buffer), its lines, the fades |
| `Drop3D=1` | the meshes it still draws (render3d's hand-backs): every draw whose immediate caller is a workspace's `Submit` (`ST3D_WorkspaceDirectX8::Submit` 0x648af0..0x648c30, `ST3D_WorkspaceDirectX8NonVB::Submit` 0x648520..0x6485d0), which every mesh the engine draws goes through |
| `DropLoading=1` | everything drawn from the engine's loading loop: any draw with a return address in `RenderLoopAssetLoadStatusUpdate` (0x598ba0..0x598ff0) among the sixteen found up its stack, which is the function the loop calls to draw each step's frame (the logo scene, the load bar) |
| `Callers=0x..., 0x...` | any draw with one of these return addresses on its call chain |

A draw is the engine's when its immediate caller is in `Armada2.exe`; the plugins draw
through Direct3D 9 and never pass here. With the refit installed and every category's
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

`Mode=stub` (`src/stub/`): the real chain is never loaded. The engine gets an
`IDirect3D8` and devices of ours that accept everything and draw nothing, and Display8
makes a Direct3D 9 device of its own (`d3d9.dll` as the loader finds it: DXVK's beside
the exe under Proton, Windows' own) on the game's window. render3d and the UI find it
exactly as they found d3d8to9's, through `QueryInterface(IID_IDirect3DDevice9)` on the
engine's device, and draw the whole frame on it.

- **One Direct3D 9 device for the game's life**, made at the engine's first
  `CreateDevice`. Its later `CreateDevice`s and `Reset`s get a new device object of
  ours on the same Direct3D 9 device, or nothing at all.
- **One display mode: the desktop's.** The engine is offered only that one (X8R8G8B8,
  the adapter's refresh), so it draws at the size the swap chain has and no mouse
  coordinate needs translating (phase 0 found the engine reads the mouse only through
  `GetCursorPos`, `ScreenToClient` and `GetClientRect`). The game's window becomes a
  borderless one covering the screen, and the swap chain is windowed on it.
- **What the plugins read of the engine's state reaches the Direct3D 9 device**: the
  transforms (render3d reads the view and the projection), the viewport, `Clear` of the
  engine's own back buffer and depth, `BeginScene` and `EndScene`. Everything else is
  kept for the engine's own `Get*` and goes nowhere.
- **Resources are memory.** A texture's levels, a surface's pixels and a buffer's bytes
  live in Display8, and a lock hands out a pointer into them, as the managed pool's
  system copy did. Draws are counted (the inventory), and dropped, or drawn by the
  passthrough on Vulkan ("Passthrough").
- **The mirror table.** A texture a plugin asks for as Direct3D 9 (the radar's two
  scan-grid sprites, phase 0's only ones; the unit view's Bink frame) is made there
  once with every level and brought up to date when the engine unlocks a level, at
  once on the device's thread, else at the next `BeginScene` or `Present`
  (`BinkThread` writes from its own).
- **The back buffer's pixels** (the movie path locks it and writes each frame) go into
  the frame when it is unlocked, as on a real device, so what is drawn after lands on
  top: the UI draws each movie frame at the `Present` that follows, and the cutscene
  proxy then writes nothing into the locked buffer. Copied at `Present` instead (until
  display 0.6.1), the untouched buffer covered the UI's frame, and every movie played
  black with its sound, on every backend. An unlock off the device's thread waits for
  `Present`. `display-movie` checks it.
- **The gamma ramp in the last pass** (`src/stub/gamma.rs`, `shaders/present.hlsl`): a
  windowed swap chain ignores `SetGammaRamp`, and on the real chain the ramp the engine
  sets at start-up lifts every frame (DXVK's presenter applies it: 2 to 3, 80 to 90).
  So the back buffer is copied and drawn back through a 256x1 lookup. Measured on the
  showcase frame against the real chain's screen: the frames the plugins render are the
  same to the last pixel (RMSE 0: scene, bloom levels, presented frame), and the screen
  differs by at most 1/255 (RMSE 0.28): DXVK's mapping of a ramp value to 8 bits is none
  of rounding (64 values of 256 per channel off by one), truncating ramp/257 (142),
  interpolating half a texel back (13) or the high byte (13, what is used).
- **Stands down** to forward mode when `HUD.asi`, `GridLayout.asi` or `Menus.asi` is
  loaded (each changes what the engine draws, which the stub drops; `GridLayout.asi`
  would leave no button bar at all, since the UI's stands down beside it), or, on
  `Backend=d3d9`, when Direct3D 9 cannot be made. The log says which. A Vulkan or
  Direct3D 12 device is made at the engine's first `CreateDevice`, after the choice
  between the modes is past: if it cannot be made, the next backend is tried there,
  Direct3D 9 last ("Choosing a backend"), and that `CreateDevice` fails only when none
  starts.

`display/bench-mode.sh stub` sets the mode in a bench clone from a scenario's `Setup:`;
`display-stub` walks the whole game on it.

## Passthrough

`Passthrough=1` in `Display8.ini` (shipped, since the GPU runs below passed, and the
code's default too since 0.12.0; `0` drops every engine draw, as before): in stub mode on Vulkan the engine's own Direct3D 8 draws are
drawn after all, on Display8's device, the way Direct3D 8's fixed function draws them
(`src/passthrough.rs`, `src/vk/pass.rs`, `shaders/ff.hlsl`). It is ROADMAP phase 4's
first slice (4a, 4b, most of 4c, 4d, 4e and 4f on Vulkan). Without it, whatever render3d
or the UI does not draw itself vanishes in stub mode: a module switched off
(`[Modules] hulls=0`), a SOD render3d hands back (one that does not load, a hologram's
class that is not a plain database), an effect class it does not know, specials with
no particle art, construction and the bees without the hulls module, a HUD part the UI
has not taken. With it, each shows as stock, as on the forward chain.

**Which draws.** Every draw on the stub's device is the engine's (the plugins draw
through `gpu`), so routing is simple: every one is drawn, except

- what `[Drop]` names (the same switches as in forward mode, so a category can still be
  A/B'd: `Drop3D=1` drops the hand-backs again);
- the frames of the engine's loading loop while `UI.asi` is loaded, whose loading screen
  draws over them: a `BeginScene` with `RenderLoopAssetLoadStatusUpdate` up its stack
  (`drop::in_loading`) marks its frame. Without the UI the engine's loading screen is
  the only one and is drawn;
- a draw into a render target of the engine's own other than its back buffer (none seen);
- a draw through a vertex shader other than the one it has a port of (below), logged
  once;
- a draw made while a plugin's `Cmd` is recording (a plugin calling the engine with a
  `Cmd` alive): logged once with its caller, never recorded into another recorder's
  render pass.

When claims arrive (ROADMAP phase 3 and 4g), they become the routing in place of "every
one".

**Where it lands.** In the frame's command buffer, at the moment the engine makes it,
between the plugins' recorders: the same order as on Direct3D 9, where a plugin's targets
stay bound after its `Cmd` and the engine's draws land in them. On Vulkan a plugin's
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
| samplers | per stage: address, filters, mip filter, `MAXANISOTROPY`, `MIPMAPLODBIAS`, `MAXMIPLEVEL`, made as DXVK makes them under the game's `dxvk.conf` (the forced anisotropy and LOD bias), as the forward chain draws them |
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
device. A texture of another format than A8R8G8B8 and X8R8G8B8 (none seen), a cube or a
volume, is drawn with white and logged once.

**The engine's one vertex shader** (ROADMAP 4d) is the Borg hulls' bump pass. Display8
keeps every vertex shader's declaration and function tokens at `CreateVertexShader`, the
vertex shader constants at `SetVertexShaderConstant` and streams 1 to 3, and knows the
shader by an FNV-1a hash of its function tokens (`passthrough::BORG_BUMP`); where its
declaration puts each input in stream 0 is read off the declaration. Its port by hand is
`vs_bump` in `shaders/ff.hlsl`: the position through the four rows the engine sets in
constants 2 to 5, and the light direction of constant 6 taken into the vertex's tangent
frame, normalised, offset and scaled by constants 0 and 1 into the diffuse colour, which
the texture stages dot with the normal map (`DOTPRODUCT3`). The engine draws a Borg hull
in three passes this way (two lights added, then the base texture multiplied over them),
all of which the stage interpreter runs. Any other shader is refused and logged once with
its hash; on the test bench its tokens are logged too, for porting.

**What is not done yet** (ROADMAP phase 4): claims as
the routing (4g, after phase 3); Direct3D 12 and the stub's Direct3D 9 (phase 10; the
latter is to go). Specular lighting, spot cones, texture transforms, a third stage and
`ZBIAS` are not drawn, each logged once when the engine uses it; none was seen in the
stock traces.

`display-passthrough` checks it (render3d's hulls off on the stub; the ships drawn),
against `display-passthrough-forward`, the same scene through the forward chain.

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
display's refresh rate: Vulkan presents FIFO, Direct3D 12 at sync interval 1, Direct3D 9
and the real chain at interval one. (The two `Sleep`s in the engine's own loop are not
a limit: `Program::GameDoOne` sleeps once, and `Transport::DoGameCycleDelay` sleeps only
for a delay that is 0 in an ordinary game.)

**`VSync=0`** in `Display8.ini` gives the device `D3DPRESENT_INTERVAL_IMMEDIATE`
instead, at the engine's `CreateDevice` (and, forward, at its `Reset`), so frames are
presented as fast as they are drawn: on Vulkan the surface's mailbox mode, else
immediate (tearing); on Direct3D 12 sync interval 0; on Direct3D 9 and the real chain
(DXVK) the immediate interval. `Display8.log` says so at start, and the swap chain's
line names the present mode (Vulkan's 1 is mailbox, 0 immediate, 2 FIFO). A windowed
device of the real chain keeps the engine's interval, since Direct3D 8 takes only the
default there; the game runs full screen. The shipped value is `VSync=1`.

## Backends

`Backend=` in `Display8.ini` says what the stub draws on: `d3d9` (above, the default),
`vulkan` (`src/vk/`), `d3d12` (`src/dx12/`), or `auto` (below). On `vulkan` and `d3d12` no `d3d9.dll` is loaded
at all: Display8 owns the device, the swap chain and the frame, and render3d's and the
UI's `gpu` record into that frame through the plain C exports of
`gpu/src/display.rs`, the contract both sides build against (its doc comment is the
specification). `gpu` has a backend for each (`gpu/README.md`, "The Vulkan backend",
"The Direct3D 12 backend"), so on `vulkan` and `d3d12` render3d and the UI draw the
whole frame as on `d3d9`. What Display8 draws itself is the engine's `Clear`s, the
movie frames the engine writes into the back buffer by lock, and the last pass through
the gamma ramp. `Validate=1` asks for the Vulkan validation layer or Direct3D 12's
debug layer, where the system has one.

### Choosing a backend

`Backend=` is an order, tried at the engine's first `CreateDevice`
(`stub::ensure_device`), with **Direct3D 9 always last**, named or not: a backend that
cannot start never fails the game while d3d9 lives.

| `Backend=` | Tried |
|---|---|
| `d3d9` | Direct3D 9, opened at `Direct3DCreate8` as before |
| `vulkan` | Vulkan, then Direct3D 9 |
| `d3d12` | Direct3D 12, then Direct3D 9 |
| `auto` (shipped) | Vulkan, then Direct3D 12, then Direct3D 9, on every system |
| a list, `d3d12,vulkan` | in that order, then Direct3D 9 |

`auto`'s order is the user's (2026-10-23), not the display plan's (phase 6 had Direct3D
12 first on Windows and none under Wine): Vulkan is the backend confirmed in game first.
Under Wine Direct3D 12 is vkd3d-proton over Vulkan, so it rarely starts where Vulkan
did not; on Windows it is the native API for a machine without a Vulkan driver. The
log names the system, told by Wine's `ntdll.dll` (`wine_get_host_version`: `Darwin` is
macOS). On macOS Vulkan is MoltenVK, which may not map memory where a 32-bit process
reaches it ("Will Vulkan work on a Mac?"); if the device or its mapped ring cannot be
made there, the next backend is what follows.

- **The engine sees one thing whatever is chosen**: the desktop's mode is offered at
  `Direct3DCreate8` from the system (`GetSystemMetrics`) when the order does not start
  with Direct3D 9, and a Direct3D 9 that comes last is opened only when its turn comes
  (no `d3d9.dll` is loaded on Vulkan or Direct3D 12); the engine keeps the mode it was
  offered.
- **A backend that fails is taken down whole** before the next is tried
  (`Vk::destroy`, `Dx::destroy`, and the partial instance and surface before them): a
  Vulkan surface or swap chain left on the game's window keeps another from being made
  on it.
- **`display8_backend` answers only once the device is made** (`stub::made`), and none
  before: a plugin is never told a backend that a fallback then leaves. `gpu` asks
  again while the answer is none and opens its backend at the first frame, after the
  engine's `CreateDevice` has returned (`gpu/src/api.rs`). Until display 0.7.0 Vulkan
  and Direct3D 12 were said from `Direct3DCreate8` on; no plugin asked that early on the
  bench, and `gpu` could not have opened Vulkan then anyway (no device yet).
- **The log says each step**, in `Display8.log`:

      Backend=auto: on Linux: vulkan, then d3d12, then d3d9
      stub mode on vulkan, then d3d12, then d3d9
      stub: trying Vulkan (1 of 3)
      stub: Vulkan could not start: <why>; falling back to Direct3D 12
      stub: trying Direct3D 12 (2 of 3)
      stub: the backend is Direct3D 12, after vulkan could not start

  and with nothing able to start, `stub: no backend could start (...)`, and the engine's
  `CreateDevice` fails as it did before.
- **`FailBackend=` (the test bench only)** makes the named backends (`vulkan`, `d3d12`,
  `d3d9`, a list) fail on purpose once everything is made, so that the teardown is
  exercised too. Honoured only in a bench clone (a game directory under
  `a2test\sessions\`) and ignored with a line in the log anywhere else; it is not in the
  shipped `Display8.ini`, and `display/bench-mode.sh` adds it to a clone's. The
  `display-fallback` scenario uses it.

### Vulkan

`Backend=vulkan` (`src/vk/`): no Direct3D 9 at all. Display8 opens `vulkan-1.dll` and
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
  `display8_backend` answers Vulkan from the engine's first `CreateDevice` on. If
  Vulkan cannot start there, the next backend in `Backend=`'s order is tried.
- **The engine's targets** are in their attachment layouts between recorders, the
  mirrors in `SHADER_READ_ONLY`; whoever changes one puts it back. The depth image is
  sampled and copied too (render3d's depth resolve, the dumps).
- **`display8_vk_flush`** submits what the frame has recorded and waits, then begins the
  same command buffer again (its pool reset): a plugin's read-back (the frame dumps).
- **Mirrors** are the engine textures a plugin samples (`display8_vk_mirror`), as the
  Direct3D 9 mirror table has them, brought up to date when the frame is asked for.
  They can be copied from (`TRANSFER_SRC`), so a recorded frame (`gpu/README.md`,
  "Record and replay") reads back the mirrors it samples as they were.
- **The physical device** is chosen by type: discrete, then integrated, virtual, other,
  and a CPU (software) device last, among those with a queue that draws and presents;
  `Display8.log` lists every device and its rank and says which it `chose`.
  **`Device=`** takes only a device whose name has it (the test bench with
  `--vulkan-device lavapipe`: `llvmpipe`, Mesa's CPU Vulkan); **`Validate=1`** the Khronos validation layer, its
  warnings and errors in `Display8.log` (`vulkan 0x...:`); **`PixelCentres=d3d9`**
  Direct3D 9's pixel centres for the plugins (`gpu/README.md`, "Pixel centres"), for
  comparing with `Backend=d3d9` pixel for pixel.

- **The pipeline cache is kept on disk**, in `%LOCALAPPDATA%\armada2-refit\Display8.pipelines`
  (`src/vk/cache.rs`), not the game directory: GOG installs into `Program Files`,
  which is not writable, and an exe with no manifest is silently redirected to
  VirtualStore there. It is read when the device is made, only if its header names this
  device and driver (vendor, device, cache UUID), and written again, through a
  temporary file and a rename, whenever it has grown (looked at every 600 frames).
- **Samplers as DXVK makes them.** On Direct3D 9 the game's `dxvk.conf` (the renderer
  layer, `renderer/README.md`) has DXVK give every linear sampler 16x anisotropy and
  every sampler with mips a LOD bias of -0.5. Display8 reads the same keys
  (`d3d9.samplerAnisotropy`, `d3d9.samplerLodBias`, `d3d9.clampNegativeLodBias`) and
  hands them to `gpu` (`VkDevice8::sampler_anisotropy`, `sampler_lod_bias`), so the two
  backends filter alike. Without it every hull read a level blurrier on Vulkan. Vulkan's cube maps are always seamless, as
  `d3d9.seamlessCubes = True` makes DXVK's.

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
- A device that is removed is logged with its reason, and nothing is drawn after.
- **The backend is said once the device is made**, as on Vulkan: `display8_backend`
  answers Direct3D 12 from the engine's first `CreateDevice` on, and `gpu` opens its
  backend then (`display8_dx12` is null before).
- **`display8_dx12_flush`** closes the frame's list, submits it, waits for its fence and
  opens the same list again (its allocator reset, the heaps set again): a plugin's
  read-back (the frame dumps, render3d's nebula bake). The frame's ring, descriptors and
  deferred calls stay as they are.
- **`PixelCentres=d3d9`** and the samplers as the game's `dxvk.conf` has DXVK make them,
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

## Will Vulkan work on a Mac?

`vkprobe/` is a small 32-bit program that does what the Vulkan backend does (a mapped
upload ring, dynamic uniform offsets, a texture upload, a present pass) and checks the
pixels: the question is whether winevulkan on MoltenVK maps host-visible memory where
a 32-bit process can reach it, in the new-style WoW64 that macOS Wine always is. How to
run it on a Mac, and what it showed here: `vkprobe/README.md`.

**Seen in game on a Mac (2026-10-28):** Apple M1, macOS 27.0.1, a Sikarugir wrapper on
`WS12WineSikarugir11.0_1` (Wine 11, new-style WoW64). Display8 on `Backend=vulkan` made
its device on MoltenVK (`vulkan: physical device Apple M1`, Vulkan 1.4), and the menu
scene drew with Render3D's nebulae baked at full size. On `Backend=d3d9` the same scene
crashed there. The wrapper's D9VK, under d3d8to9, emulates the nebulae's RESZ depth
resolve, and Metal aborts on a pipeline with `MTLPixelFormatDepth16Unorm` as colour
target 0 (issue #19). So Direct3D 9 is the fallback on a Mac, not the first choice.
