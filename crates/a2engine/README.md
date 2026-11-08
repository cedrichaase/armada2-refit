# a2engine — which executable this is, and what the plugins know of it

`no_std`, no dependency, linked into Display8, render3d (the core and every module)
and the UI; its C twin is `csdk/a2engine.h`, and `tools/engine/resolve.py` is the same
resolver in Python for the offline tools. Three parts:

- **`pe`** — the identity of an executable: `TimeDateStamp`, `SizeOfImage`, and an FNV-1a
  32 over every section header in table order (the 8 name bytes, then `VirtualAddress`,
  `VirtualSize` and `SizeOfRawData`, little-endian). Read from the main module's mapped
  headers at run time, from the file offline. The file's sha256 is recorded by the
  bench and in the profiles, never a run-time key.
- **`table`** — the types of the generated symbol tables.
- **`engine`** — the resolver: picks the build, then checks each hook's entries in live
  memory before its plugin writes anything.

Every plugin logs, right after its version line:

    engine: Armada2.exe timestamp 0x3c4c76bd, image size 0x3fa000, sections 0xa1a8a321
    engine: build armada2/gog-1.1                (or: engine: unknown build)
    engine: symbols: 15/15 resolved; 0 hooks blocked

A build that is also told by its file or by a companion module (below) adds lines between
them: `engine: <exe> file crc: .text 0x..., .test 0x...` after the identity line, and
`engine: companion <module> ...` after the build line.

