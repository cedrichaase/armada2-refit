# tools/settings: the settings descriptors and the player's overlay

Every plugin of ours reads its keys from an ini in the game directory (`Render3D.ini`,
`UI.ini`, `QOL.ini`, ...), which its installer writes. A choice the player makes goes
to **one other file that no installer writes**, `RefitPlayer.ini`, read in front of
every one of them. Each plugin describes the keys a player may set in a table of
**descriptors** it exports, `<Plugin>_Settings`, which the UI's Refit screen lists
(ROADMAP.md Phase 6). This folder holds the schema of those tables, the generator that
writes them from each layer's `settings.toml`, their lint, the overlay's writer for
the bench and the GUI installer, and the fixtures that pin all of it. The contract is
**A2SET_ABI 1.1**. Nothing here is installed; it is versioned by the root.

| File | What it is |
|---|---|
| `gen.py` | the generator of every layer's `settings_table` from its `settings.toml` |
| `lint.py` | the tables against the schema, each other, the shipped inis and the sources |
| `overlay.py` | the overlay's edit for the bench: a wrapper over the `a2ini` host tool |
| `host.sh` | builds `a2ini` (`crates/a2ini`, feature `std`) into the ignored `build/` and prints its path |
| `move-keys.py` | moves a plugin-ini key a row took over (`moved_from`); `./install` runs it |
| `inival.py` | what the tools read of an ini and of the overlay's names (reading only) |
| `fixtures/` | `NN-name.in` / `.args` / `.out`: the spec of the edit (`rename/`: of the rename); `tables/`: tables for the generator and the lint; `bad-cases.toml`: the tables the lint must refuse, one case each |
| `selftest.sh` | all of the above against the fixtures (CI runs it) |
| `bench-overlay.sh` | sets overlay keys in a bench clone from a scenario's `Setup:` line |

The code is `crates/a2ini` (the parser and the edit, `no_std`, host-tested on the same
fixtures), `crates/a2rt/src/{settings.rs,overlay.rs}` and `Ini` in `crates/a2rt`
(Render3D, UI, Display8), the `a2_ini_*` / `a2_set_*` helpers of `csdk/a2asi.h` (the
C plugins) and the hub, `ui/src/settings.rs`.

## The overlay

`RefitPlayer.ini` sits in the game directory, beside `Armada2.exe`. Installers, `a2mod`,
`tools/ini-merge.py` and the release package never write, ship, move or delete it, and
an uninstall leaves it. Its sections are `[<Stem>.<Section>]`: `<Stem>` is the plugin
ini's name without `.ini`, `<Section>` and the key are spelled as in that ini, and names
match without regard to case, as `GetPrivateProfileString` matches them:

```
[Render3D.Modules]
hulls=0
[Render3D.World]
Surfaces=0
[UI.HUD]
BarHidden=1
[QOLRules.Rules]
PayOnQueue=0
```

On Windows a game under `Program Files` has the file redirected to the VirtualStore,
as the logs were before 13.50.0; the game does not notice. The GUI installer's report
looks in both places.

**Lookup.** A key is read from the overlay, then from the plugin's ini, then the
default. Every key of every ini directly in the game directory takes part
(`RefitPlayer.ini` itself and the old `UIPlayer.ini` excepted); files in subdirectories
(`Sky\*.ini`, the recipes) do not. An overlay value that is empty or only whitespace
counts as absent. A value is parsed as the same value in the plugin's ini would be: in
Rust `parse_i32` / `parse_f32` with the default on failure, in C by
`GetPrivateProfileIntA` / `StringA` on the overlay itself. `Ini::sections()` lists the
plugin file's sections only.

