# render3d: hulls and specials switched live, the frame the same as off at load

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Clock: bench
Setup: testbench/scene/bench-setup.sh showcase Seed=7 Step=16 Pause=200 Race=federation Hud=1
Timeout: 14 min

`render3d/README.md`, "Modules and settings" and "hulls" ("Hooks"). The package W-A of
phase 7 (K9, "Live parity"): on the paused showcase, `Modules.hulls` and then
`Modules.specials` are flipped off through the overlay and the frame is compared with
the same key set at load (`roundD-base-port-showcase-hulls0`, `-specials0`, recorded on
the base), then flipped back and compared with the base. The hulls go twice, so a state
the first flip left behind (a posed hull, the shadow map lent to the gas) would show.

1. Launch the game.
2. Wait up to 240 seconds for "Scene.log" to say "paused at tick".
3. Expect "Render3D.log" contains "clock: bench".
4. Wait 5 seconds.
5. Take a screenshot called "showcase".
6. Expect the shot "showcase" matches the baseline "roundD-base-port-showcase" within rmse 0.
7. Overlay Render3D.Modules:hulls=0
8. Wait up to 20 seconds for the logs the next steps expect.
9. Expect "Render3D.log" contains "settings: applied Modules.hulls=0"
10. Wait 3 seconds.
11. Take a screenshot called "showcase".
12. Expect the shot "showcase" matches the baseline "roundD-base-port-showcase-hulls0" within rmse 0.
13. Overlay --unset Render3D.Modules:hulls
14. Wait 3 seconds.
15. Take a screenshot called "showcase".
16. Expect the shot "showcase" matches the baseline "roundD-base-port-showcase" within rmse 0.
17. Overlay Render3D.Modules:specials=0
18. Wait up to 20 seconds for the logs the next steps expect.
19. Expect "Render3D.log" contains "settings: applied Modules.specials=0"
20. Wait 3 seconds.
21. Take a screenshot called "showcase".
22. Expect the shot "showcase" matches the baseline "roundD-base-port-showcase-specials0" within rmse 0.
23. Overlay --unset Render3D.Modules:specials
24. Wait 3 seconds.
25. Take a screenshot called "showcase".
26. Expect the shot "showcase" matches the baseline "roundD-base-port-showcase" within rmse 0.
27. Overlay Render3D.Modules:hulls=0
28. Wait up to 20 seconds for the logs the next steps expect.
29. Expect "Render3D.log" contains "settings: applied Modules.hulls=0"
30. Wait 3 seconds.
31. Take a screenshot called "showcase".
32. Expect the shot "showcase" matches the baseline "roundD-base-port-showcase-hulls0" within rmse 0.
33. Overlay --unset Render3D.Modules:hulls
34. Wait 3 seconds.
35. Take a screenshot called "showcase".
36. Expect the shot "showcase" matches the baseline "roundD-base-port-showcase" within rmse 0.
37. Expect no crash.
