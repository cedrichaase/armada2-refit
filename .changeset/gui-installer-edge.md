---
layers: [gui-installer]
bump: minor
status: Installed, not yet seen in game
---

Reads package schemas 0 and 1, unchanged.

### Added
- The edge channel: the release tagged `edge` (`parse_releases`) is one extra, last entry, "Edge (latest main) · commit · date — untested", never the default; `--install edge`, `--list` marks it; only older cached edge zips are pruned on download.
- `A2_RELEASES_API` points the release list at another URL (a `file://` mock in `test.sh`).
- `--selftest` and `test.sh` cases for the release list, the default and `--install edge`.

### Changed
- Only tags `vX.Y.Z` count as releases, explicitly; `default_release` never returns edge.
- `vtuple` reads the leading X.Y.Z only, so an edge version's sha stays out of the tuple; `merge_cached` knows edge zips.
