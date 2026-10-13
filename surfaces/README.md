# Surfaces — planets made from recipes, with depth

`Surfaces.asi` makes a planet's surface from a recipe, in shaders, in place of the
painted texture: a height field (terrain, mountains, craters, fissures, ocean basins),
coloured by height, with clouds above it. Like the sky (`sky/README.md`), it is a
function of direction, here from the planet's centre in its own frame, so it has no
seams, and nothing in it comes from the game's art: a recipe is a few dozen numbers,
the look is this folder's code, and it can be redistributed like the rest of the
repository.

    surfaces/install.sh             build, then install Surfaces.asi, Surfaces.ini and Surfaces\*.ini
    surfaces/install.sh --remove

**It draws nothing itself.** `Lighting.asi` already draws every planet in shaders,
per pixel, with the scene's lights (`lighting/README.md`, "Planets on the GPU"), and
the dilithium moons' and the asteroids' rock with its hull shaders. When `Surfaces.asi` is loaded and has
a recipe for the texture, `Lighting.asi` samples the recipe's cubes instead of the
texture and lights the terrain's own normal. Without `Surfaces.asi`, without a recipe,
without `Lighting.asi`'s `PlanetShaders=1` (for planets) or `Shaders=1` (for the models),
or with no Direct3D 9 device behind d3d8, the stock texture draws as before.

**Recipes: every class planet, the dilithium moon and the asteroids.** `PB_CLSSD`,
`PB_CLSSH`, `PB_CLSSK`, `PB_CLSSL`, `PB_CLSSM` (the ground of classes D, H, K, L, M),
`PB_CLSSJ` (class J, a gas giant drawn on its atmosphere shell), `Mdmoon` (the dilithium
moons' rock, in all four moon models), and `Master1` and `Master2` (the asteroid fields'
rocks, "The asteroids" below). The named story planets (`Mearth`, `MQonos`, ...) are
ordinary models with their own art and are left alone.

## Why the gores had to go

A class planet's texture is a four-lobe gore unwrap per hemisphere (`textures/README.md`,
"The class planets"): two mirrored diamonds with black wedges between them, 256 texels
for a sphere that can fill half the screen. Upscaling it keeps it a picture of a sphere
cut into diamonds. A function of direction has no lobes to meet, and is as sharp as its
cube is big.

## The contract with Lighting.asi

`surfaces.h` is the whole interface. `Surfaces.asi` exports one function,

    int a2_surface(void *dev8, const char *name, A2Surface *out);    /* cdecl */

which `Lighting.asi` finds with `GetProcAddress` at its first planet draw (the ASI loader
loads `Lighting.asi` before `Surfaces.asi`, so not at startup). `name` is a texture name:
the planet class's `groundTextureName` for a ground hemisphere (the ODF's, without the
`1`/`2` the engine appends per hemisphere), its `atmosphereTextureName` for the shell,
or a hull mesh's first texture. It returns 1 with three cube textures and four
constants, 0 when the name has no recipe, -1 when it has one that cannot be drawn now.
The first call for a recipe bakes it.

Two plugins rather than one because the draw and the look are separate layers:
`./a2mod` can switch the surfaces without switching the lighting, and a recipe can be
tuned without touching `Lighting.asi`. They could not each hook the planet's draw: both
would want slot 11 of the `GroundMesh` vtable.

**Where Lighting.asi uses it.**

