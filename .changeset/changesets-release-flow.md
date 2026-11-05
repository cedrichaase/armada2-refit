---
layers: [root]
bump: minor
status: none
---

### Added
- Changesets: `.changeset/` (format in its README, units in `config.toml`) and `tools/changeset.py` (`add`, `status`, `version`, `check`, `convert`), with `tools/changeset-selftest.py`. Versions and changelog entries are written only by `changeset.py version`; CI runs `check` (a pull request may not change a version by hand, the release PR `release/next` excepted).
- The Release workflow (`.github/workflows/release.yml`): step `prepare` opens the "Release X.Y.Z" pull request from the pending changesets, step `publish` dispatches CI on main with `release` ticked.
- The edge channel: CI job `edge` replaces the assets of the rolling pre-release `edge` on every push to main; `publish/package.sh` builds its zip with `A2_EDGE=1` as `<next>-edge.<commit>`.

### Changed
- A push to main no longer publishes a release when the root version changes: CI's `release` job runs only on a dispatch with `release` ticked, refuses an existing tag, and keeps the newest ten `vX.Y.Z` releases (never counting `edge`).
- `CLAUDE.md` ("Changelogs and versions", "Finishing work"), `publish/README.md` ("Releasing", "The edge channel") and `README.md` describe the new flow.
