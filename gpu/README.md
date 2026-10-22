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
| `src/types.rs` | the descriptions and small values: `Format`, `Usage`, `TextureDesc`, `Contents`, `BufferDesc`, `Sampler`, `Blend`, `Depth`, `Cull`, `Topology`, `VertexAttr`, `Code`, `PipelineDesc`, `Rect`, `Viewport`, `Load`, `Pass`, `Block`, `Bindings` |
| `src/frame.rs` | `Frame`: the frame a `Cmd` records into, owned by the caller; `Span` |
| `src/pixels.rs` | `Mapped`, texels read back, and their conversion to RGB (the frame dumps) |
| `src/d3d9/mod.rs` | the Direct3D 9 backend: `Gpu`, `Texture`, `Buffer`, `Pipeline`, `Cmd`, `Section` |
| `src/d3d9/raw.rs`, `consts.rs` | Direct3D 9 through raw COM vtables, once `render3d/src/d3d` and `ui/src/d3d` (identical copies) |

`gpu::Gpu`, `gpu::Texture` and the rest are the backend in use, re-exported at the
crate's root; a later backend is chosen at build time behind the same names.

## The interface

**`Gpu`** is the backend's state for one plugin: the device, every resource made
through it (held weakly, to let go of them when the device goes) and the state
blocks. Each plugin keeps one in a static of its own (`ui/src/gfx`, `render3d/src/gfx`)
and attaches the game's device to it every frame (`attach`, `attach_d3d8`). Nothing in
it is per frame.

**Resources** are made once from a description and stay valid until the last clone is
dropped, across `Reset`s and devices. `Gpu::texture(TextureDesc, Contents)` makes a 2D
texture: sampled, a colour target, a depth target, multisampled, or rewritten from the
CPU (`Usage::DYNAMIC`, `Texture::write`). The contents are kept, so the texture can be
made again. `Gpu::buffer` makes a vertex or index buffer. `Gpu::pipeline(PipelineDesc)`
makes an immutable pipeline: shaders, vertex layout and stride, topology, blend (with a
separate alpha equation), write mask, depth test and bias, cull, whether the scissor
applies, the targets' formats and the sample count. Each is made on the device at its
first use; `ready` makes it at once and says whether the device took it.
`Gpu::wrap_d3d9_texture` / `wrap_d3d9_surface` and `engine_target` / `engine_depth`
wrap a Direct3D 9 object someone else made (the engine's minimap picture, its back
buffer) without owning its creation.

**`Frame`** is what a `Cmd` records into: its number, the rings the frame's dynamic
vertices and indices are written to (`Cmd::vertices`, `Cmd::indices`, giving a `Span`
to draw from in the same frame), and what is retired until the frame is over. The
caller owns it and calls `next` once per presented frame.

**`Cmd`** records commands (`Gpu::begin(&mut frame)`): `begin_pass(&Pass)` with its
colour targets and optional depth, each with a load operation (`Load`, `Clear(value)`,
`DontCare`) acting inside the pass's area, and `end_pass`; `set_pipeline`;
`bind(&Bindings)` (or `bind_texture` / `vs_block` / `ps_block` one by one); `draw`,
`draw_indexed`, `draw_buffer`; `set_viewport`, `set_scissor`, `clear_rects`; `resolve`
and `copy` between targets; `read_back` of a target. Uniforms go in blocks of float4
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
body, and the engine and the modules not yet ported draw into it.

**Pools.** Targets and depth surfaces are in the default pool; a texture filled once
goes to the default pool through a system-memory copy released at once, so no address
space is held for it; a `DYNAMIC` texture is in the managed pool, its contents copied
in when it is next bound after a `write`; buffers are in the default pool.

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

### Transitional

`gpu::d3d9::raw` is the old `d3d` module: what render3d's modules not yet ported draw
with, and what the plugins' engine glue uses to reach the game's d3d8 device (`query`,
`Com`, `vcall!`). `Gpu::device`, `Gpu::section`, `Texture::raw_texture` /
`raw_surface` and `Buffer::raw` let raw code and `gpu` code meet: a module calling raw
Direct3D 9 between `Cmd`s keeps working, inside a `Section` (render3d's `gfx::Guard`
is one). These go when the last module is ported.

## Porting a render3d module

The frame (`render3d/src/frame/`) and the UI are on `gpu`; the modules under
`render3d/src/mods/` are not yet. For each module, in its own commit, judged by the
bench's pixels (RMSE 0 against a baseline taken before the change, the module's
scenario, frame time no worse):

1. **List its states.** For each draw, note every `set_rs`, `set_ss`, shader, declaration
   and FVF it sets, and which it relies on from an earlier draw or from the engine.
   A state the module never sets is the engine's: find what the engine leaves there
   (`get_rs` logged once) before deciding the pipeline's value, which then sets it.
2. **Make its pipelines** once, in the module's own state: one `gfx::gpu().pipeline(desc)`
   per distinct combination, from `PipelineDesc::new(Code::d3d9(&VS), Code::d3d9(&PS),
   &LAYOUT, stride, topology, colour)` and its fields (`blend`, `depth`, `cull`,
   `scissor`, `write_mask`). Draws into the view take the scene pass's formats and
   samples from `frame::scene_formats()`. `Shader`, `Decl` and the module's
   `slot_caps` check go: a refused shader is logged by `gpu`, and `set_pipeline`
   returns false.
3. **Turn its constants into blocks**: each run of `vs_const` / `ps_const` registers
   becomes a `#[repr(C)]` struct of float4s bound with `Block::of(base, &s)`, at the
   registers the shader declares (do not renumber them; another branch ports the
   shaders).
4. **Turn its textures into `gpu::Texture`s** (`Contents::Owned` for what it uploads,
   `Usage::RENDER_TARGET | Usage::SAMPLED` for its bakes) and each `set_texture` with
   its sampler states into a `TexSlot` with a `Sampler`. An engine texture it reads is
   wrapped (`wrap_d3d9_texture`).
5. **Draw through a `Cmd`.** In place of `let _g = Guard::new(dev)`, take
   `let Some(mut cmd) = gfx::gpu().begin(gfx::frame())`. The scene pass is bound
   already; a module that draws into a target of its own (a bake, the shadow map)
   begins that target's pass, and `frame::scene_pass(&mut cmd, v.rect)` before it
   draws into the view again. `DynVb::push` becomes `cmd.vertices(&data)` and
   `cmd.draw(&span)`; a static buffer is `gfx::gpu().buffer(...)`.
6. **Drop its device handling.** What its `reset`, `forget` and `device` did for
   Direct3D objects goes: `gpu` lets go of them and makes them again. What they did for
   CPU state (caches keyed by device, half-built bakes) stays.
7. **Check** that the module no longer names `gpu::d3d9::raw`, build both plugins, and
   hand it to the bench. A module that keeps one raw call for now marks it
   **transitional** in a comment, with the reason.

## What is not there yet

Cube and volume textures (the sky's cube, the nebulae's volumes) and their uploads;
mip chains of a `DYNAMIC` texture; a second vertex stream (instancing); a 32-bit index
ring; stencil state; GPU timestamp queries (render3d's `timing` still uses raw ones);
compute. Each joins when the first module that needs it is ported.
