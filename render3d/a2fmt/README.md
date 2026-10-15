# a2fmt

Readers for Star Trek: Armada II's asset formats, for the renderer in `render3d/`:
SOD models, TGA textures and their mip chains, `.spr` sprite tables and ODFs.
`no_std` with `alloc`, one dependency (`libm`). Nothing here opens a file: callers pass
bytes, and for formats that include other files, a closure that fetches them by name.
No parser panics on bad input (a test feeds damaged copies of real files to all four).

```
cargo check --target i686-pc-windows-msvc                       # as the renderer builds it
cargo test --release --target x86_64-unknown-linux-gnu           # host tests
```

`render3d/.cargo/config.toml` makes i686 Windows the default target for anything run
from inside `render3d/`, so the host tests need `--target` spelled out. The tests read
every stock file of the installed game (`A2_GAME`, else `a2env.sh`); without a game
they print a note and pass. They write nothing.

The layouts below were established by parsing every stock file to its last byte and
by reading the engine's loaders (`ST3D_Stream::ReadDatabase` and its helpers,
`ST3D_SpriteTableParser`) for what the files alone leave open. They are described
here because the renderer reads these files in the engine's place.

## SOD (`sod`)

Little-endian. A string is a `u16` length and that many bytes. A 3x4 matrix is twelve
`f32`: the node's x, y and z axes in its parent's space, then the translation; points
are row vectors (`p' = x·X + y·Y + z·Z + T`).

1. Ten bytes of magic, `Storm3D_SW` or the older `StarTrekDB` (one reader takes both),
   then the version as an `f32`. The engine accepts 1.4 to 1.93 and refuses anything
   else, so the twelve stock `StarTrekDB` files at 1.0 and 1.2 never load in the game;
   `Sod::parse` refuses them the same way. Every gate below compares the stored `f32`.
2. **Before 1.7**, a texture-material table: a count, then per entry an ident, a
   texture file name, a `u32` and three bytes (two addressing modes and one more).
3. **Lighting materials**: a count, then per material a name, ambient, diffuse and
   specular colours, a power, and a lighting-model byte (stored one higher in 1.4
   files). From 1.9 a further byte says the material is self-illuminating.
4. **Nodes**: a count, then per node a `u16` type, its name, its parent's name and its
   matrix, then a payload by type:
   - 1, mesh (below);
   - 2, light: a kind byte, a colour, falloff start and range (none in stock);
   - 3, sprite node: nothing. The node's name, cut at its first underscore, is a
     `@sprite_node` id (`redstrobe_12` draws `redstrobe`);
   - 12, particle emitter: the name of an `@emitter`;
   - anything else: a plain transform node with no payload.

   A null node whose name starts `geometry` becomes the LOD switch: its direct
   children named `lod*` are the levels, ordered by face count, coarsest first
   (`Sod::lod_levels`). Hardpoints are null nodes named `hp` + digits.
5. **Mesh payload**: a texture-material name (a blend mode such as `opaque`,
   `additive` or `alphathreshold` from 1.7; before that an ident of the table in 2,
   which also names the texture). From 1.93: a `u32` of mesh flags (2 = bump path) and
   a count of textures, each a name and a `u32` of load flags (`0x200` = a height map
   to turn into a normal map). From 1.7 to 1.92: one texture name. From 1.92 a second
   texture name; from 1.91 a `u16` (zero throughout stock). Then `u16` counts of
   positions and UVs — **a zero in either ends the mesh there** — and of face groups;
   the positions (3 `f32`), the UVs (2 `f32`); per group a face count, a lighting
   material name and per face three corners of (position index, UV index). Last, a
   cull byte (1 = back faces culled, 0 = two-sided) and a counted list of `u16`
   (empty except in `Zomega` and `wbholegen`, meaning unknown).

   Positions and UVs are indexed separately per corner, so a GPU loader de-indexes
   (`Mesh::unindexed`). **No normals are stored**: `Mesh::smooth_normals` builds
   area-weighted ones; stock windings are not consistent.
6. **Animation channels**: a count, then per channel the node's name, a `u16` key
   count, the period in seconds, a type byte and a byte the engine ignores. Type 5 keys
   are one `f32` each, the node's uniform scale; every other type keys whole matrices.
   Keys are evenly spaced over the period.
7. **From 1.6, animation references**: a count, then per reference a type byte (4: a
   texture flipbook on the mesh's UV offset), the mesh node's name, an `@animation` id,
   and from 1.8 a time offset.

Nothing may follow. Parents are linked by name, case-insensitively.

## TGA (`tga`)

18-byte header, an ID field and colour map (both empty in stock), then pixels as
B, G, R (, A). The descriptor's bit 5 says rows run top-down; otherwise bottom-up. Stock
is all type 2 (uncompressed), 24 or 32 bits, mixed origin; `decode` also takes RLE,
greyscale and 15/16 bits and returns BGRA8 top-down.

Mip levels are separate files named after their base: `<base>_1`, `<base>_2`, ...
or `<base>1`, `<base>2`, ..., and a candidate is a level only if its width is the
base's halved per level. `TexIndex::mip_chain` resolves them over a directory listing
(case-insensitive) with widths from a caller's closure, and `mip_strays` lists what is
named like a level and is not one (as `mip_name` and `mip_strays` do in
`textures/lib/common.sh`).

## Sprite tables (`spr`)

Line-based text; `#` and `//` start comments. A sprite line is
`name texture u v w h [@flags]`, its rectangle in texels of the reference size in
force (`@reference=`, `@referenceWidth=`, `@referenceHeight=`), so a larger texture
keeps the same rectangle. `@tmaterial=` and `@skip=` likewise apply to the lines after
them. Blocks: `@sprite_node id sprite anim|const (w,h) (r,g,b) [billboard]`;
`@animation id`, a `field keys length interp` line, optional `@auto=`, then
`@keyframes` and rows (`time value`, a `+t` time relative to the previous key; no
times when the length is 0); `@emitter id` ... `@end_emitter` with `key=value` lines.
`@include` pulls another table in place. Stock quirks the parser keeps: `draw` keys
are read as integers (`0.8` is off); an animation without `@keyframes` has no keys
(`holdbeam`, `assimbore`); `Animation::plays_evenly` reproduces the engine dropping
key times it wrongly judges even, which is how the running lights blink.

## ODF (`odf`)

`key = value` text, `//` and `#` comments, `#include "file.odf"` for inheritance, and
values continued on following lines that hold no `=` (name lists). A few files have
`[Section]` headers, which lookups ignore. A file's own keys win over its includes,
searched in order, depth first; a key written twice in one file takes the later value
(stock has 45 such repeats; which one the engine keeps is not established).
