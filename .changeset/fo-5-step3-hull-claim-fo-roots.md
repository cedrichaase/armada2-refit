---
render3d: minor
root: patch
games: [fo-roots]
status: Installed, not yet seen in game
---

### Added
- Fleet Operations: Roots (FO-5 steps 3 and 4): the object step calls through the object loop's draw site when another module owns it (`render3d.frame.obj_draw`, Roots' FleetOpsHook call; read at run time, never written), with the object current around the call; stock keeps its own loop.
- hulls: the instance-level claim (`render3d.hulls.instance_draw`, the call of `ST3D_Instance::RenderInternal` in `GameObjectInstance::RenderInternal`), chosen by the profile's `hl_claim_instance` (0 on stock, which installs nothing there); a hull is taken only when the instance is the current object's own.
- hulls: the bump ("Borg") profile chosen by the ODF's `race` where the content set says `ProfileBy = race` (the new `fleetops` set: `borg`, `freeborg`; `[Hulls] BorgRaces=` overrides); `BumpMaps = normal` reads bump textures as normal maps, a flat one as none. Stock keeps "bump-mapped" and height maps.
- Roots' files: its own mount list (`Textures\`, TGA and DDS; `ODF.fpq` behind `odf\`), by the executable's name; DDS textures (DXT1/3/5 and uncompressed, `a2fmt::dds`) and textures whose sides are not powers of two on Roots.

### Changed
- The Roots profile marks `ConstructionRig::Render`, `Armada_RenderFrame`, `Terrain_Geometry::Render`, `GameObject::RenderInternal`, `GetSensorScramblerWaveLight` and `PlaceholderFeature::sRenderInternal` verified for the core, frame and hulls hooks that use them.

### Fixed
- A build without the object loop's draw site (stock) no longer logs `render3d.frame.obj_draw stands down`: the hook is asked about with a non-logging `present` before `ready`.
- hulls: a one-time log line when the instance draw is the current object's but its camera is not the view's; the DDS reader takes at most 4096 x 4096 pixels, and the texture mip chain and normal-map chain allocate fallibly (a texture that does not fit is skipped); the ODF key of the race is the set's `RaceKey` (default `race`).
