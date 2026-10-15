# models

The game's 3D geometry, where the refit changes it:

- **The loading screen**, `SOD/logo.SOD`, widened to fill a wide screen
  (`logo-sod.py`, with its art from `loading-panel.sh`). It ships with the `LOADING`
  texture target, and `a2tex install`/`revert` move the two together. The reasoning is
  in `textures/README.md`, in the loading-screen section.
- **Planets** are not here: their shape is not in their SOD, and they are drawn on a
  sphere of render3d's own (`render3d/README.md`, world module). What was learned
  before that is below, under "Planets".
- **Dilithium moons**, `SOD/Mdmoon*.SOD` and `Mmooninf.SOD`, smoothed by `moon-sod.py`.
  Below.
- **The selection bubble**, `SOD/select.sod`, rounded by `select-sod.py`. Below.
- **The hulls**, rounded where they are round by `hull-sod.py`. Below.
- **The asteroids**, `SOD/Master.SOD` and `Master1..8.SOD`, smoothed by
  `asteroid-sod.py`. Below.

`./install` runs `install.sh`, which smooths the moons and the asteroids and rounds the
selection bubble and the hulls (`.a2neb-backup` copies; `install.sh --remove` restores
them). `a2mod` switches the `SOD` backups as the `models` layer.

## Planets

The planets on the maps looked like polygons: at 3440x1440 the rim of a planet was a run
of straight segments, and the facets showed in the terminator and in the cloud shell.

### A finer SOD does nothing

Every class planet has a model, `SOD/PB_CLSSD/H/J/K/L/M.sod`, picked by the ODF's name
(`odf/stations/PB_CLSSH.odf` → `PB_CLSSH.sod`). Each is a 3ds Max `GeoSphere01` at
frequency 4, 162 vertices and 320 triangles. A 5120-triangle re-tessellation of all six,
keeping every UV, was built and installed on the bench. **The planet it drew matched
stock pixel for pixel**: 93 pixels of 102,000 differed by more than 3%, all of them the
sun flare and the planet's rotation. The engine does not draw the shape of that mesh.

`Planet_Database` builds its own meshes (`GroundMesh` for the ground, and the
`Atmosphere` cloud shell) out of latitude/longitude angle tables, and re-tessellates
them as the camera moves against a facet tolerance set for 640x480. The earlier
`Planets.asi` lowered that tolerance; `Render3D.asi` replaced it by drawing the planets
on a finer sphere of its own that carries the same UV layout, so the stock gore textures
land where they did. Don't try new planet meshes.

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
been measured with a full fleet. Only opaque meshes with something round in them are rounded, at the largest split up
to 4 that keeps a mesh within 12,500 triangles: a blended mesh (shields, glows) would be
drawn on the CPU every frame, and a box gains nothing. 176 of 225 hull models change; the
largest, a station, gains 32k triangles. Only v1.93 models, found by the eight zero bytes ahead of a
mesh and read back after writing; other versions are skipped.

## The asteroids

An asteroid field draws three to eight rocks out of the models the engine lists:
`master.sod`, `master1` to `master5`, `master4` again and `master7` (`Master6` and
`Master8` ship and are never drawn). Each is one mesh of 20 to 54 vertices (36 to 100
triangles; `Master.SOD`, version 1.6, 32 and 60), drawn at 2 or 2.7 times its size, so a
rock 40 units across has facets 10 units long and reads as a cut gem at any distance.
They are drawn from the engine's static vertex buffers, as stock always drew asteroid
fields. `hull-sod.py` leaves them alone: they are not hulls, and their material is
`default`, not `opaque`.

`asteroid-sod.py` uses `hull-sod.py`'s patches at `--split 4` (576 to 1,600 triangles a
rock), every stock vertex in place and every UV interpolated. Smoothed through every edge
(`--crease 180`) the rocks came out as eggs on the bench (`SCENE=asteroids`). Their faces
meet at a median of 22 to 39 degrees with a tail of sharp edges up to 138, and at
`--crease 55` the sharpest sixth stay edges: broken ridges with rounded faces between.
No patch folds over (a triangle turned against the stock face it was cut from) at any
crease up to 70; at 90 two do on `Master7`. The surface detail is not geometry: it is
render3d's recipes `Master1` and `Master2` (`render3d/surfaces/`), lit per pixel by its
hulls module (`render3d/README.md`, world module).

`Master6` is the one model whose node matrix is a mirror and whose triangles are wound
the other way round, which cancel under the engine's cull. render3d's hull shaders
take any mirrored draw to need the opposite cull (the Steamrunner's case), so `Master6`
would draw inside out there; since no field draws it, nothing was changed for it.

## Bump maps on Federation hulls

`hull-bump.py` (models 3.2.0) gave the 36 Federation SODs the Borg's bump-mapped
material with one flat height map, `a2flatbump.tga`, to get per-pixel lighting out of
the engine's dot3 path. Since lighting 1.11.0 `Lighting.asi`, and now render3d, lights every hull per
pixel in its own shaders (`render3d/README.md`, hulls), and a flat map gives a
patched hull exactly a plain hull's shading there. So it was removed in models 4.0.0.

What a SOD needs to ask for a bump map, read from the 167 bump-mapped materials in the
25 Borg SODs (in stock only the Borg have them): the lighting material `opaque` is
**type 6** with **two** textures, the diffuse with word `0` and then the bump map with
word `0x200`. A plain material is type 4 with one texture. The bump map is a 24-bit
greyscale height map, which the engine turns into a normal map at load. Such a hull goes
through render3d's bump path, which lights it with the Borg profile
(`render3d/README.md`, hulls). Federation hulls with real relief would take
that spelling, a height map of their own, and a profile of their own. A map derived from
the hull art (a high-pass of each texture's luminance) was tried and rejected in game: it
turns every painted speck into relief.