- Rust: `a2rt::Ini::open(path)` gives a file of the game directory its overlay (the
  stem is the file's name); `Ini::open_plain` reads the file alone. The overlay is
  cached like every ini (its stamp looked at no more than twice a second), and
  `a2rt::ini_refresh_all()` marks every cached file stale.
- C: `a2_ini_open(&ini, "QOL.ini", logline)`, then `a2_ini_int` / `a2_ini_str` for a
  key, or `a2_set_int` / `a2_set_str` for a row, whose default is the row's `def`.

**The log line**, the same in both languages, in the plugin's own log, the first time a
value is read from the overlay and again when it changes:

```
settings: RefitPlayer.ini [Render3D.World] Surfaces=0 overrides Render3D.ini's Surfaces=1
settings: RefitPlayer.ini [UI.HUD] BarHidden=1 (UI.ini has no BarHidden)
```

Bench scenarios grep for `settings: RefitPlayer.ini [`. Rust keeps the (key, value)
pairs it logged; C keeps 64 pairs of FNV-1a hashes, of `Stem.Section.key` and of the
value.

## The writer

One process writes the overlay (the game) and one DLL in it (`UI.asi`, through
`a2rt::overlay::set` / `set_row`); the C plugins have no writer, and Wine's
`WritePrivateProfileString` is not used. The edit is a pure function,
`a2ini::edit(text, section, key, value)`; the bench and the installers run the same
code through the `a2ini` host tool (`a2ini get|edit|unset|rename`, built by `host.sh`),
and `cargo test` of `crates/a2ini` runs every fixture. The text is handled as latin-1, so a byte
the edit does not touch is kept whatever the file's encoding. The rules, which the
fixtures pin:

1. Line endings are the file's own (its first); a new file gets CRLF; the result always
   ends in a newline.
2. A section line starts with `[` after trimming; its name runs to `]`, trimmed. A key
   line in a section is a line that, trimmed, does not start with `;` or `#`, holds
   `=`, and whose trimmed left side is the key. Names match without regard to ASCII
   case. A section written twice counts as one.
3. Setting an existing key: the first matching line keeps its text up to and including
   `=` and takes the value after it (a trailing comment on it goes). Later duplicates of
   the key in the section are removed.
4. Setting a missing key of an existing section: `key=value` goes after the last key
   line of the section's last occurrence, or right after its header when it has none.
5. Setting a key of a missing section: a blank line (unless the file is empty or ends
   blank already), `[section]`, `key=value`.
6. Removing: every matching key line in the section goes. An occurrence of the section
   that had a line removed and is left with nothing but blank lines loses its header
   and those lines; one that still holds a comment keeps its header.
7. Comments, blank lines, unknown sections and keys and the spelling of everything
   untouched are kept byte for byte. When nothing changes, the input comes back as it
   was.
8. A new file starts with two comment lines (`a2ini::HEADER`).
9. A value holding CR or LF, or leading or trailing whitespace, is refused (the input
   comes back unchanged; the caller logs it), as is a section holding `]` or a key
   holding `=` or starting `;`, `#` or `[`.

`a2rt::overlay::set` reads the overlay, applies the edit and does nothing when the text
is unchanged; otherwise it writes `RefitPlayer.ini.tmp`, renames it over the overlay,
calls `ini_refresh_all()` and logs `settings: RefitPlayer.ini [S] K=V written` (or
`K removed`). `set_row(row, value)` removes the key instead of storing it when the value
is what the plugin would read without the overlay: its ini's value (`open_plain`), else
the row's `def`, compared trimmed and without regard to ASCII case. With the shipped ini
that is ROADMAP's rule, that only choices which differ from the default are stored,
and it keeps a choice from being swallowed when the player has edited the plugin's ini
by hand.

## The descriptors

Each plugin exports, cdecl and undecorated (`llvm-objdump -p` shows it):

```
const A2Setting *__cdecl <Plugin>_Settings(UINT *count)                                  (C)
#[unsafe(no_mangle)] pub extern "C" fn <Plugin>_Settings(count: *mut u32) -> *const Setting   (Rust)
```

The names are fixed: `Render3D_Settings`, `UI_Settings`, `QOL_Settings`,
`QOLRules_Settings`, `Online_Settings`, `BinkProxy_Settings`, `Display8_Settings` (read
by the UI hub since ui 1.0.0). `count` may be NULL; the rows are static and live as long as the DLL. A row,
C ABI, i686, 48 bytes in version 1.1 (44 in 1.0):

| off | field | type | |
|---|---|---|---|
| 0 | size | u32 | the row's size as built; readers step by `rows[0].size` |
| 4 | kind | u8 | 1 BOOL, 2 ENUM |
| 5 | apply | u8 | 1 LIVE, 2 MISSION (next mission or load), 3 RESTART |
| 6 | scope | u8 | 1 LOCAL, 2 MATCH (must match in a network game) |
| 7 | tab | u8 | 0 NONE, 1 GRAPHICS, 2 INTERFACE, 3 GAMEPLAY, 4 MULTIPLAYER, 5 MOVIES |
| 8 | flags | u32 | 1 ADVANCED, 2 NOT_SHIPPED (not in the shipped ini), 4 NO_ROW (not on the Refit screen) |
| 12 | file | const char* | the plugin's ini, a file name in the game directory |
| 16 | section | const char* | |
| 20 | key | const char* | |
| 24 | def | const char* | the default as written in the ini |
| 28 | values | const char* | ENUM: `0\|2\|4\|8`; BOOL: NULL, meaning `0\|1` |
| 32 | value_labels | const char* | ENUM: `Off\|2x\|4x\|8x`; BOOL: NULL, meaning `Off\|On` |
| 36 | label | const char* | English, at most 40 characters |
| 40 | help | const char* | English, at most 200 characters, may be empty |
| 44 | moved_from | const char* | 1.1: `Stem.Section.Key` the value is moved from (below), or NULL; read only when `size` >= 48 |

Strings are ASCII. A BOOL is on when its value parses as a non-zero integer. An ENUM
value outside `values` (a hand edit) is kept and shown as "Custom (v)"; nothing rewrites
it until the player picks a value. Rows are in screen order within a tab. A minor
change only appends fields (the size grows) or adds enum values or flags; a reader
refuses a table whose `rows[0].size` is below 44 (1.0's row; a 44-byte table has no `moved_from`), hides a row with a kind, apply, scope
or tab it does not know, and logs each once. A major change renames the export
(`<Plugin>_Settings2`).

The row is written by hand, once per language, beside its size and offset asserts:
`crates/a2rt/src/settings.rs` (`#[repr(C)] pub struct Setting`, `KIND_*`, `APPLY_*`,
`SCOPE_*`, `TAB_*`, `FLAG_*`) and `csdk/a2asi.h` (`A2Setting`, the `A2SET_*` defines,
negative-array asserts). `gen.py` writes the tables only, so a row field is added in
those two places and in `gen.py`'s emitters, and the asserts fail a compile that missed one.

**A layer's table is data**, `<layer>/settings.toml` (or `<layer>/<name>.settings.toml`,
as `qol/rules.settings.toml`):

```toml
plugin = "Render3D"                 # the export's prefix
file = "Render3D.ini"               # the plugin's ini in the game directory
shipped = "render3d/Render3D.ini"   # the shipped ini, a repository path

[[row]]
name = "world_surfaces"             # the row's constant: WORLD_SURFACES
section = "World"
key = "Surfaces"
kind = "bool"                       # bool | enum
default = "1"
apply = "live"                      # live | mission | restart
scope = "local"                     # local | match
tab = "graphics"                    # none | graphics | interface | gameplay | multiplayer | movies
label = "Planet and asteroid surfaces"
help = ""
# values = ["0", "2", "4", "8"]     # an enum's, with as many
# labels = ["Off", "2x", "4x", "8x"]
# flags = ["advanced", "not_shipped", "no_row"]
```

`gen.py` writes it beside the table, committed and headed "generated by
tools/settings/gen.py from X; do not edit": `src/settings_table.rs` in a Rust crate
(a `pub static SETTINGS`, one `pub const NAME: &Setting` per row and the export), else
`settings_table.h` (`<PLUGIN>_SETTINGS`, `#define <PLUGIN>_ROW_<NAME>` and the export;
included once, after `a2asi.h`). `rules.settings.toml` gives `rules_settings_table.h`.
A Rust crate adds `mod settings_table;` (or `pub mod`). Plugin code reads a row's key
only through its row, `rt::settings::on(&ini, WORLD_SURFACES)` (also `int`, `float`,
`value`) or `a2_set_int(&ini, QOL_ROW_PANSPEED)`, so the code's default is the table's.

**The lint** (`lint.py`, every table, or the ones named):

1. the schema: kind, apply, scope, tab and flags known; a BOOL's default 0 or 1; an
   ENUM's default among its `values`; as many `labels` as `values`; label and help
   lengths, printable ASCII;
2. no (file, section, key) twice across the tables;
3. each row's key in the shipped ini, in its section, at `default` (unless NOT_SHIPPED);
4. a NOT_SHIPPED key absent from the shipped ini;
5. the generated file up to date;
6. a `moved_from` is `Stem.Section.Key` and not the row's own key;
7. within one plugin's table and its fragments (below), a section has one owning table,
   except the sections the plugin's table lists in `shared_sections` (render3d's
   `[Lights]`, split by key).

