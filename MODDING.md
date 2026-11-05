# Modding on the Refit base

For people who want to build on this project: take Display8 alone, write a plugin
against `gpu`, run render3d with some modules, write a quality-of-life plugin, give the
renderer new recipes, or port the stack to another build of the executable. It says
which unit needs what, what the contracts between them promise, how a build gets its
symbol profile, how to build, and what you may publish. It is a guide; the reference
for each part is the README beside its code, and the working brief for the repository is
[`CLAUDE.md`](CLAUDE.md). Players want [`PLAYING.md`](PLAYING.md).

The release zip's `sdk/MODDING.txt` is the short form of the C-plugin part of this
file.

## What is separable

The mod is separate units with explicit interfaces, so you can take one without the
others. `layers.toml` is the registry of the layers and what each requires.

| Unit | What | Needs | Language |
|---|---|---|---|
| Display8 (`display/`) | The game's `d3d8.dll`, ours, on Vulkan and Direct3D 12. Draws every engine draw a plugin does not claim as stock would (passthrough), and hands the system's own Direct3D 8 to the game when neither backend starts (the bail-out) | nothing, and it needs no plugin: alone it is a stock-looking game on a modern API | Rust, `no_std` |
| `gpu` (`gpu/`) | The graphics interface plugins draw through: one device and one frame on Display8's Vulkan or Direct3D 12 | Display8 | Rust, `no_std` |
| render3d (`render3d/`) | `Render3D.asi`: the 3D view. Eight modules and a core | Display8, `gpu`, `a2rt`, `a2fmt`, `a2engine` | Rust |
| UI (`ui/`) | `UI.asi`: HUD, menus, loading screen, the Refit screen and the settings hub | Display8, `gpu`, `a2rt`, `a2fmt`, `a2engine` | Rust |
| QOL (`qol/`) | `QOL.asi`: gameplay quality of life that stays on the player's machine | the game only | C |
| Rules (`rules/`) | `QOLRules.asi`: the rule changes every player of a network game must share | the game only | C |
| Online (`online/`) | `Online.asi`: *Internet - Online* | the game only | C |
| binkproxy (`cutscenes/binkproxy/`) | `binkw32.dll`: full-screen launch reels and AV1 movies | the game only | C |

Shared code, none of it a layer: `crates/a2rt` (the `no_std` runtime: allocator, Win32,
log, ini with the overlay, in-memory patching), `crates/a2fmt` (readers of the game's
SOD, TGA, `.spr` and ODF files), `crates/a2engine` (the symbol resolver), `crates/a2ini`
(the ini parser and the overlay's writer), `csdk/` (the C twin: `a2asi.h`,
`a2engine.h`, the import `.def` files) and `tools/shaderc`.

Layers that change what every machine of a network game simulates are a layer of
their own with `scope = 'match'` (`rules/`, `online/`), never part of a plugin that
stock players can play against. If you write such a rule, do the same.

## The contracts

Each contract is one file both sides build against, and changes on both sides together.

### Display8 and `gpu`: `display8v1_*`

`gpu/src/display.rs` is the specification, in its doc comment; `display/README.md` is
what Display8 does with it. Display8 owns the device (Vulkan or Direct3D 12), the swap
chain, the frame and its upload ring, and exports plain C functions. Rust types have
no stable layout across DLLs, so everything is `#[repr(C)]` and plain handles. A plugin
reaches them through `gpu::Gpu::with_display`, which finds the exports by name on
`d3d8.dll`; if you write your own plugin you can use `gpu` as that layer or call the
exports yourself.

- **The handshake.** `display8_abi(caller_major, caller_minor, caller, &major, &minor)`
  returns Display8's version. Its name and signature never change, whatever the major.
  `gpu` calls it first, then loads the `display8v1_*` exports. Both sides log both
  versions.
- **A minor only adds.** A new export, a constant, or a field at the end of a struct.
  Every `#[repr(C)]` struct starts with `size`: a struct Display8 hands out says its
  own size and the reader takes the fields within it; a struct the caller hands in to
  be filled says its room and the writer fills no more and sets `size` to what it wrote.
  Whatever is past the other side's size is the field's default. Use what minor N added
  only when Display8's minor is N or more. A struct inside another does not grow in a
  minor, since that would move what follows. The contract is at 1.4 (the claims at 1.1,
  Direct3D 12's pipeline library at 1.2, `CLAIM_MENUS` and `display8v1_mode` at 1.3,
  the Direct3D 12 pass at 1.4).
