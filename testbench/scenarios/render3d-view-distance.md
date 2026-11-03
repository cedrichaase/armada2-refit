# render3d's View.Distance raises the far plane and the culling distance

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh showcase Seed=7 Step=16 Pause=200 Race=federation Hud=1
Timeout: 10 min

`render3d/README.md`, "View distance". The view's two limits (ART_CFG.h's far plane and
object culling distance) are raised at the mission's start to `[View] Distance` (default
the whole map, 1000000), by a patch of `RTS_Configure`'s load of the culling distance.
A value a player set as `QOL.ini`'s `ViewDistance` is carried over once by
`tools/settings/move-keys.py` at install (checked in the settings selftest, not here).
Checked on `Render3D.log`; the live change goes through the overlay.

1. Launch the game.
2. Wait up to 240 seconds for "Scene.log" to say "paused at tick".
3. Expect "Render3D.log" contains "view: Distance=1000000".
4. Expect "Render3D.log" contains "patched".
5. Expect "Render3D.log" contains "view: ART_CFG.h parsed: far plane now 1000000, object culling now 1000000".
6. Take a screenshot called "view-whole".
7. Overlay Render3D.View:Distance=20000
8. Wait up to 20 seconds for the logs the next steps expect.
9. Expect "Render3D.log" contains "settings: applied View.Distance=20000".
10. Overlay --unset Render3D.View:Distance
11. Wait up to 20 seconds for the logs the next steps expect.
12. Expect "Render3D.log" contains "settings: applied View.Distance=1000000".
13. Expect the game is still running.
14. Quit the game.
15. Expect no crash.
