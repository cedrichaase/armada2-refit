# gpu

The graphics interface `Render3D.asi` (`render3d/`) and `UI.asi` (`ui/`) draw through:
one `no_std` crate, a path dependency of both, linked into each plugin on its own. It
is not a layer of `a2mod` and installs nothing; what it changes reaches the game
through the two plugins, and their changelogs say so.

It exists for the display plan: the game's d3d8 device becomes a null stub of ours
(Display8, `display/`), and render3d and the UI draw the whole frame on Vulkan, then
Direct3D 12. The interface is shaped for those two and implemented on three backends:
**Direct3D 9**, the device behind the game's d3d8 one (crosire's d3d8to9,
`platform/D3D9.md`, or Display8's own in its stub mode), **Vulkan**, Display8's
device in its stub mode with `Backend=vulkan` ("The Vulkan backend" below), and
**Direct3D 12**, Display8's with `Backend=d3d12` ("The Direct3D 12 backend"). Which one
draws is decided at run time, by what Display8 says; the modules see one interface.
Moving both plugins onto it changed nothing on screen, so each move is judged by pixels
(the bench's baselines, `testbench/README.md`).

## Layout

| File | What |
|---|---|
| `src/types.rs` | the descriptions and small values: `Format`, `Usage`, `TextureKind`, `TextureDesc`, `Contents`, `BufferDesc`, `Sampler`, `Blend`, `Depth`, `AlphaTest`, `Cull`, `Topology`, `VertexAttr`, `Code`, `PipelineDesc`, `Rect`, `Viewport`, `Load`, `Pass`, `Block`, `Bindings`; for compute `StorageDesc`, `ComputeDesc`, `STORAGE_SLOTS` |
| `src/frame.rs` | `Frame`: the frame a `Cmd` records into, owned by the caller; `Span` |
| `src/pixels.rs` | `Mapped`, texels read back, and their conversion to RGB (the frame dumps) |
| `src/chain.rs` | a bake too long for one shader, as a chain of passes (`Chain`), on the interface |
| `src/api.rs` | `Gpu`, `Texture`, `Buffer`, `Storage`, `Pipeline`, `ComputePipeline`, `Timer`, `Cmd` as the plugins see them: each an enum of the two backends' own, every call going to the one it was made on; the backend chosen at run time |
| `src/d3d9/mod.rs` | the Direct3D 9 backend: `Gpu`, `Texture`, `Buffer`, `Pipeline`, `Cmd`, `Timer` |
| `src/d3d9/raw.rs`, `consts.rs` | Direct3D 9 through raw COM vtables, once `render3d/src/d3d` and `ui/src/d3d` (identical copies) |
| `src/display.rs` | the contract with Display8 (`display/`, the game's `d3d8.dll` of ours): the device, frame, upload ring, targets and mirrors it owns and exports, on Vulkan (`display8v1_vk_*`) and Direct3D 12 (`display8v1_dx12_*`), as `#[repr(C)]` types, each starting with its size; its version and the handshake (`display8_abi`, "Versions" below); the claims (`display8v1_register`, `display8v1_claims`, "Claims" below); its doc comment is the specification |
| `shaders/compat.hlsli` | what the shader compilers need apart: vkd3d (d3dbc), dxc to SPIR-V, dxc to DXIL (`render3d/README.md`, "One source, two compilers") |
| `../tools/shaderc/fetch-dxc.sh` | the dxc that writes the DXIL, DirectXShaderCompiler v1.9.2609, fetched into `$A2_DATA/tools/` by version and sha256 (`tools/shaderc/hlsl-rs.sh` runs it when a DXIL file is stale) |
| `../tools/shaderc/fetch-vkd3d.sh`, `../tools/shaderc/verify-generated.sh` | CI's check that no generated shader is stale: the first builds the vkd3d-compiler that wrote the committed d3dbc (release 1.19, version string `1.19 (git 5c355b596)` as it is in every CTAB, by sha256), the second deletes every generated shader file of render3d, ui, display and gpu, rebuilds them with it and dxc 1.9.2609 (SPIR-V as well as DXIL), runs spirv-val on each module and fails on any difference from the committed files (`.github/workflows/ci.yml`, job `shaders`) |
| `src/vulkan/mod.rs` | the Vulkan backend's `Gpu`: Display8's device opened, the recording state (the pass now, layouts), samplers, render passes, the uniform ring, Display8's images wrapped |
| `src/vulkan/tex.rs`, `pipe.rs`, `cmd.rs`, `mem.rs` | its textures, buffers and timestamps; pipelines; `Cmd` and the read-back; device memory in blocks |
| `src/vulkan/spirv.rs` | what it reads out of dxc's SPIR-V (reflection) and the two patches it makes to a pixel shader; built on the host too by `spirv-selftest.sh` |
| `shaders/vk_depth.hlsl`, `src/vulkan/shaders.rs` | its own shaders (the depth resolve), SPIR-V by `vk-shaders.sh` |
| `src/dx12/mod.rs` | the Direct3D 12 backend's `Gpu`: Display8's device opened, the root signature, descriptor heaps of its own, the frame's tables, Display8's textures wrapped |
| `src/dx12/tex.rs`, `pipe.rs`, `cmd.rs`, `raw.rs` | its textures, buffers and timestamps; pipeline states; `Cmd`, the blits, the depth resolve and the read-back; Direct3D 12 by vtable slot |
| `shaders/dx12.hlsl`, `src/dx12/shaders.rs` | its own shaders (the depth resolve, the blit), DXIL by `dx12-shaders.sh` |
| `src/record/` | the recorder of a frame's calls (`mod.rs`), the file format (`wire.rs`), its kernel32 calls (`sys.rs`) ("Record and replay") |
| `src/vulkan/tex/snap.rs` | what the recorder reads back of the objects a frame finds made |
| `src/replay.rs` | a recorded frame drawn again on any backend |
| `replay/`, `replay.sh` | `a2replay.exe`, the replayer, and the script that runs it under Wine on lavapipe; `replay/inspect.py` lists a recording |

`gpu::Gpu`, `gpu::Texture` and the rest are `api.rs`'s, re-exported at the crate's
root.

## Versions

`gpu::VERSION` is this crate's version (`Cargo.toml`, 1.1.0 since root 13.49.0; 1.0.0
in 13.48.0); it is not a layer, so the changelogs of render3d and the UI say when it
moves. The contract with Display8 has a version of its own, `display::ABI_MAJOR` and
`ABI_MINOR` (1.0 since display 0.12.0, 1.1 since 0.13.0: the claims), which both sides
build against.

At the first question of which backend draws, `Gpu` shakes hands: `display8_abi`
gets our major, minor and an address in the plugin's module (for Display8's log), and
gives Display8's. Then `Display8::load` takes the `display8v1_*` exports. The log
says both versions, `gpu 1.0.0 (contract 1.0): Display8 0.12.0 (contract 1.0)`, or why
there is no contract to draw through (`display::Refused`), each case worded apart: **no
Display8** (another chain's `d3d8.dll`: Direct3D 9, as ever), **Display8 too old** (it
has the unversioned exports of before 0.12.0 and no `display8_abi`), **another major**,
or an export of our major missing (a broken build). In all but the first the plugin
stands down to Direct3D 9 only, which on Display8's own Vulkan or Direct3D 12 draws
nothing; there the engine's draws pass through instead (`display/README.md`,
"Versions of the contract").

