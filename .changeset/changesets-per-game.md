---
layers: [root]
bump: minor
games: [armada2, fo-roots]
status: none
---

### Added
- Changesets name their games (`games: [armada2]`, required; `status.<game>:` overrides the status for one game), and each game has its own version track: `tools/changeset.py version --game <id>` (default `armada2`, the version files and the published changelogs; another game's track is `games/<id>/CHANGELOG.md` only, written by hand, never released), `status --game`, `add --games`. `check` fails on a missing or unknown game, and on a changeset of a published track that names a game not offered.

### Fixed
- The Release workflow writes its status and PR body outside the checkout, so `version --commit` no longer commits a stray `status.md`; the one 15.10.0 committed is removed.