- *A ground hemisphere* (`GroundMesh`, pass 0 of `ST3D_PlanetaryMaterial`): `surface_ps`
  in `lighting/planet.hlsl` in place of `ground_ps`. The city pass (pass 1, a developed
  planet's cities) stays as it is, over the new ground.
- *The atmosphere shell* of a class with an `atmosphereTextureName` recipe (class J):
  `surface_ps`, opaque. Every other class has a stock shell too, `PA_ECFR0` by default,
  painted with cloud: over a ground made from a recipe it keeps only its halo at the
  limb (`cloud_ps` with its texture weighted 0, `c47`), since the recipe brings clouds
  of its own. Without that, the stock shell's brown cloud streaked the new ground.
- *A hull mesh* (`hook_dip`, the vertex-buffer path) whose first texture has a recipe:
  `moon_ps` in `lighting/hull.hlsl` in place of `hull_ps`. Lighting.asi finds the texture
  through the mesh: `ST3D_Mesh` +0x114 points at the mesh's list of textures (the first
  is what `ST3D_Mesh::RenderInternal` hands the device's `SetTexture`, at 0x63275f), and
  every `ST3D_DatabaseElement` keeps a copy of its name at +0x8 (its constructor,
  0x620f70, stores it there; `ST3D_Node` and `ST3D_Texture` call it). Taking +0x114
  for the texture itself, as a first try did, read a name from the list and crashed.
  Names without a recipe are remembered by pointer, so each ship texture is asked
  about once. The cubes are looked up about the mesh's bounding-sphere centre
  (`ST3D_Mesh` +0xd4, the one `RenderInternal`'s camera test reads), not the model's
  origin, with the point scaled to the mesh's extents about that centre, read off its
  positions (+0xc0, count +0xc8): "The asteroids", below.

**The frame.** A planet's `GroundMesh` hemispheres and shell are drawn with the identity
rotation (logged on the bench: axes (1,0,0) (0,1,0) (0,0,1), determinant 1, for ground
and shell), so in stock the surface does not turn. The shaders still take the world
matrix's axes, so the surface stays fixed to the mesh whatever its matrix, and a moon,
which spins (`rotationSpeed = 0.3`), carries its surface round with it.

## The bake

Four passes per recipe, each a pixel shader drawing one cube face at a time with the
edge-texel convention of the sky (`sky/README.md`, "No seam from the bake either"):

| Pass | Into | What |
|---|---|---|
| `height_ps` | a float render-target cube (`G32R32F`, else `A16B16G16R16F`, else `G16R16F`), default pool | the height field (r) and the fissures' glow (g) |
| `albedo_ps` | a managed `A8R8G8B8` cube, `Face` | the ground's colour; alpha: how wet (water glints), or with `Cut`, opacity |
| `relief_ps` | a managed `A8R8G8B8` cube, `Face` | the terrain's normal minus the direction, plus 0.5 (rgb); the glow (a) |
| `cloud_ps` | a managed `A8R8G8B8` cube, `CloudFace` | cloud density |

**The normal comes from the height cube, not from more evaluations.** `relief_ps` reads
the height four texels about the direction, along the face's own axes, and takes the
slope; the sea is flat (heights below `Sea` count as `Sea`). Evaluating the field three
times per texel instead made a shader vkd3d could not compile (below).

**The bake is done in one call**, inside a `D3DSBT_ALL` state block, and saves and
restores the render target and depth surface, as the sky's does. The height cube lives
in the default pool, which must be empty when the game resets the device, so it is made
and released inside the call; a bake spread over frames would have to survive a
`Reset` in between. The three results are managed, so they survive one.

**Levels.** The managed cubes carry a full chain, box-filtered on the CPU as each face
is copied. A planet is seen from close and far; an unfiltered relief cube shimmered on
a small one. The chain is not exact at the cube's edges (the box filter does not follow
the edge-texel convention), which matters only on a planet a few pixels wide.

**Cost** (Navi 10, 1024 faces, 512 clouds, `Surfaces.log`): 106–182 ms a recipe, once,
the first time a planet of that class is drawn. The very first bake of a session takes
2–3 s, which is DXVK compiling `height_ps` (230 KB of bytecode); the others reuse it.
Memory: 6 × 1024² × 4 bytes × 4/3 = 33.5 MB a cube, two a recipe, plus 8.4 MB of clouds:
75 MB for a recipe with clouds. Per frame, a planet's pixels take three to five cube
reads more than stock's one texture read.

**vkd3d and loops.** The compiler (`platform/d3d9/hlsl.sh`) cannot emit ps_3_0 loops
(`HLSL_IR_LOOP` not implemented) and unrolls everything. An `if`/`else` inside the
unrolled 27-cell Voronoi search took it over four minutes before it was stopped; the
same search written branch-free (`step` and `lerp`) compiles in under a second. Keep
every branch out of the cell loops.

## The look

`surface.hlsl`:

- **Noise** is the sky's: value noise on a lattice hash with no sine, a quintic fade,
  each octave turned by a fixed rotation. Heights are in units of the field's own
  spread, about a z-score, so `Sea`, the ramp's stops and the rest mean the same
  whatever `Scale` or `Detail`.
- **Terrain**: a domain-warped seven-octave sum, lifted and sunk region by region by a
  slow field (`Continents`), with ridged noise added on land (`Ridges`) for mountain
  chains whose crests carry the fine detail and whose valleys stay smooth.
- **Craters**: two sizes, each a lattice of cells of which a share holds one at a
  jittered point: a bowl, a raised rim, a faint apron. Bigger ones are shallower for
  their size.
- **Fissures**: the borders of a Voronoi net, a share of them kept, warped so they
  meander; a second, finer net branches off the first (`Branches`). Each cuts a trench
  (the width given) and glows along its core (a third as wide). A single unwarped net
  with every border drew a cracked eggshell on class D. Warped hard (2.5) with a third
  kept they meander, which suits D and L's canyons; on the moon, the same warp drew
  blobby loops where stock has straight cracks, so it runs barely warped (0.3) with most
  borders kept, which splits the rock into plates (`FissureShape`).
- **Colour** by height from a six-stop ramp, mixed by large soft regions with a second
  ramp (`Alt`), times a fine colour noise; the sea by depth from `Shallow` to `Deep`; ice
  towards the poles, ragged at its edge; and **cavities**: the height's curvature over a
  few texels darkens hollows (crater floors, trenches) and lightens crests, so the ground
  keeps depth where no light rakes it, on the night side and under haze.
- **A gas giant** (`Kind=gas`): latitude, bent by a slow sine so the bands vary in
  width, wound into `Bands`, torn by turbulence drawn out along the longitude; colour
  from the ramp across one band, a storm oval, and the poles in `Deep`.
- **Clouds**: their own warped field, drawn out along the longitude, gathered in
  latitude belts.

**Depth in the light** (`lighting/planet.hlsl`, `shade_n`): the terminator, its wrap, the
dusk colour and the limb stay the sphere's, and the terrain scales each light by how
much more or less it faces it than the sphere there does, (N_terrain·L + 0.3) /
(N_sphere·L + 0.3), up to twice. A slope facing the sun just past the terminator catches
it and one facing away falls dark: crater rims and ridges stand out along the
terminator, which is where depth reads. With the sphere's own normal the ratio is 1,
so `shade()` is unchanged for stock planets. The clouds drift about the pole
(`CloudDrift`), their shadow falls on the ground offset towards the sun (`CloudShadow`),
and the glow (fissures) shows most on the night side (`GlowDay` by day).

**The moon** (`lighting/hull.hlsl`, `moon_ps`): the relief tilts the mesh's own normal,
so the rock keeps the lumpy shape `models/moon-sod.py` smooths; the albedo's alpha is the
cut-out, and the engine's alpha test, which the rock's material already sets, makes
holes of it through which the blended glow shell shows, as stock's alpha does. The glow
lights the holes' edges.

## Matching stock's colour

The class planets are graded against stock **on screen, in the same light**: the
planets scene (`SCENE=planets`), each class framed to fill the view at two sides (yaw 300
and 140, pitch 15), shot once with `Surfaces.ini`'s `Enable=0` and once with the
recipes (`Reload=1` re-reads both, so one session does both), and compared by the mean of
each channel and the saturation inside the disc. Recipe and stock go through the same
lights, so the ratio is the albedo's. Since surfaces 1.1.1 every class is within 8% of
stock per channel from both sides, but for D's red from one (+13%) and L's blue from the
other (-13%): the recipe's ground is not stock's, so which features face the camera differs.

Three things the first recipes got wrong, which a recipe's header numbers (the stock
texture's mean) do not show:

- **The ground lives in the low stops.** Heights are z-scores, so most of a planet lies
  between `Ramp2` and `Ramp3`, and cavities darken it further. Stops picked to bracket
  the stock palette rendered 10 to 40% darker than stock. Pick the middle stops from
  the stock texture's dominant colours, not its extremes.
- **Contrast across height shows as a difference between sides.** L's dark rust
  lowlands and near-white highlands read dark from one side and washed out from the
  other, where stock is an even orange. Keep the ramp within the stock palette's
  luminance band.
- **The stock shell's cloud is part of stock's colour.** H, K, L and M all have white
  streaks from `PA_ECFR0`, which a recipe's ground suppresses ("Where Lighting.asi uses
  it"), so those recipes bring cloud of their own.

The engine adds no colour of its own (the ODFs have none): the warm Key is what turns
stock J's pastels and M's teal brownish on the lit side, for recipes as for stock.

## The asteroids

An asteroid field (`odf/other/mafield.odf`, `mafield2.odf`) is three to eight rocks, each
one of the models the engine lists: `master.sod`, `master1` to `master5`, `master4` again
and `master7` (`Master6` and `Master8` ship but are never drawn). All but `Master.SOD`
are textured `Master1` or `Master2`; `Master.SOD` is `Master2` too. Stock paints them a
256-texel cratered grey, smeared along the long faces of 20-vertex rocks.
`models/asteroid-sod.py` smooths the meshes (`models/README.md`, "The asteroids"); the
recipes give them their surface, drawn by `moon_ps` as the moon's is, with three changes
the rocks needed on the bench (`SCENE=asteroids`):

- **About the mesh's centre.** The moon is a sphere about its origin; an asteroid's
  origin can sit near its surface (`Master7`'s is 3.8 units from it, 38 from its far
  end), and the direction from there crushed one end's features and stretched the other's.
  `Lighting.asi` looks up about the mesh's bounding-sphere centre, scaled to its extents
  along each object axis, so a long rock is looked up as if it were round. The scale is
  taken out of the relief, which otherwise tilted 2 to 2.7 times as far on rocks the
  fields draw at that size (`scaleSOD`; the fields bake it into the vertices).
- **Triplanar** (`Mapping=triplanar`). Even about the centre, the direction smeared the
  rock into streaks and ripples wherever a face runs along it: the flat end of a rock, a
  box-like side. Along each object axis the shader takes the cube face on that axis's
  side at the point's other two coordinates (each -1..1 across the mesh), one continuous
  picture per side, and blends the three by the surface's facing to the fourth power. A
  side switches face where that projection's weight is nought, so no seam shows. The slope
  baked at (±1, y, z) is the height's gradient across the y-z plane, which is the
  normal's tilt for that projection; its component along the axis is dropped. Six cube
  reads instead of two.
- **Matte** (`Gloss=0.2`). The hulls' highlight takes the texture's brightness as its
  gloss, which made pale rock read wet, as metal.

The look: lumpy ground (low warp; at 0.6 the field drew marble and bark) under two sizes
of crater bowls with raised rims, cavities darkened. A relief of 0.085 against the 0.06
shipped added grain, not shape. `Master2` is paler, with smaller craters and more ridged
ground, so the two read as two kinds of stone. `Face=1024` (both recipes: 67 MB each,
126–173 ms to bake): at 512 a rock filling the screen showed its texels as soft waves in
the relief.

## Recipes

`surfaces/recipes/<texture>.ini`, section `[Surface]`. Colours are 0–255; heights are in
field units (about −4 to 5); directions are yaw (degrees about the planet's +y) and
pitch (up from its equator).

| Key | Default | |
|---|---|---|
| `Kind` | `rock` | `gas` for a gas giant |
| `Seed` | 1 | moves the noise domain: another planet of the same character |
| `Scale` | 2.5 | the largest features' frequency over the sphere |
| `Warp` | 0.4 | domain warp. High values turn the ground wormy |
| `Detail` | 0.5 | amplitude ratio between octaves |
| `Continents` | 0 | the slow field that lifts and sinks regions |
| `Ridges` | 0 | mountain chains on land |
| `Sea` | none | the sea level; leave it out for a dry world |
| `Shore` | 1 | how deep the water must be to reach `Deep` |
| `Relief` | 0.02 | the height's scale against the radius, per field unit: the normal's tilt |
| `Craters` | 0 12 1 0.3 | share of cells with one; cells across; depth; rim |
| `Fissures` | 0 0.05 1 1 | cells across (0: none); trench width; depth; glow |
| `FissureShape` | 2.5 0.33 | the warp on the fissures (high: meanders; low: straight cracks) and the share of the net's borders kept (high: plates, as on the moon) |
| `Branches` | 0.6 | the finer net of fissures, against the first |
| `Cut` | 0 | the glow at which an alpha-tested model is cut through (the moon) |
| `Ramp1`..`Ramp6` | | `height r g b` (a gas giant: `0..1 r g b`, across one band) |
| `Alt1`..`Alt6` | the ramp | a second ramp, mixed in by region |
| `Region` | 1.5 0 0.15 12 | the regions' scale; how much of `Alt`; colour noise; its scale |
| `Deep`, `Shallow` | 10 30 60, 30 90 110 | the sea; a gas giant's poles are `Deep` |
| `Ice`, `IceColour` | 0.8 0.1 0.15 0, 235 240 245 | latitude it starts at (0..1); softness; ragged edge; amount |
| `Cavity` | 0.25 0.15 | hollows darkened, crests lightened |
| `Glow`, `GlowDay` | 0 0 0, 0.3 | the fissures' light, and how much of it shows by day |
| `Clouds` | none | scale, coverage (field units: higher is clearer), softness, warp |
| `CloudShape` | 2 0.55 0.5 0 | drawn out along the longitude; detail; belts; seed |
| `CloudColour`, `CloudOpacity` | 255 255 255, 0.9 | |
| `CloudDrift` | 0.4 | degrees per second about the pole |
| `CloudShadow` | 0.5 0.012 | strength; how far towards the sun it falls (radians) |
| `Depth` | 1 | the relief's strength in the light, at draw time (no re-bake) |
| `Bands` | 8 1.5 0.3 4 | gas: bands; how far turbulence moves them; the warp's share; stretch along the longitude |
| `Storm`, `StormColour` | 0 −20 8 0 | gas: yaw, pitch, size (degrees), strength |
| `Face` | `Surfaces.ini`'s | this recipe's cube edge (64–4096) |
| `Mapping` | `radial` | a model's lookup: `radial`, the direction from its centre; `triplanar`, each axis's cube faces across its extents ("The asteroids") |
| `Gloss` | 1 | a model's highlight, times the hulls' |

`Surfaces.ini` (`[Surfaces]`): `Enable` (1), `Face` (1024), `CloudFace` (512), `Reload`
(1: the file and the recipes are looked at twice a second, and a changed recipe
re-bakes on its next draw), `Log`. `Lighting.ini`'s `Surfaces=0` stops Lighting.asi
asking.

**Tuning a recipe**: edit the installed `Surfaces\<name>.ini` with the game running; the
planet re-bakes within half a second. Each recipe's header gives the stock texture's
channel means it aims at. A recipe a model's texture is found by only at that
texture's first draw: a recipe added later for a hull texture needs a restart.

## Not covered yet

- **Seen only on the bench**: every recipe was judged on the bench (`SCENE=planets`,
  1920x1080), not yet in game.
- **Colonised planets**: the cities (`PD_*`, pass 1) are placed in the gore unwrap's UV
  and drawn over the new ground, so a city can sit on a recipe's ocean.
- **A planet larger than its cube**: at 1024 a planet half the screen high at
  3440x1440 gets about two texels a pixel; closer than that the terrain softens.
