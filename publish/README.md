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
| `HUD.asi`, `Menus.asi` — their C source, and `Render3D.asi`'s Rust | yes | our code. It patches the game at run time and ships none of it; the byte arrays in the sources are short instruction signatures, checked before each patch |
| the Python and shell tooling, `.ini`/`.conf` files | yes | ours |
| docs and changelogs | yes | ours. They quote the odd config line, which is commentary, and name functions and addresses from `armada2.map` — see "Engine findings" below |
| `armada2.map`, disassembly or decompilation listings, hex dumps of game code | **no** | the game's own code, or a mechanical translation of it. `check.sh` refuses `*.map` |
| the texture pipeline (`a2tex`, `textures/lib`, `tools`), `models/`, `binkproxy`, the movie and backdrop pipelines | yes | ours. They read the game's files from the player's own install and write only into `A2_DATA` and the game directory |
| the recipes — each target's `target.conf` and `stock.sha256`, `movie.conf`, `backdrops/*.conf` | yes | parameters, file names and SHA-256 hashes of stock files: facts about the game, not its content. The hashes are how `a2tex stock` knows it extracted the right bytes |
| **any asset** — stock copies, AI upscales, intermediates, builds, the intro, the plates, the paid seeds, the archive of candidates and comparisons, the promo footage | **no** | the game's own art and video, or derived from them. They live in `A2_DATA` |

