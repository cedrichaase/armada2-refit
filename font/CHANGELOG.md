# Changelog — font

The condensed in-game bitmap font (`ui-font-condense.py`, which rewrites the twelve
`FontFinal4_*` atlases and their `.spr` metrics in place, backed up as
`.a2font-backup`). Versioning rules: [`CLAUDE.md`](../CLAUDE.md), "Changelogs and
versions". Newest first. The derivation is in [`README.md`](README.md).

## 1.0.0 — 2026-09-25

Baseline: the first versioned release.

- Glyph art and advance widths are condensed by `1.25 * H / W` (0.5233 at 3440x1440)
  with `--method resample`, the default. `--check` verifies that the `.spr` and `.tga`
  files agree, and `--revert` restores stock. Confirmed in game.
- `--method runs` is kept but not shipped. It was rejected in game.

## Before versioning

### 2026-09-25
- Ported into `font/` when the `worktree-font-condense` branch merged. The installed
  files rebuild from stock byte-for-byte (`36ce1a7`).

### 2026-09-21
- The condense landed (`e124156`) and was confirmed in game: `OBJECTIVES:` measured
  within 0.5% of the prediction (`c5487c8`).
- `--method runs` became the default (`b6941ef`). It was rejected in game and
  `resample` became the default again (`c81f534`).
