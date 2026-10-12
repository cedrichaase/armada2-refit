# Changelog — repair

`Repair.asi`: the welding on a ship under repair drawn by shaders, in place of the stock
spark sprites. Versioning rules: [`CLAUDE.md`](../CLAUDE.md), "Changelogs and versions".
Newest first. The reasoning is in [`README.md`](README.md).

## 1.0.0 — 2026-10-12

### Added
- `Repair.asi`: the call to `NodeParticleEffect::AddParticle` in `Craft::RepairSystem`
  notes each repair spark the engine makes; the call to `ST3D_Sprite::DrawScaled3D` in
  `NodeParticleEffect::Render` skips those (and only those); they are drawn after
  `RenderChunks` instead: a flickering, stuttering blue-white arc with rays, an orange
  hot spot that cools after it, and a spray of sparks streaking off the hull and cooling
  from white to red. Without a Direct3D 9 device behind d3d8, the stock sparks.
- Each burning arc lights the hull about it: a point light registered through the
  chained call to `Ordnance::PreRenderAll`, which `Lighting.asi` hands to the GPU-drawn
  hulls.
- `Repair.ini`: `Enable`, `Reload`, `LogWelds`, `Log`, `Brightness`, `ArcTime`,
  `FlickerRate`, `Stutter`, `Flare`, `FlareMinPixels`, `ArcColour`, `FlareCore`,
  `FlareGlow`, `FlareRays`, `FlareRaySharpness`, `Sparks`, `SparkSpeed`, `SparkSpread`,
  `SparkLife`, `SparkDrag`, `SparkWidth`, `SparkMinPixels`, `StreakTime`,
  `SparkBrightness`, `HotSpot`, `HotSize`, `HotColour`, `HotTime`, `Light`,
  `LightColour`, `LightReach`, `LightLift`, `Pull`.
- `install.sh` (`--remove`); `./install` runs it, `a2mod` switches it as the `repair`
  layer.

Installed, not yet seen in game.
