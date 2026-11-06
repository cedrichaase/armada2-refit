---
layers: [render3d]
bump: patch
---

### Fixed
- Hulls on Fleet Operations: Roots: the instance claim no longer takes a `lod*` level's hide flag from the database (the engine's own level switch) and keeps the module's own level of detail; it hid the whole Enterprise-E in Roots' first allied tutorial. Render3D.log names each node the database hides (`hulls: <model> node <name> hidden in the instance's database`).
