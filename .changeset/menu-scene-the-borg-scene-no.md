---
layers: [render3d]
bump: minor
games: [armada2]
---

### Changed
- Menu scene: the Borg scene no longer shows the assimilated Intrepid, Vor'cha and Galaxy (Borg hulls only); a decloak (and its sound) waits for CloakDwell (0.5 s) of the view being asked for, so a pointer passing over the Klingon entry plays nothing, and a view left before it decloaked stays cloaked instead of popping whole with the sound
