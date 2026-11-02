# render3d settings change in a running game, at the same tick

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Clock: bench
Setup: testbench/scene/bench-setup.sh showcase Seed=7 Step=16 Pause=200 Race=federation Hud=1
Timeout: 12 min

`render3d/README.md`, "Modules and settings". The overlay keys of Render3D that are live
(`Modules.hulls`, `.sky`, `.specials`, `World.Surfaces`, ...) take effect on the paused
game without a restart. Each flip is `Overlay ...` (the hub logs the change), a short
wait for the frame, a screenshot, a pixel comparison with the main baselines.

1. Launch the game.
2. Wait up to 240 seconds for "Scene.log" to say "paused at tick".
3. Expect "Render3D.log" contains "clock: bench".
4. Wait 5 seconds.
5. Take a screenshot called "showcase".
6. Expect the shot "showcase" matches the baseline "roundC-main-port-showcase" within rmse 0.
7. Overlay Render3D.Modules:hulls=0
8. Wait up to 20 seconds for the logs the next steps expect.
9. Expect "Render3D.log" contains "settings: applied Modules.hulls=0"
10. Wait 3 seconds.
11. Take a screenshot called "showcase".
12. Expect the shot "showcase" matches the baseline "roundC-main-port-showcase-hulls0" within rmse 0.
13. Overlay --unset Render3D.Modules:hulls
14. Wait 3 seconds.
15. Take a screenshot called "showcase".
16. Expect the shot "showcase" matches the baseline "roundC-main-port-showcase" within rmse 0.
17. Overlay Render3D.World:Surfaces=0
18. Wait up to 20 seconds for the logs the next steps expect.
19. Expect "Render3D.log" contains "Surfaces=0"
20. Wait 3 seconds.
21. Take a screenshot called "showcase".
22. Expect the shot "showcase" matches the baseline "roundC-main-port-showcase-surfaces0" within rmse 0.
23. Overlay --unset Render3D.World:Surfaces
24. Wait 3 seconds.
25. Take a screenshot called "showcase".
26. Expect the shot "showcase" matches the baseline "roundC-main-port-showcase" within rmse 0.
27. Overlay Render3D.Modules:sky=0
28. Wait up to 20 seconds for the logs the next steps expect.
29. Expect "Render3D.log" contains "settings: applied Modules.sky=0"
30. Wait 3 seconds.
31. Take a screenshot called "showcase".
32. Expect the shot "showcase" matches the baseline "roundC-main-port-showcase-sky0" within rmse 0.
33. Overlay --unset Render3D.Modules:sky
34. Wait 3 seconds.
35. Take a screenshot called "showcase".
36. Expect the shot "showcase" matches the baseline "roundC-main-port-showcase" within rmse 0.
37. Expect no crash.
