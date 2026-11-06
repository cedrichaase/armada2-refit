---
root: minor
display: patch
render3d: patch
ui: patch
qol: patch
rules: patch
online: patch
cutscenes: patch
status: Untested: Fleet Operations: Roots is not offered; stock 1.1 resolves as before (103/103 hooks)
---

### Added
- Fleet Operations: Roots 5.0Z11 is a known build: `profiles/fo-roots-5.0Z11/` inherits gog-1.1, keyed by timestamp, image size 0x405000 and section fingerprint, and marks the functions Roots changes that our hooks use; their 18 hooks stand down under Roots until verified, the rules plugin is off there, and no content set is given yet. Every plugin's generated tables carry the build; `tools/engine/negative.sh` gains the Roots case (e), both ways
