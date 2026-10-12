# Changelog — shields

`Shields.asi`: the hits on a ship's shields drawn by a shader over its shield ellipsoid,
in place of the stock shell. Versioning rules: [`CLAUDE.md`](../CLAUDE.md), "Changelogs
and versions". Newest first. The reasoning is in [`README.md`](README.md).

## 1.0.0 — 2026-10-12

### Added
- `Shields.asi`: the call to `ShieldHit::DrawShield` in `ShieldEffect::Render` skips
  the ordinary hits (type 0, `Xshldx01`); they are drawn after `RenderChunks` in
  `Armada_RenderAllOurStuff`, one pass per struck ship over its shield ellipsoid: a hot
  spot, a ring running out, the hex lattice and rim lit about each hit, up to eight a
  ship. Other shield types stay stock. Without a Direct3D 9 device behind d3d8, the
  stock shell.
- The four weapon calls of `CreateShieldHit` are chained through thunks that note the
  weapon: a hit's strength follows its class's `damageBase`.
- Colour by faction (the struck ship's ODF), stock's green-to-red, or fixed; weaker,
  redder and flickering as the shields run out.
- `Shields.ini`: `Enable`, `Reload`, `LogHits`, `Log`, `Brightness`, `ColourMode`, the
  faction colours, `Colour`, `LowColour`, `LowShift`, `Spot`, `SpotSize`, `Core`,
  `Flash`, `FlashTime`, `Duration`, `Sustain`, `FadeOut`, `Ring`, `RingSpeed`,
  `RingWidth`, `RingTime`, `RingPeriod`, `RingLattice`, `Lattice`, `Rim`, `RimPower`,
  `GlowReach`, `GlowTime`, `GlowSustain`, `Pattern`, `HexCells`, `HexLine`, `Crackle`,
  `DamageRef`, `StrengthMin`, `StrengthMax`, `WeakLevel`, `Flicker`, `FlickerBelow`,
  `FlickerRate`, `TimeScale`.
- `install.sh` (`--remove`); `./install` runs it, `a2mod` switches it as the `shields`
  layer.

Installed, not yet seen in game.
