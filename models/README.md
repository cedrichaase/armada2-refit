# models

The game's 3D geometry, where the refit changes it:

- **The loading screen**, `SOD/logo.SOD`, widened to fill a wide screen
  (`logo-sod.py`, with its art from `loading-panel.sh`). It ships with the `LOADING`
  texture target, and `a2tex install`/`revert` move the two together. The reasoning is
  in `textures/README.md`, in the loading-screen section.
- **Planets**, `Planets.asi` (`planets.c`): the engine tessellates them finely enough
  for a modern resolution. Below.
- **Dilithium moons**, `SOD/Mdmoon*.SOD` and `Mmooninf.SOD`, smoothed by `moon-sod.py`.
  Below.

`./install` runs `install.sh`, which builds and installs `Planets.asi` and `Planets.ini`
and smooths the moons (`.a2neb-backup` copies; `install.sh --remove` restores them).
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

## The dilithium moons

### Unlike the planets, the SOD is what is drawn

A dilithium moon is an ordinary model (`classLabel = "scrap"`), not a `Planet_Database`,
so the engine draws the mesh its SOD stores. `mdmoon` is placed 447 times across the
maps and `mmooninf` (the inexhaustible moon) 161 times. Each of the four files holds two
meshes: the rock (`dmoon*`, textured `Mdmoon`) and a glow shell around it (`sphere2`/
`sphere3`, untextured, one texcoord). Both are 18-segment spheres, 146 vertices and 288
triangles, about 20° of arc per facet.

The rocks are deliberately not round: the radius runs 24.6–33.9 on `Mdmoon`/`Mdmoon2`
and 14.1–44.6 on `Mdmoon3`. (`Mmooninf`'s is a true sphere.) Pushing new vertices onto a
sphere, which would round a planet, would iron that shape out.

### Curved patches through the stock vertices

`moon-sod.py` replaces each triangle of the rock with a curved point-normal (PN)
triangle: a cubic patch through the triangle's corners that leaves each corner along
the surface's own direction there (the area-weighted normal of the faces around it),
sampled on a `--split N` grid. Every stock vertex stays where it is, neighbouring patches
meet exactly along their shared edge, and each new vertex takes the UV interpolated
inside the stock triangle it was cut from, so the texture keeps the mapping it was
painted for. Measured at `--split 2`: the median angle between neighbouring faces falls
from 14–16° to 7–8°, and the radius range moves by at most 0.37 units (`Mdmoon3`'s
deepest hollow, 14.13 → 13.76; 0.13 on `Mdmoon`).

The SOD versions differ, and one parser reads both: v1.8 (`Mdmoon*`) gives a mesh a
texture-material name and a texture name and ends with two empty animation tables;
v1.6 (`Mmooninf`) opens with an extra texture block, gives a mesh only a texture name,
and lists three animation entries. The parser finds the material table by requiring the
whole file to parse to its end. Every stock file is pinned by hash, and `--split 1`
reproduces it byte for byte.

### Why the glow shell stays stock

`--split 4` on both meshes looked better and cost a frame rate that was obvious in game;
`--split 2` on both still cost a noticeable amount. The cause is the shell. It is
blended, and `ST3D_Mesh::RenderInternal` (`0x6325d0`) takes `RenderInternalNonVB`
instead of `RenderInternalVB` whenever `ST3D_DeviceDirectX8::PolygonSortRequired` says
the material in use is translucent. That path transforms the mesh on the CPU and sorts
its triangles every frame, for every moon on the map. The rock is alpha-tested, not
blended, and draws from a vertex buffer. So by default only the rock is smoothed, at
`--split 2` (1,152 triangles, the shell's 288 unchanged), which the user judged a good
trade in game. `--glow` smooths the shell too.
