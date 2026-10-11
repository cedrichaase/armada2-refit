# Changelog — surfaces

`Surfaces.asi`: planets' and the dilithium moon's surfaces made from recipes, drawn by
`Lighting.asi`. Versioning rules: [`CLAUDE.md`](../CLAUDE.md), "Changelogs and
versions". Newest first. The reasoning is in [`README.md`](README.md).

## 1.0.0 — 2026-10-12

### Added
- `Surfaces.asi`: exports `a2_surface` (`surfaces.h`); bakes a recipe into albedo,
  relief and cloud cubes on its first draw (106–480 ms; the session's first 2–3 s).
- `Surfaces.ini`: `Enable`, `Face`, `CloudFace`, `Reload`, `Log`.
- Recipes `PB_CLSSD`, `PB_CLSSH`, `PB_CLSSJ`, `PB_CLSSK`, `PB_CLSSL`, `PB_CLSSM`, `Mdmoon`.
- `install.sh` (`--remove`).

Installed, not yet seen in game.
