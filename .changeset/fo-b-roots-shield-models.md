---
render3d: patch
testbench: patch
root: patch
status: Installed, not yet seen in game
---

### Fixed
- effects: on Fleet Operations: Roots the shield hits are render3d's. FleetOpsHook picks a hit's model per ship class from its ODF (`shieldHit`, `shieldDown`, ...) in place of `CreateShieldHit`'s pick from the shield database, so render3d knew none of Roots' hits and left them all to the engine. The new hook `render3d.effects.shield_models` (`site_hit_model`, Roots only; stock has no such call and installs nothing) chains that call and notes each model it sets against the shield type asked for; the shields then know a hit by its type. The `first live hit` line names the type noted.

### Changed
- hulls: the claim logs once how many of a SOD's shown nodes it found by name in the instance's database.
- testbench: `fo-render3d-shields` checks that the companion's pick is chained, that the first hit is noted as an ordinary one and that render3d draws the hits; `fo-render3d-stlogo` says why its log check cannot be written for Roots 5.0Z11 (no SOD has an `stlogo` node).
- testbench: `fo-display8` waits 25 s (not 5) after Display8 follows the game's device before its shot, which caught the loading screen; `fo-render3d-stlogo` judges the hulls textured, not a close-up the wheel does not always give.
