# Changelog — nebulae

`Nebulae.asi`: each nebula class with a recipe drawn as a volume of gas in place of its
billboards, and the recipes in `recipes/`. Versioning rules: [`CLAUDE.md`](../CLAUDE.md),
"Changelogs and versions". Newest first. The reasoning is in [`README.md`](README.md).

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
