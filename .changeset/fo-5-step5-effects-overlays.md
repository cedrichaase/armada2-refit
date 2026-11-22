---
render3d: minor
root: patch
games: [fo-roots]
status: Installed, not yet seen in game
---

### Added
- Fleet Operations: Roots (FO-5 step 5): callee hooks, a new hook kind (`Hooks::install_entry`, `a2rt::hook::patch_entry`): a function detoured at its entry, its first whole instructions (a profile value, within the bytes the profile checks there) moved to a trampoline. `render3d.effects.debris_callee` takes a dying craft's debris at `ST3D_Instance::Fragment`, and `render3d.overlays.posts_callee` the height posts at `DrawHeightPost`, each armed only while the build's companion module (`FleetOpsHook.dll`) is the caller, so Roots decides whether a craft breaks up or a post is drawn; chosen by the profile (`ef_debris_at_callee`, `ov_posts_at_callee`: 0 on stock, which installs nothing there).
- effects: shields follow `RTS_CFG.h`'s `SHIELD_EFFECT_RESCALE` (the shell scaled by the profile's `ef_shield_rescale_pct`, 200 on Roots) and `SHIELD_EFFECT_DAMAGE_COLORED` (0: stock-coloured hits white, no colour follows the shields' level) on a build whose profile says so (`ef_shield_damage_key`); stock reads neither key.
- overlays: the grid is drawn in the colour `Terrain_Geometry::DrawGrid`'s first line pushes, read at run time (`site_grid_colour`): stock's dark orange, Roots' own colour on Roots.
- hulls: at the instance claim (Roots), the node flags and mesh textures the engine and FleetOpsHook wrote into the shared database for the instance are honoured: a node hidden there is hidden, a mesh drawn with another texture (Roots' per-ship `stlogo`) is drawn with it.
- `a2engine`: `Engine::companions`, `companions_ok`, `check_bytes`.

### Changed
- The Roots profile verifies `ShieldHit::DrawShield`, `ShieldEffect::CreateShieldHit`, `Beam::Simulate`, `Armada_RenderAllOurStuff` (effects) and `Terrain_Geometry::DrawGrid` (overlays): the effects and overlays modules run on Roots.
