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
match: the timestamp alone is never the key (Fleet Operations' Roots shares stock's).

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

`in` is the enclosing function of every code site; `tools/engine/annotate.py` checks it
and `symbol` against the developer's own `armada2.map` (report only, never written).

### Inheritance

A child build lists only the entries it overrides (same `id`, any field) or adds; the
generator flattens every build, so the run-time code has no inheritance logic. Marks
accumulate down the chain. A hook is **blocked** in a build, with its reason computed
by the generator, when one of its entries is or sits `in` a `[[changed]]` function and
the hook is not in that mark's `verified`, or a `[[disable]]` glob matches it. A
verified hook runs and makes its checks as usual. Fields and values used by a
`lang = "c"` file are compile-time per game and may not be overridden (FO-3); in Rust
they are consts while every build agrees.

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
`none`. A plugin picks its tables by that id in its `content.rs` (`by_set`), and keeps
an empty `NONE` for every other case: with no set it has no built-in names or recipes
and the engine's own look shows through, as for a missing recipe. lint.py checks the
file (`schema`, `[set]`, keys that are plugins or plugin files of the game) and that a
Rust plugin with a `content.rs` beside its generated file chooses every set named for
it (`"<set>" =>`). The game directory's files (`Sky\`, `Surfaces\`, `Nebulae\`) still
win over any set. `Engine::resolved(sym)` is the quiet twin of `addr` (no warning when no hook is ready), for a read that may come before its hook (a settings load); a module's `content::get()` starts its own table at first use, since a recipe is read before any hook. C plugins carry no content (`csdk/a2engine.h` is unchanged).

## The generated code

`tools/engine/gen.py` writes each file's `out` (`--check` in CI). Rust:

```rust
pub mod sym   { pub const FIND_DISPLAY_MODE: a2engine::Sym = a2engine::Sym(0); ... }
pub mod hook  { pub const MODES: a2engine::HookId = a2engine::HookId(0); ... }
pub mod field { pub const CRAFT_TEAM: usize = 0x..; }
pub mod value { pub const FRONT_END_WIDTH: i32 = 800; }
pub const SYMS, HOOKS, BUILDS;    // consts: a value-parity assert can read them
pub static TABLE: a2engine::Table;
```

C (`<dir>/engine_syms.h`, included once after `a2engine.h`): `<P>_S_<ID>`,
`<P>_H_<HOOK>`, `<P>_F_<ID>`, `<P>_V_<ID>`, a resolved variable
`A2E_U32 a2s_<p>_<id>` per address entry (filled when its hook is ready, so an asm stub
can jump through `_a2s_<p>_<id>`), and `static const A2ETable <P>_ENGINE`.

## The resolver

```rust
a2engine::init(&engine_syms::TABLE, log)        // once, in DllMain or at first use
let e = engine_syms::TABLE.engine();
e.build()          // Some("armada2/gog-1.1") or None
e.code()           // the identity's first executable section, as mapped
e.ready(hook::MODES)   // Ok(()) or Err(StandDown), checked once and cached
e.addr(sym::X)     // relocated; 0 (logged once) unless a hook using it is ready
e.range(sym::X)    // a fn with `end`
e.site(sym::X)     // (symbol, site, expected target) for render3d's core_hooks::Site
e.note(hook) / e.note_for(section, key) / e.note_plugin()   // SettingsNote texts
```

C: `a2e_init(&QOL_ENGINE, log)`, `a2e_ready(QOL_H_X)`, `a2e_addr(QOL_S_X)`,
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

`tools/engine/selftest.sh` (CI job `engine`): the three resolvers on every fixture
(`tools/engine/README.md`, "Fixtures"; synthetic PE images written at test time, no game
file), gen's golden output, and lint. `cargo test` here runs the identity tests; with
`--features fixtures` and `A2E_FIXTURES_RS` (what `tools/engine/fixtures.py run rs`
sets) it runs the resolver fixtures too.
