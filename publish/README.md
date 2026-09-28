# Publishing — what this repository may contain

This repository is meant to be public. Armada II is not ours: its art, video, models and
data belong to Activision and the Star Trek marks to Paramount. So the rule is simple
and has no exceptions:

**Nothing derived from the game's own files is committed.** Code, configuration, recipes
and docs are. Every asset — stock art extracted from the player's own install, AI
upscales, builds — lives in `A2_DATA` (`a2env.sh`), outside any checkout.

(A working position, not legal advice. It is the conservative reading: an upscale of
copyrighted art is a derivative of it, whoever paid for the upscale, and a binary delta
from a 256px stock file to a 1024px plate *is* the plate.)

## What is here, and what is not

| | Here | Why |
|---|---|---|
| `HUD.asi`, `Menus.asi`, `MSAA.asi` — their C source | yes | our code. It patches the game at run time and ships none of it; the byte arrays in the sources are short instruction signatures, checked before each patch |
| the Python and shell tooling, `.ini`/`.conf` files | yes | ours |
| docs and changelogs | yes | ours. They quote the odd config line, which is commentary, and name functions and addresses from `armada2.map` — see "Engine findings" below |
| `armada2.map`, disassembly or decompilation listings, hex dumps of game code | **no** | the game's own code, or a mechanical translation of it. `check.sh` refuses `*.map` |
| the texture pipeline (`a2tex`, `textures/lib`, `tools`), `models/`, `binkproxy`, the movie and backdrop pipelines | yes | ours. They read the game's files from the player's own install and write only into `A2_DATA` and the game directory |
| the recipes — each target's `target.conf` and `stock.sha256`, `movie.conf`, `backdrops/*.conf` | yes | parameters, file names and SHA-256 hashes of stock files: facts about the game, not its content. The hashes are how `a2tex stock` knows it extracted the right bytes |
| **any asset** — stock copies, AI upscales, intermediates, builds, the intro, the plates, the paid seeds, the archive of candidates and comparisons, the promo footage | **no** | the game's own art and video, or derived from them. They live in `A2_DATA` |

Third-party code this project *uses* but does not contain — DXVK (zlib), the Ultimate
ASI Loader (MIT), vkBasalt (zlib), crosire's reshade-shaders (per-file), the
`STA2WidescreenPatch` (MIT), Patch Project 1.2.5 (no licence stated) — is not in the
repository, nor in the release zip. The zip's installers download what they need from
where its authors publish it, pinned by hash, and `CREDITS.txt` names each author,
licence and source.

`a2mod` switches every layer on the game directory only (`.a2neb-backup` files,
`$GAME/.a2mod/`) and needs no asset to do it.

### Engine findings

Much of this project was learned by reading `Armada2.exe` with the symbol map the game
ships. Two things keep publishing that on the right side of the line (again a working
position, not legal advice):

- **Describe, don't reproduce.** Copyright protects a program's expression, not its
  functionality (CJEU, *SAS Institute v World Programming*, C-406/10, 2012). A sentence
  saying "`[device+0x18]` holds the horizontal scale" is a fact about the program in our
  own words. A listing of the instructions that read it is the program's code. Names,
  addresses, a single instruction in a sentence and the short signatures a plugin checks
  before patching are the first kind. The map file, listings and dumps are the second.
- **Tie each finding to a plugin.** In Germany, as elsewhere in the EU, disassembly is
  allowed to make an independently created program interoperate with another
  (UrhG §69e, Software Directive art. 6), and what it yields may be passed on only as far
  as that interoperability needs. The plugins here are those programs. So a finding from
  the disassembly lives in the README of the layer whose code uses it, beside the hook
  it justifies. Engine trivia no plugin needs stays out. This limit does not apply to what
  was learned by observing, studying and testing the running game (UrhG §69d(3)), such as
  the test bench, texture measurements and crash reproductions, and a licence term
  cannot override either provision (§69g(2)).

`CLAUDE.md`, hard rule 9, is the working form of both.

### How it is enforced

- **`.gitignore`** ignores every image, video and game format anywhere. Committing one
  takes a deliberate `git add -f`.
- **`.gitignore`** also ignores every data directory a checkout from before `A2_DATA`
  may still have (`textures/targets/*/*/`, `cutscenes/movies/*/*/`,
  `menus/backdrops/*/`, `textures/.scratch/`).
- **`publish/check.sh [-C <repo>] [<commit>]`** fails on any **binary** file in the
  index (or the commit), on any game format by extension, and on any path inside those
  data directories or the old `archive/` and `promo/`. No publishable file here is
  binary, so the binary test catches a renamed image without a list of extensions.

## Commit hashes in the changelogs

This repository's history was rewritten without any binary, and `check.sh` passes on
every commit. The texture, model, cutscene and backdrop work joined it in one commit,
without its history, which is full of stock art. So not every cited hash resolves here:

- Hashes in the `textures/`, `models/` and `cutscenes/` changelogs, and `927f993` in the
  root changelog and `79695b2`, `8a3562c` and `d3e1ccb` in `menus/CHANGELOG.md`, are in
  that unfiltered history, which is not published and does not resolve here.
- Every other hash resolves in this repository.

## Before the first push — open

- **`THIRD-PARTY.md`**, naming what a release bundles. The repository itself is MIT
  (`LICENSE`, added 2026-09-27), matching the tools it sits beside.
- **The author email** is on every commit. If it should not be public, rewrite it with
  a mailmap before the first push — after that, it is too late.
- **`CLAUDE.md`** is the working brief for agent sessions. It publishes fine; decide
  whether it should.

## Release packages

