# vkprobe — does Vulkan work for Display8 here?

A small 32-bit Windows program (`vkprobe.exe`, about 150 KB, no installer, no
dependencies beyond the system's `vulkan-1.dll`) that does what Display8's Vulkan
backend does, on its own, and writes down how it went in `vkprobe.txt` beside itself.
The first line answers the question:

    VERDICT: Vulkan works for Display8
    VERDICT: no: <the first step that failed, and what it returned>

It exits 0 when it works and 1 when it does not, shows no message box, and closes its
window by itself after about five seconds, so it can run unattended.

## What it answers, and why

The display plan moves the game's drawing from Direct3D 8 to Vulkan
(`display/README.md`, "Vulkan"). On Linux that is Proton and the host's driver; on
Windows, the GPU vendor's driver. **macOS** is the open question. Mac players run the
32-bit `Armada2.exe` under Wine (CrossOver, or a Sikarugir/Kegworks/Wineskin-style
wrapper), and a Windows program there reaches Vulkan only through Wine's
`winevulkan` on MoltenVK. macOS has no 32-bit processes, so Wine there is **new-style
WoW64** only: the 32-bit game runs on a 64-bit Wine, and its Vulkan calls cross into a
64-bit MoltenVK.

That works for most of the API, but not for memory the game writes directly: a
host-visible allocation that MoltenVK maps lands somewhere in a 64-bit address space,
where a 32-bit pointer cannot reach it. winevulkan has to make the driver map it below
4 GB instead (with `VK_EXT_map_memory_placed`, or `VK_EXT_external_memory_host` on
memory it allocated itself). Display8 depends on exactly that: every frame's vertices,
indices, constants and texture uploads are written by the CPU into a mapped ring
(`display/src/vk/mod.rs`, "upload ring"). If that mapping fails, or returns a pointer
that is not really there, Vulkan gives macOS players nothing, and Direct3D 9
(DXVK, or CrossOver's own) stays their path.

So the probe does that, and checks the pixels it gets:

1. opens a 640x480 window, loads `vulkan-1.dll` with `LoadLibraryA` and creates a
   Vulkan 1.1 instance with `VK_KHR_surface` and `VK_KHR_win32_surface`, plus
   `VK_KHR_portability_enumeration` and its flag when the instance offers it (a loader
   lists MoltenVK, a portability driver, only to an instance that asks);
2. lists every physical device (name, Vulkan and driver version, vendor, queue
   families) and picks one with a queue family that draws and presents on the window;
   creates the device with `VK_KHR_swapchain`, and `VK_KHR_portability_subset` when it
   is offered (and then reports what the subset leaves out, triangle fans first);
3. creates a FIFO swap chain on the window, B8G8R8A8_UNORM when offered;
4. **the crux**: allocates an 8 MB `HOST_VISIBLE | HOST_COHERENT` buffer (vertex,
   index, uniform and transfer-source usage), maps it, checks the pointer holds 8 MB
   below 4 GB, asks `VirtualQuery` what Windows thinks is there, and writes and reads
   back every word of it;
5. draws 300 frames, two in flight, out of that ring as Display8 would: each frame
   writes 64 KB of filler, a 64x64 texture, a uniform block (at
   `minUniformBufferOffsetAlignment`, bound as `UNIFORM_BUFFER_DYNAMIC`), a triangle's
   vertices and a quad's vertices and 16-bit indices into the ring, wrapping round it;
   uploads the texture into a device-local image with `vkCmdCopyBufferToImage`; draws
   a checker quad sampled from it and the triangle into an offscreen 640x480 target;
   and draws that target into the swap chain image (Display8's present pass);
6. copies the last frame's target into a second mapped buffer and checks seven pixels
   against what the shaders must produce: the clear colour, both checker colours in
   the order of the **last** frame's upload, and the triangle in the **last** frame's
   uniform tint (so a uniform block or an upload read from the wrong place in the ring
   shows up as a wrong pixel, not as "it ran");
7. tears everything down, and reports the mean frame time.

`vkprobe.txt` also holds the instance and device extensions, the limits that matter
(`minUniformBufferOffsetAlignment`, `maxMemoryAllocationCount`, `nonCoherentAtomSize`
and others), the memory heaps and types, the surface's formats and present modes, the
mapped addresses, Wine's version and host (`wine_get_version`,
`wine_get_host_version`), what `IsWow64Process2` says, and every message the
validation layer sent, when it is on. The file is rewritten after every step, so a
probe that crashes or hangs leaves `VERDICT: no: stopped during <step>`.

The exe is **not large-address-aware**, like `Armada2.exe`: it has the game's 2 GB of
address space, so a mapping has to fit where the game's would.

## On a Mac

**CrossOver**: in the bottle the game is in (or any 32-bit-capable bottle), *Open C:
Drive*, drop `vkprobe.exe` into it, then *Run Command…*, *Browse…*, pick
`vkprobe.exe` and *Run*. A window opens for about five seconds: a dark blue field, a
yellow and blue checkerboard bottom left, a triangle top right whose colour flickers.
When it has closed, `vkprobe.txt` is beside `vkprobe.exe` in the C: drive.

**Sikarugir, Kegworks, Wineskin-style wrappers**: put `vkprobe.exe` into the wrapper's
`drive_c` (*Show Package Contents* on the wrapper, then the prefix's `drive_c`), and run
it with the wrapper's *Run .exe* / *Install Software → Choose Executable*; or from
Terminal, with the wrapper's `wine` and `WINEPREFIX` set to its prefix.

**What to send back**: `vkprobe.txt`, whole, and the Mac's model (Apple silicon or
Intel) and macOS version. If there is no `vkprobe.txt` at all, say so, and what the
window did.

The probe touches nothing outside its own window and file: it installs nothing, writes
no setting, and leaves the bottle as it was.

## Building

    display/vkprobe/build.sh      # -> display/vkprobe/build/vkprobe.exe

Like Display8 (`display/build.sh`): rustup's `i686-pc-windows-msvc` target, `lld-link`,
no C runtime (`crates/a2rt/src/mem.rs` and `win.rs` are shared, not copied), kernel32
and user32 through import libraries `build.rs` makes with `llvm-dlltool`, and a
version resource. The shaders are `shaders/probe.hlsl`, compiled by
`tools/shaderc/hlsl-rs.sh` (dxc to SPIR-V, validated by `spirv-val`; vkd3d's Direct3D 9
bytecode is made too, and unused) into `src/shaders/`, committed with their stamps.

## Running it here

    display/vkprobe/run-wine.sh [--validate] [--wow64] [--keep] [<work dir>]

runs it under the project's Proton Wine (`a2env.sh`: `A2_PROTON`) on **lavapipe**,
Mesa's CPU Vulkan, so the GPU is never touched, and prints `vkprobe.txt`:

- **The display** is a headless sway of its own, with Xwayland forced on (the test
  bench's display, `testbench/bench/session.py`), but rendered with
  `WLR_RENDERER=pixman` and given no render node, so the compositor does not touch the
  GPU either. Wine's X11 driver opens the window on that Xwayland.
- **Vulkan** is `VK_ICD_FILENAMES` (and `VK_DRIVER_FILES`) pointing at
  `/usr/share/vulkan/icd.d/lvp_icd.json` only, with `VK_LOADER_DEBUG=error`. In
  old-style WoW64 the 32-bit process loads the host's 32-bit loader and lavapipe
  (Arch: `lib32-vulkan-icd-loader`, `lib32-vulkan-swrast`); in new-style, the 64-bit
  ones.
- **`--validate`** sets `VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation`. The host's
  loader takes it from the environment (Wine's own `vulkan-1.dll` lists no layers),
  inserts the layer of the right bitness (`lib32-vulkan-validation-layers` for
  old-style), and its messages reach the probe through `VK_EXT_debug_utils`, which
  winevulkan passes through: the probe counts them, and any error fails the run.
  (Checked by binding a misaligned dynamic offset on purpose: ten errors reported,
  the triangle's pixel wrong, verdict no.)
- **`--wow64`** makes a second prefix with `WINEARCH=wow64`. Proton's Wine is built
  with both 32-bit and 64-bit Unix sides, and runs 32-bit programs in old-style WoW64
  by default; `WINEARCH=wow64` on a new prefix runs them as macOS Wine must, a 32-bit
  PE side over the 64-bit host driver, with winevulkan placing every mapping below
  4 GB. This is the nearest Linux gets to the Mac's case without a Mac.
- A fresh prefix takes half a minute to make; `<work dir>` with `--keep` keeps it for
  the next run.

## The result here (2026-10-23)

Proton-CachyOS 11.0 (wine-11.0), lavapipe (Mesa 26.2.5, LLVM 23.1.3), all four runs:
**`VERDICT: Vulkan works for Display8`**, every pixel right, 0 validation errors and 0
warnings with the layer on, in both WoW64 modes. Mean frame time 1.6–1.9 ms (lavapipe,
640x480; FIFO is not throttled on this headless output).

| | old-style WoW64 (default) | new-style WoW64 (`WINEARCH=wow64`) |
|---|---|---|
| Vulkan below winevulkan | 32-bit loader and lavapipe (`/usr/lib32`) | 64-bit loader and lavapipe (`/usr/lib`) |
| the ring's address | `0xca0b1000`–`0xd06b1000`, run to run | `0x01570000` |
| `VirtualQuery` there | reserved, `PAGE_NOACCESS`, inside a region from `0x80000000` | committed, `PAGE_READWRITE`, exactly the 8 MB |
| writing 8 MB | 1.3–1.4 GB/s | 2.1–4.3 GB/s |
| lavapipe's one heap | 3072 MB (a 32-bit driver's cap) | 15912 MB (the machine's RAM) |
| device extensions shown | 183 | 182 (no `VK_KHR_external_memory_win32`) |

lavapipe offers `VK_EXT_external_memory_host` and not `VK_EXT_map_memory_placed`, so
new-style placement worked here through the former. MoltenVK's support of either, and
which one the Mac's Wine uses, is what a Mac run will show.

What this means for Display8's own Vulkan code:

- **A mapped pointer can be above 2 GB in a process that is not large-address-aware**
  (old-style WoW64, where a 32-bit host driver `mmap`s it into Wine's reserved upper
  half). Keep offsets and pointers unsigned; never hand one to anything that takes
  `int`.
- **Never judge a mapping by `VirtualQuery`** (or `IsBadReadPtr` and the like): in
  old-style WoW64, Windows' bookkeeping says the ring is reserved and not accessible
  while it is perfectly writable. Display8's trace finds a caller's module with
  `VirtualQuery`; that is fine for code, not for mapped memory.
- **The portability flag** is offered by Wine's instance here too
  (`VK_KHR_portability_enumeration`), and setting it costs nothing on a driver that is
  not a portability one; Display8's `Vk::start` sets it when the instance offers it
  (`ENUMERATE_PORTABILITY_KHR`), and enables `VK_KHR_portability_subset` on the device
  when the device offers it (`display/src/vk/mod.rs`; the log says "a portability
  device").
- **The surface** under Wine's X11 driver offers only `B8G8R8A8_UNORM` and
  `B8G8R8A8_SRGB` (`SRGB_NONLINEAR`), FIFO, FIFO_RELAXED, MAILBOX and IMMEDIATE, and
  composite alpha OPAQUE or INHERIT; the window manager resizing the window made the
  swap chain `SUBOPTIMAL` twice and it had to be re-made (640x480 to 1280x701 under
  sway's tiling), with no `OUT_OF_DATE` at all. Display8's present already re-makes
  it on `SUBOPTIMAL` as well as `OUT_OF_DATE`; that has to stay.
