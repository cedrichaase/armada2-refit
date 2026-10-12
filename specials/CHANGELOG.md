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
- The flash lights the hulls about it: a point light registered through the chained
  call to `Ordnance::PreRenderAll`, which `Lighting.asi` hands to the GPU-drawn hulls.
- `Specials.ini`: `[Specials]` `Enable`, `Reload`, `Log`; `[Warp]` `Enable`,
  `Brightness`, `Size`, `MinPixels`, `Core`, `Glow`, `Star`, `StarSharpness`, `Colour`,
  `Streak`, `StreakLength`, `StreakSharpness`, `StreakColour`, `Rise`, `Burst`,
  `Afterglow`, `Ring`, `RingSpeed`, `RingWidth`, `RingTime`, `RingColour`, `Light`,
  `LightColour`, `LightReach`.
- `install.sh` (`--remove`); `./install` runs it, `a2mod` switches it as the `specials`
  layer.

Installed, not yet seen in game.
