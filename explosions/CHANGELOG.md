# Changelog — explosions

`Explosions.asi`: each ship's and station's fireball drawn as a volume of gas, and each
shockwave as a ring of light, in place of stock's sprite model and textured disc.
Versioning rules: [`CLAUDE.md`](../CLAUDE.md), "Changelogs and versions". Newest first.
The reasoning is in [`README.md`](README.md).

## 1.0.0 — 2026-10-12

### Added
- `Explosions.asi`: slot 15 (`Submit`) of the FireballExplosion and ShockwaveExplosion
  vtables notes each explosion the player sees, per view, in place of submitting stock's
  model or disc; the call to `RenderChunks` at 0x598446 draws them after every hull.
  Slots 0 and 14 of FireballExplosion's vtable, which `Lighting.asi` wraps, are left
  alone. Without a Direct3D 9 device behind d3d8, or with `Enable=0`, stock.
- The fireball: 28 steps through a sphere per pixel, a spectral noise volume made at
  start-up (`NoiseSize=`), seeded per explosion from its position; a white flash, hot
  billows growing over `Growth=` of its life and cooling through yellow and orange to
  smoke, a halo; its own depth written per pixel against the hulls.
- The ring: a thin rim of the shockwave class's colour, whitened, broken up by the
  noise, at the shockwave's own radius and fading as stock fades the disc (`Rings=`).
- `Explosions.ini`: `Enable`, `Rings`, `Reload`, `Brightness`, `Size`, `MinRadius`,
  `Growth`, `Cooling`, `Smoke`, `Turbulence`, `Opacity`, `Flash`, `Glow`, `Detail`,
  `RingBrightness`, `RingWidth`, `RingBreakup`, `NoiseSize`, `Age` (bench), `Timing`,
  `Log`.
- `explosions/install.sh` (`--timing`, `--remove`); `./install` runs it, `a2mod`
  switches `Explosions.asi` and `Explosions.ini` as the `explosions` layer.

Installed, not yet seen in game.
