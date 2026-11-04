# The release installers and their tests

The release package's installers live here: `install.sh` (Linux and macOS, `/bin/bash`
3.2 and BSD tools), `install.ps1` with its `install.bat` launcher (Windows), and
`prereqs.txt`, what they put in first. `publish/package.sh` copies them into the zip.
What they do, and the package they read, is in [`../README.md`](../README.md),
"Release packages". The tests:

| Script | What it checks | Where it runs |
|---|---|---|
| `test.sh <zip>` | `install.sh` against a mock game: what it copies, backs up, merges and puts back, error codes for the GUI; again with `sha256sum` and `tac` hidden and with `uname` saying Darwin | CI `package` (Linux), `macos-gui` |
| `test.ps1 <zip>` | the same for `install.ps1` | CI `windows-installer` (Windows PowerShell 5.1) |
| `equiv.sh [<zip>]` | that `install.sh`, `install.ps1` and the repository's `./install` **do the same thing** (below) | CI `installer-equivalence` (Linux, pwsh 7); anywhere |
| `equiv-ini.sh` | that the three ini mergers write the same bytes (below); `equiv.sh` runs it | with `equiv.sh` |

## Equivalence

The install is written three times: `install.sh`, `install.ps1` and the per-layer scripts
behind `./install`. The ini merge is written in Python (`tools/ini-merge.py`), awk
(`install.sh`) and PowerShell (`install.ps1`), with hand-copied upgrade tables, and the
retire step three times (`retire/retire.sh`, and each release script reading
`retire.txt`). `tools/layers.py --check` compares only the `::step` ids they print;
`equiv.sh` compares what they do (PLAN-ARCHITECTURE.md, AR-1).

```sh
publish/installer/equiv.sh                  # a stub package from package.sh --stub
publish/installer/equiv.sh dist/armada2-refit-15.7.0.zip   # a real one (CI's)
publish/installer/equiv.sh --no-repo        # without ./install (faster)
publish/installer/equiv-ini.sh --show       # only the ini mergers, with each output
```

**The package.** Given none, `equiv.sh` runs `publish/package.sh --stub`, which builds
nothing: the layout is the real one, and each plugin is a placeholder of a few bytes that
carries only the marker the installers know it by (Display8's panic message,
`BinkProxy`). The package is then copied and both scripts and `retire.txt` are told the
hashes of three stand-ins (GOG's `d3d8.dll`, crosire's d3d8to9, DXVK's `d3d9.dll`), as
`test.sh` does, so the cases that hinge on them run. No game file is used anywhere: the
"exe" is three bytes.

**The scenarios.** Each starts from identical copies of a mock game, runs one installer
into each copy step by step, and after every step compares the trees: every path, the
bytes of every file, and an `.ini` that differs key by key (`equiv-diff.py`), plus the
`::step` lines and the exit codes.

- `fresh`: install, reinstall, the player edits `QOL.ini` (CRLF, a value, a key gone) and a
  prerequisite, reinstall, `--without qol,online`, a plain re-run (the remembered choice),
  `--without ui,render3d,models,cutscenes`, `--all`, uninstall;
- `upgrade`: what a 14.x install left (an old Display8 before the d3d8to9 and DXVK chain, a
  marked `dxvk.conf`, `Display8.ini` with `Mode=forward` and `Backend=d3d9,vulkan`, `UI.ini`
  at Defaults 1 in CRLF with keys that moved to `Display8.ini` and `QOL.ini`, `Render3D.ini`
  with retired defaults, the legacy and per-part plugins, a shipped sky recipe, the hud
  fixes' backups, the old bloom, a remembered "without rules"): install, again, uninstall;
- `own-d3d8`, `empty-slot`, `unknown-d3d8`: the three other states of the `d3d8.dll` slot,
  install and uninstall.

`./install` runs from a copy of the checkout whose build scripts are stand-ins that copy
the package's plugins where the build would put them, with `A2_GAME` the mock and an empty
`A2_DATA`; it is compared with `install.sh` on `fresh` (install, `--without qol,online`),
`upgrade`, `own-d3d8` and `empty-slot`.

**The ini fixtures** are `tools/ini-merge-fixtures/<case>/`: `ship.ini`, the player's file
as `game/<Name>.ini` (the name matters: `Render3D.ini`'s retired defaults go by it), a
`game/dxvk.conf` where the case needs one, and `want.ini`, which every merger must write
byte for byte. A merger known to write something else gets `want.<impl>.ini` and a line in
the case's `KNOWN`. `equiv-ini.sh` cuts `ini_merge` and `Merge-Ini` with their tables out
of the installers as they are, so the shipped code is what is tested, and also runs the
three on every shipped `.ini` (unchanged, every third key missing, a section missing),
where they must agree.

**Known differences** are `equiv-known.txt`, one a line, each with its reason: printed as
`known`, never silent, and a line no comparison needed is noted. Today they are the
comment in `armada2-refit-layers.txt` (each script names its own flag), the Windows-only
ReShade bloom preset, and between `install.sh` and `./install`: the prerequisites (the
platform layer's, by hand on a development machine), GOG's `d3d8.dll` kept as
`d3d8.dll.gog-backup` against `d3d8.dll.a2display-backup`, `./install` keeping no memory of
a choice of layers, and a `d3d8.dll` of the player's own, which `./install` replaces and
the release keeps (AR-9's one installer core settles the last three). In the ini merge,
`./install` places a key it adds to `Render3D.ini` before the comments that open the next
section (`--before-comments`); the keys and values are the same.

**Not covered** (`equiv.sh` prints this list too):

- `install.ps1` under Windows PowerShell 5.1 on Windows: here it is pwsh 7 on Linux, with a
  case-sensitive file system. `test.ps1` runs it on Windows, but compares nothing;
- the GUI installer, which runs these scripts (`gui-installer/test.sh`);
- the macOS branch of `install.sh` (`test.sh` fakes `uname`);
- the model rewrites on real SODs: a mock has none, so every installer finds nothing to
  do, and `./install`'s `models/install.sh`, which stops without them, is a no-op here;
- `./install`'s builds (stand-ins) and its asset layers with assets built (textures, the
  menu plates, the movies: an empty `A2_DATA` here); `./install` has no uninstall to compare;
- what Windows alone has: the GOG registry lookup, the Visual C++ runtime warning, the
  pipeline cache in `%LOCALAPPDATA%`.

**pwsh.** CI's ubuntu runners have it. Elsewhere, without it, `equiv.sh` says so and runs
`install.sh` alone (checking it succeeds), the `./install` comparison and the Python and
awk mergers; `A2_EQUIV_REQUIRE_PWSH=1` makes a missing pwsh an error (CI sets it). To get
it on Linux without a package manager, unpack Microsoft's release tarball
(`powershell-<version>-linux-x64.tar.gz` from github.com/PowerShell/PowerShell/releases)
and put its `pwsh` on `PATH`, or point `PWSH=` at it.

**When it fails**, the line names the scenario, the step and the path, and for an `.ini`
the key and both values. Make the scripts agree; if the difference is right, add it to
`equiv-known.txt` with the reason, and to the list above. A new ini case is a new folder
under `tools/ini-merge-fixtures/`.
