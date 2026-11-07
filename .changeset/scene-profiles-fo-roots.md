---
layers: [testbench, root]
bump: minor
games: [fo-roots]
---

### Added
- Scene.asi on the symbol profiles (profiles/gog-1.1/scene.toml, Roots' marks in fo-roots-5.0Z11): each part stands down on its own and a command refuses with `! scene.<hook> stood down`; objects by handle (`#<id>`) in every command; `objects` writes Scene.objects.jsonl; `select+`/`add`, `camera on <ref>`, `move ... [to]`, `world x y z` points; scenes/game.ini; install and bench-setup.sh put Scene.asi into the executable's folder (Data/ on Roots)
- `a2test drive objects [FILTER...] [--json]` and the scenario steps on objects (`Expect an object of class "X" owned by player N`, `Wait up to N seconds until …`, `Select …`, `Add … to the selection`, `Center the camera on …`, `Order … to move to X,Y,Z`, `Order … to attack …`), read from Scene.objects.jsonl (bench/objects.py, objects_selftest.py)
- Roots straight into a map: `Launch: -nointro tutorialgameplay.bzn` (a mission by its .bzn), `Launch: -nointro save\<name>.sav` with `Setup: testbench/bench/roots-save.sh <name>` (an Instant Action save from $A2_DATA); launch arguments keep a double-quoted run as one word
