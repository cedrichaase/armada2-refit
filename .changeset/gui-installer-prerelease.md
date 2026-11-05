---
layers: [gui-installer]
bump: minor
status: Installed, not yet seen in game
---

Reads package schemas 0 and 1, unchanged.

### Added
- The pre-release build of the latest main: the newest release tagged `vX.Y.Z-pre.N` (`parse_releases`) is one extra, last entry, "Pre-release (latest main) · version · commit · date — untested", never the default; `--install pre-release`; older cached `-pre.` zips are pruned on download.
- `A2_RELEASES_API` points the release list at another URL (a `file://` mock in `test.sh`).
- `--selftest` and `test.sh` cases for the release list, the default and `--install pre-release`.

### Changed
- Only tags `vX.Y.Z` and `vX.Y.Z-pre.N` count, explicitly; any other tag is ignored. `default_release` never returns the pre-release build.
- A versioned release GitHub flags as a pre-release is now an "untested release" (`Release.untested`; `--list` kind `untested`); caches that say `prerelease` still read.
- `vtuple` reads the leading X.Y.Z only, so a `-pre.N` version's build number stays out of the tuple; `merge_cached` knows pre-release zips.
- With only the pre-release build listed, nothing is selected (Tk `ri = -1`, GTK `INVALID_LIST_POSITION`).
