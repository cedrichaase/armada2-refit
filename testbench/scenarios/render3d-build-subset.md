# render3d: a build without a module draws as that module off at load

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Clock: bench
Setup: render3d/bench-asi.sh --without hulls showcase Seed=7 Step=16 Pause=200 Race=federation Hud=1
Timeout: 16 min

`render3d/README.md`, "Crates". Each module of render3d is a crate of its own and a
Cargo feature of the DLL (`render3d/build.sh --features`); `render3d/bench-asi.sh
--without hulls` builds this working copy's Render3D.asi without the hulls module and
puts it into the clone (run with `--install .`). A module left out is not in the DLL at
all, so in a mission the frame must be the one the full build draws with `[Modules]
hulls=0` at load (`roundD-base-port-showcase-hulls0`, RMSE 0), and `Render3D_Settings`
has no row of it: the core's 11 rows less its `[Modules]` row (10), then world's 5 and
overlays' 7, 22 in all (23 in the full build).

The other modules are checked by this scenario's variants, from
`render3d/subset-scenario.sh <module>` (`world`, `particles`; `ordnance` on the firing
scene). The front end is not checked: without hulls, sky or world there are no ships,
sky or planet in the menu scene, since nothing else draws them (README.md, "Crates").

1. Launch the game.
2. Wait up to 240 seconds for "Scene.log" to say "paused at tick".
3. Expect "Render3D.log" contains "clock: bench".
4. Expect "Render3D.log" contains "modules not built in: hulls".
5. Expect "Render3D.log" contains "settings: 22 rows exported (10 the core's)".
6. Expect "Render3D.log" not contains "module hulls:".
7. Wait 5 seconds.
8. Take a screenshot called "showcase".
9. Expect the shot "showcase" matches the baseline "roundD-base-port-showcase-hulls0" within rmse 0.
10. Expect no crash.
