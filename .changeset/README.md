# Changesets

Each file here (other than this README and `config.toml`) is one **pending change**: what
it changes, which layers it touches, and how far each layer's version moves. Nobody edits
a version or a `## X.Y.Z` heading by hand. The release (`tools/changeset.py version`, run
by the Release workflow) consumes every file here. It computes the versions, writes the
changelog entries and deletes the files. The rules are in `CLAUDE.md`, "Changelogs and
versions", and the release steps are in `publish/README.md`, "Releasing".

    python3 tools/changeset.py add --layers qol --bump patch --section Fixed -m "..."
    python3 tools/changeset.py add             # asks, in a terminal
    python3 tools/changeset.py status          # what the next release would bump
    python3 tools/changeset.py check           # what CI checks

## The file

    ---
    layers: [render3d, qol]
    bump: minor
    status: Installed, not yet seen in game
    ---

    Optional text before the first section, kept at the top of each layer's entry.

    ### Added
    - Move paths drawn until flown (`[Overlays] MovePaths`; `PathStartsAtOnce`).

    ### Fixed
    - ...

- **The frontmatter** is pnpm's changesets format, with two keys added: `layers` plus
  `bump`. Write the layers either way:
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
- **`status`** is the in-game status line, and is carried into the entry. It defaults to
  `Installed, not yet seen in game`. Write `Confirmed in game YYYY-MM-DD` once the user
  has seen it, or `none` for a change that installs nothing (CI, docs). A line starting
  `Pre-release` holds the release back as a pre-release, like "not yet seen" does.
- **The body** is one text for every layer listed. A layer that needs other words gets
  a changeset of its own. Sections are `### Added`, `### Changed`, `### Fixed` and
  `### Removed`. Text under no section is `### Changed`. Each item says *what* changed
  in one or two lines and names the key or target. The *why* belongs in the layer's README.

## What the release writes

For each layer, a `## X.Y.Z — date` entry is added at the top of its `CHANGELOG.md`.
Its sections hold the items of every changeset that names the layer, oldest first. Each
item ends with the hash of the commit that added its changeset, and then the status
line. When all of an entry's changesets share one status, the line stands alone at the
end of the entry. Otherwise each item carries its own. The version is also written
everywhere else it lives: `Cargo.toml`, the `Cargo.lock` files, and the files in
`config.toml`. The root `CHANGELOG.md` gets `Layers: a 1.2.0, b 0.4.1; testbench 4.3.0.`
and every item, each led by the layers it touches.
