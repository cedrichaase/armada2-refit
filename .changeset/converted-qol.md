---
layers: [qol]
bump: minor
status: Confirmed in game 2026-11-06
---

### Added
- QOL-11, `RigIgnoresSelf=1` (a live row, Gameplay tab): a construction ship no longer blocks its own blueprint. `ConstructionRig::GetAction` asks the class's `CanPlaceHere` with the rig as the object to ignore instead of nothing, so the ghost stays green over the selected rig and the click sends the ordinary BUILD; own warships, allies, enemies, the grid and the map edge still block. Stock's arrival check already ignores the rig on every node, so a stock player in the same match is unaffected. The hook `qol.rig_self` (two patches in `GetAction`, signatures checked, standing down when they are not stock); `rig: the blueprint's spot is clear|blocked` in `QOL.log`. Off: stock. Issue #51, step 1 of #47.
