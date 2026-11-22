---
layers: [testbench, root]
bump: minor
games: [armada2, fo-roots]
---

### Added
- Scene.asi on the symbol profiles (profiles/gog-1.1/scene.toml): each part stands down on its own and a command refuses with `! scene.<hook> stood down`; objects by handle (`#<id>`) in every command; `objects` writes Scene.objects.jsonl; `select+`/`add`, `camera on <ref>`, `move ... [to]`, `world x y z` points; scenes/game.ini; install and bench-setup.sh put Scene.asi into the executable's folder
- `a2test drive objects [FILTER...] [--json]` and the scenario steps on objects (`Expect an object of class "X" owned by player N`, `Wait up to N seconds until …`, `Select …`, `Add … to the selection`, `Center the camera on …`, `Order … to move to X,Y,Z`, `Order … to attack …`), read from Scene.objects.jsonl (bench/objects.py, objects_selftest.py)
