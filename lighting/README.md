# Lighting — ships on the GPU, two scene lights

`Lighting.asi` changes how Armada II lights its ships and stations, in two parts that
`Lighting.ini` switches separately:

- **`GPU=1`**: ships and stations are drawn through the engine's own static vertex
  buffers. That is fixed-function Direct3D: the GPU transforms and lights them, where stock
  does both on the CPU.
- **`Lights=1`**: each map's own lights are replaced by two of the plugin's: a warm key
  (`KeyColour`, `KeyAxis`) and a dim blue fill from the opposite side (`FillColour`,
  `FillAxis`). Every renderer lights from that one list, so the CPU, GPU and bump-mapped
  paths agree.

`./install` runs `install.sh`; `install.sh --remove` takes the three files out again
(`Lighting.asi`, `Lighting.ini`, `Lighting.log`). The exe is patched in memory only, and
`a2mod` switches the plugin as the `lighting` layer. Each launch writes `Lighting.log`:
`call sites patched 6` means every hook took, and the lines after it name every model
switched to vertex buffers and the map's own lights.

## Settings

| Key | Default | Meaning |
|---|---|---|
| `GPU` | `1` | static vertex buffers for every game object type's model |
| `Lights` | `1` | replace the map's lights with Key and Fill |
| `KeyColour`, `FillColour` | `1.00 0.96 0.90`, `0.10 0.12 0.24` | linear RGB, 1 = full |
| `KeyAxis`, `FillAxis` | `0.35 -0.80 0.50`, the negation | the light matrix's third axis, in the engine's own convention: the stock key on the first Federation map is `0 -0.707 0.707` and lights the hulls from above |
| `Ambient` | `0.10 0.10 0.12` | light every GPU-drawn surface gets, whatever its direction |
| `FixMirrored` | `1` | light meshes that a model mirrors back with its node matrix the right way round (below) |
| `Log` | `1` | write `Lighting.log` |

Requires *Hardware Vertex Processing* on (Graphics Options; the default). With it off
the engine keeps every mesh on the CPU and only the lights part applies.

## How the engine draws a mesh

Traced with `testbench/d3dtrace` and read from `armada2.map`. `ST3D_Mesh::Update`
(0x631c10) picks one of three renderers from the mesh's flag word: the dot3 bump path
(`ST3D_Dot3_MeshVB`, bit 0x2, which stock gives only the Borg), a plain vertex-buffer
path (`ST3D_Standard_MeshVB`, bit 0x1), or neither, the CPU path. The CPU path lights,
transforms, clips and sorts every frame and hands Direct3D screen-space triangles.

Bit 0x1 is set only by `ST3D_Database::EnableStaticVertexBuffers(bool)` (0x6207a0),
which flags every mesh of a model and rebuilds it. Stock calls it from the
`AsteroidFieldClass` constructor and from the debug command `ToggleAsteroidVertexBuffers`,
so only asteroids ever used the GPU path. `ST3D_Standard_MeshVB::DeviceIsSupported`
(0x63e310) requires the *Hardware Vertex Processing* option and a device with hardware
T&L. `ST3D_Mesh::RenderInternal` (0x6325d0) still sends a draw to the CPU path while a
per-render effect is attached (cloak, warp-in), as stock does for asteroids.

## The hooks

| Where | What | Why |
|---|---|---|
| call at 0x4ccd66 and 0x4ccd7e, in the `GameObjectClass` constructor (once per ODF) | wraps `FindLogicalDatabase` / `FindVisibleDatabase`; every plain `ST3D_Database` returned gets `EnableStaticVertexBuffers(true)` | what the asteroid field does, for every object type. A `Planet_Database` is skipped: it re-tessellates itself every frame |
| call at 0x63e4e4, `ST3D_Standard_MeshVB::Render` | replaces `SetMaterial`: diffuse white, ambient zero, emissive `Ambient=`; and turns `NORMALIZENORMALS` on | below |
| slot 3 of the `ST3D_Standard_MeshVB` vtable (0x6bcbdc), `Render` | reverses the enabled lights for a draw whose object matrix (0x7ad640) is mirrored | below |
| call at 0x597f83, `GameObject_PreRenderAll` | wraps `ST3D_GraphicsEngine::RegisterLight`: the frame's first directional light is replaced by Key and Fill, the map's others are dropped | below |
| call at 0x598193 | wraps `GameObject_PreRenderAll` | counts the frame for the hook above |

Each call site is checked (an `E8` to the expected function, the `SetMaterial` call's
bytes, the vtable slot's address) before any is written. A different `Armada2.exe`
logs `NOT PATCHED` and is left alone.

### The material

`Render` (0x63e450) hands Direct3D the SOD's lighting material as it stands: its
diffuse colour, and its ambient as both ambient and emissive. The CPU path
(`LightVertices_Lambert`, 0x646bd0) never tints the texture by that diffuse colour,
and many SODs carry a leftover one. On the bench the GPU path showed it at once: pure
red (1, 0, 0) on the freighter's cockpit, brick (0.745, 0.227, 0.157), a purple seam
along the Galaxy's back. Some materials also have a reddish ambient that turned a whole
hull pink as emissive. So the plugin's material is diffuse white with its own neutral
emissive. Feeding in the engine's colour at +0x28, which the CPU path also adds, was
tried: on the first Federation map it is (0.66, 0.34, 0.34), and every hull went pink.

`NORMALIZENORMALS` is off in stock, which on this path made lighting depend on a
model's scale.

### Mirrored meshes

Some models carry a mesh built mirrored and mirrored back by its node matrix. The
Akira's distant mesh (`Fcruise1.sod`, the 258-face one the engine switches to when
zoomed out) is drawn with its near mesh's matrix with the X axis negated, determinant
−1. On the vertex-buffer path it lit as if its normals pointed the other way: dark and
blue from above, bright from below, flipping at the zoom where the engine changes mesh.
Its normals agree with its winding, and its world matrix is a clean rotation apart
from the mirror; negating that mesh's normals put it right on the bench. The plugin
does the equivalent without touching the mesh: for a draw under a mirrored matrix it
reverses the enabled lights' directions and restores them after (N·−L = −(N·L)).

Two approaches that failed: flipping every mesh whose normals point "inward" from its
centre flipped the correct ones too (by that measure most stock meshes point inward),
and turned the near hulls dark.

### The lights

A map's lights are game objects of class `dlight` that the mission editor placed: 64
of the 72 maps have two, 6 have three, 2 have one, each with its own colours. On the
first Federation map they are a white key and a fairly strong blue fill (0.05, 0.29,
0.53); with the GPU path that blue lit the Akira's top. `GameObject_PreRenderAll`
(0x597f30) registers them every frame, and the CPU path, `PreRender` of both
vertex-buffer classes and the dot3 shader's constants all read the one list. A
registered light is a colour and a `Matrix34`, and a directional light shines along
the matrix's third axis (`ST3D_Standard_MeshVB::PreRender`, 0x63e340, reads floats
6–8). The plugin builds a symmetric matrix (a reflection taking z to the wanted axis),
so the third row and column agree however a reader indexes it.

## Not covered yet

- Planets keep their own renderer (`Planet_Database`), and the Borg and any hull
  `models/hull-bump.py` patched keep the dot3 path, which takes precedence over the
  vertex buffers.
- Translucent materials still go through the CPU path for sorting (see the moons in
  `models/README.md`).
- Seen on the first Federation campaign map only. Other races, combat effects and frame
  rate have not been measured.
- The nebulae as light sources: the engine already has `Nebula::Simulate_Nebula_Lights`
  (0x4a5160).