Separate from the repository. **`publish/package.sh [<out-dir>]`** builds the four
plugins and writes `armada2-refit-<version>.zip`:

- `game/`: what goes beside `Armada2.exe`, which is `HUD.asi`, `Menus.asi`, `MSAA.asi`
  and `binkw32.dll`, each with its `.ini`, and `dxvk.conf` at stage 3
  (`postfx/renderer-config.sh --print`);
- `bloom/`: `postfx/postfx.py --export`, which is the same bloom for vkBasalt and as a
  ReShade preset, and `shaders.txt`;
- `install.sh` (Linux) and `install.ps1` + `install.bat` (Windows), from
  `publish/installer/`;
- `prereqs.txt`, what the installers download first, and `CREDITS.txt`, generated from
  it and from `shaders.txt`;
- `README.txt` (layer versions, commit, what each needs), `LICENSE` and `SHA256SUMS`.

It is our code only, compiled, and our configuration. It holds nothing of the game,
none of the test tools, and **no third-party shader**. `ReShadeUI.fxh` carries no
licence, so the installers fetch MagicBloom and ReShade's two headers from the
commits `postfx/vkbasalt/build.sh` pins. They are checked against the hashes in
`shaders.txt`, and `package.sh` refuses a pin `build.sh` does not share. The version
is the root `CHANGELOG.md`'s newest entry.

The installers stand alone, and **ask nothing**: no flags but `--uninstall`, no
prompts unless they cannot find the game. Each installs what can work on the machine
and names what it skipped. What they cannot do themselves, installing vkBasalt or
ReShade, is documented in `README.txt` and optional. They do the same on both systems:

- find the game: the argument, else the folder they were unzipped into, else Heroic's
  default or GOG's registry entry;
- install the prerequisites in `prereqs.txt` that are missing: `STA2WidescreenPatch`
  1.0, which brings the Ultimate ASI Loader 4.68 as `winmm.dll` (from its GitHub
  release), and Patch Project 1.2.5 (the ZIP from armadafiles.com, the page the
  widescreen patch's own README links; its NSIS installer refuses GOG installs). Each is
  checked against a pinned SHA-256; both match the install this project is developed
  on byte for byte. A zip with the right hash in `downloads/` beside the installer is
  used instead, for offline installs. **A file already there is never overwritten**,
  with one exception: GOG's `d3d8.dll` (d3d8to9) gives way to Patch Project's proxy, as
  Patch Project's instructions say, and is kept as `d3d8.dll.gog-backup`. A DXVK
  `d3d8.dll` stays, and the proxy goes to `d3d8.dll.proxy-backup`, where
  `platform/d3d8-chain.py` keeps it. What was added is recorded with its hash in
  `armada2-refit-prereqs.txt` in the game directory;
- on Windows, warn if Microsoft's Visual C++ runtime (x86) is missing, which the
  widescreen patch needs. Installing it takes administrator rights and an unpinned
  download, so they only name it;
- keep the stock `binkw32.dll` as `binkw32_orig.dll`, which the proxy forwards to, and
  as `binkw32.dll.a2neb-backup`, the name `cutscenes/binkproxy/install.sh` uses;
- install `MSAA.asi` only when the `d3d8.dll` in the game directory is DXVK's, told
  by its content: Patch Project and GOG put a `d3d8.dll` there too. Without DXVK the
  minimap goes black;
- set up bloom when a vkBasalt layer (Linux) or ReShade's `ReShade.ini` (Windows) is
  there. On Windows the preset also becomes `ReShadePreset.ini` when the player has
  none, which is the preset ReShade loads by default, so there is no step in the
  overlay. A failed download skips bloom and does not fail the install;
- write `dxvk.conf` only over one that carries `renderer-config.sh`'s marker;
- keep a changed `.ini` as `.ini.bak`;
- with `--uninstall` / `-Uninstall`, put all of it back. A prerequisite file goes only
  if its hash still matches the record, so nothing replaced since is removed.

`publish/installer/test.sh` and `test.ps1` run them against a mock game of placeholder
files, so they prove the downloads and the file handling, not that anything loads.
They download for real, so CI also notices when a pinned download moves.

**CI** (`.github/workflows/ci.yml`) runs on every push and pull request:

- `check.sh` on each new commit, so a binary or a game file fails the push;
- `package.sh`, with the zip kept as a workflow artifact, named by commit, and both
  installers run against a mock game: `test.sh` on Linux, `test.ps1` on a Windows
  runner;
- on `main` only, when the root version has no release yet: tag `vX.Y.Z` and publish a
  GitHub Release with the zip, the changelog entry as its notes. An entry that still
  says "not yet seen in game" goes out as a pre-release. So **bumping the root version
  and pushing `main` is what releases**; a push that does not bump it releases nothing.

### Next

The installers could also set up DXVK. On Linux the launcher settings
(`WINEDLLOVERRIDES`, and bloom's two variables) are still the player's job; `install.sh`
prints them. Patch Project comes from a mirror over plain HTTP, whose certificate does
not match; the hash is what makes that safe, and a second mirror would make it
sturdier. What stands in the way:

- **No plugin checks which `Armada2.exe` it is in.** Each should verify the exe and stand
  down on any other build rather than patch the wrong bytes.
- **`MSAA.asi` still defaults on wherever it is installed.** The installers leave it out
  without DXVK, but copied by hand beside native D3D8 it blacks out the minimap: the
  minimap's `CopyRects` from a multisampled surface is illegal there (`msaa/README.md`).
- **The ReShade preset has never run.** Only the vkBasalt path is seen in game.
- **Untested**: Windows 11, a desktop other than Hyprland, and running without Patch
  Project 1.2.5 or the widescreen patch.
