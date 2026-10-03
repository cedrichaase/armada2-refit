# models

The game's 3D geometry, where the refit changes it:

- **The loading screen**, `SOD/logo.SOD`, widened to fill a wide screen
  (`logo-sod.py`, with its art from `loading-panel.sh`). It ships with the `LOADING`
  texture target, and `a2tex install`/`revert` move the two together. The reasoning is
  in `textures/README.md`, in the loading-screen section.
- **Planets**, `Planets.asi` (`planets.c`): the engine tessellates them finely enough
  for a modern resolution. Below.
- **Hull lighting**, `hull-bump.py`: the Federation hulls are lit per pixel through the
  engine's own dot3 bump path. Below. Not part of `./install` yet: run
  `models/hull-bump.py --install` (`--revert`, `--status`).

`./install` runs `install.sh`, which builds and installs `Planets.asi` and `Planets.ini`.
`a2mod` switches both, and the `SOD` backups, as the `models` layer.

## Planets.asi

### What was wrong

The planets on the maps look like polygons: at 3440x1440 the rim of a planet is a run
of straight segments, and the facets show in the terminator and in the cloud shell.

### The dead end: a finer SOD does nothing

Every class planet has a model, `SOD/PB_CLSSD/H/J/K/L/M.sod`, picked by the ODF's name
(`odf/stations/PB_CLSSH.odf` → `PB_CLSSH.sod`). Each is a 3ds Max `GeoSphere01` at
frequency 4, 162 vertices and 320 triangles, which would explain a polygon outline of
about 21 sides. A 5120-triangle re-tessellation of all six, keeping every UV, was built
and installed on the bench. **The planet it drew matched stock pixel for pixel**: 93
pixels of 102,000 differed by more than 3%, all of them the sun flare and the
planet's rotation. The engine does not draw the shape of that mesh.

### What the engine does instead

`Planet_Database` (constructor `0x595670`, from `armada2.map`) builds its own meshes:
`GroundMesh` (constructor `0x595ba0`) for the ground, named `GeoSphere01`/`GeoSphere02`,
and for the `Atmosphere` cloud shell, out of latitude/longitude angle tables
(`GroundMesh::sInitAngleTables`, `0x595d60`). It re-tessellates them as the camera moves:
`Planet_Database::RenderInternal` (`0x595910`) picks an angle step from the camera
distance *d* and the planet radius *r*,

    step = 2 · acos(1 − k · d / r),   clamped to [π/64, π/4]

and passes it to `GroundMesh::Recompute` (`0x596200`).

1 − cos(step/2) is how far the middle of a facet sags inside the true sphere, as a
fraction of *r*, so this holds the sag at *k · d* in world units. Seen through the
perspective camera, that is *k* times the focal length in pixels: a fixed number of
pixels at a given resolution. The constant was set for 640x480: **k = 0.0015625**
(1/640), and 0.00046875 in the preset view where `View_Record` is 2. At 1440 lines the
same *k* leaves facets several pixels deep.

### The hook

The two `flds` instructions that load *k* (`0x595972` and `0x59597c`) are pointed at the
plugin's own copies, divided by `Detail=` in `Planets.ini` (default 8). No other
instruction in `.text` reads the two constants. The [π/64, π/4] clamp stays as it is,
so the finest mesh the engine can be asked for is the one it always built for a camera
close in, and its angle tables are sized for that. `Detail` only decides how far out in
the zoom range that mesh is used.

Both sites' bytes, and the constants they load, are checked against this build before
either is written, as bit patterns: the exe's 0.00046875 is one unit in the last place
away from the nearest float to that decimal, so a float literal does not match it. A
different `Armada2.exe` leaves the plugin inert, and `Planets.log` says so. `Detail=1`
(or 0) patches nothing. The exe is patched in memory only. Removing `Planets.asi`,
`Planets.ini` and `Planets.log` uninstalls it completely.

On the bench at 3440x1440, stock draws the rim of a class H planet as straight segments.
With `Detail=8` it is a clean circle, the cloud shell included.
`testbench/scenarios/no-assets.md` checks that both sites are patched.

Built like the other plugins (`menus/build.sh`): clang, `lld-link`, no CRT.

## Hull lighting

### What the engine does

Traced with `testbench/d3dtrace` (its README has the frame breakdown). Every hull is
lit **per vertex on the CPU** and reaches Direct3D pre-transformed, with no normals.
The exception is a mesh whose material names a bump map. With *Graphics Settings →
Bump Mapping* on (the default), such a mesh is drawn through a dot3 vertex shader in
four passes: per-pixel N·L for each of the two directional lights, then the texture,
then the night-lights. In stock only the Borg have bump maps.

### How a SOD asks for it

Read from the 167 bump-mapped materials in the 25 Borg SODs, all spelled alike: the
lighting material `opaque` is **type 6** with **two** textures, the diffuse with word
`0` and then the bump map with word `0x200`. A plain material is type 4 with one
texture. The bump map is a 24-bit greyscale height map. The engine takes the normals
from its slope at load.

`hull-bump.py` rewrites the 59 plain `opaque` materials of the 36 Federation SODs in
`hull-bump.sha256` that way, from the stock bytes, with a `.a2neb-backup` of each.

### Why the height map is flat

A height map derived from the hull art was tried first: a high-pass of each texture's
luminance. It turns every painted speck into relief. In game the user called it
"ugly as hell": "it adds a lot of detail where there should be none". The lighting
itself read as better. A **flat** map keeps the per-pixel lighting and adds no relief,
because a constant height has zero slope everywhere. So every material names one
8x8 mid-grey map, `Textures/RGB/a2flatbump.tga`.

It is the one file this layer adds to `Textures/RGB`, and it gets its own name on
purpose. A stock texture that happens to be one colour (`Gshroud`, `Mdmoonglo`) would
also be flat. But it is a real texture, and whether the engine caches a texture loaded
as a bump map under the same name as the colour texture is not known. A clash would
draw the fog of war as a normal map, or light a hull from a wrong direction. A
unique name rules that out. `a2mod` switches the file with the SODs, and the texture
inventory skips it.

### Measured

First Federation mission, 1920x1080, refit, one camera. Grey mean / standard deviation
over each ship:

| | Bump Mapping off (stock path) | `hull-bump` |
|---|---|---|
| Enterprise-E | 52.3 / 70.3 | 43.0 / 60.7 |
| Akira | 69.2 / 71.9 | 54.5 / 55.5 |

Smooth light and shade across saucers and nacelles, painted detail unchanged, and
**18–21% darker**. The dot3 passes add no ambient or emissive term. The CPU path adds
the material's (0.18, 0.065, 0.065), the warm lift stock hulls have. Restoring it means
replacing the dot3 passes' colour maths in a plugin. *Bump Mapping: Off* in game
returns the hulls to the stock path at any time.

