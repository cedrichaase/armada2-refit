# a2fmt

Readers for Star Trek: Armada II's asset formats, for the renderers in `render3d/` and `ui/`:
SOD models, TGA textures and their mip chains, `.spr` sprite tables and ODFs, and `vfs`,
the one file system the plugins read the game's files through.
`no_std` with `alloc`, one dependency (`libm`). Nothing here makes a system call: callers
pass bytes, and for formats that include other files, a closure that fetches them by
name; `vfs` reads through functions the caller supplies.
No parser panics on bad input (a test feeds damaged copies of real files to all four).

```
cargo check --target i686-pc-windows-msvc                       # as the renderer builds it
cargo test --release --target x86_64-unknown-linux-gnu           # host tests
```

Outside the plugins' directories no `.cargo/config.toml` applies, so the host is the
default target and the plugins' target is spelled out. The tests read
every stock file of the installed game (`A2_GAME`, else `a2env.sh`); without a game
they print a note and pass. They write nothing.

The layouts below were established by parsing every stock file to its last byte and
by reading the engine's loaders (`ST3D_Stream::ReadDatabase` and its helpers,
`ST3D_SpriteTableParser`) for what the files alone leave open. They are described
here because the renderer reads these files in the engine's place.

## The file system (`vfs`)

Each game keeps its files in its own layout: Armada II's textures in `Textures/RGB/`,
Fleet Operations: Roots' in `Textures/`, TGA and DDS mixed, with its ODFs packed in
`ODF.fpq`; Armada 1 packs its data in `.zfs` archives (`PLAN-ARCHITECTURE.md`, F5). `vfs`
makes that layout data: the plugins open files by a virtual path (`textures\<file>`,
`odf\<file>`), and a mount list says where each virtual folder is.

- **`trait Vfs`**: `open(path) -> Option<Vec<u8>>`, `exists(path)`, `list(dir)`, plus
  `kind`, `status` and `label` for the log. Paths are case-insensitive (hard rule 3:
  stock mixes `.tga` and `.TGA`) and take either slash; empty and `.` components are
  skipped, and a path with `..` names nothing.
- **`Loose::new(io, root)`**: a folder of loose files. The system calls are an `Io` the
  caller supplies (`read`, `is_file`, `list`, its separator and whether the file system
  ignores case); `FnIo` builds one from plain functions, which is how render3d and the
  UI hand in `a2rt`'s Win32 reads (`render3d/core/src/files/mod.rs`, `ui/src/files.rs`).
  Where the file system ignores case (Windows, Wine) a path goes to it as written, so a
  lookup costs what it did before; where it minds case (the host tests) a path that does
  not open as written is matched through the folders' listings, each listed once.
- **`Layered`**: sources mounted at virtual folders (`""` is the root). A path is served
  by the mounts at the deepest folder that holds it, so a mount shadows the folder of its
  name in the root mount, as a mounted file system does. Among those, sources are
  searched by level (`mount_level`: higher first; `mount` is level 0), then by kind
  (loose folders, then `Fpq`, then `Zfs`), then in the order they were mounted; the
  first that has the file serves it. `list` merges them, the first of each name in any
  case kept, with the mount folders inside as folders. Levels are a game's mods: Fleet
  Operations: Roots layers the active mod's folders over its parent's over the game's
  own, whatever their kind (render3d mounts each folder of the chain a level up).
  `sources(dir)` hands out the sources serving a folder in that order, for a caller that
  indexes each apart (render3d's asset index finds a name in the top source that has it,
  wherever it sits), and `open_nth(path, n)` reads a file's `n`th version down.
- **`Fpq`** serves Fleet Operations: Roots' `ODF.fpq` (below). `Fpq::new(label, bytes)`
  parses the archive (the caller reads the file) and mounts at `odf`: the archive's single
  top folder is the mount's name, so `system/x.odf`, `odf/system/x.odf` and `x.odf` all open
  the same file. It is `Kind::Fpq`, so a loose file in a folder mounted before or after
  it still wins. This is the whole of the layer, kept small: read-only, the archive held
  in memory (4 MB for Roots), no caching.
- **`Zfs`** is declared, with its place in that order, and serves nothing: its
  `status()` is `Unsupported`, with a reason fit for a log line. Its reader comes with
  the work that needs it (`PLAN-ARMADA1.md`, A1-11), and nothing here describes the format.
- **Planned, not written: an `Engine` source**, which reads through the game's own file
  functions, for an archive whose format is not ours to read (FO-5 chooses between it
  and the `Fpq` reader). It would mount last, the fallback behind every source of ours.

The mount list itself is the game descriptor's `[files]` (`games/README.md`, "Files"),
which `tools/layers.py --check` holds equal to the plugins' `LAYOUT` tables.
`tests/vfs.rs` checks case-insensitivity on a case-sensitive disk, layering order,
shadowing, missing files and the declared `Zfs`, on a temporary directory.

## Fleet Operations: Roots' archive (`fpq`, `lzo`)

