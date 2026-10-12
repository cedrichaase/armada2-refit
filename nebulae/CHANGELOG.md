# Changelog — nebulae

`Nebulae.asi`: each nebula class with a recipe drawn as a volume of gas in place of its
billboards, and the recipes in `recipes/`. Versioning rules: [`CLAUDE.md`](../CLAUDE.md),
"Changelogs and versions". Newest first. The reasoning is in [`README.md`](README.md).

## 1.4.0 — 2026-10-13

### Added
- `Swirl=` (with `SwirlScale=`, `SwirlPeriod=`): the gas read round two small circles,
  the second the other way and 1.618 times as fast, at phases that vary from place to
  place, so each speck wanders in slow loops out of step with its neighbours. The
  occupancy record is widened by its reach as by the flow's. On in `mnebula10` (the
  Cerulean: 18, scale 1, 30 s).

Installed, not yet seen in game.

## 1.3.1 — 2026-10-13

### Changed
- `mnebula7`: `Lightning=` 15 → 7.5, half as many flashes.

Installed, not yet seen in game.

## 1.3.0 — 2026-10-13

### Added
- `Fringe=` (with `FringeReach=`, `FringeScale=`): a thin, soft haze about a body of gas,
  out past its edge and fading all the way, so a field thins out into space instead of
  stopping at its outline. On in `mnebula6` (0.25) and `mnebula7` (0.3).
- `Fuzz=` (with `FuzzSpeed=`): the thin gas at the outline feathered by the fine octave,
  which now drifts slowly (two reads drifting opposite ways), so the edges are wispy and
  their wisps come and go. On in `mnebula6`, `7`, `8`, `9`, `12`.

### Changed
- The flow is two reads drifting opposite ways instead of one: the gas changes shape
  slowly rather than wobbling in place (every recipe with `Flow=`).
- `mnebula7`: `Softness=` 0.7 → 1.0.

Installed, not yet seen in game.

## 1.2.1 — 2026-10-13

### Fixed
- Ships' shadows are read at both points the gas is read at (`Samples=2`, the default),
  which together cover the whole gap to the next slice; the jittered point alone covers
  half of it, so a shaft seen along its length, looking towards the Key, drew as one
  copy of the hull's silhouette per slice with gaps between them.

Installed, not yet seen in game.

## 1.2.0 — 2026-10-12

### Added
- `EdgeRefine=` (default 1): at depth edges (hulls in or before the gas) the half-size
  gas is drawn again at full size, so silhouettes are exact instead of in two-pixel steps;
  half size elsewhere. 0 is half size everywhere, 2 full size everywhere (a reference).
  `Timing=1` logs the pixels refined.

Installed, not yet seen in game.

## 1.1.0 — 2026-10-12

### Added
- The gas lit by the Key (`LightingKey`, lighting 1.19.0): its light is its own glow and
  a share of the Key's (`KeyShare=0.8`), scattered forward (`Scatter=0.2`,
  Henyey-Greenstein), brighter looking towards the Key and dimmer with it behind; the
  self-shading steps towards the Key instead of the recipe's `Light=`. Both keys in
  `Nebulae.ini` and per recipe; `KeyShare=0` is the gas as before.
- Ships' shadows in the gas: a hull takes the Key's share of the gas's light behind it,
  from `Lighting.asi`'s shadow map (`LightingShadowMap`), sharp beside the hull and
  widening down the shaft; lightning is not shadowed. `Nebulae.ini`: `Shadows=1`,
  `ShadowSoftness=0.02`.

Installed, not yet seen in game.

## 1.0.3 — 2026-10-12

### Fixed
- Rebuilt with platform 3.4.1's `d9_device()`, which tells a new Direct3D device by the
  d3d9 device rather than the d3d8 device's address, so the gas's shaders are made
  again on a device at a re-used address.

Installed, not yet seen in game.

## 1.0.2 — 2026-10-12

### Fixed
- The blue-noise sampler (stage 4) reads linear: `SRGBTEXTURE` was set to 1 with the
  point filters, which biased the dither thresholds low (mean about 0.21, not 0.5).
  Changes the look slightly: the gas was tuned with the bias in place.

Installed, not yet seen in game.

## 1.0.1 — 2026-10-12

### Fixed
- The classes are forgotten when the game frees its own: the two calls to
  `GameObjectClass_Clean` (0x485c06, 0x4873ea) are wrapped. Kept by class pointer, a
  later game's classes took over stale entries at recycled addresses, their textures
  stayed held, and after 16 classes every nebula fell back to billboards.
- A noise, envelope or bake that cannot be made is no longer retried every frame: the
  noise once per device or `Nebulae.ini` read, a class's bake once per recipe read. A
  class whose bake failed gets its stock billboards (it vanished at `Resolution=2`).
- `Resolution=2`: no full-screen composite when no gas box was drawn; the half-size
  depth and clear passes turn the alpha test off.
- The viewport is put back after a bake (it covered the whole target in the HUD's
  action camera); a timing query pair is never left half made; the recipe log line no
  longer overflows its buffer on a long install path.

Installed, not yet seen in game.

## 1.0.0 — 2026-10-10

### Added
- `Nebulae.asi`: `Nebula::Render` (slot 11 of the Nebula vtable) and `GameObject::Render`
  for the LatinumNebula vtable skip the billboards and the latinum model of a class with
  a recipe; the gas is drawn after `RenderParticleList` in `Armada_RenderAllOurStuff`,
  as view-aligned slices on a ladder fixed in distance from the eye, kept per view (the
  HUD's action camera is a second one). Fog-of-war ghosts keep their gas. Without a
  Direct3D 9 device behind d3d8, or a recipe, the stock billboards.
- The gas: an envelope of the class's nebulae (domes at each nebula's own height), a
  spectral 3D noise turned off the world's axes, self-shading, knots, lanes, flow,
  filaments; baked into a volume per field when its nebulae change.
- `Resolution=2`: the gas at half size through an INTZ copy of the depth, composited
  depth-aware and scissored to the gas on screen. `Opacity=` per recipe levels thick gas
  off so it is as bright from every angle; `Obscure=` sets how much it hides behind it.
- Recipe keys (`README.md`, "Recipes"), among them `Lightning=`, `HueCycle=`, `Core=`,
  `Pulse=`, `Ring=` and `Radius=`.
- Recipes for every class: `mnebula6` (radioactive), `mnebula7` (metreon, a
  thunderstorm), `mnebula8` (Mutara), `mnebula9` (metaphasic), `mnebula10` (cerulean,
  spotty), `mnebula12` (tachyon), `fluidicnebula`, `generatednebula`, `latinum` (a gold
  ball with a pulsing core and a colour-cycling patch about it).
- `Nebulae.ini`: `Enable`, `Reload`, `Slices`, `MinStep`, `Bake`, `Resolution`,
  `Samples`, `Obscure`, `VolumeSize`, `VolumeHeight`, `VolumeTexel`, `NoiseSize`,
  `Timing`, `Log`.
- `install.sh` (`--timing`, `--remove`); `./install` runs it, `a2mod` switches
  `Nebulae.asi`, `Nebulae.ini` and `Nebulae/*.ini` as the `nebulae` layer. Not in the
  release package yet.

Confirmed in game 2026-10-10.
