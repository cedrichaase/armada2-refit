---
layers: [root]
bump: patch
games: [armada2]
status: none
---

### Added
- `tools/changeset.py check` fails when a file players receive (the release package's files, the guides, the GUI installer, the settings tables, the changelogs) names a game that is not offered; `publish/README.md` says which files.
