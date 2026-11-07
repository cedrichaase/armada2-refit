---
layers: [render3d, testbench]
bump: patch
status: Installed, not yet seen in game
---

### Fixed
- Particles: a sprite of the `alpha` or `alpha_nof` material is blended by its alpha, sampled from the texture's top level, instead of cut out at one half: Fleet Operations: Roots' ice asteroid sets no longer read as see-through flecks over their haze (stock's sprite tables never use either material).

### Added
- Scenario `fo-render3d-asteroids`: Roots' first allied tutorial's ice asteroid field, drawn by the particles module, then by the engine for the reference.