Third-party code is in the repository and the release zip **only where its licence
expressly allows redistribution**. It is vendored unmodified under
`platform/vendor/<name>-<version>/` with its licence text and a `SOURCE.txt` (where it
came from, and each file's sha256), and `check.sh` accepts a binary nowhere else. Today
that is `STA2WidescreenPatch` 1.0 (MIT) with the Ultimate ASI Loader 4.68 it bundles
(MIT).

| Third-party | Licence | Here |
|---|---|---|
| `STA2WidescreenPatch` 1.0, Ultimate ASI Loader 4.68 | MIT | vendored; in the zip |
| MagicBloom, ReShade's `.fxh` headers | MIT; CC0 and none | no: fetched with bloom, since `ReShadeUI.fxh` states no licence and the three go together |
| DXVK, vkBasalt | zlib | not yet: whether the installers should use them is open (below) |

`CREDITS.txt` in the zip names each author, licence and source.

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

Separate from the repository. **`publish/package.sh [<out-dir>]`** builds the plugins
and writes `armada2-refit-<version>.zip`:

- `game/`: what goes beside `Armada2.exe`, which is `Render3D.asi`, `UI.asi`, `QOL.asi`,
  `QOLRules.asi`, `Online.asi` and `binkw32.dll`, each with its `.ini`, Render3D's
  recipes in `game/Surfaces/` and `game/Nebulae/`, and `dxvk.conf` at stage 3
  (`postfx/renderer-config.sh --print`). `Render3D.asi` and `UI.asi` need a Direct3D 9
  device behind `d3d8.dll`, so the installers put them in only where `d3d8.dll` is
  crosire's d3d8to9 after the chain step, taking out `HUD.asi`, `Menus.asi` and
  `GridLayout.asi` as `ui/supersede.sh` does; elsewhere those three, also in `game/`, go
  in instead and the 3D view stays the engine's. Either way the installers take out the
  per-part plugins (`MSAA`, `Planets`, `Lighting`, `Sky` and the rest) that earlier
  packages put in, with the `Sky` recipes they copied. `package.sh` needs rustup's
  cargo with the `i686-pc-windows-msvc` target for the two Rust plugins;
- `chain/`: crosire's d3d8to9 (`d3d8.dll`) and DXVK 3.1.1's `d3d9.dll`, from
  `platform/vendor/`. `install.sh` puts them in unless a `d3d8.dll` or `d3d9.dll` there is
  none of GOG's, DXVK's or these; a DXVK `d3d9.dll` already there stays. What it
  replaces is kept (`d3d8.dll.gog-backup`, else `.a2chain-backup`), recorded with the
  prerequisites, and put back by `--uninstall`. `install.ps1` puts only `d3d8.dll` in
  (Windows' own Direct3D 9 is behind it) and leaves a DXVK `d3d8.dll` the player put
  there; `install.sh` on macOS puts only `d3d8.dll` in too, on Wine's wined3d, since
  DXVK cannot run Direct3D 9 on a Mac (D3DMetal, DXMT and CrossOver's DXVK cover
  Direct3D 10 to 12), and skips `dxvk.conf` and bloom there;
- `models/` and `a2env.py`: `moon-sod.py`, `select-sod.py`, `hull-sod.py`,
  `asteroid-sod.py`, run by
  `install.sh` with python3 on the player's own SODs (`--revert` on `--uninstall`);
- `bloom/`: `postfx/postfx.py --export`, which is the same bloom for vkBasalt and as a
  ReShade preset, and `shaders.txt`;
- `install.sh` (Linux and macOS) and `install.ps1` + `install.bat` (Windows), from
  `publish/installer/`. `install.sh` keeps to `/bin/bash` 3.2 and BSD tools (no
  `sha256sum`, `tac` or GNU `sed`); `test.sh` checks that on Linux by hiding those tools
  and by faking `uname -s`;
- `prereqs.txt`, what the installers download first, and `CREDITS.txt`, generated from
  it and from `shaders.txt`;
- `README.txt` (layer versions, commit, what each needs), `LICENSE` and `SHA256SUMS`;
- `manifest.json`, the same for the GUI installer (package schema 1), and
  `armada2-refit-installer.py` itself; CI attaches the script, the AppImage, the
  Windows `.exe` and the macOS app's zip built from it to the release on their own
  (`gui-installer/README.md`). The manifest's `windows` and `macos` entries are what
  lets those two install a package.

`install.sh` and `install.ps1` read three variables the GUI sets: `A2_PROGRESS=1` prints
`::step <id>` as each stage starts, `A2_SKIP=cutscenes` leaves the Bink proxy out (and
puts back a stock `binkw32.dll` an earlier install replaced), and `A2_PYTHON` names the
program that runs the SOD scripts (the frozen installer itself on Windows and macOS).
With `A2_PROGRESS=1` a failure they understand is also named as `::error <code> <path>`,
one line per file, before the sentence for a person (`av-locked`, `av-missing`,
`damaged`, `game-running`, `no-game`, `write-denied`: `gui-installer/README.md`,
"Errors"). Run by hand, they do as before, and say which file failed and why.

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
  1.0, which brings the Ultimate ASI Loader 4.68 as `winmm.dll`, from the zip's
  `vendor/` -- the same files as on the install this project is developed on, byte for
  byte. **A file already there is never overwritten**, and what was added is recorded
  with its hash in `armada2-refit-prereqs.txt` in the game directory;
- on Windows, warn if Microsoft's Visual C++ runtime (x86) is missing, which the
  widescreen patch needs. Installing it takes administrator rights and an unpinned
  download, so they only name it;
- keep the stock `binkw32.dll` as `binkw32_orig.dll`, which the proxy forwards to, and
  as `binkw32.dll.a2neb-backup`, the name `cutscenes/binkproxy/install.sh` uses;
- set up bloom when a vkBasalt layer (Linux) or ReShade's `ReShade.ini` (Windows) is
  there. On Windows the preset also becomes `ReShadePreset.ini` when the player has
  none, which is the preset ReShade loads by default, so there is no step in the
  overlay. A failed download skips bloom and does not fail the install;
- write `dxvk.conf` only over one that carries `renderer-config.sh`'s marker;
- keep a changed `.ini` as `.ini.bak`;
- with `--uninstall` / `-Uninstall`, put all of it back. A prerequisite file goes only
  if its hash still matches the record, so nothing replaced since is removed.

`publish/installer/test.sh` and `test.ps1` run them against a mock game of placeholder
files, so they prove the file handling, not that anything loads. They download the
bloom shaders for real, so CI also notices when a pinned one moves.

**CI** (`.github/workflows/ci.yml`) runs on every push and pull request:

- `check.sh` on each new commit, so a binary or a game file fails the push;
- `package.sh`, with the zip kept as a workflow artifact, named by commit, and both
  installers run against a mock game: `test.sh` on Linux, `test.ps1` on a Windows
  runner;
- the GUI installer built three ways, each tested against a mock game: the AppImage
  (ubuntu-24.04), a portable `.exe` (windows-2025, `gui-installer/test-windows.ps1`) and
  a universal2 `.app` (macos-15, `gui-installer/test-macos.sh`, after
  `publish/installer/test.sh` on macOS);
- on `main` only, when the root version has no release yet: tag `vX.Y.Z` and publish a
  GitHub Release with the zip, the GUI installer as a script, an AppImage, a Windows
  `.exe` and a macOS app's zip, the changelog entry as its notes. An entry that still
  says "not yet seen in game" goes out as a pre-release, and so does one with a line
  starting "Pre-release" (confirmed on one system, not yet on another; the user makes
  it a release on GitHub once it is). So **bumping the root version
  and pushing `main` is what releases**; a push that does not bump it releases nothing.
- after that, on every push to `main`: delete all but the newest ten releases. Their
  tags stay, so a version named anywhere still resolves to its commit.
- on `main` and on dispatch: every program in the release scanned on VirusTotal, and the
  reports attached to the release (below).

### VirusTotal

Some antivirus programs flag the plugins (issue #4: Bitdefender quarantined four of
them): they are unsigned DLLs that hook a game's process, a classic heuristic signal.
CI measures it on every release, so a detection is seen before players meet it, and
anyone who meets one can look up our files' scans.

**The scan** (`publish/virustotal.py`, CI job `virustotal`) takes every PE in the
release zip (`game/`, `chain/` and `vendor/`, matched by extension and `MZ` header, never
by name, so a plugin that is added or merged is followed without an edit) and
`armada2-refit-installer-windows.exe`. It looks each hash up first, which costs no upload
and covers the vendored DLLs, uploads the rest and polls their analyses (30 minutes at
most; a file still being analysed is reported as pending). It writes `virustotal.json`
and `virustotal.md` (file, sha256, detections, the engines and their labels, the
VirusTotal link), shown in the job's summary and kept as the artifact
`virustotal-<commit>`. `--selftest` runs the whole scan against a fake VirusTotal, with
no key and no network.

- **When:** on pushes to `main` and on `workflow_dispatch` (any branch), not on every
  push: the public API allows 4 requests a minute and 500 a day. The script spaces its
  requests 15.5 s apart (`VT_INTERVAL` for a premium key), waits a minute on HTTP 429,
  and the job runs one at a time across runs.
- **The secret `VT_API_KEY`** (Settings → Secrets → Actions) is a VirusTotal API key; a
  free public one is enough. Without it, in forks and PRs from forks, the scan is
  skipped with a notice and the job passes.
- **Report only.** The release does not wait for the scan and does not depend on it:
  `release` is unchanged, and a separate job, `virustotal-release`, runs after both and
  attaches `virustotal.json` and `virustotal.md` to `vX.Y.Z` and adds a **VirusTotal**
  line to its notes, each file linked to its scan. It only touches a release made from
  the same commit, replaces its own line on a re-scan (a dispatch on `main`), and does
  nothing when the release was skipped or failed or there is no report. Making the
  release `need` the scan instead would hold every release for the scan's half hour and
  let a VirusTotal outage, or one engine's false positive, stop it.
- **The variable `VT_FAIL_ABOVE`** (Settings → Variables, unset by default) makes the
  scan fail when any file has more "malicious" verdicts than that. The reports are
  written and attached all the same, and the release is still published: it marks the
  commit red, it does not block. To make it a real gate, add `virustotal` to `release`'s
  `needs`; not before there is a baseline, since a false positive must not block a
  release.
- **Uploading shares the files.** A file VirusTotal has not seen is uploaded, and
  uploaded files are available to VirusTotal's partners (the antivirus vendors and its
  premium users). These are the published release files, so that is fine; never point
  the script at anything that is not.

**False positives** (`.github/workflows/virustotal-fp.yml`, run by hand with a release
tag, and `publish/virustotal-fp.py`). There is no one API to dispute a detection: each
vendor has its own form or address. So the workflow prepares everything and leaves the
submitting to a person:

- it reads the release's `virustotal.json`, groups the flagged files by vendor and opens
  (or updates, keeping what is ticked) the issue "False positives: vX.Y.Z": one checklist
  item per vendor, with where to submit, the files with their hashes, VirusTotal links
  and detection names, and a text to paste (what the project is, the source at the
  tagged commit, that CI builds it in public, the build log);
- it uploads the flagged files from the release, zipped with the password `infected`
  that vendors expect for samples, as an artifact to attach to the forms;
- with `vt_comment`, it comments on each flagged file on VirusTotal (the source link and
  "built from source by CI at <commit>") and votes it harmless, once per commit. That
  needs only `VT_API_KEY`, and helps anyone who looks the hash up.

The addresses are in **`publish/av-vendors.json`, kept by hand**: an entry is filled in
only where the address is known, and an engine listed nowhere becomes an item of its
own with "no contact on file". Fill one in, or correct it, once a submission has gone
through (VirusTotal keeps a list of vendor contacts, linked in the file). It works on a
release that still exists: CI keeps the newest ten.

Later, not now: sending the emails from CI, once the addresses have proven themselves
by hand. Web forms and Microsoft's portal (it needs an account) stay manual.

### Next

**Open: DXVK.** Bundling it is allowed (zlib), and the installers could put it in the
game directory in place of GOG's `d3d8.dll`, which would make `dxvk.conf` apply. Not decided yet. On Linux the launcher settings
(`WINEDLLOVERRIDES`, and bloom's two variables) are still the player's job; `install.sh`
prints them. What stands in the way:

- **No plugin checks which `Armada2.exe` it is in.** Each should verify the exe and stand
  down on any other build rather than patch the wrong bytes.
- **The ReShade preset has never run.** Only the vkBasalt path is seen in game.
- **Untested**: Windows 11, a desktop other than Hyprland, and running without the
  widescreen patch.
