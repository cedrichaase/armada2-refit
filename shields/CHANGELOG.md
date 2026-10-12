# Changelog — shields

`Shields.asi`: the hits on a ship's shields drawn by a shader over its shield ellipsoid,
in place of the stock shell. Versioning rules: [`CLAUDE.md`](../CLAUDE.md), "Changelogs
and versions". Newest first. The reasoning is in [`README.md`](README.md).

## 1.1.0 — 2026-10-12

### Added
- The shields-down ring (shield type 1, `xshlddie`) drawn by a second pixel shader over
  the ship's shield ellipsoid in place of stock's turning band: jagged blue-white arcs
  (the zero lines of warped gradient noise, a core of constant pixel width) in
  discharges that flare and die about the shell, mostly about the equator, struck afresh
  `DownRate` times a second in two fields half a step apart, so nothing turns; a flash as
  the shields collapse, a fade as they come back. `Down=0` leaves stock's ring.
- The brightest discharge lights the ship's own hull: a point light registered after
  `GameObject_PreRenderAll` through the chained call to `Ordnance::PreRenderAll`
  (0x598199), which `Lighting.asi` hands to the GPU-drawn hulls.
- `Shields.ini`: `Down`, `DownBrightness`, `DownArc`, `DownGlow`, `DownHaze`,
  `DownScale`, `DownWarp`, `DownCore`, `DownGlowWidth`, `DownBranches`, `DownLit`,
  `DownRate`, `DownStrikes`, `DownStrikeSize`, `DownStrikeMin`, `DownStrikeMax`,
  `DownStrikeGap`, `DownCrackle`, `DownBand`, `DownFlash`, `DownFlashTime`,
  `DownFadeOut`, `DownFarSide`, `DownRimPower`, `DownLight`, `DownLightColour`,
  `DownLightReach`.

Installed, not yet seen in game.

## 1.0.0 — 2026-10-12

### Added
- `Shields.asi`: the call to `ShieldHit::DrawShield` in `ShieldEffect::Render` skips
  the ordinary hits (type 0, `Xshldx01`); they are drawn after `RenderChunks` in
  `Armada_RenderAllOurStuff`, one pass per struck ship over its shield ellipsoid: a hot
  spot, a ring running out, a soft glow and the rim lit about each hit, up to eight a
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