The rule for changing the contract (`src/display.rs`, "Versions"):

- **A minor adds, and only at the end.** A new export, constant, or a field appended to
  a struct; `ABI_MINOR` goes up. Every `#[repr(C)]` struct starts with `size: u32`. A
  struct Display8 hands out (`VkDevice8`, `VkFrame8`, `Dx12Device8`, `Dx12Frame8`) is
  read with `display::read`, which copies only the fields within the size Display8
  says and leaves the rest at their defaults; a struct the plugin hands in to be filled
  (`VkImage8`, `VkAlloc8`, the `Dx12*` ones) carries its room in `size`, and Display8
  fills it with `display::write`, no more than that, setting `size` to what it wrote;
  a struct the plugin hands in to be read (`VkPass8`) Display8 reads with `read`. Code
  that uses what minor N added checks `Display8::minor >= N` first (and Display8 checks
  the size of what a plugin hands in). A struct inside another does not grow in a
  minor, since that would move what follows it.
- **Anything else bumps the major**: removing, moving, retyping or redefining a field;
  changing an export's signature or meaning; changing a rule between recorders (the
  layouts and states between `Cmd`s, the heaps, who opens and closes a render pass).
  The exports are renamed with it (`display8v2_*` for major 2) and `ABI_MINOR` starts
  again at 0. `display8_abi` keeps its name and signature for ever. A major bump makes
  the d3d8.dll, `Render3D.asi` and `UI.asi` of that release ship together, as 13.48.0
  did for the first.

## Claims

