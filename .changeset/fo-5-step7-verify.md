---
render3d: patch
root: patch
status: Installed, not yet seen in game
---

### Changed
- The Roots profile (FO-5 step 7) verifies `Starfield_Load_Background_Geometry` (`render3d.sky.reads`), `GameObject::RenderInternal` (`render3d.particles.billboard`), `Armada_RenderAllOurStuff` (`render3d.ordnance.warp`), `RTS_Configure` (`render3d.view.distance`), `cOverViewImp::mNormalModeRender` and `cOverViewImp::GetSelectList` (`render3d.overlays.paths`): the sky, the billboard effects, the warp flash, `View.Distance` and the move paths run on Roots.

### Fixed
- overlays: the move paths read the overview through `g_pOverView` (`overview_ptr`) instead of the static object's address, so they follow an overview constructed elsewhere (Roots: FleetOpsHook's own); on stock the pointer holds the same object.
