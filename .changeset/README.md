# Changesets

Each file here (other than this README and `config.toml`) is one **pending change**: what
it changes, which layers it touches, which games' version tracks it belongs to, and how
far each layer's version moves. Nobody edits a version or a `## X.Y.Z` heading by hand.
The release (`tools/changeset.py version --game armada2`, run by the Release workflow)
consumes the files naming Armada II. It computes the versions, writes the changelog
entries and deletes the files (see "Games" for the other tracks). The rules are in `CLAUDE.md`, "Changelogs and
versions", and the release steps are in `publish/README.md`, "Releasing".

    python3 tools/changeset.py add --layers qol --bump patch --section Fixed -m "..."
    python3 tools/changeset.py add --layers ui --bump minor --games armada2,fo-roots -m "..."
    python3 tools/changeset.py add             # asks, in a terminal
    python3 tools/changeset.py status          # what the next release would bump, per game
    python3 tools/changeset.py check           # what CI checks

## The file

    ---
    layers: [render3d, qol]
    bump: minor
    games: [armada2]
    status: Installed, not yet seen in game
    ---

    Optional text before the first section, kept at the top of each layer's entry.

    ### Added
    - Move paths drawn until flown (`[Overlays] MovePaths`; `PathStartsAtOnce`).

    ### Fixed
    - ...

- **The frontmatter** is pnpm's changesets format, with three keys added: `layers`,
  `bump` and `games`. Write the layers either way:
  - `layers: [a, b]` with `bump: major|minor|patch`, the same bump for each;
  - one line per layer, `a: minor` and `b: patch` (pnpm's own form), when they differ.
    A per-layer line overrides `bump:` for that layer.
- **Layer ids** are those of `layers.toml` (not the retired ones), the units in
  `config.toml` (`gui-installer`, `testbench`, `films`), and `root` for a change that
  touches no layer (`a2mod`, the repo layout, a cross-layer convention), which bumps only
  the project. `./a2mod status` and `tools/changeset.py add` (without `-m`) list them.
- **`bump`** is required. MAJOR, MINOR and PATCH mean what `CLAUDE.md` says. A layer's
  next version is its current one plus the largest bump among the pending changesets
  that name it. The project's (root) next version is its current one plus the largest
  bump of all of them.
- **`games`** is required: the ids of `games/<id>.toml` whose version track the change
  belongs to (`[armada2]`, `[fo-roots]`, `[armada2, fo-roots]`). `add` writes
  `[armada2]`, the default game, unless `--games` says otherwise. See "Games".
- **`status`** is the in-game status line, and is carried into the entry. It defaults to
  `Installed, not yet seen in game`. Write `Confirmed in game YYYY-MM-DD` once the user
  has seen it, or `none` for a change that installs nothing (CI, docs). A line starting
  `Untested` (for example, not yet past the Windows gate) makes the release an untested
  release, as "not yet seen" does. That is GitHub's *Pre-release* flag; it is not the
  pre-release build of the latest main.
- **The body** is one text for every layer listed. A layer that needs other words gets
  a changeset of its own. Sections are `### Added`, `### Changed`, `### Fixed` and
  `### Removed`. Text under no section is `### Changed`. Each item says *what* changed
  in one or two lines and names the key or target. The *why* belongs in the layer's README.

## Games

Each game has a **version track** of its own: a version per layer and one for the
project, per game. A changeset counts on the track of every game in its `games:`.

- **Armada II** (`armada2`, the default game) is the track players get: the layers'
  `CHANGELOG.md`, the root `CHANGELOG.md`, `Cargo.toml` and every version file below,
  the release notes and the GitHub releases. Nothing about it changed.
- **Every other game** has one file, `games/<id>/CHANGELOG.md`, and nothing else: no
  version file, no tag, no GitHub release. Its entries are root-style: a `Layers:` line
  with every layer's version on that track, then the items, each led by its layers. A
  layer's version on the track is the newest `Layers:` line that names it; a layer not
  released there yet starts from 0.0.0, and so does the game's own version.
- `version --game <id>` (default `armada2`) consumes the changesets naming that game: it
  takes the game off their `games:` and deletes a file only when no game is left. So a
  shared changeset waits for the other game's release. `version --game <other>` is run
  by hand and refuses `--commit` (only the default game's release commit carries the
  trailer CI publishes on); commit `games/<id>/CHANGELOG.md` like any change.
- `status` prints every game's track; `--game` picks one. `status --json`, `--markdown`
  and `--next-root`, which CI publishes, are the default game's unless `--game` says
  otherwise.
- **`status.<game>: ...`** gives one game its own status line
  (`status.fo-roots: Confirmed in game 2026-11-07`); `status:` is every other game's.
  Whether a release is an untested release is decided by Armada II's status alone.
- A layer whose `games` in `layers.toml` leaves a game out takes no bump on that game's
  track; a changeset that names no unit of one of its games is refused.
- **A game that is not offered is never named on a published track**: `check` fails on a
  changeset whose `games:` holds an offered game and whose text names one that is not
  (its id, its label, and its descriptor's `[changesets] quiet` words). Word the shared
  change without it, or split it in two: one changeset with `games: [armada2]` and
  neutral words, one with `games: [fo-roots]` and the specifics.

## What the release writes

For each layer, a `## X.Y.Z — date` entry is added at the top of its `CHANGELOG.md`.
Its sections hold the items of every changeset that names the layer, oldest first. Each
item ends with the hash of the commit that added its changeset, and then the status
line. When all of an entry's changesets share one status, the line stands alone at the
end of the entry. Otherwise each item carries its own. The version is also written
everywhere else it lives: `Cargo.toml`, the `Cargo.lock` files, and the files in
`config.toml`. The root `CHANGELOG.md` gets `Layers: a 1.2.0, b 0.4.1; testbench 4.3.0.`
and every item, each led by the layers it touches.
