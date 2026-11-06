# tools/engine — the symbol profiles' tools

The format, the generated code and the resolver are described in
`crates/a2engine/README.md`. Stdlib Python only; nothing here reads a game file except
`annotate.py` (the developer's own map, report only).

| Tool | What it does |
|---|---|
| `profiles.py` | loads every profile set (`profiles/`, `*/profiles/`), flattens builds, computes blocks; shared by the others |
| `gen.py` | writes every profile file's `out`; `--check`, `--emit FILE [--lang rust\|c]`, `--list-hooks PLUGIN`, `--profiles DIR...` |
| `lint.py` | the validator (rules 1–6; `--publish` runs 1, 2 and 5 for `publish/check.sh`; `--selftest` on `fixtures/lint/`). A literal in a plugin source that is in the image's range but no address takes a `// not an address` comment; a migration's transitional const-asserts take `// value parity`. A literal struct offset fails too (C `p + 0x..` and asm `0x..(%reg)`; Rust `field(x, 0x..)`, `rd(x + 0x..)`, `(x + 0x..) as *`, `const X: usize = 0x..`), except on a line that says `PE header` (the module's own headers) or `not an offset` |
| `moved.py` | AR-3's equivalence test: every offset and value moved out of a plugin's source (`moved-gog-1.1.tsv`, with the value the source had) resolves to it on `gog-1.1` |
| `resolve.py` | the resolver in Python, offline on an exe file (`--exe`, `--build` to force a profile, `--base`, `--test-ini`); what `a2test syms` reports with |
| `annotate.py` | checks each entry's `symbol` and `in` against `$A2_GAME/armada2.map`; prints names only, writes nothing |
| `mkpe.py` | writes a fixture's synthetic PE image (`--identity` prints its three numbers, to put in the fixture set's `build.toml`) |
| `fixtures.py` | runs the fixtures on the three resolvers (`run [py c rs]`); `bless` writes Python's transcripts into them, to be reviewed as the spec |
| `ctest.c` | the C resolver's host driver (`csdk/a2engine.h` with `A2E_HOST`) |
| `negative.sh` | the negative tests on scratch copies of the game's exe (timestamp changed, one checked entry changed per plugin file, a forced child build, `a2test syms --record`), and on Roots' exe: its marks, a FleetOpsHook.dll that is not the profiled one (a wrong CRC, none, unreadable, and a modified file beside the exe), one byte of `.text` or `.test` changed; skips the exe-based ones without a game |
| `bench-exe.sh`, `bench-mismatch.sh` | the bench's `Setup:` scripts: a clone's exe stamped (unknown build), a clone's `A2EngineTest.ini` (`testbench/README.md`, "The symbol profiles, offline and on the bench") |
| `fo-scan.py`, `fo-scan.toml`, `fo_scan_test.py` | profiles a new Fleet Operations: Roots build in one command (below); the test runs it on synthetic images |
| `selftest.sh` | all of the above, as CI's `engine` job runs it |

## Fixtures

`fixtures/NN-name.toml`: a synthetic image (`[image]`: timestamp, image base and size,
sections, and the bytes, calls and words memory holds), the profile set it is resolved
against (`set`, a directory of `fixtures/sets/`), optionally `base` (where it is mapped),
`module` (the process's module file name, which an unknown build is logged under;
default `Fixture.exe`, while a matched build is logged under its `exe`), `test_ini`, `addr` (symbol ids queried after the readies), `field` (field and value ids
whose per-build offset or value is queried after them) and `note_for` rows, and
`[expect]`: per plugin table, the transcript every resolver prints:

    <init's log lines>
    <for each hook in table order: the lines its ready logs, then> ready <hook> ok|fail
    <for each addr query: its lines, then> addr <id> 0x<hex>
    field <id> 0x<hex>                (each field query: the matched build's, the root's
                                       on an unknown build, 0 where the build has none)
    note <hook> <text>                (each hook with a note)
    note_for <Section.Key> <text|->
    note_plugin <text|->

Besides the transcripts a fixture may carry `[content]`, the content set the table
resolves to (`none` for no set), checked by Python (`profiles.Build.content_for`) and by
Rust (`Engine::content()`); `07-content-none.toml` is the child build `b3`, which gives
`p` no set and `s` another (`sets/basic/b3/content.toml`), and 01 to 03 expect `stock`
and, on the unknown timestamp, `none`.

They cover stock, an unknown timestamp, stock's timestamp with another image size and
no build for it, a child build with `[[changed]]` (one hook verified), `[[disable]]`, an
overridden and an added entry (the Roots shape), a field the child build moves (`fd`,
0x1c in `b1`, 0x20 in `b2`) and one only the child has (`fx`: in `b1` the hook
`p.fieldhook` that uses it stands down), a chained call (and one in a hook that
fails, which must not log it), a call and a slot that hold something else, scans with
zero, one and two matches, a relocated base, and the test mismatch file. The images are
written into a temp directory at test time; no binary is committed.

`fixtures/golden/` holds gen's Rust and C output for the fixture set's `p.toml`;
`fixtures/lint/` the validator's good set and one bad set per rule it enforces.

## fo-scan.py: profiling a new Roots build

    ./tools/engine/fo-scan.py [--game fo-roots] [--bench | --memdiff LOG | --no-runtime] [--verify] [--gen]

For a new `ArmadaL.exe` (a Roots release), one command takes the installed file to a
`profiles/fo-roots-<version>/` and a private report. It replaces the scratch scripts of
FO-2/FO-3 (the file diff, the log reader and the mark writer) and runs, in order:

1. **identity**: timestamp, image size, section fingerprint (the profile's key), the sha256
   and the CRC-32 of each section's content (`min(VirtualSize, SizeOfRawData)` bytes of the
   file), the file and the headers, and the version from `Roots_Download_Version.txt`, which
   names the build `fo-roots-<version>`. The CRCs of the sections `fo-scan.toml` names
   (`.text`, `.test`) are written as `[pe] file_crc`, and the CRC of the `.text` of each
   companion module it names (`FleetOpsHook.dll`, found beside the install, one directory
   up, or `--module NAME=PATH`) as a `[[module]]` (crates/a2engine/README.md, "Identity
   beyond the header");
2. **file diff** against stock 1.1's `Armada2.exe` (checked against `gog-1.1`'s identity)
   section by section, ranges of changed bytes (a gap of up to three equal bytes joins
   two), named by function from stock's own `armada2.map`, read at run time from the
   player's install and never copied. A function is a `detour` when a changed `jmp` or
   `call` leads into a section only Roots has, else an `edit`;
3. **run-time diff**: `--bench` runs `testbench/memdiff/` in a bench clone (one game at
   1280x960, 4 GB free required, session always stopped) and keeps the log under
   `$A2_DATA/research/fo-roots/memdiff/MemDiff-<build>.log`; later runs reuse it, or take
   `--memdiff LOG`. `--patches TSV` cross-checks the log against a private patch table;
4. **marks**: the changed functions are intersected with every function the profiles name
   (an entry's `symbol`, or its `in`). File changes are `detour`/`edit`, the rest
   `runtime`. A function listed as a *callee* in `fo-scan.toml` (ours only call it) whose
   changes all lie in its first eight bytes (a jump or trampoline) does not stand its
   hooks down: its entries get `check = ""` in a child plugin file. `[[disable]]` blocks
   also come from `fo-scan.toml`. Changed globals, vtables and slots of ours are only
   reported. The report goes to `$A2_DATA/research/fo-roots/scan-<build>.md`;
5. `--verify` runs `a2test syms --game fo-roots` and `negative.sh` (its case (e) takes the
   build from `FO_ROOTS_BUILD`); `--gen` runs `gen.py`.

If the build's directory already exists the scan is written beside the report and compared
with it (`--compare DIR` for any directory): `5.0Z11` reproduces its committed profile with
30 file marks (13 detour, 17 edit), 80 runtime marks, the same five callees, the same
`file_crc`, `[[module]]` and `verified` lists (the last from `fo-scan.toml`'s `[verified]`,
which the scan writes again); the one difference is that the scan also drops the check of
`ui.info.toml`'s `closeup_operator_new`, which the committed profile left (its hook
stands down anyway). What the scan does not write, and `--compare` does not look at, is the
profile's own entries: Roots-only sites such as `render3d.frame.obj_draw`'s call.

**Defaults taken without asking the Fleet Operations developers** (each a flag or a line in
`fo-scan.toml`): the build is `fo-roots-<version file's version>` (`--build-name`); the
only Roots variant is the DxWrapper install as shipped (`--display8` for Display8 in its
place); the reference is the GOG 1.1 `Armada2.exe` and its map (`--stock`, `--map`); the
callee list, the disabled rules plugin and the empty content set are `fo-scan.toml`'s.
