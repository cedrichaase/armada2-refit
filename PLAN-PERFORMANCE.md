# Plan: renderer performance follow-ups (phase 14)

This plan comes from the renderer performance review of 2026-11-05. The review was
read-only: nothing was built, run or measured. It covered `main` at root 15.6.21, and
its references were checked again against `main` at 15.10.0: the cited files that
changed in between still show what is described, at the line numbers given. It covered
five areas:

- `gpu/`, the Vulkan and Direct3D 12 backends;
- `display/`, Display8, the Direct3D 8 layer and its passthrough;
- `render3d/core`, the frame, the post chain, the clusters and the per-frame CPU work;
- the `world` and `sky` modules, with the nebula march;
- the other draw modules: hulls, effects, ordnance, particles, specials and overlays.

**Nothing below is a measurement.** Each finding gives the code it is based on
(`file:line` at 15.10.0) and an estimate of its cost. Before a change ships, measure it
with `render3d/perf/ab.sh` (`render3d/PERF.md`, "A/B: any two checkouts or commits"),
on the bench, alone, with `RUNS=3` for anything under about 0.1 ms. Judge a change that
can alter the image in game, not on a table.

It is a plan, not a commitment of dates. Effort is given in ROADMAP units: agent hours,
bench passes and sign-off round.

## Before measuring anything

- **Timing runs include the tracer.** `Trace=auto` turns Display8's call tracer on in
  every bench clone (`display/src/lib.rs:248-252`). The timing scripts never set
  `Trace=0`, so every `Timing=1` figure includes:
  - a spinlock and a hashed table lookup on every call;
  - a stack scan of up to 16 KB on every draw and on every buffer or texture
    `Lock`/`Unlock` (`display/src/trace.rs:121`).

  PF-0 sets `Trace=0` in `perf/probe-new` and `perf/ab.sh`, and takes a new baseline.
- **`render3d/PERF.md` is out of date.** Its method and its first-measurement section
  still describe the Direct3D 9 path through DXVK. gpu 2.0.0 removed that path, and the
  `recorded_block` it names is gone. Its numbers are from 2026-10-15. PF-0 measures
  again on today's backends and rewrites its method.

## What holds

The review found much that is already right, and this plan keeps it:

- **Frames and uploads.** Two frames in flight, and no queue or fence waits in the hot
  path (`vk_flush` runs only for read-backs). Memory is suballocated, and the uniform
  ring is mapped once.
- **Redundant binds are filtered.** Pipeline, vertex buffer and index buffer binds that
  repeat are skipped. Direct3D 12's SRV, storage and UAV tables are cached per frame.
- **The resolve and the bloom filters.** The MSAA resolve is the hardware's, limited to
  the view's rectangle. The bloom filters are already cheap: a 4-tap prefilter and
  dual-filter levels.
- **The sky is baked once per recipe** into a cube. Each frame costs one cube read per
  pixel.
- **The nebula march is already economical.** It runs 8 geometric steps, bounded by
  the scene's depth and the gas box, with an occupancy column, baked self-shading and
  temporal accumulation. Nothing shades per MSAA sample.
- **Several modules already batch.** Particle looks are cached, the particle batch
  reuses its buffers, and ordnance effects, welds and sparks go out as one vertex list
  each.
- **The off switches cost almost nothing.** With tracing off, the tracer is one relaxed
  load. `Timing=0` and the frame-dump poll are close to free.

## What the review found

### Draw structure and synchronisation (gpu, render3d)