- **Anything else is a new major**: a field removed, moved, retyped or given a new
  meaning, an export's signature or meaning, a rule between recorders (layouts,
  states, heaps, who closes a pass). The exports are named with the major
  (`display8v2_*`), so a plugin of another major finds none of them instead of
  misreading them.
- **A major mismatch makes the plugin stand down**: it draws nothing, the engine's draws
  pass through, and the log says which versions met. A plugin must handle "no
  Display8", "Display8 too old" and "another major" as stand-downs, never as errors.
  With the bail-out taken, `display8v1_backend` answers none and `gpu` opens no
  backend.
- **One device, one frame.** Plugins record, on the game's thread, into the frame's
  command buffer or list, in the order the calls come; Display8 submits and presents at
  the engine's `Present`. The layout and state rules between recorders (what each
  attachment is in outside a `Cmd`, no pass open between recorders) are in the doc
  comment, and a plugin must put back what it changes.
- **Look at the device by epoch, not by polling the engine's.** `display8_device_epoch`
  tells a plugin when the device was made again.

### Claims

A plugin that draws something the engine also draws says so, so that Display8 draws
only the rest. At load a plugin calls `display8v1_register` with a `Plugin8` (its size,
an address inside its module, and a callback). Display8 asks each callback at every
`BeginScene`, logs what changed, and `display8v1_claims(except)` tells one plugin what
the others claim. The callback returns the categories the plugin draws **now**: sky,
hulls, world, particles, effects, ordnance, specials, fog of war, selection, 2D HUD,
loading, credits, cursor, menus. Rules:

- Claim only what you draw this frame. A module you switched off, a part that stood
  down, a plugin with no device: claim nothing, and the engine's draws show (stock look
  on every backend).
- A category nobody claims is the engine's, drawn by passthrough.
- Display8 drops the engine's draws of a claimed category only where the engine draws
  anyway under the plugin's (loading, credits, cursor). The 3D categories are never
  dropped by Display8: the engine issues them only when no plugin draws the part,
  because render3d skips the engine's step.
- Plugins find each other by claims, never by DLL name.

### Settings descriptors: `A2SET_ABI 1.1`

Each plugin that has keys a player may set exports a static table of descriptors, and
the UI's Refit screen lists every table it finds. The full contract is
[`tools/settings/README.md`](tools/settings/README.md).

- **The export** is `<Plugin>_Settings(UINT *count)`, cdylib and cdecl, returning
  `const A2Setting *`. The hub walks the loaded modules and asks each for the fixed
  names, so no DLL is looked up by file name. A row carries its size, kind (bool or
  enum), apply class (live, next mission, restart), scope (local, or must match in a
  network game), tab, flags (advanced, not shipped, no row), the ini file, section, key,
  default, values and labels, a label and a help text, and, from 1.1, the key it moved
  from. A reader steps by `rows[0].size`.
- **Optional exports.** `<Plugin>_SettingsChanged()` is called on the game's thread
  after the player's overlay changed: re-read your live rows, be idempotent and cheap.
  `<Plugin>_SettingsNote(row)` returns NULL, or a short static string saying this part
  is not running now and why; NULL for the row means a note about the plugin as a whole.
- **Rows are data.** Write `<layer>/settings.toml`; `tools/settings/gen.py` generates
  the table (`settings_table.rs` or `settings_table.h`); `tools/settings/lint.py`
  checks tables against the schema, against each other, against the shipped `.ini` and
  against the sources. Read a key only through its row, so the code's default is the
  table's.
- **A minor adds a field at the end; a major renames the export** (`<Plugin>_Settings2`).
- **The overlay.** Player choices go in one file, `RefitPlayer.ini`, that no installer
  writes. A key is read overlay first, then the plugin's `.ini`, then the default.
  In Rust, `a2rt::Ini::open` and `rt::settings::on/int/float/value`; in C,
  `a2_ini_open` and `a2_set_int/str` from `a2asi.h`. The only writer is `UI.asi`
  (`a2rt::overlay::set`), through `a2ini::edit`, which keeps the file's lines byte for
  byte. A menu of ours never writes a game setting (`ARMADA.PRF`, `RTS_CFG.h`);
  settings of ours go to the overlay only.
