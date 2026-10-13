# Changelog — venting

`Venting.asi`: the plasma and gas a damaged ship vents, drawn by shaders, in place of
the stock sprites. Versioning rules: [`CLAUDE.md`](../CLAUDE.md), "Changelogs and
versions". Newest first. The reasoning is in [`README.md`](README.md).

## 1.0.0 — 2026-10-13

### Added
- `Venting.asi`: slot 50 of `ST3D_DeviceDirectX8`'s vtable (`RenderParticle`) notes the
  particles of the `ventplas` and `ventgas` sprite nodes (the damage emitters
  `plasmasml`..`plasmaxl`, `steamtiny`..`steamxl`) and skips them; slot 11 of
  `ST3D_ParticleEmitter_Spray`'s (`RenderInternal`) notes where each vent emitter is;
  they are drawn after `RenderChunks` instead: each particle a puff of gas carved by
  rolling noise, plasma cooling from white-hot through orange to red, gas from pale to
  blue; a flickering glow and jet core at each vent; on a moving ship each puff
  stretched back to the next, one unbroken trail. Without a Direct3D 9 device behind
  d3d8, the stock sprites.
- Each vent lights the hull about it, flickering: a point light registered through the
  chained call to `Ordnance::PreRenderAll`, which `Lighting.asi` hands to the GPU-drawn
  hulls.
- `Venting.ini`: `Enable`, `Reload`, `Log`, `LogPuffs`, `Plasma`, `Gas`, `PlasmaSprite`,
  `GasSprite`; per kind `Brightness`, `Size`, `Stretch`, `MinPixels`, `Glow`, `GlowSize`,
  `GlowMinPixels`, `GlowColour`, `Jet`, `JetLength`, `JetWidth`, `Flicker`, `Light`,
  `LightColour`, `LightReach`, `LightLift`, `Cooling`; `PlasmaHot`, `PlasmaMid`,
  `PlasmaCool`, `GasBright`, `GasCold`, `BillowScale`, `BillowFlow`, `BillowContrast`,
  `EdgeSoftness`, `MaxLink`, `Pull`.
- `install.sh` (`--remove`); `./install` runs it, `a2mod` switches it as the `venting`
  layer.

Installed, not yet seen in game.