| # | Finding | Evidence | Estimate |
|---|---|---|---|
| P1 | **Every `Cmd` ends the render pass, and every render pass drains the GPU.** render3d opens a `Cmd` per hull per draw mode, per rock and so on (34 `gfx::cmd()` sites), so a battle frame has roughly 100–300 of them. `leave()` closes the pass. The next `open()` creates a new framebuffer and begins a pass with LOAD/STORE on the 4x MSAA half-float target. Every render pass has external dependencies of ALL_COMMANDS → ALL_COMMANDS, MEMORY_WRITE → MEMORY_READ\|WRITE, on both sides | `gpu/src/vulkan/mod.rs:600-614` (`leave`), `:789-805` (dependencies); `gpu/src/vulkan/cmd.rs:231-235` (`create_framebuffer`, retired per open); `render3d/mods/hulls/src/draw.rs:448`, `world/src/rocks.rs:398` | Hundreds of full drains per frame: the largest likely GPU idle. About 5–10 µs CPU per `Cmd` |
| P2 | **Image transitions are one ALL_COMMANDS barrier each.** Each one has MEMORY_* access masks. `open()` issues one per attachment, `write_storage` and `dispatch` put a global barrier on each side, and the post chain's 10 passes and the resolve each pay them | `gpu/src/vulkan/mod.rs:1124-1141`; `render3d/core/src/frame/post.rs:218-245`; `render3d/core/src/scene/clusters.rs:312,327` | ~20 drains per view in the post alone, twice with the HUD inset |
| P3 | **Texture descriptor sets are rewritten too often (Vulkan).** A new set 1 is allocated and written whenever the view list changes, on the first draw of every `Cmd`, and on every pipeline change. All pipelines share one layout, so the rewrite on a pipeline change is unnecessary. The storage set is rewritten per `Cmd` for clustered pipelines | `gpu/src/vulkan/cmd.rs:319` (`tex_dirty = true` in `set_pipeline`), `:600`, `:674-701`; Display8's pool is sized at 4096 sets a frame (`display/src/vk/mod.rs:830`) | Low tens of µs per frame, rising with the number of draws |
| P4 | **Constants upload both stages whenever either changes.** `vs_block`/`ps_block` mark a stage dirty even when the data is the same. Any dirty flag re-uploads both register files up to their high-water mark. Hulls read c0–c19 in the VS and up to c103 in the PS (the clusters' constants sit at `PS_BASE=100`), so a world-matrix change uploads ~2 KB instead of 320 B. On Direct3D 12 it also re-sets 16 root CBVs | `gpu/src/vulkan/cmd.rs:351-365, 702-712`; `gpu/src/dx12/cmd.rs:613-626`; `render3d/core/src/scene/clusters.rs:38` | ~2 KB per hull draw; tens of µs per frame |
| P5 | **Fixed work for every `Cmd`.** `enter()` runs the garbage `collect()` every time, which drains the whole list into two new Vecs even when the frame has not changed. `Cmd::new` allocates and zeroes three register files (~11.5 KB) | `gpu/src/vulkan/mod.rs:121-140, 563`; `gpu/src/dx12/mod.rs:196-217, 574`; `gpu/src/vulkan/cmd.rs:94-95, 113` | Grows with `Cmd`s × garbage |
| P6 | **Allocations on every draw in `prepare`.** Vulkan clones the texture list and collects attachment Vecs. Direct3D 12 clones the whole `PassState`. Samplers are found by linear search under a RefCell borrow | `gpu/src/vulkan/cmd.rs:597-609, 678-684`; `gpu/src/dx12/cmd.rs:521-528`; `gpu/src/vulkan/mod.rs:700-702`; `gpu/src/dx12/mod.rs:749-751` | 4–6 heap allocations per draw |
| P7 | **Direct3D 12's root signature is near its limit.** 58 of 64 DWORDs: 8 spaces × 2 root CBVs, 16 single-sampler tables and 8 root constants. Every constant upload sets 16 CBVs, and every draw does 16 BTreeMap lookups for sampler tables | `gpu/src/dx12/mod.rs:326-337`; `gpu/src/dx12/cmd.rs:606-612` | Driver-dependent; unmeasured |

### Hitches: work done the first time something is drawn

Nothing in the draw modules is prepared at map load. Each item below is built on the
game thread the first time it is needed. Together they are the likely cause of
PERF.md's worst view of 7.8 ms in the `showcase` scene.

| # | Finding | Evidence | Estimate |
|---|---|---|---|
| H1 | **The fireballs' noise volume is built at the first explosion.** It is three 64³ FFTs and ~786k `sin`/`cos` on the CPU. `reset()` drops it, so a device reset builds it again | `render3d/mods/effects/src/explosions.rs:206-227`, called from `draw` at `:239`; dropped at `:135` | Tens of ms, once. The log line `effects: explosion noise 64^3 in N ms` gives the real figure |
| H2 | **A dying ship's debris is broken up in the frame it dies.** `mesh.unindexed()` inserts into a sorted Vec, which is O(V²). The clustering rescans every unused triangle per chunk, and `seen.contains` is O(k²). One index buffer is made per mesh | `render3d/mods/effects/src/chunks.rs:250-273, 277`; `crates/a2fmt/src/sod.rs:166-191` | Several ms for a cube or a station |
| H3 | **Pipelines are compiled at their first draw.** `ready()` builds only the shader modules. Hulls alone have 7 pixel shaders × 5 modes × 3 culls, so the first cloak, ghost, faded hull or mirrored node each pays. So do the explosions, shields, the shields-down effect and the debris. Display8's disk cache and pipeline library help only later sessions | `gpu/src/vulkan/pipe.rs:240-264`; `gpu/src/dx12/pipe.rs:255-277`; `render3d/mods/hulls/src/draw.rs` `pipe()` | One driver compile per new key, on the game thread |
| H4 | **Textures, models and normal maps are made at first draw.** `assets::texture` decodes the TGA, builds the mips and uploads them. `scene::model` parses the SOD and builds its buffers. `normal_map` builds a Borg normal map and its mips per pixel on the CPU | `render3d/core/src/assets/mod.rs:231`; `render3d/core/src/scene/mod.rs:245`; `render3d/mods/hulls/src/draw.rs` `normal_map` | Per new class on screen |
| H5 | **Display8 reads files on the render thread, whether tracing is on or not.** `drop::present` opens and parses `Display8.ini` every 60 presents. `detail::present` checks for `Display8.go` every 30 | `display/src/drop.rs:122`; `display/src/detail.rs:223` | 0.1–1 ms each under Wine: a periodic spike |
| H6 | **A SUBOPTIMAL present rebuilds the swap chain.** `Ok(true)` from `vkQueuePresentKHR` does `device_wait_idle` and `make_swapchain`. A surface that stays suboptimal does this every frame | `display/src/vk/mod.rs:1242` | Conditional; not observed |
| H7 | **The pipeline-cache check copies the whole cache.** Every 600 frames it fetches the full blob only to compare its length | `display/src/vk/mod.rs:1305`; `display/src/vk/cache.rs:55` | A multi-MB copy every ~10 s |

### The nebula march (world)

The README measures the march at 4.8 ms at 8 steps, with filter and composite at 0.8 ms
(1920x1080, RX 5700, inside a cloud: `render3d/README.md`, "world", the march table).
PERF.md gives the volume pass as 3.7 ms. Either way it is the largest GPU cost in the
renderer.

| # | Finding | Evidence | Estimate |
|---|---|---|---|
| N1 | **The march, its temporal pass and its composite run at full resolution**, one ray per pixel, on ~2.1M pixels at 1080p. The gas's structure is several pixels wide: the volume's texels are 6.6 units there. The output is already filtered by a 1.5 px Gaussian and about ten frames of history | `render3d/mods/world/src/nebulae.rs:1621-1640` (targets at `depth.size()`), `:1663-1747, 1829-1907` | Half resolution: about 4x fewer rays, perhaps 3.7 → 1.2–1.5 ms. See PF-6 for the look |
| N2 | **The 3D reads use `SampleGrad` with explicit gradients.** The gradients are a scalar per step (`k_ray * s * footprint * freq`), so `SampleLevel` with an analytic level gives the same result. Explicit gradients cost address registers in a fully unrolled loop. The README's unexplained "~2.5 ms of the march does not depend on the step count" may be register pressure | `render3d/mods/world/shaders/world_nebula.hlsl:242, 250, 508, 516-517, 619, 625, 631` (8 sites) | 10–25% of the march; check the VGPR count and occupancy in RGP first |
| N3 | **Temporal and composite are two full-resolution passes** with a half-float round trip between them. The temporal pass reads ~40 texels per pixel. The composite reads 25 depth and 25 gas texels and does 25 `exp` and 25 `view_z` divides | `world_nebula.hlsl:801-925` | ~0.5 ms as one compute pass with a groupshared tile; no change to the arithmetic |
| N4 | **The baked volume has no mip chain.** It is up to 256x160x256 BGRA8 and is sampled at level 0. Zoomed out, the reads are minified and thrash the texture cache | `nebulae.rs:2101` | Gain in zoomed-out views; far gas slightly softer |
| N5 | **The march's second target is RGBA16F**, but only `.xy` are used | `nebulae.rs:1631` | RG16F: 0.05–0.1 ms, no change to the image |
| N6 | **The gas pass is set up before the frustum cull.** The depth copy and the clear of two full-resolution targets run even when every box is then culled | `nebulae.rs:1320-1325` against the cull at `:1411` | 0.05–0.1 ms with the gas off screen |
| N7 | **Each bake layer is read back synchronously** (up to 160). This happens at load and whenever the nebulae or the Key change | `nebulae.rs:2077` | A hitch, not frame time; a candidate for the loading screen (PF-4) |

The shadow lookup in each step (`key_lit`, `world_nebula.hlsl:290-307`) could be cheaper
behind a branch. That would break the rule that shader loops have no branch inside
(`CLAUDE.md`, "render3d"), so it is listed only as a rotation identity: rotate the
pixel's disc sample by a constant angle per step, in place of two `sincos` and two
`sqrt` per step.

### The post chain and the clusters (render3d core)

At 1080p with 4x MSAA in RGBA16F, the post moves about 150 MB per view: 66 MB read and
16.6 MB written for the resolve, the rest for the bloom and the final pass. That comes
to 0.35–0.4 ms at the RX 5700's bandwidth, which matches PERF.md. The post is limited by
bandwidth, and the resolve is more than half of it.

| # | Finding | Evidence | Estimate |
|---|---|---|---|
| C1 | **The light clusters' depth slices cover empty space.** With the default `Distance=1000000`, `P22 ≈ 1.00002` fails the `> 1.0001` test, so far becomes `near × 1e4 = 200000`. The 24 slices then span 13.3 octaves, each 1.47x deeper than the last, and the 300–4000 units where ships and lights are fall in about 7 of them. Deep clusters hold more lights, so every lit hull, planet and rock pixel loops over more of them. In battles the cap of 24 per cluster also drops lights | `render3d/core/src/scene/clusters.rs:290-291`; `render3d/core/shaders/clustered.hlsli` `cl_add` | Possibly the largest GPU win in heavy scenes; no change to the image (fewer lights dropped) |
| C2 | **The bloom levels are RGBA16F.** They hold only RGB, which is never negative | `render3d/core/src/frame/post.rs:109` | R11G11B10: ~7–8 MB per view, ~0.02 ms; needs a `gpu::Format` |
| C3 | **The resolve is its own transfer pass.** The MSAA target moves to TRANSFER_SRC, which may make the driver decompress its FMASK and DCC data | `post.rs:210`; `gpu/src/vulkan/cmd.rs:879-885` | A resolve attachment on the scene's last pass: plausibly 0.03–0.08 ms |
| C4 | **The small bloom levels cost mostly pass overhead.** Levels 2–4 (240x135 to 60x34) do almost no work; most of their cost is the drains of P2 | `post.rs:18` (`BLOOM_LEVELS=5`), `:218-245` | 20–60 µs per view; one compute dispatch with groupshared memory |
| C5 | **The cluster build's compute shader keeps per-thread arrays in scratch memory.** `idx/sc/gr[KMAX]` are 96 dynamically indexed dwords per thread, and each batch runs a 64-step LDS loop per thread for the prefix. It also dispatches when there are no lights | `render3d/core/shaders/clusters.hlsl:90-92, 127`; `clusters.rs:312, 327` | Plausible; use a wave prefix or ballot |
| C6 | **An R11G11B10 scene target, as an option.** It would about halve the post and the bandwidth of every blended draw. There is no destination alpha (`sprites.rs` `blend_of` maps DESTALPHA factors), and it bands near 1.0 | `post.rs:72-85` | Opt-in at most, behind a setting, with dither |

### Per-frame CPU work (render3d)

| # | Finding | Evidence | Estimate |
|---|---|---|---|
| U1 | **String allocations per sprite and per quad.** `eng::look` scans and allocates a `String` per shot and spark, batching compares the strings, and each run looks up its texture by a new lowercased key. `mem::take` throws the queue's capacity away every frame. The sky's 770 star quads each allocate a lowercased `String` | `render3d/core/src/scene/sprites.rs:32, 86, 132, 148`; `render3d/core/src/engine/sprite.rs:86`; `render3d/mods/sky/src/sprites.rs:61` | A good part of the effects' 0.1–0.2 ms and the sky's 0.2 ms |
| U2 | **Asset and model lookups allocate a lowercased key on every call.** This covers `draw_part`'s texture per part per frame, `model()`, and `sod_of` per instance per frame | `render3d/core/src/scene/mod.rs:245, 421`; `render3d/core/src/assets/mod.rs:231`; `render3d/mods/hulls/src/lib.rs:217` | 20–100 µs in battles with debris and rock fields |
| U3 | **The hull sprite nodes (running lights, fires) are found by reverse linear scans.** Each lookup scans the whole sprite table with `eq_ignore_ascii_case`, per node, per object, per view. Each quad is its own draw and clones its texture name | `render3d/mods/hulls/src/sprites.rs:94, 114-118, 151, 271-277`; `crates/a2fmt/src/spr.rs:335-346` | 0.1–0.3 ms; resolve once per model as `particles/src/look.rs:62` does |
| U4 | **`points_near` rescans every light for every part.** Each call does a `sqrt`, a BTreeMap flag lookup and a key recomputation per light, allocates two Vecs and sorts | `render3d/core/src/scene/lights.rs:238-261`, from `scene/mod.rs:427-436` | Tens of µs with many lights |
| U5 | **Debris chunks are drawn one by one with no cull.** There are up to `MaxChunks=1500`, each with 448 bytes of constants, a scan of the lights and a lowercased texture name | `render3d/mods/effects/src/chunks.rs:436-450` | Large in the seconds after a big ship dies |
| U6 | **Hull draws redo per-part work every frame.** A texture name is lowercased per part, `flipbook` scans, the materials are compared by string, an `order` Vec is allocated per draw, and `model_sphere`/`model_box` are recomputed | `render3d/mods/hulls/src/draw.rs:426-650` (`:541, 590, 617, 693`) | Under half of the hulls' time |
| U7 | **Posing covers off-screen objects too, in both views.** Hulls pose every object in the list before the caster test, and particles pose each emitter's SOD again | `render3d/mods/hulls/src/lib.rs:664-671`; `render3d/mods/particles/src/sim.rs:537` | ~0.07 ms per view, could be halved |
| U8 | **Torpedo models do string work and a full draw per shot per view** | `render3d/mods/ordnance/src/models.rs:125, 179, 204, 233` | Rises with salvo size |

### Shaders in the draw modules

| # | Finding | Evidence | Estimate |
|---|---|---|---|
| S1 | **The hull shadow map is an R32F colour target plus D24**, read with 16 point taps per pixel. A depth-only map with a comparison sampler gives the same tent filter in 4 bilinear `SampleCmp` taps | `render3d/mods/hulls/src/shadow.rs`; `hulls.hlsl:128-145` | 12 fewer texture reads per hull pixel, and one less colour write |
| S2 | **The fireball march writes `SV_Depth`, which turns off early-Z.** Pixels hidden behind hulls are shaded through 28 steps × 3 3D reads | `render3d/mods/effects/shaders/effects_explosions.hlsl` `fire_ps` | `SV_DepthGreaterEqual` keeps early-Z; steps scaled by on-screen radius |
| S3 | **City lights are computed on the day side.** `city_night()` (18 `town_at`, 5 Voronoi searches, ~160 `sin`) runs for every pixel of a colonised planet's city pass, and its result is multiplied by `night` afterwards | `render3d/mods/world/shaders/world_planet.hlsl:210-255`; `planets.rs:561-575` | 0.1–0.3 ms with a colonised planet filling the view; a branch outside any loop, same image |
| S4 | **The sky reads its fine detail with `Fine` off.** `fine_z` does three 3D reads per sky pixel while its amount is 0 | `render3d/mods/sky/shaders/sky.hlsl:346-364`; `sky/src/lib.rs:279-282` | ~0.05 ms; same image |
| S5 | **The shield shaders cost a lot of noise.** `down_ps` evaluates 18 `gnoise` (8 `hash33` each). Both shield shaders draw an unculled ellipsoid, so the front and back faces are both shaded | `render3d/mods/effects/shaders/effects_shields.hlsl:194-212` | Medium visual risk: the user settled these looks |

### The passthrough (Display8)

With the shipped config the engine reaches the passthrough only in the shell. These
costs matter when a module or UI part is off, for hand-backs and for the Borg bump
passes. Some of them may be overtaken by AR-7 (Display8 on `gpu`, `PLAN-ARCHITECTURE.md`).

| # | Finding | Evidence |
|---|---|---|
| D1 | Static vertex and index buffers are copied into the ring on every draw. `buf_lock` ignores its flags, so there is no write generation to cache against | `display/src/passthrough/mod.rs:142-168`; `display/src/vk/pass.rs:603-612`; `display/src/stub/res.rs` |
| D2 | A `Vec<u32>` of fan indices is allocated per fan draw, and the engine's 2D is mostly fans | `display/src/passthrough/mod.rs:200`; `passthrough/state.rs:133` |
| D3 | The fixed-function constant blocks (1008 bytes, two 4x4 multiplies, a memcmp) are rebuilt on every draw, and every lit draw allocates its lights Vec | `display/src/passthrough/state.rs:246, 265`; `display/src/vk/pass.rs:589-601` |
| D4 | Pipelines, samplers and texture sets are found by linear search on every draw | `display/src/vk/pass.rs:361, 449, 644`; `display/src/dx12/pass.rs:169` |
| D5 | A texture `Unlock` re-uploads the whole level mid-frame with two ALL_COMMANDS barriers. It closes the passthrough's render pass. Direct3D 12 has no staging path, and its 4 MB ring chunks are never freed | `display/src/stub/res.rs:346, 355`; `display/src/stub/mod.rs:249`; `display/src/vk/mod.rs:929-943`; `display/src/dx12/mod.rs:756` |
| D6 | A back buffer `Unlock` uploads the whole screen even after a READONLY lock (cutscene frames, 14 MB at 1440p) | `display/src/stub/res.rs:496-510`; `display/src/stub/mod.rs:318-340` |
| D7 | `discard` in `ps_ff` turns off early-Z for every passthrough draw, opaque ones included | `display/shaders/ff.hlsl:264` |
| D8 | Many `once(&sfmt!(…))` sites build the string before the trace gate, so they allocate with tracing off | `display/src/trace.rs` callers |

## Work packages

The packages are listed in order. PF-0 to PF-6 are specified below, and the rest are
sketched. Each package is one branch, with its own changelog entries, and is measured
with `ab.sh` against `main`.

| WP | What | Depends on | Agent time · bench · round |
|---|---|---|---|
| PF-0 | A measurement baseline: `Trace=0` in the timing scripts; PERF.md re-measured on Vulkan and Direct3D 12 and its method rewritten | nothing | 1–2 h · 3 timing runs · none |
| PF-1 | Display8's quick fixes: H5, H6, H7, D8 | nothing | 2–3 h · 1 · with the next round |
| PF-2 | The gpu fixed costs: P5, P6, and in P3 the `set_pipeline` rewrite and a per-frame set cache | PF-0 | 3–5 h · 1–2 pixel-identical · with the next round |
| PF-3 | Render passes and barriers: P1 (framebuffer cache, narrow dependencies), P2 (batched transitions with exact stages), P4 (per-stage constants) | PF-2 | 6–10 h · 2–3 pixel-identical, Vulkan validation and lavapipe · a round |
| PF-4 | **Preparation on the loading screen**: H1–H4 and N7 | PF-0 | 8–14 h · 3–4 · a round |
| PF-5 | The CPU lookups: U1, U2, U3, U4, U6 | PF-0 | 4–8 h · 1–2 pixel-identical |
| PF-6 | **Nebula quality as a player option**: N1, with N2, N5 and N6 first | PF-0 | 8–14 h · 3–5 · a round, judged in game |
| PF-7 | The clusters: C1 (depth range fitted to the lights), C5 | PF-0 | 3–6 h · 2 |
| PF-8 | Batching across `Cmd`s: one `Cmd` per mode for all hulls and rocks; instanced debris (U5) and torpedoes (U8); sprite nodes in one run per texture | PF-3 | 8–16 h · 3–4 pixel-identical |
| PF-9 | The post: C2, C3, C4 | PF-3 | 4–8 h · 2 |
| PF-10 | Shader items: S1, S2, S3, S4, N3, N4 | PF-0 | 1–4 h each |
| PF-11 | The passthrough: D1–D7, unless AR-7 replaces this code first | AR-7 decided | 6–12 h · passthrough scenarios (`p10-vk-*`, `p10-d12-*`) |
| — | Not planned: C6 (R11G11B10 scene target, a fidelity risk) and S5 (the settled shield looks). Each needs the user's word first | — | — |

### PF-0: a baseline worth comparing against

**Goal:** a `Timing=1` figure that measures the renderer, not the bench's tracer.

- `render3d/perf/probe-new`, `probe-old` and `ab.sh` set `Trace=0` in the clone's
  `Display8.ini`. The bench's tracing stays on for every other run.
- With `--no-validate` (CLAUDE.md, "display": timing runs only), `RUNS=3`, take
  `showcase`, `match` and `nebula` at 1920x1080 on Vulkan. Take `showcase` on
  Direct3D 12 too, within the one-game limit.
- Add a zone count for `Cmd`s, render passes and barriers per view to `Timing=1`'s
  report. P1 and PF-8 are judged by those counts as much as by milliseconds.
- **Acceptance.** PERF.md's method describes today's chain and its results table is
  from this run, with the noise floor measured again.

### PF-4: preparation on the loading screen

**Goal:** nothing is first built in the middle of a battle. Everything a mission can
draw is built while the loading screen is up, and the loading screen keeps animating
while it happens.

**What is built.**

- **Once per session:**
  - the fireballs' noise volume (H1), kept across `Reset` on Vulkan and Direct3D 12,
    where Display8's device is never lost;
  - every pipeline permutation of every enabled module, for the scene pass's key
    (formats, samples) (H3). This needs `ready()` in `gpu` to take a pass key and build
    the pipeline itself, not only its modules.
- **Once per mission:**
  - for every class that can appear (the map's objects and the classes each player's
    race can build): the SODs through `scene::model`, the textures through
    `assets::texture`, and the Borg normal maps (H4);
  - the debris partition of each of those classes (H2). Only velocity and spin are
    left random at death, and a per-frame spawn budget caps the rest;
  - the nebula bake (N7), when the map has nebulae;
  - the sky bake, which already happens once per recipe; it moves here if it is not
    already done before the first frame.

**How it reaches the loading screen.** The UI owns the loading screen. Its hooks on
`LoadBarNext`'s status update, `LoadDatabase` and `LoadPixelData` already present
frames of ours between the engine's steps at 30 a second (`ui/LOADING.md`, "Frames
inside a step"). render3d must not hook the same call sites.

- render3d exports `Render3D_Prepare(budget_us) -> remaining`. Each call does work for
  up to `budget_us` from a queue, and returns how much is left. The UI finds it with
  `GetProcAddress`, as it finds `Render3D_MenuWarp`.
- The queue is filled when the load begins, from `GameSetup::getMapName` and the
  players' races, the same data `loading::begin` reads.
- The UI calls it after each of its frames, with the budget left in the frame period
  (about 25 ms of 33). The screen keeps animating, and a step that is already long gets
  no longer.
- Each queue item records where it came from in `Render3D.log`
  (`prepare: <what> in N ms`), so the next PF-0 run can see what was moved.
- **Without the UI's loading screen** (`[Loading] Enable=0`, or no UI.asi), render3d
  empties the queue in one go when it first sees the mission's frame. The hitch then
  comes once, before play, instead of in battle.
- **In a network game** the loading screen does not wait for this work: a load is
  already timed against the other players. A budget call never holds the load past the
  engine's last step. Whatever is left goes to the first frame, as above.
- The work is render only. Nothing the simulation reads is written, so a player with it
  can still play a player without it.

**Acceptance.**

- In `showcase` and `match`, the worst view drops, and no `prepare:` line appears after
  the mission's first frame.
- The loading screen's frames keep coming as often as `ui/LOADING.md` measures them now
  (30 a second between the engine's steps).
- The load's total time grows by no more than the work moved, measured from the
  splash to the first frame.

### PF-6: nebula quality as a player option

**History.** The old `Nebulae.asi` drew half-size slices. render3d drew the gas at half
size too, until render3d 0.4.0 (2026-10-16) replaced the half-size pass, its depth-weighted composite
and the edge refinement with today's full-size march. That removed the `[Nebulae]` keys
`Resolution=` and `EdgeRefine=` (`render3d/CHANGELOG.md`; `render3d/README.md`, "world").
The full-size march was chosen for its look: hull edges stay a pixel sharp against the
gas. Any return to half resolution is therefore **the player's choice, never the new
default**.

**Goal:** a player with a weaker GPU can trade the nebulae's sharpness for frame rate on
the Refit screen. A player who leaves the setting alone sees exactly today's gas.

- **First, the changes that do not alter the image:** N2 (`SampleLevel`), N5 (RG16F),
  N6 (cull before setup). These help every quality level and are measured on their own.
- **A half-resolution path for the march, not the old slices.** The depth copy also
  writes a half-resolution linear depth (the far depth of each 2x2, with the near one
  kept for the upsample). The march and `temporal_ps` run at half size, with the
  in-pixel jitter widened to two full-resolution pixels. `comp_ps` becomes a joint
  bilateral upsample from the four nearest half-resolution texels, falling back to the
  nearest in depth where the scene's depth differs by over 5%, as today's filter does.
  This keeps hull edges as close to a pixel sharp as the half grid allows.
- **The option.** It is one row in `render3d/mods/world/settings.toml`:

  ```toml
  [[row]]
  name = "nebulae_quality"
  section = "Nebulae"
  key = "Quality"
  kind = "enum"
  default = "high"
  values = ["low", "medium", "high", "ultra"]
  labels = ["Low", "Medium", "High", "Ultra"]
  apply = "live"
  scope = "local"
  tab = "graphics"
  label = "Nebula quality"
  help = "How finely nebula gas is drawn. Lower is faster on a weaker GPU, with softer gas at ship edges."
  ```

  The levels map onto keys that already exist, plus one new key:

  | Level | Resolution | `Steps=` | `Temporal=` |
  |---|---|---|---|
  | Low | half | 8 | 0.1 |
  | Medium | half | 12 | 0.1 |
  | High (default, today's look) | full | 8 | 0.1 |
  | Ultra | full | 12 | 0.1 |

  An explicit `Steps=` (or the new `Resolution=`) in an ini still wins over the level,
  for the bench and for modders. The row is `apply = "live"`: the march's targets are
  re-made at the new size, and the history is dropped for one frame.
- **A quality slider for the whole renderer is a later step**, and the settings contract
  has to allow it first. One `Graphics quality` row would set several rows at once:
  this one, `MSAA`, the shadow map's size, and later others. A2SET_ABI 1.1 has no
  preset row that sets other rows, so that is an addition to the contract
  (`tools/settings/README.md`) and a decision for the user. Until then each option is
  its own row, and the levels above are what such a preset would select.
- **Acceptance.**
  - At High, the image is pixel-identical to today's (`--clock bench`), and its time is
    no worse.
  - At Low, the `nebula` scene's march zone falls by at least half.
  - The user has judged Low and Medium in game, at the Mutara edge and inside a cloud,
    with ships moving across the gas.
  - `PLAYING.md` names the option.

### The rest, sketched

- **PF-1.** Gate the `[Drop]` reload and the `Display8.go` check behind `bench_clone()`
  or the tracer being on. Rebuild the swap chain only on `OUT_OF_DATE`, or on
  SUBOPTIMAL when the extent or the transform changed. Query the cache's size alone,
  and only when a pipeline was made since the last save. Add a `once_fmt!` that checks
  the gate before formatting.
- **PF-2.** `collect` returns at once when the frame has not changed. Register files go
  inline or into a buffer kept in `Gpu`. `prepare` borrows instead of cloning. Each
  slot's sampler is cached when it changes. A per-frame set cache is keyed by
  (binding, view, sampler), as `srv_tables` is on Direct3D 12.
- **PF-3.** Framebuffers are cached per frame by (render pass, views, extent).
  External dependencies are narrowed to the attachment stages, since sampled, copied
  and compute hazards already get their own transitions. Transitions are batched into
  one barrier call with stage and access masks per layout. Constants get a dirty flag
  and an offset per stage, and identical data is not marked dirty. The Vulkan
  validation layer and the lavapipe runs are the gate.
- **PF-7.** `gather` fits the clusters' near and far to the lights' own view depths,
  plus and minus their reach. A pixel outside that range is clamped to the first or
  last slice, which no light reaches anyway, so no light is lost. The cluster build is
  skipped when there are no lights.
- **PF-8.** The largest structural change: draw every hull of one mode in one `Cmd`, and
  every rock of one field. It needs the modules' draw loops reshaped, so it waits for
  PF-3 to show how much of P1 remains once the passes are cheap.

## Open decisions for the user

1. PF-6: is the four-level row right, or is a plain Full / Half toggle enough? Should a
   renderer-wide quality preset come now, as a settings-contract addition?
2. PF-4: is an extra second or two of loading acceptable, worst case in a long campaign
   load, to have no first-use hitch in battle?
3. C6 and S5 stay unplanned unless the user asks for them.