### Fragments

A plugin made of modules (render3d) keeps each module's rows beside the module's code,
in a **fragment**: a `settings.toml` in a directory below the plugin's table, with the
same `plugin` and `file` and a `module`:

```toml
plugin = "Render3D"
file = "Render3D.ini"
module = "world"          # the module's [Modules] key; `shipped` is the plugin table's

[[row]]
...
```

`gen.py` writes `settings_table.rs` beside a fragment (`src/settings_table.rs` beside a
Cargo.toml): `pub static ROWS`, one `pub const NAME: &Setting` per row and
`pub const MODULE`, and no export. A plugin table with fragments (a Rust crate only)
gets an export that returns `crate::settings_rows()`, which the plugin provides: its own
rows, then each built-in module's `ROWS` in the plugin's module order, copied once into
one contiguous array (so the export is still one table of 48-byte rows, A2SET_ABI 1.1;
a module not built in has no rows, and the Refit screen shows none). The plugin table
may list `shared_sections`, and `rows_in = "<dir>"`, a crate directory below it: the
table's generated file then goes into `<dir>/src/settings_table.rs`, without the export,
which the plugin's cdylib defines itself (render3d: the rows are `r3d-core`'s, the
export `render3d/src/lib.rs`'s, which leaves out the `[Modules]` row of a module not
built in). The lint takes a plugin's table and its fragments as one
table: rule 2 across them, rule 3 against the plugin's one shipped ini, rule 5 over the
whole plugin's sources, rule 7 between them. `gen.py --rows TABLE.toml [MODULE...]`
prints the exported rows (`Section Key`) with the fragments in the order given;
`selftest.sh` checks render3d's against `fixtures/render3d-rows.txt` (the order before
the split) with the modules in their `priority()` order.

