---
layers: [root]
bump: minor
status: none
---

### Added
- Changesets: `.changeset/` (format in its README, units in `config.toml`) and `tools/changeset.py` (`add`, `status`, `version [--commit]`, `released`, `check`, `convert`), with `tools/changeset-selftest.py`. Versions and changelog entries are written only by `changeset.py version`. CI runs `check`: a pull request may not change a version by hand, except the release PR `release/next`.
- The Release workflow (`.github/workflows/release.yml`) opens the "Release X.Y.Z" pull request from the pending changesets. Merging it publishes vX.Y.Z (CI jobs `release-commit` and `release`, on the commit's `Release-Version` trailer). A root version that changes without the trailer, such as after a squash merge, fails `release-commit` with what to do.
- Pre-releases of the latest main: CI job `prerelease` publishes every push to main as the GitHub pre-release `vX.Y.Z-pre.N` (the next version, N the run number), keeping the newest three. `publish/package.sh` builds its zip with `A2_PRERELEASE=N`. `changeset.py status --next-root` gives the current version plus a patch when nothing is pending.

### Changed
- A push to main publishes a release only when it merges the release PR. A version changed by hand releases nothing, and a dispatch with `release` ticked is the fallback. The `release` job refuses an existing tag and keeps the newest ten `vX.Y.Z` releases.
- A versioned release not yet seen in game, or carrying a status line starting "Untested" (was "Pre-release"), is an "untested release": GitHub's prerelease flag.
- `CLAUDE.md` ("Changelogs and versions", "Finishing work"), `publish/README.md` ("Releasing", "Pre-releases: the latest main") and `README.md` describe the new flow.