Contract 1.1 (display 0.13.0) added the claims (`src/display.rs`, "Claims";
`display/README.md`, "Claims"): `Gpu::register(callback)`, called once at the plugin's
load (render3d's `startup` once its frame seam is in, the UI's before its parts are
patched), makes the handshake if it has not been made and hands Display8 a `Plugin8`
(`size`, an address in the plugin's module, the callback) through
`display8v1_register`. The callback returns the `CLAIM_*` the plugin draws now, and
Display8 asks it at each of the engine's scenes; a category nobody claims is the
engine's, drawn by passthrough. `Gpu::claimed_by_others()` is what the other registered
plugins claim (`display8v1_claims`), None while this plugin is not registered (no
Display8, one of minor 0, or one this `gpu` does not talk to), when the caller keeps to
what it did before claims. `Display8::register` and `claims` are None on a Display8 of
minor 0, by the rule above: `load` takes them only when the minor says they are there.

## The interface

**`Gpu`** is the backends' state for one plugin: on Direct3D 9 the device, every
resource made through it (held weakly, to let go of them when the device goes) and the
state blocks; on Vulkan and Direct3D 12 Display8's device and what the backend keeps
for recording. Each plugin keeps one in a static of its own (`ui/src/gfx`,
`render3d/src/gfx`), made with `Gpu::with_display(log, lookup)`, `lookup` its
`GetProcAddress` on `d3d8.dll`, and attaches the game's device to it every frame
(`attach_engine`: the Direct3D 9 device behind the engine's, or Display8's own;
`engine_id` names it without attaching; `on_display` says which). Nothing in it is per
frame. **The backend** is Display8's answer to `display8_backend`, asked until it gives
one: Vulkan or Direct3D 12 from the engine's `Direct3DCreate8` when the stub runs on it
(on Direct3D 12 the backend opens once Display8 has made the device), Direct3D 9
otherwise, and Direct3D 9 for good with no Display8, an older one, or once the engine
has a device and Display8 has said nothing (forward mode). An object made before the
answer is made for Direct3D 9, and the log says so once.

**Resources** are made once from a description and stay valid until the last clone is
dropped, across `Reset`s and devices. `Gpu::texture(TextureDesc, Contents)` makes a
texture: 2D, a cube (`TextureDesc::cube`, six faces) or a volume (`TextureDesc::d3`);
sampled, a colour target (a cube's faces each a target: `ColorTarget::face`), a depth
target (sampled too: `Usage::DEPTH | SAMPLED`, filled by `Cmd::resolve_depth`),
multisampled, with a chain the device makes (`Usage::GEN_MIPS`, `levels` 0,
`Cmd::generate_mips`), or rewritten from the CPU (`Usage::DYNAMIC`, `Texture::write`).
The contents are kept, so the texture can be made again; `Contents::Transient` lets
go of them once it is made, for art that would hold too much of the process's 2 GB
(its owner makes another after the device is lost). `Gpu::buffer` makes a vertex or
index buffer (`buffer_transient` likewise). `Gpu::pipeline(PipelineDesc)` makes an
immutable pipeline: shaders, vertex layout and stride, topology, blend (with a
separate alpha equation), write mask, depth test and bias, the alpha test, cull,
whether the scissor applies, the targets' formats and the sample count. Each is made on
the device at its first use; `ready` makes it at once and says whether the device took
it. `Gpu::timer(n)` makes a set of GPU timestamps (`Timer`: `begin`, `stamp`, `end`,
`read` a few frames later). `Gpu::wrap_d3d9_texture` / `wrap_d3d9_surface` and
`engine_target` / `engine_depth` / `engine_viewport` wrap a Direct3D 9 object someone
else made (the engine's minimap picture, its fog, its back buffer) without owning its
creation, or read the engine's viewport. The backend-neutral ones: `engine_texture`
(an engine `IDirect3DTexture8`: d3d8to9's Direct3D 9 texture behind it, or Display8's
Vulkan mirror), `engine_target` / `engine_depth` (the engine's back buffer and depth:
Display8's images on Vulkan), `engine_viewport` and `engine_transform` (Display8's
record of the engine's calls on Vulkan), and `half_pixel` ("Pixel centres").

**Compute** (Vulkan and Direct3D 12; `Gpu::compute` says whether it is there, and on
Direct3D 9 it never is): `Gpu::storage(StorageDesc)` makes a storage buffer (elements
of `stride` bytes), `Gpu::compute_pipeline(ComputeDesc)` a compute pipeline (dxc's
SPIR-V and DXIL; no d3dbc), and a texture made with `Usage::STORAGE` is a storage
image (its first level; `R32F` is the format Direct3D 12's feature level 11_0 also
loads, `Rgba16F` it only stores). A `Cmd` binds read-only storage buffers
(`bind_storage`, any stage: draws read them too), read-write ones
(`bind_storage_rw`) and storage images (`bind_storage_image`) by slot, the compute
shader's register file (`cs_block`, its cbuffers at b0), writes a storage buffer from
the CPU through the frame's ring (`write_storage`), and `dispatch`es. Ordering is the
backend's: a copy into a storage buffer and every dispatch are fenced off from what
comes before and after, so what a dispatch writes is what the next draw or dispatch
reads; `barrier` is there for anything else. One queue: no async compute. The
shaders spell the slots `STORAGE(n)` (t16 + n), `STORAGE_RW(n)` (u n) and
`STORAGE_IMAGE_R32F(n)` / `_RGBA16F(n)` (u 8 + n) (`shaders/compat.hlsli`), and an
entry dxc alone compiles is marked `:dxc` (`tools/shaderc/hlsl-rs.sh`). On Direct3D 9
every storage object is an empty one, a compute pipeline is never ready, and a
dispatch does nothing: what uses compute asks `Gpu::compute` first and keeps its
Direct3D 9 path as it was. render3d's clustered lights are the first user
(`render3d/README.md`, "Clustered lights").

**`Frame`** is what a `Cmd` records into: its number, the rings the frame's dynamic
vertices and indices are written to (`Cmd::vertices`, `Cmd::indices`, giving a `Span`
to draw from in the same frame), and what is retired until the frame is over. The
caller owns it and calls `next` once per presented frame.

**`Cmd`** records commands (`Gpu::begin(&mut frame)`): `begin_pass(&Pass)` with its
colour targets and optional depth, each with a load operation (`Load`, `Clear(value)`,
`DontCare`) acting inside the pass's area, and `end_pass`; `set_pipeline`;
`bind(&Bindings)` (or `bind_texture` / `set_sampler` / `vs_block` / `ps_block` one by
one); `draw`, `draw_indexed` (from the frame's rings, or a static buffer as a span:
`span_of`), `draw_buffer`; `set_viewport`, `set_scissor`, `clear_rects`; `resolve`,
`resolve_depth` and `copy` (filtered) between targets; `generate_mips`; `read_back`
of a target (`read_back_layer`: a cube's face). Uniforms go in blocks of float4
registers from a base (`Block::of(base, &my_repr_c_struct)`).

## Why it is shaped so

- **Immutable pipelines.** Vulkan and Direct3D 12 compile all fixed-function state
  into one object up front, and a draw that changes one blend factor needs another
  pipeline. Describing them so now keeps the modules from depending on state left
  over from the draw before, which is also what the state-block bugs here were.
- **Explicit passes with load operations.** Both newer APIs bind targets per pass and
  clear by load operation (a render pass, or dynamic rendering; `OMSetRenderTargets`
  with clears on Direct3D 12). The resolve is a command between passes for the same
  reason: on Vulkan it is a resolve attachment or a blit, on Direct3D 9 `StretchRect`.
- **Uniform blocks at a register base.** Direct3D 9 sets constant registers; Vulkan
  and Direct3D 12 bind buffers. A block of a `#[repr(C)]` struct at the base the
  shader declares fits both: the base is Direct3D 9's truth, and Vulkan and Direct3D 12
  give each block a buffer binding, in the order a binding group lists them. The existing
  register layouts (`k_tile` at c29, the post passes' c0..c3) keep working unchanged.
- **Binding groups** of a few (texture, sampler) slots and uniform blocks per draw map
  onto a descriptor set and onto a root table alike. The sampler goes with its slot:
  Direct3D 9 has no separate sampler objects, and the newer APIs take static or
  combined samplers well.
- **Resources valid until dropped, and no device-loss callbacks.** On Vulkan and
  Direct3D 12 Display8 owns the device, which is never lost there. The Direct3D 9 backend handles losing it
  inside (below), so no module is written around `Reset`.
- **Usage flags** (`SAMPLED`, `RENDER_TARGET`, `DEPTH`, `DYNAMIC`, `COPY_SRC`,
  `COPY_DST`) say what a texture is for, so Vulkan and Direct3D 12 can place their barriers.
  Direct3D 9 picks its pool and usage from them.
- **Per-frame state from outside.** Each plugin links this crate statically, so two
  copies of it run in one process, and on Vulkan and Direct3D 12 Display8 owns the frame and exports it
  through an `extern "C"` interface of plain handles. So the frame is a value handed
  in, and this crate keeps no static of its own for anything per frame. Descriptions
  and handles are plain data (`#[repr(C)]`, `#[repr(u32)]`) where they may cross that
  boundary.
- **Cubes and volumes as kinds of texture.** A cube is a 2D image of six layers with
  a cube view on Vulkan, a 2D array with a cube SRV on Direct3D 12; a volume a 3D image
  or resource. A pass draws into one face (`ColorTarget::layer`), as a Vulkan render
  pass takes a single-layer view. A chain the device makes (`GEN_MIPS`) is a blit
  chain there.
- **A sampled depth, filled by a resolve.** `resolve_depth` is a command between
  passes, as on Vulkan (a depth resolve attachment, or sampling a single-sample depth
  directly) and Direct3D 12 (`ResolveSubresource`, or a shader); on Direct3D 9 the
  vendors' RESZ into an `INTZ` texture.
- **The alpha test as pipeline state.** Neither newer API has it: a backend there makes
  a variant of the pixel shader that discards (a specialisation constant or a define).
- **Timestamps** are a query set made once and read later without waiting: a Vulkan
  timestamp query pool (reset at `begin`, `vkCmdWriteTimestamp`, the period from the
  device's limits, never disjoint), a Direct3D 12 timestamp heap resolved into a buffer.
- **Compute as a capability.** Direct3D 9 has none, so it is asked for
  (`Gpu::compute`), never assumed, and the Direct3D 9 path of whatever uses it stays as
  it was. Storage is bound by slot, as textures are by stage: a descriptor set on
  Vulkan, a root table on Direct3D 12. Barriers are placed by the backend around each
  dispatch and copy, broad rather than tight: a frame has a handful of dispatches.

## The Direct3D 9 backend

**Immediate.** Each command is applied as it is recorded: a pipeline as render,
sampler and shader state, a pass as `SetRenderTarget`, `SetDepthStencilSurface`, the
viewport and `Clear` (one clear for all targets: the first colour target's value; the
stencil is kept), a span as a lock of the frame's ring with `NOOVERWRITE` (or
`DISCARD` when the frame has not written it yet). A ring a frame outgrows is replaced
by one twice the size, the old one retired to the frame, so a span stays valid for the
whole frame.

**The engine's state is put back.** d3d8to9 keeps its own record of what the game
bound, so whatever we change must be as the engine left it when it next calls. A
`Cmd` captures the device's state at its first command that changes any, and applies it
again when dropped; copies, resolves and read-backs change none and capture nothing.
The block is recorded with Begin/EndStateBlock and holds only what our draws change
(every render, texture-stage and sampler state, textures, shaders, declaration and FVF,
streams, indices, constants, viewport, scissor rectangle), which is what made
render3d's `Guard` cheap (`render3d/PERF.md`). The UI used a `D3DSBT_ALL` block once
a frame before; it uses the recorded one now. Nested sections each have a block of
their own (render3d's `Guard`s shared one, so a nested pair would have put the outer
one's changes back instead of the engine's state; the log says once if any nest).

**Targets stay bound.** A state block never holds the render targets or the depth
surface, so the targets of the last pass begun stay bound after the `Cmd` ends. That
is how render3d's frame works today: the scene pass is begun before the engine's
body, and the engine draws into it.

**A pipeline is applied whole**, every state it names, so no draw depends on what the
one before it left, and the clip planes are off; binding the same pipeline again in a
`Cmd`, with nothing in between that changed state (a pass that cleared, a clear, a
depth resolve), sets nothing. A colour clear (a load operation, `clear_rects`) writes
every channel whatever the last pipeline masked. Samplers set in a `Cmd` are
remembered in it, and a sampler bound again as it is sets nothing.

**Pools.** Targets and depth surfaces are in the default pool; a texture filled once
goes to the default pool through a system-memory copy released at once, so no address
space is held for it; a `DYNAMIC` texture is in the managed pool, its contents copied
in when it is next bound after a `write`; buffers are in the default pool. Cubes and
volumes go the same ways (a cube target is made with each face's surface held; a
sampled depth is an `INTZ` texture). Queries do not outlive a `Reset` and are made
again at their next use.

**Losing the device.** `before_reset` (from the plugin's `Reset` hook) lets go of the
state blocks and everything in the default pool; `release_device` (from the hook of
the engine's `DestroyRenderSurfaces`, or when `attach` sees another device) lets go of
everything, as native Direct3D 9 needs before the engine's next `CreateDevice` (a
device lives while anything made on it is held; `render3d/README.md`, "Letting go of
the device"). Each resource keeps its description and contents and is made again at
its next use on the device there is then. A target's contents are lost, as Direct3D 9
loses them. A wrapped object is let go of with the device and draws nothing after,
until wrapped anew. `raw::live` still counts every object made and not released.

**Logs** go through the function the plugin gives `Gpu::new`, prefixed `gpu:`: the
state block chosen and the card's shader slots once per device, a shader, declaration,
texture or buffer the device refused (once per device each).

### What stays raw

`gpu::d3d9::raw` is the old `d3d` module: the backend's own calls, and what the
plugins' engine glue uses to reach the game's d3d8 device (`query`, `Com`, `vcall!`):
render3d's `frame/` (the hooks of `Reset` and `DestroyRenderSurfaces`, which let go of
the Direct3D 9 device, and the menu scene's viewport) and the UI's `engine/` (the same
hooks, the dumps' back buffer, the swap chain's parameters). No module draws with it.
What both backends answer went behind the interface: the engine's device
(`attach_engine`), its viewport and transforms (`engine_viewport`,
`engine_transform`), its textures (`engine_texture`).

## The Vulkan backend

`src/vulkan/` draws on the device Display8 makes in its stub mode with
`Backend=vulkan` (`display/README.md`, "Vulkan"). Display8 owns the instance, the
device and queue, the swap chain, images standing for the engine's back buffer and
depth, the pipeline cache and the frame (two in flight, each with a command buffer, a
descriptor pool and an upload ring); the backend reaches them through the exports of
`src/display.rs`, loads the device's functions through the `vkGetInstanceProcAddr`
Display8 hands over (`ash`, no loader of its own), and records into the frame's command
buffer on the game's thread, in the order the calls come. Display8 submits it at the
engine's `Present`. How each part of the interface maps, and why:

- **The pass now.** On Direct3D 9 a pass's targets stay bound after its `Cmd`, and the
  modules rely on it (render3d's scene pass is begun before the body and drawn into by
  every module after). So a begun pass stays *the* pass (`PassState`), across `Cmd`s,
  until another is begun; a draw opens a render pass on it when none is open. A render
  pass is closed whenever something must happen outside one (an upload, a layout
  change, a copy, a resolve, Display8's mirror updates when the frame is asked for) and
  at every `Cmd`'s end, so none is ever open between recorders, as the contract has it,
  and the next draw opens it again keeping what was drawn. A pass that clears does so at
  once: by the render pass's load operation when the area is the whole target, else by
  `vkCmdClearAttachments` over the area; the first colour target's clear colour clears
  every colour target, as Direct3D 9's `Clear` does. `DontCare` keeps the contents, as
  Direct3D 9 did (a render pass may not draw over all of it).
- **Render passes, not dynamic rendering**: Vulkan 1.1 and MoltenVK have them without
  an extension. One per kind (the attachments' formats and samples, and the load
  operations), every attachment in its attachment layout before and after, with
  dependencies on everything before and after (broad masks: a frame recorded once,
  correct before tight). A framebuffer is made for each render pass opened and let go
  of with the frame (cheap, and nothing has to know when a view dies).
- **Layouts** are tracked per image, the whole image in one layout, and changed by a
  barrier outside a render pass when the next use needs another: a sampled texture is
  put in `SHADER_READ_ONLY` before the render pass opens (a draw looks at every texture
  its shaders read, each time), a target in its attachment layout, a copy's two ends in
  the transfer layouts. Display8's images (its back buffer, depth and mirrors) are put
  back in their own layout when the `Cmd` that changed it ends. A texture that is
  sampled by a draw into its own pass reads the placeholder instead (logged once).
- **Pipelines** are made per kind of pass they are drawn in, at the first draw there,
  from the SPIR-V beside the d3dbc (`Code::spirv`): the description's target formats
  are not trusted (render3d's name the scene's, the UI's the back buffer's). The SPIR-V
  is read (`spirv.rs`): the vertex input is each input the vertex shader declares,
  matched to the layout's attribute **by semantic** (dxc names them `in.var.TEXCOORD3`);
  one the layout lacks reads (0, 0, 0, 1) from a buffer of its own at stride 0, as a
  Direct3D 9 element left out does. dxc numbers each stage's interface by its own
  struct, so the pixel shader's inputs are **moved to its vertex shader's outputs of
  the same semantic**. The **alpha test**, which Vulkan does not have, is put into the
  pixel shader: after the store of its colour, a fragment whose alpha, as Direct3D 9
  compares it (255ths, rounded), fails is discarded. The vertex shaders are compiled
  with `-fvk-invert-y`, so a triangle lands where Direct3D 9 puts it and is wound on
  screen as there: `Cull::Clockwise` is Vulkan's back face with a counter-clockwise
  front. The **depth bias** is Direct3D 9's in depth units, scaled to the format's step
  (2^16, 2^24, 2^23 for a float depth, as DXVK does). Depth state is off in a pass
  without depth. Viewport and scissor are dynamic.
- **Uniform blocks**: each stage's SPIR-V uniform block is its Direct3D 9 register file
  (a member at byte 16 n for `cn`). A `Cmd` keeps a shadow of both files (from zero, as
  a fresh `Cmd` on Direct3D 9 finds the engine's, which never sets ours), and a draw
  after a change uploads each file up to what the pipeline's shaders read into a ring of
  the backend's own (1 MB chunks per frame slot, mapped once), bound as two dynamic
  uniform buffers of one set per chunk, with the offsets per draw. render3d makes a
  thousand and more draws a frame: one set write per chunk, not per draw.
- **Textures** are combined image samplers at the Direct3D 9 stage's binding (set 1);
  a set from the frame's pool is written when what a draw reads changes. An empty
  stage the shader reads gets a placeholder of its dimension, (0, 0, 0, 0), as DXVK's
  empty slots read; a stage with no sampler set has Direct3D 9's default (point, no
  mips, wrap). Samplers are made once per description. Formats Vulkan lacks are views
  with a swizzle: Direct3D 9's `L8` is `R8` read as (r, r, r, 1), `A8` as (0, 0, 0, r),
  `X8R8G8B8` `B8G8R8A8` with an alpha of one, and a float format's missing channels read
  one, as Direct3D 9's do. `D24` without a device format is `D32_SFLOAT`.
- **Memory** comes in blocks (32 MB of device memory, 4 MB of host memory mapped once),
  first fit, images and buffers apart (`mem.rs`): a mission's thousand textures and
  models would otherwise be a thousand allocations, past some drivers' 4096. Contents go
  in through a staging copy: Display8's upload ring for a small one, a buffer of its own
  for a big one, mapped only while it is written and let go of with the frame, so a
  mission's textures, all made in the frame that loads them, do not hold their size of
  a 32-bit process's address space.
- **Copies**: `resolve` is `vkCmdResolveImage`; `copy` a copy at the same size and
  format, a blit otherwise (a multisampled source resolved into an image of its own
  first when it is also scaled); `generate_mips` a chain of 2:1 linear blits over every
  face; `resolve_depth` a pass of its own (`shaders/vk_depth.hlsl`) writing each pixel's
  depth from the source's texel, sample 0 of a multisampled one, as RESZ takes.
- **Read-back** (the frame dumps) copies the target into a buffer and has Display8
  submit what the frame has recorded so far and wait for it (`display8_vk_flush`); the
  frame goes on in the same command buffer, and every `Cmd` binds everything again.
- **Nothing is destroyed while the GPU may read it**: what is let go of waits until
  Display8 has begun the frame two after (its fence has passed).
- **Timers** are a timestamp query pool, reset at `begin`, read without waiting once
  the frame that wrote it is done; never disjoint, the frequency from `timestampPeriod`.
- **Compute**: a compute pipeline layout of sets 0..3 (the register file at set 0's
  binding 0, which the vertex stage's file takes in a draw; the textures at set 1; set 2
  the read-only storage buffers, which draws read too and the graphics layout also
  has; set 3 the read-write ones at bindings 0..7 and the storage images at 8..15). The
  storage sets come from pools of the backend's own, one per frame slot and reset when
  its slot comes round (Display8's frame pool has no storage descriptors); an empty
  slot gets a small placeholder buffer. A storage image has a view of its own without
  the format's swizzle (a storage view takes none) and is in `GENERAL` while compute
  uses it. A copy into a storage buffer and a dispatch are recorded outside any render
  pass, between broad memory barriers.
- **Never lost**: Display8's device lives as long as the game. `before_reset` and
  `release_device` do nothing on Vulkan, and nothing is made twice.

### Read-backs

`read_back` copies the target into a buffer of its own, has Display8 submit the frame
so far and wait (`display8_vk_flush`), and hands the mapped bytes over. The buffer is
**host-cached** memory where the device has it, and only then the first host-visible
coherent type: on the RX 5700 (RADV, resizable BAR) that first type is the card's own
memory through the PCIe window, which the CPU reads at a few tens of MB/s. render3d's
nebula bake reads its whole volume back, 6 to 34 MB a class, and took 0.4 to 1.5 s a
class there (0.1 to 0.2 s on Direct3D 9 on the same card), so a menu or a mission's
first frame stalled for seconds; from cached memory a class takes 20 to 230 ms. The
flushes themselves were not the cost: stacking 16 layers to a read-back changed nothing
measurable.

### Pixel centres

Direct3D 9 puts a pixel's centre on whole coordinates, Vulkan (and Direct3D 12) at the
half. DXVK moves a Direct3D 9 viewport by 0.5 - 1/128 of a pixel to keep Direct3D 9's
rule, and the callers that map pixels to clip space move their positions by half a
pixel the other way (the UI's `VP`, render3d's post passes). On Vulkan that half pixel is
the caller's to leave out: `Gpu::half_pixel()` is 0.5 on Direct3D 9 and 0 on Vulkan, and
a 3D scene, whose projection is the engine's, lands half a pixel from where DXVK puts
it. For comparing a frame with Direct3D 9's pixel for pixel, Display8's
`PixelCentres=d3d9` keeps Direct3D 9's rule on Vulkan: the viewport moved as DXVK moves
it and `half_pixel` 0.5 (`VkDevice8::features`, `FEATURE_D3D9_CENTRES`).

### Samplers as DXVK makes them

On Direct3D 9 the frame is filtered as the game's `dxvk.conf` has DXVK make every
sampler (`renderer/README.md`): 16x anisotropy in place of every linear filter, and a
LOD bias of -0.5 on every sampler with mips. The plugins never ask for either; DXVK
adds them. So on Vulkan Display8 reads the same keys and hands them over
(`VkDevice8::sampler_anisotropy`, `sampler_lod_bias`), and `Gpu::sampler` makes each
sampler as DXVK would: a linear filter anisotropic at that much, the bias on a sampler
with mips (none on one without, which has no level to pick). Without it every hull read
a level blurrier than on Direct3D 9. Cube maps are seamless on Vulkan, as
`d3d9.seamlessCubes` makes them on DXVK.

### Measured

render3d and the UI on Vulkan (`Backend=vulkan`, `PixelCentres=d3d9`, the validation
layer on) against the same scenes on Direct3D 9 on the GPU (the `pre-gpu-*` baselines,
which the Direct3D 9 backend draws at RMSE 0), the screenshot of each port scene,
2026-10-23. On the GPU (RX 5700, RADV) and on lavapipe (the bench's default for Vulkan):

| Scene | GPU: RMSE (/255) | over 2/255 | lavapipe: RMSE | over 2/255 |
|---|---|---|---|---|
| `port-showcase` | 0.87 | 0.47% | 1.68 | 4.2% |
| `port-nebula` | 0.36 | 0.02% | 0.57 | 0.6% |
| `port-firing` | 0.40 | 0.12% | 0.97 | 1.0% |

No validation message in any of them, nor in `display-stub` (the whole game, intro to
Exit) on Vulkan on the GPU, which passes as on Direct3D 9. What is left is one or two
levels over the sky
and the hulls, and single pixels at edges and on small bright details (the maxima, 117
to 221, are on the Borg hulls' lights and the sprites): the rasteriser and the filtering of a CPU
driver against a GPU's, and dxc's floating point against DXVK's translation of
vkd3d's. The procedural bakes are where that shows most: their hashes
(`render3d/shaders/world_surface.hlsl`, `hash13`) take the fraction of products in the
tens of thousands, so one rounding apart, a fused multiply-add or not, moves a lattice
value. The same scene on Direct3D 9 on lavapipe (DXVK there) is further from the GPU's
than Vulkan on lavapipe is (showcase 4.1, nebula 25.8, firing 7.0): DXVK on a CPU
driver is not a reference for these.

lavapipe cannot take the whole game: a campaign mission (`display-stub`'s `a2_fed01`)
kills the game while it loads, on `Backend=vulkan` and on `Backend=d3d9` (DXVK) alike,
as the nebulae are baked; on the GPU both pass. Every allocation of a CPU driver is
host memory, likely mapped into the 32-bit process's address space, which a mission's
textures and nebula volumes fill. So the whole game on Vulkan is a run on the GPU
(the bench's default since testbench 3.70.0), after the port scenes have come through the validation layer
clean on lavapipe.

## The Direct3D 12 backend

`src/dx12/` draws on the device Display8 makes in its stub mode with `Backend=d3d12`
(`display/README.md`, "Direct3D 12"): Display8 owns the device and queue, the swap
chain, the textures standing for the engine's back buffer and depth, the two
shader-visible descriptor heaps and the frame (two in flight, each with a command
allocator and list, an upload ring and a range of each heap). The backend reaches them
through the `display8_dx12_*` exports of `src/display.rs` and records into the frame's
list, on the game's thread, in the order the calls come. It is the Vulkan backend's
plan in Direct3D 12's terms; what differs, and why:

- **One root signature** for every pipeline, serialized once per plugin: each stage's
  register file as a root constant buffer, at b0 (vertex) or b1 (pixel) in each of the
  eight spaces a cbuffer may take (`tools/shaderc/dxil-spaces.py` puts the k-th cbuffer of a
  stage in space k, all of them views of the one register file); one table of the
  sixteen stages' shader views (t0..t15); one table of one sampler per stage (s0..s15);
  eight root constants at b2 (the alpha test, the blit's region); one table of the eight
  read-only storage buffers (t16..t23). 58 of the 64 words (a root descriptor takes
  two, a table and a constant one each). A
  draw after a change uploads each register file whole up to what the pipeline's
  shaders read into Display8's ring and points all eight spaces of its stage at it.
- **Descriptors.** Every view of the backend's lives in heaps of its own that shaders
  do not see (blocks of 1024 per type). A draw copies the sixteen views it reads into a
  range of the frame's (`display8_dx12_descriptors`), once a frame per set of views,
  and each sampler once a frame; the sampler heap's 1024 a frame would not take a table
  of sixteen per set. A freed view waits with its resource, so that no handle stands for
  two textures in one frame (the frame's tables are kept by handle). An empty stage the
  shader reads gets a null view of the dimension it declares, which reads (0, 0, 0, 0)
  as DXVK's empty slots do.
- **What the shaders read** comes from the SPIR-V of the same source (`vulkan::spirv`):
  the DXIL is built without reflection. The vertex inputs come from the DXIL's own
  input signature (`ISG1`), by semantic; one the layout lacks reads (0, 0, 0, 1) from a
  placeholder buffer per instance.
- **Pixel shaders' inputs** must sit at the registers their vertex shader writes them
  to: Direct3D 12 links stages by register, where Direct3D 9 and Vulkan match
  semantics. A pixel shader that leaves out the position its vertex shader writes first
  (render3d's post passes, the sky's quad shaders, the ghosts) takes it first in DXIL
  alone (`DXIL_POS`, `DXIL_POS_MEMBER`, `DXIL_POS_FIRST` in `shaders/compat.hlsli`);
  the d3dbc and the SPIR-V are as they were.
- **The alpha test** cannot be patched into DXIL (it is LLVM bitcode, and signed), so a
  pixel shader drawn with one has an alpha-tested twin built beside it
  (`tools/shaderc/dxil-alpha.py`, an entry marked `:at`, `Code::dxil_at`), which discards
  as Direct3D 9's test would by the interval at b2 (`spirv::alpha_interval`).
- **The pass now** stays the pass across `Cmd`s, as on Vulkan; Direct3D 12 has no render
  pass to open or close, so its targets are set at the first draw of a `Cmd` (and when
  the pass changes), and a clear (a load operation, `clear_rects`) is
  `ClearRenderTargetView` / `ClearDepthStencilView` over the area at once.
- **States** are tracked per texture, the whole of it in one, and changed by a barrier
  when the next use needs another (the shaders read a texture in
  `PIXEL | NON_PIXEL_SHADER_RESOURCE`). Display8's textures go back to theirs when the
  `Cmd` that changed them ends. Buffers are made in `COMMON` and promoted by their use,
  as Direct3D 12 does with buffers.
- **Formats** as on Vulkan, through the view's component mapping: `L8` is `R8` read
  (r, r, r, 1), `X8R8G8B8` `B8G8R8A8` with an alpha of one, a float format's missing
  channels one; `A8` is Direct3D 12's own `A8_UNORM`. A depth texture is typeless (its
  depth view `D16`/`D24S8`/`D32`, its shader view the matching colour format), so any
  of ours can be sampled; Display8's depth cannot.
- **What Direct3D 12 has no command for** is a draw of the backend's own shaders
  (`shaders/dx12.hlsl`, DXIL by `dx12-shaders.sh`): `copy` at another size or format
  and `generate_mips` (each level drawn from the one above, linear, every face) are a
  blit; `resolve_depth` a pass writing each pixel's depth from the source's texel,
  sample 0 of a multisampled one. `resolve` is `ResolveSubresourceRegion`
  (`ID3D12GraphicsCommandList1`; the whole subresource without it), `copy` at the same
  size and format `CopyTextureRegion`.
- **Read-back** copies the target into a read-back buffer and has Display8 submit the
  frame so far and wait for it (`display8_dx12_flush`); every `Cmd` sets everything
  again after. Timers are a timestamp query heap resolved into a read-back buffer.
- **Pixel centres and samplers** as on Vulkan ("Pixel centres", "Samplers as DXVK makes
  them"): `Dx12Device8::features` carries `FEATURE_D3D9_CENTRES`, and the samplers are
  made with `dxvk.conf`'s anisotropy and LOD bias.
- **Compute** has a root signature of its own: the register file at b0 in each space,
  the sixteen textures, a sampler per stage, the read-only storage (t16..t23) and the
  read-write storage and storage images (u0..u15, one table), each slot without
  anything a null view. A storage buffer is a structured buffer in the default heap that
  allows unordered access, with a shader view and an unordered access view of its own.
  Its state is tracked as a texture's is, but a buffer decays to `COMMON` at every
  submission (`Gpu::epoch` moves on at each new frame and flush), so its first use in a
  list starts from there. A dispatch puts what it writes in `UNORDERED_ACCESS` (a
  barrier between unordered accesses when it already was), the rest in shader
  resource, and ends with a barrier between unordered accesses; it sets a compute
  pipeline state, so every recorder sets its own again. Feature level 11_0: structured
  buffers and `R32` storage images, no typed half-float loads.
- **Pipeline states** are made at the first draw in a pass of their kind and kept. No
  cache of ours on disk yet: vkd3d-proton keeps its own (`vkd3d-proton.cache`), and a
  Windows driver keeps its own shader cache.

### Measured

render3d and the UI on Direct3D 12 (`Backend=d3d12`, `PixelCentres=d3d9`), through
Proton's vkd3d-proton on the GPU (RX 5700, RADV), against the same scenes on Direct3D 9
on the GPU (the `pre-gpu-*` baselines), the screenshot of each port scene, 2026-10-23:

| Scene | RMSE (/255) | over 2/255 | max |
|---|---|---|---|
| `port-showcase` | 0.97 | 0.75% | 221 |
| `port-nebula` | 0.39 | 0.19% | 32 |
| `port-firing` | 0.41 | 0.12% | 55 |

As on Vulkan: one or two levels over the sky and the hulls, and single pixels on the
Borg hulls' lights and small bright details, which is dxc's floating point (fused
multiply-adds in the procedural bakes, `hash13`) against DXVK's translation of vkd3d's
d3dbc. `display-stub` (the whole game, intro to Exit) passes on `Backend=d3d12`; one
earlier run found the in-mission menu's "Save Not Allowed" while the mission's opening
still ran (the scenario's fixed waits), and the next passed. **Validation**: Microsoft's debug layer is not there under vkd3d-proton
(`D3D12GetDebugInterface` answers `DXGI_ERROR_SDK_COMPONENT_MISSING`), and the Khronos
layer under it reaches no one: it loads (the loader says so) and writes nothing, and
vkd3d-proton's own check (`VKD3D_CONFIG=vk_debug`) finds its messenger hears nothing
through the 32-bit winevulkan. So the bench checks pictures, not API use; the debug
layer on Windows is for a machine there.

## render3d on gpu

Every part of render3d draws through the interface: the frame and its post passes, and
every module under `render3d/src/mods/`, the scene layer (`scene/`), the assets'
textures, the menu scene and the timing. How a module draws, as they all do now:

1. **Pipelines** are made once per combination of the states the module draws with,
   kept by a small key in a `gfx::Pipes` (`view` for the scene pass's formats and
   samples, made again when they change; `own` for a target of its own). Each says
   every state a draw relies on, so no draw depends on what the one before left; a
   state the module never set before is the engine's (the depth bias, the clip planes:
   0), which the pipeline now sets.
2. **Constants** are blocks at the registers the shader declares
   (`Block::new(base, &floats)`, `Block::of`). Between `set_pipeline`s of one `Cmd`
   they persist, as Direct3D 9's registers do; Vulkan and Direct3D 12 give each draw the
   blocks it reads.
3. **Textures** are `gpu::Texture`s bound with their sampler (`bind_texture`); the
   game's art is `assets::texture` (made at once, keeping no copy: `Contents::Transient`,
   loaded again after a `Reset`), an engine texture is wrapped.
4. **Draws go through a `Cmd`** (`gfx::cmd()`), which puts the engine's state back
   when dropped. The scene pass is bound already; a module that draws into a target of
   its own (a bake, the shadow map, the nebulae's march) begins that pass, and
   `gfx::view_pass` before it draws into the view again. Geometry rebuilt every frame
   goes into the frame's rings (`cmd.vertices`), a model's into static buffers
   (`gfx::buffer`, `buffer_once`).
5. **A module's `reset` and `forget`** are for what the objects held: a target's
   contents are lost (a bake to do again), and a `Transient` texture or buffer cannot be
   made again (its owner makes another).

## The bake chain

`src/chain.rs` draws a bake too long for one pixel shader as a chain of passes through
float targets, tile by tile (render3d/README.md, "Shaders on Windows"): render3d's
sky and planet surfaces. It is built on the interface, not the Direct3D 9 backend: the
split is in the HLSL source, so the SPIR-V has the same passes, and a Vulkan or
Direct3D 12 backend runs the same chain. A backend that takes the whole function in one
shader may later be handed it as a single pass, with nothing changed for the callers.

## Record and replay

One frame of the plugins' calls can be recorded to files and drawn again with no game,
on any backend, by a small 32-bit program under Wine on lavapipe: so a backend can be
judged on a frame of the real game without starting it, and a difference between two
backends found on one frame, not a scene. The recorder is `src/record/`, the replay
`src/replay.rs`, the program `replay/` (`a2replay.exe`), the entry point `replay.sh`.

    ./a2test drive --session ID record NAME          # the frame, into $A2_DATA/replays/NAME/
    gpu/replay.sh NAME                                # replay-vulkan.ppm, replay-d3d9.ppm there
    gpu/replay.sh NAME --backend d3d12                # vkd3d-proton, from A2_PROTON
    ./a2test baseline diff $A2_DATA/replays/NAME/game.ppm $A2_DATA/replays/NAME/replay-vulkan.ppm

**Asked for** as a frame dump is (`testbench/README.md`, "Frame dumps"): a trigger file
beside the exe, `Gpu.record`, whose first line is the name. Each plugin's recorder
looks for it twice a second (the clock is read at each `Cmd` begun: a plugin that
begins a few a frame still looks twice a second at one frame a second, which a count of
`Cmd`s did not); the first to see it picks one of Display8's frames a second and a half
ahead (by the frame rate it has seen), so that every plugin has looked before it comes,
and writes the frame and the request into a page every recorder in the process shares
(`Local\a2-gpu-record-<pid>`, a file mapping: each plugin links its own copy of this
crate). A plugin that sees the request only once its frame has begun records nothing
rather than half of it. The page also holds the counter every record of every plugin
takes its `seq` from, which is the order the calls were made in. While a recording is
asked for or under way, every call asks Display8 which frame it is (`display8_vk_frame`),
so what a plugin does before its first `Cmd` of the frame (the UI asks for its mirrors
while it builds its draw list) is in it. Off, the recorder costs two flags per call and
a read of the clock per `Cmd`; on a backend other than Vulkan it says once that it needs
Vulkan and records nothing (the snapshots below are read back through the Vulkan
backend).

**What is recorded** is every call of the interface in that frame, in a stream per
plugin (`Dumps\<name>\<module>.a2s`, moved by the bench into `$A2_DATA/replays/<name>/`),
and `<module>.txt` beside it (records, snapshots, what could not be read):

- the objects the frame makes (`Gpu::texture`, `buffer`, `storage`, with what they were
  made with), the pipelines and compute pipelines it binds (the whole description, each
  shader blob once: d3dbc, SPIR-V, DXIL and its alpha-tested twin), the `Cmd`s begun and
  ended, the `Frame` each records into and **the viewport each starts with** (the
  engine's, which no call of the plugin's names: a `Cmd` on Vulkan and Direct3D 12
  starts with Display8's record of it, on Direct3D 9 with the device's; render3d's
  modules draw the action camera's inset with it, and without it the inset came out
  empty), `Frame::next`;
- every command: passes with their load operations, viewports, scissors, clears,
  pipelines, samplers, textures, uniform blocks (the floats as given), storage, writes,
  dispatches, barriers, the vertices and indices written into the frame's rings (their
  bytes and stride), static buffers drawn from, draws, mips, resolves, copies, texture
  writes. A draw names the span written it is part of, so a span a module narrows
  (`first`, `count`) is replayed narrowed;
- **what the frame finds made** (a texture, buffer or storage buffer made in an earlier
  frame, the engine's back buffer and depth, Display8's mirrors of the engine's
  textures): its description and, at its first use in the frame, its contents as they
  are then, every level and face (the contents it still holds when nothing has drawn
  into it, else read back through the Vulkan backend: `src/vulkan/tex/snap.rs`; a
  mirror has every level the engine's texture has, `GetLevelCount`). Reading back
  submits the frame so far and waits, which changes what the frame draws in nothing,
  only how long it takes. A target whose first use clears it whole needs no contents.

Contents are **blobs**, files in `blobs\` named by their FNV-1a hash and length
(`record/wire.rs`), so what both plugins read is written once; the stream holds the
rest, little-endian (`wire.rs` has the layout); `replay/inspect.py` lists what a
recording holds and each record's `seq`. Everything in a recording is derived from the game (the textures, the
frame), so it lives in `A2_DATA`, never in a checkout (hard rule 8).

**The frame as presented** comes with it: the UI's `Present` hook asks
`Gpu::record_present`, which ends the UI's stream at the engine's `Present`, and dumps
the back buffer as its `UI.dump` does, as `<name>-game` (`game.ppm` in the recording).
render3d's stream ends at its first `Cmd` of the next frame.

**The replay** (`a2replay.exe`, `no_std` Rust linking this crate, built by `replay.sh`
with the toolchain of the plugins) is a game without the game: it opens a window at the
recording's size, loads Display8 (`display/build/d3d8.dll`) in stub mode on the backend
asked for, with the recording's own `Display8.ini` (its `PixelCentres=`) and
`dxvk.conf` (the samplers' anisotropy and LOD bias), makes the device as the engine
does (`Direct3DCreate8`, `CreateDevice` at the recorded size and depth format), and
gives each stream a `Gpu` of its own on it (`Gpu::with_display`, `attach_engine`), as
each plugin has. `replay::Replay` then makes every call again in `seq` order:

- an object the frame made is made as it was; one it found made is made from its
  snapshot. A target found made (a sky's cube, a bake) is a sampled texture holding the
  snapshot while the frame only reads it, and is made a target at the first command
  that draws into it, its first level (each face's) drawn from the snapshot (`promote`);
- the engine's back buffer is Display8's, given the snapshot the first plugin to use it
  took (one image for every plugin: the UI's snapshot holds what render3d drew, and
  putting it back would hide render3d's replay): cleared to its colour when it was one
  colour, else drawn in; its depth is cleared to 1;
- a snapshot goes into a target through the replayer's blit (`replay/shaders/blit.hlsl`,
  point-sampled, the quad moved half a pixel on Direct3D 9): Direct3D 9 copies only
  between targets, and a texture with contents is not one;
- vertices go into the frame's ring with the stride they had (`Cmd::vertices` of a
  type that size), and each draw reads the span it read.

At the end the engine's back buffer is read back and written as a binary PPM,
`replay-<backend>.ppm`, beside `replay-<backend>.log` (what was replayed and skipped)
and the run's `Display8.log`. `replay.sh` runs it on a headless sway of its own at the
recording's size (as `display/vkprobe/run-wine.sh`), in a scratch prefix kept in
`~/.cache/a2replay/`, always on **lavapipe with the Khronos validation layer**
(`VK_ICD_FILENAMES`, `VK_INSTANCE_LAYERS`), whatever the backend: Vulkan is gpu's own
on Display8's device, `d3d9` is Display8's Direct3D 9 device through DXVK
(`platform/vendor/dxvk-3.1.1`), `d3d12` vkd3d-proton (`A2_PROTON`'s). The user's GPU is
never touched. Its validation messages are counted (Display8's messenger on Vulkan,
the layer's own lines under DXVK and vkd3d-proton).

**What a replay cannot have**: what the engine draws itself (nothing in stub mode,
which is the only mode a frame is recorded in), a multisampled target's samples and a
depth target's contents found made (read back neither; each starts cleared), a mirror's
levels when the `Texture` was had from `engine_texture` before the frame (one level
then), and timers. A frame recorded with compute (render3d's clustered lights, `[Lights]
Clustered=1`) dispatches on Vulkan and Direct3D 12 and does nothing on Direct3D 9,
where render3d would have drawn without clusters: record with `Clustered=0` to compare
Direct3D 9 with the others.

### Measured

`showcase` (`Seed=7 Step=16 Pause=200 Race=federation Hud=1`, `--clock bench`), 1280x720,
Display8 on Vulkan on lavapipe with `PixelCentres=d3d9` and the validation layer,
`[Lights] Clustered=0`, recorded paused, 2026-10-29. render3d's stream: 978 records, 57
snapshots (264 MB: the sky's 1536 cube in half floats is 113 MB of it, three 8-bit cubes
with their chains 75 MB); the UI's: 78 records, 8 snapshots (9 MB); the recording 282
MB in all, the two streams 720 KB of it. The frame the game presented, recorded or not, is the same to the
bit (RMSE 0 between `game.ppm` and a `UI.dump` of a frame not recorded). Replayed, each
against `game.ppm` (`a2test baseline diff`):

| Backend | RMSE (/255) | max | over 2/255 | validation messages |
|---|---|---|---|---|
| Vulkan (gpu's, on Display8's device) | **0** | 0 | 0 | 0 |
| Direct3D 12 (vkd3d-proton) | 0.058 | 27 | 35 px | 10, vkd3d-proton's own (`VUID-VkWriteDescriptorSet-dstSet-04611`, its mutable descriptors on lavapipe) |
| Direct3D 9 (DXVK 3.1.1) | 3.37 | 217 | 8.1% | 0 |

Vulkan replays the frame exactly. Direct3D 12 is within single pixels of it. Direct3D 9
is the d3dbc of the same shaders through DXVK on a CPU driver, which the port
measurements above already put furthest from the others ("The Vulkan backend",
"Measured": DXVK on lavapipe is not a reference); the differences are spread over the
hulls' plating, the beams and faintly over the sky, not a part missing. The first replays
found what a recording must carry beyond the calls: the viewport each `Cmd` starts with
(the inset came out empty without it), a mirror's X8R8G8B8 alpha (the minimap's sensor
circle was transparent), and that the engine's back buffer is one image for both plugins
(the UI's snapshot of it, put back, had hidden render3d's replay under the game's own
pixels).

## What is not there yet

Mip chains of a `DYNAMIC` texture; a second vertex stream (instancing: nothing uses
it); a 32-bit index ring (every index is 16-bit); stencil state (only ever off); render
targets in a volume's slices or a cube's levels below the first; async compute; a
storage image's levels below the first. Each joins
when something needs it. On Vulkan also: sRGB sampling (a sampler asking for it reads
linear; nothing does), sampling a multisampled colour target, render passes into a
volume. On Direct3D 12 also: sRGB sampling, render targets in a volume, a copy of part
of a depth target, a depth resolve from Display8's depth (which has no shader view), a
pipeline cache of its own on disk.