Roots packs its ODFs in `Data/ODF.fpq`, and a loose file overrides a packed one.
`fpq::Archive::parse(bytes)` checks the tables and `find`, `read`, `open`, `entries` and
`path` use them; `lzo::decompress(packed, size)` is the decoder. The format was
established from the engine-side reader's behaviour and checked against the real archive
(1799 files, every one decoding to its stated size and found again through the hash);
the code is our own, from the format alone.

- **Header**, seven little-endian `u32`: magic `FPQ\0`, version (exactly 2; any other is refused),
  folder count, file count, hash-table size (a power of two), data-blob size,
  names-blob size.
- **Then, in this order and exactly to the file's last byte**: the hash table (12 bytes a
  slot: `hash_a`, `hash_b`, file index; an **empty slot is all zero**, and index 0 is a
  real file), the folders (12 bytes: parent or `0xFFFFFFFF`, name offset, name length),
  the files (25 bytes: data offset, unpacked size, packed size, name offset, name length,
  method `u8`, folder index), the names blob, the data blob. Names are single-byte text,
  not terminated, 1 to 255 bytes each, and one path component (no `/`, `\\` or `:`, not
  `.` or `..`); the names must not overlap, so their lengths add up to at most the
  blob's size (a hostile table cannot make a small file decode to a large pile of
  strings). A folder chain deeper than 64 is taken for a loop and refused. Roots has two folders, `odf` and its child `system`, and every file is
  in `system`.
- **Methods**: 0 is stored, 1 is LZO1X. 82 files are stored, 1717 compressed. Compressed
  streams end with their input (the end marker is accepted but not needed).
- **Lookup** is the MPQ string hash of the file's own name (no folders; upper-cased by
  ASCII, so lookups ignore case; taken over the name's single bytes, so a name with a
  byte above 0x7F is found by its Latin-1 spelling): a 0x500-word table from seed `0x100001`, three
  hashes of kinds 0, 0x100 and 0x200 (start slot masked by the table size, `hash_a`,
  `hash_b`), then linear probing to the first empty slot. A path's folders are checked
  against the file's own afterwards.
- **LZO1X** is a byte-oriented instruction set: literal runs, and matches of four
  distance classes whose instruction byte also carries the count of 0 to 3 literals that
  follow; what `b < 16` means depends on whether a match or a literal run came before
  (`src/lzo.rs` has the table). Every read and write is checked: input overrun,
  output past the stated size, distance before the start and a short output are
  errors, and a size above 64 MiB or beyond what the input could expand to is refused
  before any allocation (an allocation that fails is an error, not an abort). The unpacked
  size is the file table's, so the decoder needs it.
- **Real-archive surprises**: the first entry is itself a complete `.fpq` (1521 files);
  an empty slot is zeros, not `0xFFFFFFFF`; the folder index is the file entry's last
  field.

`tests/fpq.rs` builds its own archives and LZO streams in code (nothing from a game),
and fuzzes both readers for panics. `cargo test --test fpq -- --ignored --nocapture`
also runs the whole real archive when Roots is installed (or `A2_FO_ODF_FPQ=<file>`) and
prints counts only. For agents, `cargo run --release --example fpq -- list|cat|extract
<archive>` lists a file, prints one, or extracts to a directory, and refuses a
directory inside a git checkout (extracted files are assets: `$A2_DATA` or a scratch
directory).

`cargo run --release --example shipsurvey -- <Roots Data dir> [out-dir]` surveys the
craft: every ODF that names a `race` and a `basename` (class label, ship class, SOD), each
SOD's meshes (blend material, flags, bump or not, textures with their load words), the
race ODFs' keys, the `RTS_CFG.h`/`ART_CFG.h` lines about race, colour or team, and counts
per race and of textures shared between races. A mesh counts as bump-mapped when its flags
carry the dot3 bit or a texture carries the `0x200` height-map word. It writes
tab-separated tables and `summary.txt` to `$A2_DATA/research/fo-roots/ship-survey/` (the
tables are derived from the game; the tool refuses a checkout) and reads only.

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

## DDS (`dds`, `image`)

Fleet Operations: Roots' textures: a 128-byte header, then the base level. `dds::decode` reads
DXT1 (with its 1-bit alpha), DXT3 and DXT5, and uncompressed 8, 16, 24 and 32-bit formats by
their channel masks, to the same BGRA8 top-down `Image` as `tga::decode`; the file's own mips
and DX10 headers are not read (bounded: 16384 a side, 8192 x 8192 pixels). `image::decode` and
`image::width` take either format by the `DDS ` magic, and `TexIndex::with_extensions(names,
&[".tga", ".dds"])` indexes both, the earlier extension winning a name present twice.

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

`Odf::load_layered(name, fetch)` reads over layered files (`fetch(name, v)`: version `v`
of a file, 0 the top one): Fleet Operations' file system keeps every version of a file
by name, a mod's over its parent's over the game's, and an include of the very file
being loaded (`x.odf` including `x.odf`) is its next version down, so a mod's file
inherits the one it replaces and changes only what it writes; each such include in the
tree takes the version below the last one taken, as the game's own parser does (Roots
5.0Z11, read off FleetOpsHook's include hook). In `Odf::load`, with no versions, the
same include is a loop and is cut.