**The executable's name** in these lines and in the notes is not a literal of the
resolver: it is the matched build's `exe` (the first of `build.toml`'s list, which
`gen.py` emits as `Build.exe`), or, when no build matches, the process's own module file
name (`GetModuleFileNameA`, read through `Image::name`; `a2e_start`'s `module` in C), or
`exe` when there is none (a buffer). The line keeps its shape, `engine: <exe> timestamp
...`, so the bench and the tools that read it work for any game; plugins that log it once
over several tables recognise it with `a2engine::is_identity_line`. `Engine::exe()` gives
the name and `Engine::game()` the table's game (`armada2`), which names its descriptor in
[`games/`](../../games/README.md).

## The profiles

A profile is keyed by **(game, build)**. Armada II's are in `profiles/<build>/` at the
repository root; another game's in `<game>/profiles/<build>/`. `profiles/` is not a
layer: it is versioned by the root. One directory per build:

- `build.toml` — the identity, the inheritance, the marks;
- `<plugin>.toml` or `<plugin>.<part>.toml` — the entries of one plugin, or of one part
  of it (a render3d module, a UI context). One generated module per file, so packages'
  files stay apart and a module's symbols leave with its crate;
- `content.toml` — the content set (phase 8f).

**What a profile may hold** (hard rules 8 and 9): names from `armada2.map`, addresses,
offsets, values and short check signatures (at most 32 bytes), each tied by `used_by` to a
hook that code in this repository uses, and each hook named in its layer's README beside
the finding that justifies it. Never a listing, a hex dump, a map excerpt, or an entry no
code reads. `tools/engine/lint.py` enforces it, and `publish/check.sh` runs its publish
rules.

### `build.toml`

```toml
schema = 1
game = "armada2"
build = "gog-1.1"
label = "Star Trek: Armada II 1.1 (GOG)"
exe = ["Armada2.exe"]             # one of games/<game>.toml exe.names; the first is logged
inherits = ""                     # or another build of the same game
content = "stock"
[pe]
timestamp = 0x3c4c76bd
image_size = 0x3fa000
sections = 0xa1a8a321             # the FNV fingerprint
image_base = 0x400000
code = [0x401000, 0x6ae000]       # where code kinds may sit (lint); at run time the
                                  # identity's own first executable section is used
[[known]]                         # files seen with this identity: information only
sha256 = "..."
note = "GOG, patch 1.1, as installed"
# Optional, for a build the three numbers cannot tell apart from another (Roots):
#   [pe]  file_crc = { ".text" = 0x.., ".test" = 0x.. }
#   [[module]] name = "FleetOpsHook.dll", text_crc = 0x..          ("Identity", below)
# Only in a build that inherits:
[[changed]]                       # a function this build changes
fn = "mCheckGroupSelect"          # a map name
how = "detour"                    # detour | edit | runtime
verified = ["qol.pan_speed"]      # hooks verified beside the change (globs)
[[disable]]
hooks = ["qolrules.*"]            # globs over hook ids
why = "Roots has its own rules"
```

A build is recognised only when all three of `timestamp`, `image_size` and `sections`
match (and, where it sets `file_crc`, the CRCs below): the timestamp alone is never the key
(Fleet Operations' Roots shares stock's).

### Identity beyond the header (`file_crc`, `[[module]]`)

Roots' builds cannot be told apart by their headers: the next release has the same
timestamp, image size and section table as this one, with other bytes. Two optional parts
of a build (PLAN-FLEETOPS.md, "FO-5: render3d on Roots, the design"):

- **`[pe] file_crc`**: CRC-32 (IEEE, zlib's) of named sections as **the file on disk** has
  them: the first `min(VirtualSize, SizeOfRawData)` bytes at `PointerToRawData`, so the file
  alignment's padding does not count. The file, never the mapped image: FleetOpsHook
  rewrites `.text` and `.rdata` in memory before a plugin of ours starts (`.test` it
  leaves alone). A build that sets it is matched only when the three numbers *and* every
  CRC match; a build that sets none keeps matching on the three numbers (stock 1.1 sets
  none, and `init` reads no file for it). The resolver streams the file in 4 KiB steps
  (`pe::section_crc_file`, `a2e_section_crc_file`; `a2rt`'s `read_file` would hold the whole
  4.8 MB, and `a2engine` has no dependency), once per plugin however many tables it has.
  A match that fails logs the CRCs it read and `engine: unknown build`.
- **`[[module]]`**: a companion module whose run-time patches the build's marks describe,
  named by the CRC-32 of its `.text` **in its file** (`GetModuleHandle`, then the path of
  that module: not its image, which may be relocated or patched). It is not part of the
  match. The log says `engine: companion FleetOpsHook.dll .text crc 0x.. matches the
  profile`, or that it is not the profiled CRC, not loaded, or cannot be read. With
  anything but a match, **every hook that depends on the module's patches stands down**
  (`is patched by FleetOpsHook.dll, which is not the profiled one`; the settings note
  says the same), whatever a mark's `verified` says, since `verified` was beside that
  module. A hook depends on it when one of its entries is or sits `in` a `how = "runtime"`
  function, is a `call`, `jmp` or `slot` with `chain`, or is a `fn` whose check the build
  drops (`check = ""`, a callee the module takes over). The generator computes the list
  (`blocked_mod`, with `n_resolved_mod` for the summary line); the symbol count and
  blocked count in the `symbols:` line are those with the module counted bad.

`a2engine::companion_crc(name)` gives a loaded module's CRC by the same rule, for a part
keyed to the companion alone across the executable's builds (the UI's forms on Roots,
`ui/MENUS.md`, "Roots' forms").

Offline (`resolve.py`, `a2test syms`) the module is looked for beside the executable and
one directory up (Roots keeps `FleetOpsHook.dll` above `Data/`); `syms.report(...,
modules={name: crc | None})` says what the process holds instead (the negative tests).

### `<plugin>[.<part>].toml`

```toml
schema = 1
plugin = "display"                # the hook prefix: hook ids are "<plugin>.<hook>"
readme = "display/README.md"      # where each hook id is written, with its finding
lang = "rust"                     # rust | c
out = "display/src/engine_syms.rs"

[hook.modes]
doc = "..."                       # at most 120 characters
rows = ["Section.Key"]            # settings rows whose SettingsNote this hook feeds
unknown = "stand_down"            # or "run": only when every entry is a scan
planned = "FO-5 step 3"           # optional: profile data ahead of its code (below)

[[sym]]
id = "find_display_mode"          # the code identifier, unique in the build (snake case)
symbol = "ST3D_DisplayDevice::FindDisplayMode"   # the map name, as logged
kind = "fn"
addr = 0x626bf0
check = "55 8b ec 8b 51 14 53 56 57 8b 0a 3b ca"
used_by = ["modes"]
```

| kind | fields | resolves to | checked by `ready` |
|---|---|---|---|
| `fn` | `addr`, optional `check`, optional `end` | the entry (`range`: entry and end) | `check` at the entry |
| `call`, `jmp` | `addr`, `target` (a `fn` id), `in`, optional `chain` | the site | E8/E9, and rel32 to the target; with `chain` a target outside the identity's code is accepted and logged `(chained)` |
| `slot` | `vtable` (a `vtable` id), `index`, `target` (a `fn` id or `"any"`), optional `chain` | the slot's address | the slot holds the target (or chained) |
| `label` | `addr`, `in`, optional `check` | a code address inside a function | `check` |
| `patch` | `addr`, `in`, `check` (required: what is overwritten) | the site | `check` |
| `scan` | `pattern` (hex, `??` wild, at most 32 bytes), `section`, `count`, optional `addr`, `in` | the first match | exactly `count` matches; the one kind that runs on an unknown build |
| `vtable`, `global`, `rtti` | `addr` | the address | none |
| `field` | `struct`, `offset`, `type` (`u8 u16 u32 i32 f32 ptr v3 m34`) | the offset | none |
| `value` | `value` (integer), optional `unit` | the value | none |

A hook with `planned = "<step>"` is written ahead of the code that will use it (a Roots
site whose module is not written yet): lint does not count it, nor an entry only planned
hooks use, as an orphan, and the generated constants of both carry `#[allow(dead_code)]`.
Take the key out when the code reads the hook; it must still be written in its layer's
README. A hook whose every entry a build lacks stands down there (`has no address in
<build>`): a Roots-only site, `render3d.frame.obj_draw`, stands down on stock, which
never asks (`negative.sh` knows it). A plugin that may or may not have such a hook asks
`Engine::present(hook)` first (every entry has an address in the matched build; no
check, no log line) and calls `ready` only when it is true, so stock logs no stand-down
for a site it never had. Its entry still counts in `symbols: N/M` (M is the table's size).

`in` is the enclosing function of every code site; `tools/engine/annotate.py` checks it
and `symbol` against the developer's own `armada2.map` (report only, never written).

### Inheritance

A child build lists only the entries it overrides (same `id`, any field) or adds; the
generator flattens every build, so the run-time code has no inheritance logic. Marks
accumulate down the chain. A hook is **blocked** in a build, with its reason computed
by the generator, when one of its entries is or sits `in` a `[[changed]]` function and
the hook is not in that mark's `verified`, or a `[[disable]]` glob matches it. A
verified hook runs and makes its checks as usual. A child build may override a `field`'s
`offset` or a `value`'s `value` like any other entry, in Rust and C plugins alike:
fields and values are resolved per build at run time ("Fields and values").

Globs: `*` matches any run of characters, everything else itself, over the whole id.

### Content

A plugin's tables of the game's own names (ODFs, sprites, SOD nodes, factions, races,
recipes) are not addresses and do not belong to a build's executable, but to the game
data it runs with. `build.toml`'s `content` names the **content set** its plugins read
(`stock` for `gog-1.1`), and an optional `content.toml` beside it maps a plugin, or one
plugin file (`render3d.hulls`), to another set:

    schema = 1
    [set]
    render3d = "stock"
    "render3d.hulls" = "fleetops"

The nearest build's entry for the file, then for the plugin, wins; a build that names
neither takes the nearest `content` of a `build.toml` up its `inherits` chain; the set
`none` is no set. gen.py writes the resolved id into each table's `Build` (`content`),
and `Engine::content()` returns it: `None` on an unknown build, before `init`, and for
`none`. A set is a folder of the plugin's data, not code: `render3d/content/sets/<set>/`
(one ini table per module) and `ui/content/sets/<set>/` (`ui.ini`), built in at build
time; a plugin's `content.rs` looks up the folder of that id (`by_set` is that lookup) and
parses its table, and keeps an empty `NONE` for every other case: with no set, or a set
with no table for it, it has no built-in names or recipes and the engine's own look shows
through, as for a missing recipe (MODDING.md, "Content sets"). lint.py checks the file
(`schema`, `[set]`, keys that are plugins or plugin files of the game) and that every
set named for a Rust plugin with a `content.rs` beside its generated file is a folder of
the nearest `content/sets/` above that `content.rs`. The game directory's files (`Sky\`, `Surfaces\`, `Nebulae\`) still
win over any set. `Engine::resolved(sym)` is the quiet twin of `addr` (no warning when no hook is ready), for a read that may come before its hook (a settings load); a module's `content::get()` starts its own table at first use, since a recipe is read before any hook. C plugins carry no content (`csdk/a2engine.h` is unchanged).

## The generated code

`tools/engine/gen.py` writes each file's `out` (`--check` in CI). Rust:

```rust
pub mod sym   { pub const FIND_DISPLAY_MODE: a2engine::Sym = a2engine::Sym(0); ... }
pub mod hook  { pub const MODES: a2engine::HookId = a2engine::HookId(0); ... }
pub mod field { pub fn CRAFT_TEAM() -> usize { TABLE.engine().field(Sym(k)) } }
pub mod value { pub fn FRONT_END_WIDTH() -> i32 { TABLE.engine().value(Sym(k)) } }
pub const SYMS, HOOKS, BUILDS;
pub static TABLE: a2engine::Table;
```

C (`<dir>/engine_syms.h`, included once after `a2engine.h`): `<P>_S_<ID>`,
`<P>_H_<HOOK>`, a resolved variable `A2E_U32 a2s_<p>_<id>` per address entry (filled
when its hook is ready, so an asm stub can jump through `_a2s_<p>_<id>`), a variable
`A2E_U32 a2f_<p>_<id>` per field and value (the build's offset or value, below) with
`<P>_F_<ID>` and `<P>_V_<ID>` reading it as an `int`, and
`static const A2ETable <P>_ENGINE`.

### Fields and values

A `field`'s offset and a `value` are the matched build's, read at run time, never a
compile-time constant: a build whose structs differ (another game, a patch) gets its own
offsets from its profile, and a child build overrides one by id. In Rust `field::X()`
and `value::X()` are a load and an index into the matched build's entries
(`Engine::field`, `Engine::value`); in C `<P>_F_X` reads `a2f_<p>_<x>`, which
`a2e_start` fills from the matched build (`a2e_field(k)` is the same read), so an asm
stub reads a field as `movl _a2f_<p>_<x>, %eax` and then `(%ebx,%eax)`. Before `init`
and on an unknown build they are the root build's (no hook of ours runs there but the
scans, so nothing of ours reads them); in a build that has no such field they are 0, and
every hook whose `used_by` names it stands down (`has no address in <build>`). They are
not tied to a hook's readiness: code that reads many fields of many objects reads each
accessor once, outside the loop. A frame local of a patched function, read by a stub
that runs inside it, is a `value` with `unit = "ebp"`, its offset from `ebp`
(negative values are written in decimal: TOML has no negative hex).

## The resolver

```rust
a2engine::init(&engine_syms::TABLE, log)        // once, in DllMain or at first use
let e = engine_syms::TABLE.engine();
e.build()          // Some("armada2/gog-1.1") or None
e.code()           // the identity's first executable section, as mapped
e.ready(hook::MODES)   // Ok(()) or Err(StandDown), checked once and cached
e.addr(sym::X)     // relocated; 0 (logged once) unless a hook using it is ready
e.field(Sym(k)) / e.value(Sym(k))   // the build's offset or value (field::X() calls it)
e.range(sym::X)    // a fn with `end`
e.site(sym::X)     // (symbol, site, expected target) for render3d's core_hooks::Site
e.note(hook) / e.note_for(section, key) / e.note_plugin()   // SettingsNote texts
```

C: `a2e_init(&QOL_ENGINE, log)`, `a2e_ready(QOL_H_X)`, `a2e_addr(QOL_S_X)`, `a2e_field(k)`,
`a2e_build()`, `a2e_note(...)`, `a2e_note_for(...)`, `a2e_note_plugin()`,
`a2e_code(&lo, &hi)`, `a2e_call_target(at, &dest)`; the host core is `a2e_start` (the
image as a buffer), compiled with `A2E_HOST`.

**`init`** identifies the process's own image (its headers only: safe in `DllMain`),
picks the build whose three identity numbers match, relocates by `base − image_base`,
and logs the identity, the build and `symbols: N/M resolved; H hooks blocked` (`N`: the
build's entries minus those used only by blocked hooks; on an unknown build, the scan
entries alone).

**`ready(hook)`**, before the plugin writes anything for that hook, decides in this
order: an unknown build (unless the hook is `unknown = "run"`); a block of the build; a
test mismatch; then every entry is resolved and checked in memory, all before any
failure is reported, and the first failure is the reason. One line per stand-down:

    engine: <hook> stands down: <symbol> <why>

with *why* one of `is not stock (bytes differ)`, `calls something else`, `is changed by
<build> (not yet verified)`, `is off in <build>: <why>`, `unknown build`, `test mismatch
(A2EngineTest.ini)`, `has no address in <build>` (an entry only a child build has) or
`is found <n> times, not <count>` (a scan). A chained site is logged
(`engine: <hook>: <symbol> at 0x.. calls 0x.. (chained)`) only when the hook is ready.

**Notes** (at most 120 characters): `Stands down: <symbol> is not stock in this
<exe>`, `Stands down: <symbol> is changed by <build>, not yet verified`,
`Stands down: off in <build>: <why>`, `Stands down: <symbol>: test mismatch
(A2EngineTest.ini)`, and for a whole plugin on an unknown build `This <exe> is a
build Refit does not know: it runs stock`, `<exe>` named as in the identity line
(`Armada2.exe` on Armada II).

### The test mismatch file

When `A2EngineTest.ini` sits beside the exe, `[Test] Mismatch=<glob>[,<glob>]` makes
`ready` fail the matching hooks as a failed check would, and every plugin logs
`engine: TEST: A2EngineTest.ini forces a mismatch for <globs>` at init. It proves the
per-hook stand-down end to end on a stock exe without editing code bytes the game runs.
It is the bench's: nothing ships it, and `./install` deletes it.

## Tests

Fixtures 08 to 12 are the identity beyond the header (build `b4`: `file_crc` and a
`[[module]]`): the module matching, differing, absent and unreadable, and a `.text` byte
changed so that the three numbers match and the CRC does not. The C leg reads the image
file for the CRCs (`ctest.c`, `module NAME CRC` arguments), the Rust leg's `Mapped`
implements `Image::file_crc` and `Image::companion`.

`tools/engine/selftest.sh` (CI job `engine`): the three resolvers on every fixture
(`tools/engine/README.md`, "Fixtures"; synthetic PE images written at test time, no game
file), gen's golden output, and lint. `cargo test` here runs the identity tests, `tests/fields.rs` (a child build that moves
a field, an unknown build reading the root's) and its C twin (`tests/identity.c`, run by
`tests/c_twin.rs`); with
`--features fixtures` and `A2E_FIXTURES_RS` (what `tools/engine/fixtures.py run rs`
sets) it runs the resolver fixtures too.
