# tools/engine — the symbol profiles' tools

The format, the generated code and the resolver are described in
`crates/a2engine/README.md`. Stdlib Python only; nothing here reads a game file except
`annotate.py` (the developer's own map, report only).

| Tool | What it does |
|---|---|
| `profiles.py` | loads every profile set (`profiles/`, `*/profiles/`), flattens builds, computes blocks; shared by the others |
| `gen.py` | writes every profile file's `out`; `--check`, `--emit FILE [--lang rust\|c]`, `--list-hooks PLUGIN`, `--profiles DIR...` |
| `lint.py` | the validator (rules 1–6; `--publish` runs 1, 2 and 5 for `publish/check.sh`; `--selftest` on `fixtures/lint/`). A literal in a plugin source that is in the image's range but no address takes a `// not an address` comment; a migration's transitional const-asserts take `// value parity` |
| `resolve.py` | the resolver in Python, offline on an exe file (`--exe`, `--build` to force a profile, `--base`, `--test-ini`); what `a2test syms` reports with |
| `annotate.py` | checks each entry's `symbol` and `in` against `$A2_GAME/armada2.map`; prints names only, writes nothing |
| `mkpe.py` | writes a fixture's synthetic PE image (`--identity` prints its three numbers, to put in the fixture set's `build.toml`) |
| `fixtures.py` | runs the fixtures on the three resolvers (`run [py c rs]`); `bless` writes Python's transcripts into them, to be reviewed as the spec |
| `ctest.c` | the C resolver's host driver (`csdk/a2engine.h` with `A2E_HOST`) |
| `selftest.sh` | all of the above, as CI's `engine` job runs it |

## Fixtures

`fixtures/NN-name.toml`: a synthetic image (`[image]`: timestamp, image base and size,
sections, and the bytes, calls and words memory holds), the profile set it is resolved
against (`set`, a directory of `fixtures/sets/`), optionally `base` (where it is mapped),
`test_ini`, `addr` (symbol ids queried after the readies) and `note_for` rows, and
`[expect]`: per plugin table, the transcript every resolver prints:

    <init's log lines>
    <for each hook in table order: the lines its ready logs, then> ready <hook> ok|fail
    <for each addr query: its lines, then> addr <id> 0x<hex>
    note <hook> <text>                (each hook with a note)
    note_for <Section.Key> <text|->
    note_plugin <text|->

They cover stock, an unknown timestamp, stock's timestamp with another image size and
no build for it, a child build with `[[changed]]` (one hook verified), `[[disable]]`, an
overridden and an added entry (the Roots shape), a chained call (and one in a hook that
fails, which must not log it), a call and a slot that hold something else, scans with
zero, one and two matches, a relocated base, and the test mismatch file. The images are
written into a temp directory at test time; no binary is committed.

`fixtures/golden/` holds gen's Rust and C output for the fixture set's `p.toml`;
`fixtures/lint/` the validator's good set and one bad set per rule it enforces.
