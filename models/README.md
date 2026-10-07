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
- **The selection bubble**, `SOD/select.sod`, rounded by `select-sod.py`. Below.

`./install` runs `install.sh`, which builds and installs `Planets.asi` and `Planets.ini`
smooths the moons and rounds the selection bubble (`.a2neb-backup` copies; `install.sh --remove` restores them).
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

That was judged without `d3d9.cachedWriteOnlyBuffers`. Before postfx 1.1.1, the CPU
path read back every vertex it wrote from a buffer DXVK keeps in GPU memory, which made
it far slower than its triangle count suggests (`postfx/README.md`, "Reading back a
dynamic vertex buffer"). The shell's cost with that key set has not been measured.

## The selection bubble

### What was wrong

The translucent ellipse around each selected ship shows its polygon: a run of straight
segments around the rim, with corners, and flat facets in its shading. It is
`SelectionEffect`, an instance of `SOD/select.sod` scaled to the ship's shield ellipse
(`postfx/README.md`, "Reading back a dynamic vertex buffer"). The model is a 3ds Max
GeoSphere at frequency 4: 162 vertices, 320 triangles, about 20 segments around a great
circle, every vertex at radius 89.943 about the origin. Like the moons, and unlike the
planets, the engine draws the mesh the SOD stores.

### A finer sphere

`select-sod.py` cuts each triangle into `--split N` x N and pushes every new vertex out
onto the sphere. A true sphere allows that, and the moons' PN patches exist only to keep
a shape that is not round. Stock vertices keep their place and index, the winding is the
stock one, and the mesh's one dummy texcoord stays the only one. The file is SOD v1.92,
which `moon-sod.py`'s parser does not read. The script finds the mesh by requiring it to
end at the file's closing seven bytes, the stock file is pinned by hash, and `--split 1`
reproduces it byte for byte.

Seen on the bench (3440x1440, four selected Galaxy class, close and at play distance):
`--split 2` (1,280 triangles, about 40 segments around) is round, with no corners on the
rim, and `--split 4` (5,120) cannot be told from it at either distance. So the default
is 2.

### What it costs

The bubble is blended, so it goes down the CPU path (`RenderInternalNonVB`), as the
moons' glow shell does, and its cost grows with the face count. With
`d3d9.cachedWriteOnlyBuffers`, 30 selected stock bubbles took 0.60–0.81 ms
(`postfx/README.md`). Four times the faces suggests about 3 ms at 30 selected. That is
an estimate from that measurement, not a measurement of its own.

## Rounder hulls

A saucer is a ring of flat facets, about 16 degrees of arc each (the Galaxy's: 22 around
its outline), and a new vertex on a flat face stays flat, so splitting triangles does
nothing. `hull-sod.py` uses the moons' curved patches (above) with one change, because a
hull has real corners the moons do not. An edge is *smooth* when its two faces meet at
under `--crease` degrees (40); a corner's normal averages only the faces reachable through
smooth edges, and a *hard* edge is drawn on its own curve on both sides, so patches still
meet exactly. Two things came out of the bench: a saucer's rim is itself a hard edge (top
against bottom), so with hard edges straight the outline did not change at all; a vertex
where exactly two hard edges meet at a gentle turn now gives the edge the chain's tangent.
Measured on the Galaxy saucer's outline: turn per facet 16.4 to 8.8 degrees at `--split 2`,
which still read as facets at 3440x1440; `--split 4` reads as an ellipse. Box edges,
pylons and nacelle ends stay crisp. Cost: 4x the triangles per `--split 2`, 16x at 4 (the
Galaxy goes 759 to 12,144); an opaque hull is drawn from a vertex buffer, but this has not
been measured with a full fleet. Only v1.93 models, found by the eight zero bytes ahead of a
mesh and read back after writing; other versions are skipped.

## Bump maps on Federation hulls

`hull-bump.py` (models 3.2.0) gave the 36 Federation SODs the Borg's bump-mapped
material with one flat height map, `a2flatbump.tga`, to get per-pixel lighting out of
the engine's dot3 path. Since lighting 1.11.0 `Lighting.asi` lights every hull per
pixel in its own shaders (`lighting/README.md`, "Shaders"), and a flat map gives a
patched hull exactly a plain hull's shading there. So it was removed in models 4.0.0.

What a SOD needs to ask for a bump map, read from the 167 bump-mapped materials in the
25 Borg SODs (in stock only the Borg have them): the lighting material `opaque` is
**type 6** with **two** textures, the diffuse with word `0` and then the bump map with
word `0x200`. A plain material is type 4 with one texture. The bump map is a 24-bit
greyscale height map, which the engine turns into a normal map at load. Such a hull goes
through `Lighting.asi`'s bump path, which lights it with the Borg profile
(`lighting/README.md`, "The Borg profile"). Federation hulls with real relief would take
that spelling, a height map of their own, and a profile of their own. A map derived from
the hull art (a high-pass of each texture's luminance) was tried and rejected in game: it
turns every painted speck into relief.