## The hub and `_SettingsChanged`

`void __cdecl <Plugin>_SettingsChanged(void)` is optional, exported only by a plugin
with LIVE (or MISSION) rows. The hub calls it on the game thread, from the UI's frame,
after the overlay has been renamed into place. It is idempotent and cheap (no file I/O
beyond the ini re-read, nothing blocking). A Rust plugin calls `a2rt::ini_refresh_all()`
first (each DLL has its own ini cache), then reads its live rows again. It logs one line,
`settings: applied <Section.Key=v, ...>` or `settings: re-read, nothing changed`. MISSION
rows are latched at the next mission start; RESTART rows are not applied again.

The hub is `ui/src/settings.rs`. At its first frame (shell or mission) it migrates
`UIPlayer.ini` (below), walks the loaded modules (`K32EnumProcessModules`) and asks each
for the fixed export names, so no DLL is looked up by file name, and logs
`settings: <Export> in <module>, N rows` or `settings: no <Export>`. Once a second it
looks at the overlay's stamp; a change it did not make (a hand edit, the bench) is
passed to every `_SettingsChanged`, logged as
`settings: RefitPlayer.ini changed; told Render3D, QOL, ...`. After its own writes it
calls them at once (`... written; told ...`). Without UI.asi an overlay change applies
at the next launch.

