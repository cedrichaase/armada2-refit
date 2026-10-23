# gpu

The graphics interface `Render3D.asi` (`render3d/`) and `UI.asi` (`ui/`) draw through:
one `no_std` crate, a path dependency of both, linked into each plugin on its own. It
is not a layer of `a2mod` and installs nothing; what it changes reaches the game
through the two plugins, and their changelogs say so.

It exists for the display plan: the game's d3d8 device is to become a null stub of
ours (Display8), and render3d and the UI are to draw the whole frame on Vulkan, then
Direct3D 12. The interface is shaped for those two and, for now, implemented on
Direct3D 9 only, the device behind the game's d3d8 one (crosire's d3d8to9,
`platform/D3D9.md`). Moving both plugins onto it changes nothing on screen, so the move
is judged by pixels alone (per-pixel RMSE 0 against the bench's baselines, frame time
no worse), apart from the change of API that comes later.

## Layout

| File | What |
|---|---|
| `src/types.rs` | the descriptions and small values: `Format`, `Usage`, `TextureKind`, `TextureDesc`, `Contents`, `BufferDesc`, `Sampler`, `Blend`, `Depth`, `AlphaTest`, `Cull`, `Topology`, `VertexAttr`, `Code`, `PipelineDesc`, `Rect`, `Viewport`, `Load`, `Pass`, `Block`, `Bindings` |
| `src/frame.rs` | `Frame`: the frame a `Cmd` records into, owned by the caller; `Span` |
| `src/pixels.rs` | `Mapped`, texels read back, and their conversion to RGB (the frame dumps) |
| `src/chain.rs` | a bake too long for one shader, as a chain of passes (`Chain`), on the interface |
| `src/d3d9/mod.rs` | the Direct3D 9 backend: `Gpu`, `Texture`, `Buffer`, `Pipeline`, `Cmd`, `Timer` |
| `src/d3d9/raw.rs`, `consts.rs` | Direct3D 9 through raw COM vtables, once `render3d/src/d3d` and `ui/src/d3d` (identical copies) |
| `src/display.rs` | the contract with Display8 (`display/`, the game's `d3d8.dll` of ours): the device, frame, upload ring, targets and mirrors it owns and exports, on Vulkan (`display8_vk_*`) and Direct3D 12 (`display8_dx12_*`), as `#[repr(C)]` types; its doc comment is the specification |
| `shaders/compat.hlsli` | what the shader compilers need apart: vkd3d (d3dbc), dxc to SPIR-V, dxc to DXIL (`render3d/README.md`, "One source, two compilers") |
| `shaders/fetch-dxc.sh` | the dxc that writes the DXIL, DirectXShaderCompiler v1.9.2609, fetched into `$A2_DATA/tools/` by version and sha256 (`render3d/hlsl-rs.sh` runs it when a DXIL file is stale) |

`gpu::Gpu`, `gpu::Texture` and the rest are the backend in use, re-exported at the
crate's root; a later backend is chosen at build time behind the same names.

## The interface

**`Gpu`** is the backend's state for one plugin: the device, every resource made
through it (held weakly, to let go of them when the device goes) and the state
blocks. Each plugin keeps one in a static of its own (`ui/src/gfx`, `render3d/src/gfx`)
and attaches the game's device to it every frame (`attach`, `attach_d3d8`). Nothing in
it is per frame.

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
creation, or read the engine's viewport.

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
render3d's `frame/` (finding the d3d9 device, the view's transforms, the hooks of
`Reset` and `DestroyRenderSurfaces`) and `gfx::engine_texture` (an `ST3D_Texture`'s
d3d9 texture, wrapped). No module draws with it. Display8 will answer those questions
itself, and they go then.

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
when something needs it. A Vulkan or Direct3D 12 backend has two of today's features to
carry differently: the alpha test (a variant of the pixel shader that discards) and
the depth resolve (a resolve attachment, or a shader).
