# Changelog — films

`./a2film`: storyboards filmed on the test bench and cut into a rough cut and an edit
manifest. It installs nothing into the game: every take runs on a bench clone. Versioning
rules: [`CLAUDE.md`](../CLAUDE.md), "Changelogs and versions". Newest first. The why is
in [`README.md`](README.md).

## 1.4.0 — 2026-10-17

### Added
- A board over several files: `Include: <file>.md` splices a file of the film's folder
  in where it stands (sections and shots, in order, nested); shot ids are unique across
  all of them. `check` lists the files and marks each shot's; the manifest carries
  `includes` and each shot's `file`. Where a shot is written is not in its spec, so
  moving it between files keeps its takes (13be105).
- `a2film adopt <source> <shot>... <target> [--as ID]`: take over another board's takes
  of a shot whose spec is the same, less the film's and shot's names; copied (reflinked)
  with the `.spec.json` rewritten and an `adopted` key (13be105).
- The shot field `Tier:` (`1080`, `1440`, `2160`, `4k`, `WxH` or `final`): the shot is
  probed at that size, or its final take filmed, in the same probe run; takes larger
  than the probes run after the others, one at a time (13be105).
- `films/reference/`, the reference catalogue (`README.md`, `catalogue.md` with one
  example entry), and `From: ref:<id>` for footage shots, with the entry's `In`/`Out` (13be105).
- `--ignore-code` on `animatic` and `sheet` (13be105).

### Changed
- `animatic`, `sheet` and `manifest` at the probe tier cut each shot from its best take,
  final or probe (current first, then the larger), scaled to the cut; a manifest take
  says its `tier`. With probe takes only, as before (13be105).
- `films/film/selftest.py` also pins included files, `Tier:`, the take a cut picks,
  `adopt` and the catalogue (13be105).

Tooling only; nothing to see in game.

## 1.3.0 — 2026-10-16

### Added
- `films/origin`: the tractor beam as a cone of light (`tr15`) and shield hits with
  mist (`sh14`), the art direction merged with render3d 0.6.0; `as17` notes why the
  assimilation beam does not form on the bench.

Tooling only; nothing to see in game.

## 1.2.0 — 2026-10-16

### Added
- The board `films/origin` for the origin video's game shots that `films/promo` lacks:
  an Akira class stock and refit (`ak13`, scene `films/origin/akira.ini`), the Metreon
  nebula (`mt14`, scene `films/origin/metreon.ini`) and the Borg cube's technology
  assimilation beam (`as17`) (522f9b7).

Tooling only; nothing to see in game.

## 1.1.0 — 2026-10-16

### Changed
- A take is current only while the code it shows is: every take records its commit,
  whether the tree was dirty, the paths it depends on and a hash of its uncommitted
  changes to them (`.spec.json` keys `commit`, `dirty`, `scope`, `code`), and is stale
  once any of those paths changed since, committed or not. Docs, `films/` and other
  paths that cannot change a take are left out; the stock variant sees only the bench
  and its installs. A take of 1.0.0 from a dirty tree, or without a commit, is
  `unknown` (3937489).
- `status`, `probe`, `take` and `manifest` say why a take is not current ("spec
  changed", "code changed since 139e18a (render3d)"); the manifest's takes carry `dirty`
  and `stale` beside `current` (3937489).

### Added
- `--ignore-code` on `status`, `probe`, `take` and `manifest`: only a changed spec makes
  a take stale (3937489).
- `films/film/selftest.py`, which pins when a take is current, on a throw-away
  repository (3937489).

Tooling only; nothing to see in game.

## 1.0.0 — 2026-10-14

### Added
- The storyboard format (`films/<name>/storyboard.md`): sections and shots with scene,
  variants, camera (```` ```setup ````, ```` ```action ````), layout, crop, transition,
  titles, key hints, audio, music beds per section, cards, `From:` footage and a status.
- Desktop shots (`Kind: desktop`, `Run:`, `Wallpaper:`, the `run` command): a program,
  the GUI installer, filmed on a bare bench desktop, installing into a stock clone of the
  game. The film field `Pointer:` (1920x1080): pointer coordinates scaled to each take.
- Two games in one shot (`Players:`, `Prepare:`, `PLAYER: CMD` lines, `typelog`), filmed
  together and laid side by side by `split [w]`: online play, host and joiner.
- `./a2film check`, `status`, `probe` (1280x720, `--jobs 4`), `take` (3840x2160,
  `--jobs 1`), `sheet`, `animatic` and `manifest` (`a2film-manifest/1`). A take is
  re-filmed only when its spec changes.
- `films/selftest` and the draft boards `films/whats-new` and `films/promo` (with its review plan, `PLAN.md`).
- The `film` skill (`.claude/skills/film/SKILL.md`).

Tooling only; nothing to see in game.
