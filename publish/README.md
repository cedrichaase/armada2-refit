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
| the C plugins' source (`QOL.asi`, `QOLRules.asi`, `Online.asi`, the cutscene proxy), and the Rust of `Render3D.asi`, `UI.asi` and Display8 | yes | our code. It patches the game at run time and ships none of it; the byte arrays in the sources are short instruction signatures, checked before each patch |
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
| DXVK | zlib | no: not used since 15.6.0 (Display8 draws on Vulkan and Direct3D 12 itself); its builds left `platform/vendor/` and only their hashes stay, in the retire table |

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
  `ui/backdrops/*/`, `textures/.scratch/`).
- **`publish/check.sh [-C <repo>] [<commit>]`** fails on any **binary** file in the
  index (or the commit), on any game format by extension, and on any path inside those
  data directories or the old `archive/` and `promo/`. No publishable file here is
  binary, so the binary test catches a renamed image without a list of extensions.

### Nothing of a game that is not offered in what players read

A game that is not offered (`games/<id>.toml`, no `offered`; today Fleet Operations: Roots,
bash-only, installed by `./install --game <id>` and not advertised) is never named in a
file a player or a reader of the published pages receives. `tools/changeset.py check`
(CI, "The changesets") greps those files for each such game's id, label and `[changesets]
quiet` words and fails with `file:line`. The list is derived, not kept by hand
(`shipped_files()`): every repository file a `cp` in `publish/package.sh` puts in the
package (the plugins' inis, the model scripts, `a2env.py`, the installers, the SDK), the
installers' sources (`publish/installer/`), the GUI installer (`gui-installer/`), the
guides (`README.md`, `PLAYING.md`, `MODDING.md`, `publish/MODDING.txt`), the root and the
layers' `CHANGELOG.md` (the release notes), `retire/known.txt`, and each layer's
`settings.toml`, whose labels and help the Refit screen draws. Developer material may name
it: the layers' READMEs and sources, `HUD.md` and the like, the plans, `games/`,
`profiles/`, the games' defaults (`ui/defaults/`, `qol/defaults/`) and the content sets.
A key's comment in a shipped ini says what the key does for a player, without the game;
the specifics go in the layer README or the game's defaults file.

## Commit hashes in the changelogs

This repository's history was rewritten without any binary, and `check.sh` passes on
every commit. The texture, model, cutscene and backdrop work joined it in one commit,
without its history, which is full of stock art. So not every cited hash resolves here:

- Hashes in the `textures/`, `models/` and `cutscenes/` changelogs, and `927f993` in the
  root changelog and `79695b2`, `8a3562c` and `d3e1ccb` in the retired menus layer's changelog (`retire/history/menus-CHANGELOG.md`), are in
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
and writes `armada2-refit-<version>.zip` (with `--stub`, `armada2-refit-<version>-stub.zip`
from placeholder plugins, for the installer tests only):

- `layers/<id>/`, one folder per layer, whose files are copied beside `Armada2.exe`
  (14.3.0 on; `game/` and `display/` before it): `render3d` (`Render3D.asi`, its `.ini`
  and the recipes in `Surfaces/` and `Nebulae/`), `ui` (`UI.asi`), `qol`, `rules`
  (`QOLRules.asi`), `online`, `cutscenes` (`binkw32.dll` and `BinkProxy.ini`), `display`
  (Display8, below). There is no `renderer/` (`dxvk.conf`) and no `chain/` (crosire's
  d3d8to9 and DXVK 3.1.1's `d3d9.dll`) from 15.6.0: Display8 stands on neither. `Render3D.asi` and
  `UI.asi` draw on Display8's device, so the installers put them in only where Display8 is
  the game's `d3d8.dll`; with a `d3d8.dll` of the player's own neither goes in and the game
  keeps its own look (`HUD.asi`, `Menus.asi` and `GridLayout.asi` are not in the package any
  more). The **retire** step takes out what earlier versions installed and nothing
  installs now, from `retire.txt` (baked from `retire/known.txt` by `package.sh`; the same
  table `retire/retire.sh` reads): those three plugins and the per-part plugins (`MSAA`,
  `Planets`, `Lighting`, `Sky` and the rest) with the `Sky` recipes they copied;
  `retire/hud-files.py` puts the hud 2.x file fixes back; the chain's `d3d8_real.dll` and
  `d3d9.dll` go **by hash** (the vendored builds' sha256, which stay in the table after the
  binaries left `platform/vendor/`), and a `dxvk.conf` by our marker, after the display
  step has carried its sampler values into `Display8.ini [Samplers]`. A file that is none of
  these is the player's and stays. `package.sh` needs rustup's
  cargo with the `i686-pc-windows-msvc` target for the two Rust plugins;
- `sdk/`: `a2asi.h`, `a2engine.h`, `def/` (the import definitions) and `MODDING.txt`
  (`publish/MODDING.txt`): what a mod needs to be built on the same base. The source stays
  at the release tag in the repository; nothing is published to crates.io;
- `layers.txt` and `moved.txt`, baked by `package.sh` for the scripts, which have no
  Python: `layers.txt` is `tools/layers.py`'s id, `select`, `requires` and `scope` per
  layer, and `moved.txt` the settings that moved between files (a settings row's
  `moved_from`: from file, section, key, to file, section, key, the new default), also
  the manifest's `moved[]`;
- `display/`: Display8 (`display/build/d3d8.dll`, `display/build.sh`) and its
  `Display8.ini` as shipped (`Backend=auto`, `VSync=1`). Both installers put it in the
  `d3d8.dll` slot when that is empty, GOG's (kept once as `d3d8.dll.gog-backup`, and put back
  by `--uninstall`), the old chain's d3d8to9 or a DXVK `d3d8.dll` (replaced), or an older
  Display8; any other `d3d8.dll` is the player's own and stays, and Display8, `Render3D.asi`
  and `UI.asi` stay out. `Display8.ini` is copied if missing, else merged as
  `tools/ini-merge.py` merges it (the missing keys added, the player's values kept, the
  `Defaults=` upgrade applied; Defaults 3 drops `Mode`, `Real` and the `d3d9` token and
  carries a marked `dxvk.conf`'s sampler values into `[Samplers]`, with a stub at
  `Defaults=1` when there is no `Display8.ini` yet), in awk and in PowerShell, since neither may assume
  Python; keep their upgrade tables (`INI_UPGRADES`, `$DisplayUpgrades`) in step
  with `ini-merge.py`'s. On macOS too it goes in as shipped, on `Backend=auto`: Vulkan
  on MoltenVK under a 32-bit process works there (`display/README.md`, "Will Vulkan work
  on a Mac?"), and the system's own `d3d8.dll` draws the game if it cannot start. Display8 is known
  by content (the panic message every build carries), never by hash. `--uninstall` takes it
  out of the slot (an older Display8's `d3d8_real.dll`, if it is not the vendored d3d8to9, goes
  back in), then `Display8.ini`, its logs and trace files (and on Windows its Vulkan pipeline
  cache in `%LOCALAPPDATA%\armada2-refit\`), then puts `d3d8.dll.gog-backup` back;
- `models/` and `a2env.py`: `moon-sod.py`, `select-sod.py`, `hull-sod.py`,
  `asteroid-sod.py`, run by
  `install.sh` with python3 on the player's own SODs (`--revert` on `--uninstall`);
- `install.sh` (Linux and macOS) and `install.ps1` + `install.bat` (Windows), from
  `publish/installer/`. `install.sh` keeps to `/bin/bash` 3.2 and BSD tools (no
  `sha256sum`, `tac` or GNU `sed`); `test.sh` checks that on Linux by hiding those tools
  and by faking `uname -s`;
- `prereqs.txt`, what the installers put in first, and `CREDITS.txt`, generated from
  it;
- `README.txt` (layer versions, commit, what each needs), `LICENSE` and `SHA256SUMS`;
- `manifest.json`, the same for the GUI installer (package schema 1), and
  `armada2-refit-installer.py` itself; CI attaches the script, the AppImage, the
  Windows `.exe` and the macOS app's zip built from it to the release on their own
  (`gui-installer/README.md`). The manifest's `windows` and `macos` entries are what
  lets those two install a package.

`install.sh` and `install.ps1` read three variables the GUI sets: `A2_PROGRESS=1` prints
`::step <id>` as each stage starts, `A2_SKIP="cutscenes qol ..."` leaves those layers out
(see "Per-layer selection" below; `cutscenes` also puts back a stock `binkw32.dll` an
earlier install replaced), and `A2_PYTHON` names the
program that runs the SOD scripts (the frozen installer itself on Windows and macOS).
With `A2_PROGRESS=1` a failure they understand is also named as `::error <code> <path>`,
one line per file, before the sentence for a person (`av-locked`, `av-missing`,
`damaged`, `game-running`, `no-game`, `write-denied`: `gui-installer/README.md`,
"Errors"). Run by hand, they do as before, and say which file failed and why.

It is our code only, compiled, and our configuration. It holds nothing of the game and
none of the test tools. The version is the root `CHANGELOG.md`'s newest entry.

The installers stand alone, and **ask nothing**: no flags but `--uninstall`, no
prompts unless they cannot find the game. Each installs what can work on the machine
and names what it skipped. They do the same on both systems:

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
- take out the bloom a package before 13.0.0 set up, since `Render3D.asi` blooms the
  3D view itself: the vkBasalt config folder `install.sh` wrote (by its marker), and on
  Windows `A2Bloom.ini` with the `ReShadePreset.ini` copy of it, if it is still that
  copy. vkBasalt or ReShade themselves are the player's and stay;
- take out a `dxvk.conf` only when it carries `renderer-config.sh`'s marker, and a `d3d9.dll` or `d3d8_real.dll` only by the vendored builds' hash (retire step);
- merge every shipped `.ini` into the player's, never replace it (keys added, values kept; a `.ini.bak` copy is no longer made, 14.3.0), and never touch `RefitPlayer.ini`;
- with `--uninstall` / `-Uninstall`, put all of it back. A prerequisite file goes only
  if its hash still matches the record, so nothing replaced since is removed.

### Per-layer selection

Every layer whose `select` is `on` or `off` in `layers.toml` (qol, rules, online,
cutscenes, models, render3d, ui) can be left out: `./install.sh --without qol,rules`,
`install.bat -Without qol,rules` or `A2_SKIP="qol rules"`; a core layer (the platform,
Display8) or an unknown id is refused. A layer left out is taken out when an
earlier install left it, its plugin, `.ini` and log with it (the cutscene player as
`unproxy` does, the models as `--revert` does). The choice is written to
`armada2-refit-layers.txt` in the game directory (`without <id>` lines): a plain re-run,
an upgrade, keeps it, `--all` / `-All` forgets it, and uninstall deletes it. The GUI
installer builds its "Parts" list from the same manifest keys and passes the choice as
`A2_SKIP`. The `rules` layer is ticked by default; a player without it plays stock rules,
and a network game of players with and without it plays stock rules (`rules/README.md`).

The scripts merge an `.ini` with the repository's `tools/ini-merge.py` logic, without
Python: `INI_UPGRADES` / `$IniUpgrades` carry its `UPGRADES` (Display8.ini's and UI.ini's
`Defaults=`) and `ini_retired` / `Get-Retired` its `RETIRED`; keep them in step. After the
merge they apply `moved.txt`, as the repository's `./install` does with
`tools/settings/move-keys.py`. `install.ps1` has been read for syntax only here: no
PowerShell runs on the development machine, so `test.ps1` is CI's.

`publish/installer/test.sh` and `test.ps1` run them against a mock game of placeholder
files, so they prove the file handling, not that anything loads.
`publish/installer/equiv.sh` runs `install.sh`, `install.ps1` (pwsh) and the repository's
`./install` into copies of one mock game and compares what each leaves, file by file and
ini key by key, and the three ini mergers on shared fixtures; `package.sh --stub` stages a
package for it from placeholder plugins ([`installer/README.md`](installer/README.md),
"Equivalence").

**CI** (`.github/workflows/ci.yml`) runs on every push and pull request:

- `check.sh` on each new commit, so a binary or a game file fails the push;
- `package.sh`, with the zip kept as a workflow artifact, named by commit, and both
  installers run against a mock game: `test.sh` on Linux, `test.ps1` on a Windows
  runner, and against each other: `equiv.sh` (job `installer-equivalence`);
- the GUI installer built three ways, each tested against a mock game: the AppImage
  (ubuntu-24.04), a portable `.exe` (windows-2025, `gui-installer/test-windows.ps1`) and
  a universal2 `.app` (macos-15, `gui-installer/test-macos.sh`, after
  `publish/installer/test.sh` on macOS);
- `tools/changeset.py check` and its self-test: every pending changeset names real
  layers and a valid bump, every version file agrees with its changelog, and on a pull
  request no version changed by hand (the release PR, branch `release/next`, excepted);
- **on the push to `main` that merges the release PR** (job `release-commit` finds its
  `Release-Version: X.Y.Z` trailer; "Releasing" below), and on a dispatch on `main` with
  `release` ticked as the fallback for a failed run: tag `vX.Y.Z` and publish a GitHub
  Release with the zip, the GUI installer as a script, an AppImage, a Windows `.exe` and a
  macOS app's zip, the root changelog entry as its notes. It refuses when `vX.Y.Z`
  exists. An entry that still says "not yet seen in game" goes out as an **untested
  release** (GitHub's *Pre-release* flag), and so does one with a line starting
  "Untested" (confirmed on one system, not yet on another, e.g. before the user's Windows
  gate; the user makes it a full release on GitHub once it is; entries before 15.10.0
  wrote "Pre-release"). Then it deletes all but the newest ten `vX.Y.Z` releases. Their
  tags stay, so a version named anywhere still resolves to its commit;
- **on every push to `main`: a pre-release**, `vX.Y.Z-pre.N`, the build of the latest
  main (job `prerelease`, "Pre-releases: the latest main" below);
- on `main` and on dispatch: every program in the release scanned on VirusTotal, and the
  reports attached to the release (below).

### Releasing

Versions come from **changesets** (`.changeset/README.md`; `CLAUDE.md`, "Changelogs and
versions"), never from hand edits. A release starts with one action by hand and is
published by merging its pull request. No other push to `main` releases anything, a
version changed by hand included.

1. **See what is pending**: `python3 tools/changeset.py status` prints each layer's
   current → next version and the changesets behind each bump. CI prints the same, and
   every pre-release's notes list them.
2. **Prepare**: Actions → **Release** → *Run workflow* (`.github/workflows/release.yml`).
   It runs `tools/changeset.py version --game armada2 --commit` on `main`. That writes
   each layer's entry and version files and the root entry with its `Layers:` line from
   the changesets naming Armada II, consumes them (`.changeset/README.md`, "Games": a
   changeset that names another game too stays, for that game's track), and commits it all as "Release X.Y.Z" with the trailer
   `Release-Version: X.Y.Z`. The workflow pushes that to the branch `release/next`, opens
   (or updates) the pull request **"Release X.Y.Z"**, and dispatches CI on it, since a
   pull request opened with the workflow's own token starts no workflow. Running it again
   replaces the branch with a fresh version of `main`.
3. **Review**: this is the place to edit a status line. "Confirmed in game YYYY-MM-DD"
   in place of "Installed, not yet seen in game" makes a full release, and an "Untested
   ..." line keeps it an untested release.
4. **Merge it with a merge commit or a rebase; do not squash.** A squash drops the
   trailer. If it happens anyway, job `release-commit` fails with that message and
   publishes nothing: use the fallback below. The push to `main` runs CI as usual. Job
   `release-commit` runs `tools/changeset.py released BEFORE..HEAD`, which answers with
   the version only when the root version changed in this push *and* a commit in it
   carries the trailer for that version. Then job `release` publishes `vX.Y.Z` once
   every build and test job has passed. The VirusTotal reports follow as before.

**Fallback**: if that run failed (a flaky job, a squash that lost the trailer), re-run
it, or dispatch CI on `main` with `release` ticked (`gh workflow run ci.yml --ref main -f
release=true`). It publishes the root version `main` has, and refuses if it exists.

**Why a release PR** and not a commit to `main`: the version step writes a dozen
changelogs that deserve a read before they are published for good, and a pull request is
where that read and the status edits happen. It also needs no token that can bypass
branch protection. It needs *Settings → Actions → General → Allow GitHub Actions to
create and approve pull requests*, which is on.

**Locally**, the same: `python3 tools/changeset.py version --commit` on a branch from
`main` (`--dry-run` shows the plan first), then `git merge --no-ff` it into `main` and
push.

**Another game's track** is never released here: `tools/changeset.py version --game
<id>` (by hand, no `--commit`) writes only `games/<id>/CHANGELOG.md`, committed like any
change. The release notes, the release PR and the pre-releases' notes are Armada II's
track alone (`--game armada2`).

### Pre-releases: the latest main

Every push to `main` publishes a **pre-release**, a build of that commit, so the latest
main can be installed with the GUI installer without a release. It is for testing, and
for players who want to live on the edge. CI job `prerelease` runs after the package,
installer and GUI jobs pass, one run at a time:

- the zip is built again with `A2_PRERELEASE=N` (`publish/package.sh`), versioned
  `<next>-pre.N`, e.g. `armada2-refit-15.10.0-pre.4182.zip`. `<next>` is
  `tools/changeset.py status --next-root`: the version the pending changesets give, or
  the current one plus a patch when none are pending, so a pre-release always sorts
  above the release it follows (semver: `15.10.0-pre.4182` < `15.10.0`, and
  `15.10.1-pre.4190` > `15.10.0`). **N is CI's run number** (`GITHUB_RUN_NUMBER`). It is
  unique and rising, so two pushes in quick succession cannot claim the same tag, as a
  counter per version read from the existing tags could. A re-run keeps its number and
  finds its release already made;
- it is published as the GitHub release **`vX.Y.Z-pre.N`**, tagged on the commit, with
  GitHub's *Pre-release* flag and `--latest=false`, and with the zip and the four
  installer assets. The notes name the commit and date and list the pending changesets
  (`changeset.py status --markdown`);
- the newest **three** are kept, and older ones are deleted with their tags (a
  pre-release is never named in a changelog). The ten `vX.Y.Z` releases job `release`
  keeps are counted apart: each pruning matches only its own tag pattern;
- nothing takes a `-pre.N` tag for a release. The `release` job publishes only `v` plus
  the root changelog's version, `changeset.py` reads versions from the changelogs, and
  the GUI installer counts only `vX.Y.Z` tags as releases. It lists the newest `-pre.N`
  last, as "Pre-release (latest main) · 15.10.0-pre.4182 · commit · date — untested",
  and never picks it by default (`gui-installer/README.md`).

**Two words, two things.** A *pre-release* (`vX.Y.Z-pre.N`) is always a build of the
latest main. A versioned `vX.Y.Z` that is not yet seen in game, or not yet past the
Windows gate, is an **untested release**: GitHub shows it with its *Pre-release* flag,
like the pre-releases, but the installer and these docs call it "untested release".
Neither is ever chosen for the player.

### VirusTotal

Some antivirus programs flag the plugins (issue #4: Bitdefender quarantined four of
them): they are unsigned DLLs that hook a game's process, a classic heuristic signal.
CI measures it on every release, so a detection is seen before players meet it, and
anyone who meets one can look up our files' scans.

**The scan** (`publish/virustotal.py`, CI job `virustotal`) takes every PE in the
release zip (`layers/` and `vendor/`, matched by extension and `MZ` header, never
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

**DXVK is out** (15.6.0): Display8 draws on Vulkan and Direct3D 12 itself, so the package
carries no DXVK, no d3d8to9 and no `dxvk.conf`, and the launcher setting is
`WINEDLLOVERRIDES=winmm=n,b;d3d8=n,b`, which `install.sh` prints. What stands in the way:

- **Other builds of `Armada2.exe`.** The plugins check their hooks against the symbol
  profile of the build (`profiles/gog-1.1/`) and stand down on any other rather than
  patch the wrong bytes; only the GOG 1.1 profile exists.
- **Untested**: Windows 11, a desktop other than Hyprland, and running without the
  widescreen patch.
