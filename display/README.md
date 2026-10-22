# Display8 — the game's `d3d8.dll`, ours

`Armada2.exe` imports `Direct3DCreate8` from `d3d8.dll` and does all its drawing through
the Direct3D 8 device it gets back. Display8 is a `d3d8.dll` of our own, in `no_std`
Rust like render3d and the UI. The aim (the plan in the user's `plans/`) is that the
engine ends up drawing nothing: Display8 gives it a device that accepts everything, and
`Render3D.asi` and `UI.asi` draw the whole frame on Vulkan or Direct3D 12 directly.

    Armada2.exe -> d3d8.dll (Display8) -> d3d8_real.dll (crosire's d3d8to9) -> d3d9.dll (DXVK; Windows' own)

## Status

**Forward mode, bench only.** Display8 stands in front of the real chain and passes
every call through, tracing it. `display/install` puts it into an `a2test` clone and
refuses anything else; `./install` and `a2mod` do not know it yet.

    ./a2test session start --res 1280x720 --install . --install display

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

## Naming a draw

- **`[Drop] Callers=`** (`src/drop.rs`): a draw whose call chain has one of these return
  addresses is accepted and not passed on. Read again every 60 presents, so one session
  can shoot a frame with and without it.
- **`Display8.go`** (`src/detail.rs`): while it exists beside the exe (looked for every
  30 presents, then deleted), the next `[Trace] Frames=` frames (default 3) log a `DRAW`
  line for each draw from `Armada2.exe`: the blend and depth state, the texture, and for
  vertices that can be read, their box on the screen and their colours. The engine's 2D
  goes through one dynamic vertex buffer it writes by lock (its trifan buffer), so
  during a capture each `Unlock` copies what was written. This names a draw without a
  picture: the fade from black is a full-screen black quad whose alpha falls frame by
  frame, the drag box four white lines.