- **Moving a key** between files: give the row `moved_from = "Stem.Section.Key"`. The
  hub moves the overlay's value; `./install` (`move-keys.py`) and the release scripts
  (`moved.txt`) move the plugin inis'.

### Engine symbol profiles: (game, build)

A plugin does not hold addresses in its code. Every address, offset and short signature
it uses is in a **profile**, data per executable build, generated into a Rust module or
a C header. The format, the resolver and the lints are described in
[`crates/a2engine/README.md`](crates/a2engine/README.md) and
[`tools/engine/README.md`](tools/engine/README.md).

- A profile is keyed by **(game, build)**. Armada II's are in `profiles/<build>/`
  (`gog-1.1` today); another game's would be in `<game>/profiles/<build>/`. The game is
  one of [`games/`](games/README.md)'s descriptors (`games/<id>.toml`), which the tools
  read for its executable, its folders, its settings file and the layers on offer; a
  build's `exe` must be one of its descriptor's `exe.names`
  (`tools/layers.py --check`).
- A build is recognised by three numbers read from the process's own PE headers: the
  timestamp, the image size and a fingerprint of the section table. All three must
  match; the timestamp alone is never the key (Fleet Operations: Roots shares stock's).
- One file per plugin or part: `<plugin>.toml`, `<plugin>.<part>.toml`. It lists
  **hooks** (each with a one-line description and the settings rows it feeds) and
  **entries** (function, call, jmp, slot, label, patch, scan, vtable, global, field,
  value), each tied by `used_by` to a hook. A `check` signature is at most 32 bytes.
- **Struct offsets are per build too.** A `field` (a struct's offset) and a `value` (a
  count, an id, a frame local's offset from `ebp`) are resolved at start-up like an
  address: the code reads `field::X()` in Rust or `<P>_F_X` in C, never a literal, so a
  build whose structs differ gets its own offsets from its profile, and a field the build
  has no entry for makes the hooks that use it stand down. `tools/engine/lint.py` and
  `render3d/lint-layers.sh` fail on a literal struct offset in a plugin's source.
- **At start-up** the plugin identifies the executable, picks the profile, relocates,
  and logs `engine: <exe> timestamp ...` (the executable named by the build's
  `build.toml` `exe`, or on an unknown build by the process's own file name),
  `engine: build armada2/gog-1.1` and `symbols: N/M resolved; H hooks blocked`.
  Before a hook writes anything, `ready(hook)` checks its entries in live memory. A
  hook that fails stands down alone and logs `engine: <hook> stands down: <symbol> <why>`;
  the rest of the plugin runs. On an unknown build only hooks that resolve by scan, and
  read no offsets, run. This is what makes a new executable fail soft.

**How a new build gets a profile.**

1. Identify it. Start the game with any plugin of ours and read the log's
   `engine: <exe> timestamp ..., image size ..., sections ...` line, or run
   `./a2test syms --exe <path>` on the file, read only. It says `unknown build` and lists
   the hooks that stood down.
2. Make `profiles/<build>/build.toml` with that identity. If the build is another
   patch of a known one, set `inherits` and list only what differs: a `[[changed]]` for
   each function the build changes (with `how` and the hooks `verified` beside it) and
   `[[disable]]` globs for hooks that must not run. A hook that touches a changed
   function is **blocked** until someone verifies it; a child's entries override the
   parent's by `id`, a moved field included (`offset = 0x..` under the field's `id`).
3. `tools/engine/gen.py --check` and `tools/engine/lint.py` validate it; `./a2test syms
   --exe <path> --build <build>` runs every hook's checks against the file and prints
   each `ready` or its reason. Fix by adding an override entry or disabling the hook,
   then repeat. The bench's own negative tests are `tools/engine/negative.sh`.
4. `./a2test syms --exe <path> --record` appends the file's sha256 to the matched build's
   `[[known]]` list as information. It refuses a build that is not matched, and the hash
   is never a run-time key.
5. Run the game on the bench to see what only a running game shows (`engine: ...` lines
   in each plugin's log). The plans for the first two additional builds are
   [`PLAN-FLEETOPS.md`](PLAN-FLEETOPS.md) and [`PLAN-ARMADA1.md`](PLAN-ARMADA1.md).

**Content profiles.** A plugin's tables of the game's own names (ODFs, sprites, SOD
nodes, factions, recipes) are not addresses and are not tied to the executable. A
build's `content` names the set its plugins read (`stock` for `gog-1.1`), a
`content.toml` may map a plugin to another, and the generator hands the plugin its
resolved set id. A set is a folder of data, not code ("Content sets" below): a plugin
looks up the set's folder by that id and has no built-in names or recipes when there is
none, and the engine's own look shows through. A file in the game folder still wins
over any set.

### Content sets

A content set is a folder of small ini tables, one per plugin part:

| Folder | Tables |
|---|---|
| `render3d/content/sets/<set>/` | `effects.ini`, `hulls.ini`, `ordnance.ini`, `overlays.ini`, `particles.ini`, `sky.ini`, `specials.ini`, `world.ini`: each module's names (nodes, sprites, SODs, faction keys and colours, the special effects' kinds) and the recipes it builds in, named by their file under `render3d/content/` |
| `ui/content/sets/<set>/` | `ui.ini`: the races (with the theme's look for each), resources, campaign cards, the credits' and menus' views, the game files the UI reads and the widget gallery's lists |

Every `.ini` under `render3d/content/` and `ui/content/` is built into the plugin
(`render3d/core/build.rs`, `ui/build.rs`), so a set needs nothing installed; the modules
parse their table once, at first use. A set with no table for a part is the empty table
for it: that part has no names and draws the engine's own. `stock` is the set of
`gog-1.1`; `games/fo-roots.toml` names no set yet, so Fleet Operations: Roots has none.

To add a set:

1. Copy the stock folders: `render3d/content/sets/stock/` to
   `render3d/content/sets/<set>/` and `ui/content/sets/stock/` to `ui/content/sets/<set>/`.
2. Edit the copies to the game's names. A recipe of the set's own goes in its folder
   (`render3d/content/sets/<set>/sky/<name>.ini`) and its table names it by that path;
   leave out a table, or a line, the game has no use for.
3. Name the set in the build's profile: `content = "<set>"` in
   `profiles/<build>/build.toml`, or per plugin or part in
   `profiles/<build>/content.toml` (`[set] render3d = "<set>"`, `"render3d.hulls" = "<set>"`,
   `ui = "<set>"`).
4. `tools/engine/selftest.sh` (`lint.py` checks that every set a build names has its
   folder), then build the plugins.

No set is chosen in code: there is no `match` to extend. `render3d/lint-layers.sh` (rule 4)
and `ui/lint-content.sh` keep the names out of the sources: a game file's or race's name
as a literal outside a `content.rs` fails CI, unless it is the engine's own format or
default, marked `// lint-allow(content): <reason>`.

## render3d

render3d is a Cargo workspace: the cdylib, `r3d-core` (the frame, scene, assets, the
module interface, hooks and services) and one crate per module (`mods/<module>/`): world,
particles, hulls, effects, ordnance, specials, sky, overlays. The module list is in
[`render3d/README.md`](render3d/README.md), which is also where each module's hooks,
content and settings are.

- **A module depends on `r3d-core` and on no other module.** What one draws for another
  goes through `r3d_core::services`. `lint-layers.sh` enforces it.
- **Features.** The cdylib has one Cargo feature per module, all on by default.
  `render3d/build.sh --features hulls,sky` builds a subset into `build/subset/`. A
  module that is not built in is not in the DLL; the engine draws its part, and its
  settings rows are not exported. The front end's menu scene needs hulls, sky and world
  to have ships, sky and a planet.
- **A module is one `Module`** (version 2): name, priority, the claims it makes, its
  rows, `configure` (read at load and at every settings change), per-view calls
  (`lights`, `begin_view`, `step`, `object`, `feature`, `end_view`), and `hooks`, `on`
  and `off`. Every patch of `Armada2.exe` goes through `Hooks::install`, which checks
  the site against the profile before writing and returns the previous target to
  chain to. Every detour starts with a gate on the module's on flag.
- **Render only.** Nothing the simulation reads is written, and nothing draws from
  `g_random`; the cosmetic simulation is render3d's own. That is what keeps it
  stock-compatible in multiplayer.

### Content recipes

Skies, planet and moon surfaces, nebulae, ordnance and the menu scene are **recipes**:
text in `render3d/content/<kind>/` (`sky/`, `surfaces/`, `nebulae/`, `ordnance/`,
`menu/`), built into the plugin; a content set's table names the ones its modules take
("Content sets"). A recipe in the game folder wins over
the built-in one of the same name:

| Folder in the game directory | Replaces |
|---|---|
| `Sky\<name>.ini` | the sky recipe of that name (the stock skybox's prefix, for example `mbgaqu`) |
| `Surfaces\<name>.ini` | a planet, moon or asteroid surface recipe |
| `Nebulae\<name>.ini` | a nebula's gas recipe |
| `Menu\map.ini` | the main menu's scene |

These folders are mounts of render3d's file system (`a2fmt::vfs`, the `LAYOUT` table in
`render3d/core/src/files.rs`), like the game's texture roots and its `SOD`, `Sprites` and
`odf` folders (`games/README.md`, "Files"): a recipe is looked up as `sky\<name>.ini`
and the mount says which folder of the game directory that is. In Armada II each is the
folder of that name, so a recipe file goes where it always did.

A missing recipe means the engine's behaviour for that object, as a missing asset does.
The format of each kind's keys is in the README, "Sky recipes", "world" and the other
modules' sections; the shipped recipes in `render3d/content/` are the worked examples.
A recipe is built from measurement, not by eye: the sky recipes are fitted to the
stock sky with `render3d/perf/sky-shape.py` and `sky-grade.py`. A sky recipe for a
new map is a file, with no code.

### Shaders

Shaders are HLSL in each crate's `shaders/`; `tools/shaderc/hlsl-rs.sh` compiles each to
SPIR-V (Vulkan) and DXIL (Direct3D 12) with dxc. The DXIL needs a pinned release,
`tools/shaderc/fetch-dxc.sh` fetches it into `$A2_DATA/tools/` by version and sha256. The
generated files are committed with a stamp, so a machine without the compiler builds
as long as they are current; CI (`verify-generated.sh`) fails on a stale one.
`gpu/shaders/compat.hlsli` holds the few places the two forms part. Constants keep
their registers and loops keep a fixed count and no branches.

## Building a plugin

### The SDK in the release zip

The zip's `sdk/` folder is what a C plugin needs:

| File | What |
|---|---|
| `a2asi.h` | header-only, no CRT: Win32 types and imports, string and log helpers, signature checks and `VirtualProtect` patching (`a2_sig_is`, `a2_poke`, `a2_write_call`, `a2_write_jmp`), `a2_find_export` to reach another plugin's export, the settings types and the overlay lookup |
| `a2engine.h` | the executable's identity and the profile resolver, the C twin of `a2engine` |
| `def/` | one set of import-library `.def` files (kernel32, user32, winmm, ole32, ws2_32) |
| `MODDING.txt` | the short form of this guide |

The source of every layer stays in the repository at the release's tag; nothing is
published as a library and nothing is on crates.io. The Rust crates (`a2rt`, `gpu`,
`a2fmt`, `a2engine`, `a2ini`) are path dependencies in the repository. To build a Rust
plugin on them, work in a checkout of the tag, the way `ui/` and `render3d/` do.

### A C plugin (no MSVC, no MinGW)

A plugin is a 32-bit Windows DLL named `*.asi`. `clang`, `lld-link` and `llvm-dlltool`
are enough, on Linux or macOS:

    llvm-dlltool -m i386 --kill-at -d def/kernel32.def -l kernel32.lib
    clang --target=i386-pc-windows-msvc -O2 -ffreestanding -fno-builtin \
          -fno-stack-protector -mno-stack-arg-probe -c myplugin.c -o myplugin.obj
    lld-link /dll /machine:x86 /nodefaultlib /entry:DllMain@12 \
          /out:MyPlugin.asi myplugin.obj kernel32.lib

`--kill-at` matters: the `.def` files carry decorated names and the real DLLs export
plain ones. `qol/build.sh` is the worked example.

### A Rust plugin

`rustup target add i686-pc-windows-msvc`, `lld-link` for the link, and `/nodefaultlib`
with `kernel32` the only import (an import library from `llvm-dlltool --kill-at`). The
plugin is `no_std` and brings its own allocator and panic handler; `a2rt` supplies
the rest. `alloc::format!` does not link (the prebuilt `alloc` unwinds): use `sfmt!`
or `log!`. `render3d/build.sh` and `ui/build.sh` are the examples.

### Where it goes and how it behaves

Beside `Armada2.exe`, as `<Name>.asi` with an optional `<Name>.ini`. The Ultimate ASI
Loader (`winmm.dll`) loads every `*.asi`; under Wine it needs the DLL override
`winmm=n,b`. A plugin that does not recognise the executable must patch nothing and say
so in its log. Rules for a plugin that sits next to ours:

- Check every site before patching (signature, or the profile's `ready`). Patch in
  memory only.
- **Write a log** with `a2_log_path` (`%LOCALAPPDATA%\armada2-refit\logs\`, falling
  back to the game folder). Log the engine identity and each stand-down.
- **Find another plugin by its export** (`a2_find_export`, which walks the loaded
  modules), never by file name.
- **Never edit `RTS_CFG.h`.** Network games compare a CRC of its bytes, and an edited
  file locks the player out of games with stock players. Scale a value in memory after
  the parse instead.
- Export `<Plugin>_Settings` so the Refit screen shows your keys.
- **Separate rule changes from local ones.** Anything that changes what every machine
  simulates belongs in a plugin that is required of every player and is checked when a
  network game is set up (see `rules/README.md` for the protocol `Online.asi` speaks).

The installers know our layers only. A plugin of yours goes in by hand, or beside ours
in the game directory; they never touch a file they did not put there.

## Testing

The test bench (`./a2test`, [`testbench/README.md`](testbench/README.md)) runs the game
headless on a reflink clone of the install, at any resolution, with scenarios that read
the plugins' own log lines and compare frames against baselines. It runs on the
developer's GPU, so use 1280x720 and stop sessions promptly. Checks that need no game:
`tools/settings/selftest.sh` (descriptors and overlay), `tools/engine/selftest.sh`
(profiles and resolvers), `tools/layers.py --check` (registry against the release
scripts), `retire/selftest.sh` and `publish/check.sh`.

## What you may publish

This repository is public and your fork will be too. The rules are in
[`publish/README.md`](publish/README.md) and are the working form of the law, as the
project reads it (a working position, not legal advice).

1. **Commit nothing derived from the game's own files.** Code, configuration, recipes
   and documents are fine. Textures, models, video, screenshots, stock copies and upscales
   are not, at any path: they go in a data directory outside the checkout (`$A2_DATA`).
   Recipes may hold names, parameters and the SHA-256 of a stock file; they may not hold
   the file. `publish/check.sh` fails on any binary in the index and on an unvendored
   one.
2. **Describe the engine, never reproduce it.** In docs, comments and changelogs you may
   use function and symbol names, addresses and field offsets, a single instruction named
   in a sentence, a short config line quoted in passing, and the short byte signatures a
   plugin checks before patching. You may not publish the symbol map or any excerpt of
   it longer than a name, a disassembly or decompilation listing (not even a few lines
   in a code block), a hex dump, a reconstructed struct or function body, or a game file
   quoted at length. Say what the code does in your own words.
3. **Every finding belongs to a plugin.** Disassembly is permitted to make your own
   program interoperate with the game, and what it yields may be passed on only as far
   as that needs. So a finding goes in the README of the layer whose code uses it, next
   to the hook it justifies, and the profile entry names that hook (`used_by`). An address
   no code reads does not belong in the repository. What you learned by observing the
   running game or measuring its files is not limited this way.
4. **Profiles are data with limits.** Addresses live in the profile files, not in
   prose; a check signature is at most 32 bytes; `tools/engine/lint.py --publish`
   enforces it.
5. **Third-party binaries** only where the licence expressly allows redistribution
   (MIT, zlib and the like), vendored unmodified under `platform/vendor/<name>-<version>/`
   with the licence text and a `SOURCE.txt` listing each file's sha256. "Freeware" and
   no licence do not count. Microsoft's DirectPlay is not redistributable, and nothing
   may be copied from DirectPlay Lite: it is GPL-2.0 and this repository is MIT.
6. **Run `publish/check.sh` before you commit anything unusual**, and
   `tools/doclinks.py` after touching a document.

If you ship a mod built on this base, ship code, recipes and instructions for building
the assets from the player's own copy of the game, the way `textures/PACKS.md` does.

## Where to start

- A quality-of-life change: read `qol/README.md` and `csdk/a2asi.h`, copy `qol/` as a
  skeleton, add a profile file and a `settings.toml`.
- A new look for one object: a recipe under `render3d/content/`.
- A renderer feature: a module in `render3d/mods/`, after the module interface section
  of its README.
- A different build of the executable: a profile, per "How a new build gets a profile".
- A different game: `PLAN-ARMADA1.md` for what moves with it and what does not.