**`UIPlayer.ini`** (ui 0.13 to 0.28) held the one setting of the UI's before the
overlay, `[HUD] BarHidden`. When it exists and the overlay has no `[UI.HUD] BarHidden`,
a non-zero value is set there; then the file is renamed `UIPlayer.ini.migrated`, and one
line is logged (`settings: UIPlayer.ini migrated to RefitPlayer.ini [UI.HUD] (...)`).

## Moved keys (`moved_from`, A2SET_ABI 1.1)

A row may carry `moved_from = "<Stem>.<Section>.<Key>"` in its `settings.toml`: the
key it took over from another file (`UI.UI.KeepClip` for `Display8.ini`'s
`[Window] KeepClip`; the stem has no dot, the section may). Two places hold a player's
value, and each has one mover:

- **The overlay**: at its first frame the hub (`ui/src/settings.rs`) checks each loaded
  table's rows, size 48 or more. For every `moved_from` key present in `RefitPlayer.ini`
  it moves the value to the row's own `[<Stem>.<Section>] Key` (`a2ini::rename`; when
  both are present the new key wins and the old one is deleted), logs
  `settings: RefitPlayer.ini [UI.HUD] BarKeys moved to [QOL.QOL] BarKeys` and tells the
  plugins. The UI stays the overlay's one writer. Without `UI.asi` an old overlay key is
  ignored and the shipped value applies.
- **The plugin inis**: `./install` runs `move-keys.py <game dir>` after every layer. When
  the old file holds the old key, a value other than the row's default goes to the row's
  file and key, and the old key is removed either way; one line per key. The release
  scripts do the same from `moved.txt`, which `publish/package.sh` bakes from these tables.

Both are idempotent: the old key is gone after the first run.

## `<Plugin>_SettingsNote` (C5 amendment)

`const char *__cdecl <Plugin>_SettingsNote(const A2Setting *row)` (C), or
`#[unsafe(no_mangle)] pub extern "C" fn <Plugin>_SettingsNote(row: *const Setting) -> *const u8`
(Rust), is optional and exported next to `_SettingsChanged`; it is not part of the row layout. `row == NULL` asks about the plugin as a whole; otherwise
`row` is a pointer into that plugin's own table, as `_Settings` returned it, so it may be
compared by address with the plugin's row constants. It returns NULL when there is
nothing to say, else a static NUL-terminated ASCII string of at most 120 characters,
valid until the next call. The meaning is always "this part is not running now, and
why"; the choice is still stored and applies when the part runs. The hub calls it on the
game thread when the Refit screen opens and once a second while it is open, so it must be
cheap and must not write. The hub finds it with `GetProcAddress` beside
`_SettingsChanged` (`NoteFn` in `crates/a2rt/src/settings.rs`, `A2SettingsNoteFn` in
`csdk/a2asi.h`, both outside the generated parts). UI, Render3D, QOL, QOLRules, Online and
BinkProxy export it (the C plugins since the symbol profiles of Phase 8).

## On the bench

A scenario sets overlay keys before launch with

```
Setup: tools/settings/bench-overlay.sh Render3D.Modules:sky=0 showcase Seed=7 Pause=200
Setup: tools/settings/bench-overlay.sh UI.HUD:Minimap=0 ui/bench-asi.sh --menus
Setup: tools/settings/bench-overlay.sh --into Display8.ini Samplers:Anisotropy=8 ui/bench-asi.sh
```

the word syntax of `render3d/bench-ini.sh`: each `Section:Key=value` sets a key (the
section as in the overlay), `--unset Section:Key` removes one, and what follows is the
scene or script the scenario would have run. `overlay.py FILE WORD...` (the `a2ini` tool) does the same by
hand, and `overlay.py --get FILE Section:Key` prints a value. `selftest.sh` runs every
check here without a game.
