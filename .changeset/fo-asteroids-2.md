---
layers: [render3d, testbench]
bump: patch
games: [fo-roots]
status: Installed, not yet seen in game
---

### Fixed
- Hulls: a latinum nebula (Fleet Operations: Roots' asteroid sets) is never claimed as a hull; with the particles module off it is the engine's again, as on stock, instead of being drawn through the hulls module's sprite path.

- Textures: a DDS file's own mip levels are used (`a2fmt::dds::decode_level`) in place of render3d's box-filtered ones, as the engine samples them; the `alpha` sprite material goes through the chain again instead of its top level only. Roots' ice asteroid field now matches the engine's draw at three zooms.

### Added
- Scenario `fo-render3d-asteroids` takes render3d's and the engine's draw of the ice field a moment apart at three zooms; `fo-render3d-asteroids-stock` shoots the same views with Display8 alone.
