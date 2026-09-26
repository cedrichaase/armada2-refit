# Publishing — what this repository may contain

This repository is meant to be public. Armada II is not ours: its art, video, models and
data belong to Activision and the Star Trek marks to Paramount. So the rule is simple
and has no exceptions:

**Nothing derived from the game's own files is committed.** Code, configuration and docs
are.

(A working position, not legal advice. It is the conservative reading: an upscale of
copyrighted art is a derivative of it, whoever paid for the upscale, and a binary delta
from a 256px stock file to a 1024px plate *is* the plate.)

## What is here, and what is not

| | Here | Why |
|---|---|---|
| `HUD.asi`, `Menus.asi`, `MSAA.asi` — their C source | yes | our code. It patches the game at run time and ships none of it; the byte arrays in the sources are short instruction signatures, checked before each patch |
| the Python and shell tooling, `.ini`/`.conf` files | yes | ours |
| docs and changelogs | yes | ours. They quote the odd config line, which is commentary |
| **the texture work** — `a2tex`, `textures/`, `models/`, `archive/`, `promo/` | **no** | it cannot work without the game's own textures in `stock/`, and everything it makes is derived from them. It is the private repository **`~/armada2-remastered-private`** |
| **the cutscenes** — `binkproxy` and the upscaled intro — and **the menu backdrop pipeline** — `backdrop.sh`, its plates and paid seeds | **no** | the intro and the plates are the game's own video and art, upscaled; they moved to the private repository with the code that makes and plays them |

Third-party code this project *uses* but does not contain — DXVK (zlib), the Ultimate
ASI Loader (MIT), vkBasalt (zlib), crosire's reshade-shaders (per-file), the
`STA2WidescreenPatch`, Patch Project 1.2.5 — is not in the repository either. A release
package may bundle the first four with their notices, and links to the last two.

`a2mod` still switches the `textures`, `models` and `cutscenes` layers, and the
private repository's `./install` puts the backdrop plates in `Menus/`: it works on the game
directory only (`.a2neb-backup` files, `$GAME/.a2mod/`) and needs none of their source or art.

### How it is enforced

- **`.gitignore`** ignores every image, video and game format anywhere. Committing one
  takes a deliberate `git add -f`.
- **`publish/check.sh [-C <repo>] [<commit>]`** fails on any **binary** file in the
  index (or the commit), and on any path the private work has ever had (`textures/`,
  `models/`, `archive/`, `promo/`, `a2tex`, `cutscenes/`, `menus/backdrop.sh`,
  `menus/backdrops/`, and the layout before 2026-09-25's `targets/`, `lib/`,
  `error-mission-finish/`, `movies/`, `tools/binkproxy/`, `tools/menuscale/backdrop*`). No publishable file here is
  binary, so the binary test catches a renamed image without a list of extensions.

## The split, 2026-09-26

Until then, this repository and the texture work were one, and its history held 1669
stock textures and every archive plate. On 2026-09-26:

- **`~/armada2-remastered-private`** was made from the full, unfiltered history, then
  trimmed at its tip to the texture work. Commit hashes cited in the `textures/` and
  `models/` changelogs resolve there.
- **This repository's history was rewritten**: every commit, with the texture work's
  paths — and, in a second pass the same day, the cutscenes and the menu backdrop
  pipeline — and every binary removed, and commits left empty by that dropped. `check.sh`
  passes on every commit. Hashes cited in this repository's changelogs were remapped to
  the rewritten commits in the same step. A cited commit the rewrite dropped (one that
  held only private work) was given its original hash, which resolves in the private
  repository: `927f993` in the root changelog, `79695b2`, `8a3562c` and `d3e1ccb` in
  `menus/CHANGELOG.md`.
- **The pre-split repository** is kept whole as
  `~/armada2-remastered-pre-split-2026-09-26.bundle` (`git clone` it to look).

## Before the first push — open

- **A licence.** There is none yet, so the code is all-rights-reserved by default. MIT
  or zlib would match the tools it sits beside (the ASI loader, DXVK). Add `LICENSE` and
  a `THIRD-PARTY.md` naming what a release bundles.
- **The author email** is on every commit. If it should not be public, rewrite it with
  a mailmap before the first push — after that, it is too late.
- **`CLAUDE.md`** is the working brief for agent sessions. It publishes fine; decide
  whether it should.

## The release package (next)

Separate from the repository: a zip that mirrors the game folder and overwrites no game
file, so uninstalling is deleting it. The plugins, `winmm.dll` (the ASI loader),
`dxvk.conf`, optionally DXVK and the bloom shader for ReShade (Windows) and vkBasalt
(Linux). On Linux one launcher setting, `WINEDLLOVERRIDES="winmm=n,b"`. What stands in
the way:

- **No plugin checks which `Armada2.exe` it is in.** Each should verify the exe and stand
  down on any other build rather than patch the wrong bytes.
- **`MSAA.asi` should default off without DXVK**: the minimap's `CopyRects` from a
  multisampled surface is illegal in native D3D8 (`msaa/README.md`).
- **Untested**: Windows 11, a desktop other than Hyprland, and running without Patch
  Project 1.2.5 or the widescreen patch.
