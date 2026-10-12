# Changelog — specials

`Specials.asi`: the warp flash and the special weapons drawn by shaders, in place of
stock's sprites and models. Versioning rules: [`CLAUDE.md`](../CLAUDE.md), "Changelogs
and versions". Newest first. The reasoning is in [`README.md`](README.md).

## 1.0.0 — 2026-10-13

### Added
- `Specials.asi`, the warp flash: the call to `ST3D_Sprite::DrawScaled3D` in
  `WarpSpriteEffect::Render` notes each flash and draws nothing; the flash is drawn
  after `RenderChunks` instead: a white-hot core in a blue glow, stock's four-pointed
  star and a long horizontal streak, growing as the craft closes on the point, bursting
  as it gets there, then an afterglow and a ring running out. Without a Direct3D 9
  device behind d3d8, the stock sprite.
- The special weapons' shots: the call to `Submit` in `Ordnance::RenderAll` goes
  through the plugin; an ordnance whose ODF has an `[Ordnance.<odf>]` recipe is drawn
  as a beam (core, glow, flowing pulses, twisting strands, crackle, a cone with
  `Taper`, flares at both ends), a bolt (core, glow, tail, turning rays) or a field;
  every other ordnance is submitted as before. Recipes for 23 special weapons' ordnance
  (Borg technology assimilation, holding, auto-assimilator and ion beams, the tractor
  beams, the plasma cannon, 8472's clairvoyant link, psionic disruption and resource
  extraction beams, the psychonic blast, holo-generator, shield harmonics
  disruptor and shield inversion beams; the ion storm torpedo, energy dissipator,
  micro-organisms, Gemini effect, nanites, nebula flux, computer override, plasma bank
  overload and shield disruptor bolts).
- The special weapons' waves: slot 15 of the `ShockwaveExplosion` vtable (`Submit`),
  chained over whatever holds it (`Explosions.asi`'s rings): a wave whose ODF has a
  `[Wave.<odf>]` recipe is drawn as a band of light in its own plane, rays reaching in
  from it and a fill behind it, fading as stock's disc fades. Recipes for the 11 special
  waves, in their stock textures' colours; the death and self-destruct shockwaves stay
  `Explosions.asi`'s.
- Each flash, shot and wave lights the hulls about it: point lights registered through
  the chained call to `Ordnance::PreRenderAll`, which `Lighting.asi` hands to the
  GPU-drawn hulls.
- `Specials.ini`: `[Specials]` `Enable`, `Reload`, `Log`, `Ordnance`, `Waves`, `Survey`
  (logs every ordnance drawn: ODF, class, ends, owner, target); `[Warp]` `Enable`,
  `Brightness`, `Size`, `MinPixels`, `Core`, `Glow`, `Star`, `StarSharpness`, `Colour`,
  `Streak`, `StreakLength`, `StreakSharpness`, `StreakColour`, `Rise`, `Burst`,
  `Afterglow`, `Ring`, `RingSpeed`, `RingWidth`, `RingTime`, `RingColour`, `Light`,
  `LightColour`, `LightReach`; `[Ordnance.<odf>]` `Kind`, `Colour`, `CoreColour`,
  `Brightness`, `Flicker`, `FlickerRate`, `FadeIn`, `Width`, `MinPixels`, `CoreWidth`,
  `Taper`, `Flow`, `Helix`, `Crackle`, `Emitter`, `Impact`, `Star`, `Size`, `Tail`,
  `Rays`, `Spin`, `Pulse`, `PulseRate`, `On`, `Radius`, `Rim`, `Swirl`, `Fill`, `Light`,
  `LightColour`, `LightReach`; `[Wave.<odf>]` `Colour`, `CoreColour`, `Brightness`,
  `Band`, `RayCount`, `RayShare`, `Fill`, `Turbulence`, `Flicker`, `Light`,
  `LightColour`.
- `install.sh` (`--remove`); `./install` runs it, `a2mod` switches it as the `specials`
  layer.

Installed, not yet seen in game.
