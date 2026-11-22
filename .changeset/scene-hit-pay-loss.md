---
layers: [testbench]
bump: minor
games: [armada2]
---

### Added
- Scene commands `damage <name> <amount> [<attacker>]` (a hit on a hull through Craft::DamageHull, so the engine destroys it its own way) and `team <name> <team>` (ScriptInterfaceImp::SetRealTeam, what a capture does to a producer), hook `scene.hit` of the gog-1.1 scene profile; scenario `qol-pay-on-queue-loss` (a yard destroyed, handed to another team, and saved and loaded, each time checking the refund happens once). Ported from the unmerged bench-pay-on-queue-loss branch, adapted to the rules layer; not yet run.
