---
root: minor
display: patch
render3d: patch
ui: patch
qol: patch
rules: patch
online: patch
cutscenes: patch
testbench: patch
status: Installed, not yet seen in game
---

### Added
- Build identity beyond the header (FO-5 step 1): a build may set `[pe] file_crc`, the CRC-32 of named sections (`.text`, `.test`) as the file on disk has them, and name a companion module by `[[module]]` (`FleetOpsHook.dll`, the CRC-32 of its `.text` in its file). A build that sets the CRCs is matched only when they match; with another companion loaded, none, or an unreadable one, every hook that depends on its run-time patches (a `runtime` mark, a chained site, a dropped callee check) stands down, `is patched by FleetOpsHook.dll`, whatever `verified` says. In `a2engine`, the generator, the C twin (`csdk/a2engine.h`), the Python resolver and `a2test syms`; stock 1.1 sets neither and resolves as before. The plugins log `file crc` and `companion` lines only on a build that sets them
- Roots 5.0Z11 sets `file_crc` (`.text`, `.test`) and `FleetOpsHook.dll`; `fo-scan.py` writes both, keeps `verified` from its config, and `--compare` checks them. `negative.sh` gains Roots cases (f) a foreign, absent or unreadable FleetOpsHook.dll and (g) a changed `.text` or `.test` byte
- Roots profile entries for render3d (FO-5 step 2): `render3d.frame.obj_draw` (the call at 0x598297 FleetOpsHook writes in the object loop; Roots only), `render3d.hulls.instance_draw` (the call of `ST3D_Instance::RenderInternal` in `GameObjectInstance::RenderInternal`), and the callee hooks `render3d.effects.debris_callee` (`ST3D_Instance::Fragment`) and `render3d.overlays.posts_callee` (`DrawHeightPost`); a hook may say `planned = "<step>"` until the code that reads it exists. No render3d code reads them yet

### Changed
- `Armada_RenderAllOurStuff` is verified for `render3d.frame.*` on Roots 5.0Z11
- `fo-scan.py`'s section CRC-32s are of the content (`min(VirtualSize, SizeOfRawData)` bytes), the definition the profile uses
