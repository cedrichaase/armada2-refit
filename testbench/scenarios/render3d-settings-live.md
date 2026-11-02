# render3d settings change in a running game, at the same tick

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Clock: bench
Setup: testbench/scene/bench-setup.sh showcase Seed=7 Step=16 Pause=200 Race=federation Hud=1
Timeout: 12 min

`render3d/README.md`, "Modules and settings". The overlay keys of Render3D that are live
(`Modules.hulls`, `.sky`, `.specials`, `World.Surfaces`, `Nebulae.Lit`, `Overlays.*`) take
effect on the paused game without a restart. It needs a driver (`a2test drive overlay`
has no scenario step yet): start `./a2test session start --res 1280x720 --clock bench
--install . --no-launch`, run the `Setup:` script with the session's state file, `drive
launch -nointro a2_borg01`, and wait for "paused at tick" in Scene.log, then:

1. Take a screenshot called "showcase" (all on). Expect RMSE 0 against `roundC-main-port-showcase`.
2. `drive overlay Render3D.Modules:hulls=0`, wait 4 seconds, screenshot "showcase". RMSE 0 against `roundC-main-port-showcase-hulls0`.
3. `drive overlay --unset Render3D.Modules:hulls`, screenshot. RMSE 0 against `roundC-main-port-showcase`.
4. `drive overlay Render3D.World:Surfaces=0`, screenshot. RMSE 0 against `roundC-main-port-showcase-surfaces0`.
5. `drive overlay --unset Render3D.World:Surfaces`, screenshot. RMSE 0 against `roundC-main-port-showcase`.
6. `drive overlay Render3D.Modules:sky=0`, screenshot. RMSE 0 against `roundC-main-port-showcase-sky0`.
7. Expect Render3D.log to contain `settings: RefitPlayer.ini [Render3D.Modules] hulls=0 overrides` and `settings: applied Modules.hulls=0`.

Compare with `./a2test baseline compare <baseline> --shot showcase --from <shot.png>`.
