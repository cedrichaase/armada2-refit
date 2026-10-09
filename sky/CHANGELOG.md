# Changelog — sky

`Sky.asi`: the map's sky drawn from a recipe, in a shader, in place of the six painted
cube faces, and the recipes in `skies/`. Versioning rules: [`CLAUDE.md`](../CLAUDE.md),
"Changelogs and versions". Newest first. The reasoning is in [`README.md`](README.md).

## 1.0.0 — 2026-10-09

### Added
- `Sky.asi`: the call to `ST3D_Instance::Render` in `Background_Render` (0x590e17) draws
  the sky from `Sky\<name>.ini` when the map's sky has one and a Direct3D 9 device is
  behind d3d8; otherwise the stock cube, as before. Domain-warped value noise, baked
  once per map into a 10-bit managed cube with edge texels on the edges, sampled per
  frame.
- `Sky.ini`: `Enable`, `Face` (cube edge, 1536; 0 = per pixel), `Reload`, `Timing`, `Log`.
- Recipes `mbgaqu` (Aqua: dim teal and blue curtains) and `mbgkling` (Klingon: red,
  orange and gold billows about two cores).
- `install.sh` (`--timing`, `--remove`); `./install` runs it, `a2mod` switches `Sky.asi`,
  `Sky.ini` and `Sky/*.ini` as the `sky` layer.

Installed, not yet seen in game.
