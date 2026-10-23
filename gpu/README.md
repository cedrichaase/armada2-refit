# gpu

The graphics interface `Render3D.asi` (`render3d/`) and `UI.asi` (`ui/`) draw through:
one `no_std` crate, a path dependency of both, linked into each plugin on its own. It
is not a layer of `a2mod` and installs nothing; what it changes reaches the game
through the two plugins, and their changelogs say so.

It exists for the display plan: the game's d3d8 device becomes a null stub of ours
(Display8, `display/`), and render3d and the UI draw the whole frame on Vulkan, then
Direct3D 12. The interface is shaped for those two and implemented on two backends:
**Direct3D 9**, the device behind the game's d3d8 one (crosire's d3d8to9,
`platform/D3D9.md`, or Display8's own in its stub mode), and **Vulkan**, Display8's
device in its stub mode with `Backend=vulkan` ("The Vulkan backend" below). Which one
draws is decided at run time, by what Display8 says; the modules see one interface.
Moving both plugins onto it changed nothing on screen, so each move is judged by pixels
(the bench's baselines, `testbench/README.md`).

## Layout

| File | What |
|---|---|
| `src/types.rs` | the descriptions and small values: `Format`, `Usage`, `TextureKind`, `TextureDesc`, `Contents`, `BufferDesc`, `Sampler`, `Blend`, `Depth`, `AlphaTest`, `Cull`, `Topology`, `VertexAttr`, `Code`, `PipelineDesc`, `Rect`, `Viewport`, `Load`, `Pass`, `Block`, `Bindings` |
| `src/frame.rs` | `Frame`: the frame a `Cmd` records into, owned by the caller; `Span` |
| `src/pixels.rs` | `Mapped`, texels read back, and their conversion to RGB (the frame dumps) |
| `src/chain.rs` | a bake too long for one shader, as a chain of passes (`Chain`), on the interface |
| `src/api.rs` | `Gpu`, `Texture`, `Buffer`, `Pipeline`, `Timer`, `Cmd` as the plugins see them: each an enum of the two backends' own, every call going to the one it was made on; the backend chosen at run time |
| `src/d3d9/mod.rs` | the Direct3D 9 backend: `Gpu`, `Texture`, `Buffer`, `Pipeline`, `Cmd`, `Timer` |
| `src/d3d9/raw.rs`, `consts.rs` | Direct3D 9 through raw COM vtables, once `render3d/src/d3d` and `ui/src/d3d` (identical copies) |
| `src/display.rs` | the contract with Display8 (`display/`, the game's `d3d8.dll` of ours): the device, frame, upload ring, targets and mirrors it owns and exports, on Vulkan (`display8_vk_*`) and Direct3D 12 (`display8_dx12_*`), as `#[repr(C)]` types; its doc comment is the specification |
| `shaders/compat.hlsli` | what the shader compilers need apart: vkd3d (d3dbc), dxc to SPIR-V, dxc to DXIL (`render3d/README.md`, "One source, two compilers") |
| `shaders/fetch-dxc.sh` | the dxc that writes the DXIL, DirectXShaderCompiler v1.9.2609, fetched into `$A2_DATA/tools/` by version and sha256 (`render3d/hlsl-rs.sh` runs it when a DXIL file is stale) |
| `src/vulkan/mod.rs` | the Vulkan backend's `Gpu`: Display8's device opened, the recording state (the pass now, layouts), samplers, render passes, the uniform ring, Display8's images wrapped |
| `src/vulkan/tex.rs`, `pipe.rs`, `cmd.rs`, `mem.rs` | its textures, buffers and timestamps; pipelines; `Cmd` and the read-back; device memory in blocks |
| `src/vulkan/spirv.rs` | what it reads out of dxc's SPIR-V (reflection) and the two patches it makes to a pixel shader; built on the host too by `spirv-selftest.sh` |
| `shaders/vk_depth.hlsl`, `src/vulkan/shaders.rs` | its own shaders (the depth resolve), SPIR-V by `vk-shaders.sh` |

`gpu::Gpu`, `gpu::Texture` and the rest are `api.rs`'s, re-exported at the crate's
root.

## The interface

**`Gpu`** is the backends' state for one plugin: on Direct3D 9 the device, every
resource made through it (held weakly, to let go of them when the device goes) and the
state blocks; on Vulkan Display8's device and what the backend keeps for recording.
Each plugin keeps one in a static of its own (`ui/src/gfx`, `render3d/src/gfx`), made
with `Gpu::with_display(log, lookup)`, `lookup` its `GetProcAddress` on `d3d8.dll`, and
attaches the game's device to it every frame (`attach_engine`: the Direct3D 9 device
behind the engine's, or Display8's Vulkan one; `engine_id` names it without attaching).
Nothing in it is per frame. **The backend** is Display8's answer to `display8_backend`,
asked until it gives one: Vulkan from the engine's `Direct3DCreate8` when the stub runs
on it, Direct3D 9 otherwise, and Direct3D 9 for good with no Display8, an older one, or
once the engine has a device and Display8 has said nothing (forward mode). An object
made before the answer is made for Direct3D 9, and the log says so once.

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
  shader declares fits both: the base is Direct3D 9's truth, and a later backend gives
  each block a buffer binding, in the order a binding group lists them. The existing
  register layouts (`k_tile` at c29, the post passes' c0..c3) keep working unchanged.
- **Binding groups** of a few (texture, sampler) slots and uniform blocks per draw map
  onto a descriptor set and onto a root table alike. The sampler goes with its slot:
  Direct3D 9 has no separate sampler objects, and the newer APIs take static or
  combined samplers well.
- **Resources valid until dropped, and no device-loss callbacks.** Display8 will own
  the device, which is never lost then. The Direct3D 9 backend handles losing it
  inside (below), so no module is written around `Reset`.
- **Usage flags** (`SAMPLED`, `RENDER_TARGET`, `DEPTH`, `DYNAMIC`, `COPY_SRC`,
  `COPY_DST`) say what a texture is for, so a later backend can place its barriers.
  Direct3D 9 picks its pool and usage from them.
- **Per-frame state from outside.** Each plugin links this crate statically, so two
  copies of it run in one process, and later Display8 owns the frame and exports it
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
- **No compute yet**: Direct3D 9 has none. A `Cmd::dispatch` and a compute pipeline
  join when the Vulkan backend does; nothing in the interface stands in their way.

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
(`A2TEST_VULKAN_GPU=1`), after the port scenes have come through the validation layer
clean on lavapipe.

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
   they persist, as Direct3D 9's registers do; a later backend gives each draw the
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

## What is not there yet

Mip chains of a `DYNAMIC` texture; a second vertex stream (instancing: nothing uses
it); a 32-bit index ring (every index is 16-bit); stencil state (only ever off); render
targets in a volume's slices or a cube's levels below the first; compute. Each joins
when something needs it. On Vulkan also: sRGB sampling (a sampler asking for it reads
linear; nothing does), sampling a multisampled colour target, render passes into a
volume. Direct3D 12 carries the alpha test and the depth resolve as Vulkan does (the
patched pixel shader, a pass of its own).
