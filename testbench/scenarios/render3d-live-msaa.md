# render3d: MSAA and HDR change in a running game, at the same tick

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Clock: bench
Setup: testbench/scene/bench-setup.sh showcase Seed=7 Step=16 Pause=200 Race=federation Hud=1
Timeout: 14 min

`render3d/README.md`, "How the frame works". `MSAA` and `HDR` are live since render3d
0.42.0: `Render3D_SettingsChanged` swaps the configuration, the scene target and the post
passes are made again at the next view, and every module answers `formats_changed`. On
the paused showcase each flip must equal the baseline recorded with the same key at
load (`-msaa0`, `-msaa8`, `-hdr0`), and flipping back must equal the base again. Each
flip is `Overlay ...`, a short wait for the frame, a screenshot and a pixel comparison.

1. Launch the game.
2. Wait up to 240 seconds for "Scene.log" to say "paused at tick".
3. Expect "Render3D.log" contains "clock: bench"
4. Wait 5 seconds.
5. Take a screenshot called "showcase".
6. Expect the shot "showcase" matches the baseline "roundD-base-port-showcase" within rmse 0.
7. Overlay Render3D.Render3D:MSAA=0
8. Wait up to 20 seconds for the logs the next steps expect.
9. Expect "Render3D.log" contains "settings: applied Render3D.MSAA=0"
10. Wait 3 seconds.
11. Take a screenshot called "showcase".
12. Expect the shot "showcase" matches the baseline "roundD-base-port-showcase-msaa0" within rmse 0.
13. Overlay Render3D.Render3D:MSAA=8
14. Wait up to 20 seconds for the logs the next steps expect.
15. Expect "Render3D.log" contains "Render3D.MSAA=8"
16. Wait 3 seconds.
17. Take a screenshot called "showcase".
18. Expect the shot "showcase" matches the baseline "roundD-base-port-showcase-msaa8" within rmse 0.
19. Overlay Render3D.Render3D:MSAA=4
20. Wait up to 20 seconds for the logs the next steps expect.
21. Expect "Render3D.log" contains "Render3D.MSAA=4"
22. Wait 3 seconds.
23. Take a screenshot called "showcase".
24. Expect the shot "showcase" matches the baseline "roundD-base-port-showcase" within rmse 0.
25. Overlay Render3D.Render3D:HDR=0
26. Wait up to 20 seconds for the logs the next steps expect.
27. Expect "Render3D.log" contains "Render3D.HDR=0"
28. Wait 3 seconds.
29. Take a screenshot called "showcase".
30. Expect the shot "showcase" matches the baseline "roundD-base-port-showcase-hdr0" within rmse 0.
31. Overlay Render3D.Render3D:HDR=1
32. Wait up to 20 seconds for the logs the next steps expect.
33. Expect "Render3D.log" contains "Render3D.HDR=1"
34. Wait 3 seconds.
35. Take a screenshot called "showcase".
36. Expect the shot "showcase" matches the baseline "roundD-base-port-showcase" within rmse 0.
37. Expect no crash.
