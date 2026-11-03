# render3d: world and overlays switched live, the frame the same as off at load

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Clock: bench
Setup: testbench/scene/bench-setup.sh showcase Seed=7 Step=16 Pause=200 Race=federation Hud=1
Timeout: 18 min

`render3d/README.md`, "Modules and settings" and "World" ("Hooks"). The package W-B of
phase 7 (K9, "Live parity"): on the paused showcase, `Modules.world` and then
`Modules.overlays` are flipped off through the overlay and the frame is compared with
the same key set at load (`roundD-base-port-showcase-world0`, `-overlays0`, recorded on
the base), then flipped back and compared with the base. The world goes twice, so a
state the first flip left behind (a baked surface, a registered light, the ghosts'
hook) would show. The rows `World.Planets`, `Rocks`, `Nebulae` and `Overlays.Fog`,
`Grid`, `Selection`, `Ghosts` are flipped once each and back, to the base. The nebula
scene is `render3d-live-world-nebula`.

1. Launch the game.
2. Wait up to 240 seconds for "Scene.log" to say "paused at tick".
3. Expect "Render3D.log" contains "clock: bench".
4. Wait 5 seconds.
5. Take a screenshot called "showcase".
6. Expect the shot "showcase" matches the baseline "roundD-base-port-showcase" within rmse 0.
7. Overlay Render3D.Modules:world=0
8. Wait up to 20 seconds for the logs the next steps expect.
9. Expect "Render3D.log" contains "settings: applied Modules.world=0"
10. Wait 3 seconds.
11. Take a screenshot called "showcase".
12. Expect the shot "showcase" matches the baseline "roundD-base-port-showcase-world0" within rmse 0.
13. Overlay --unset Render3D.Modules:world
14. Wait 3 seconds.
15. Take a screenshot called "showcase".
16. Expect the shot "showcase" matches the baseline "roundD-base-port-showcase" within rmse 0.
17. Overlay Render3D.Modules:overlays=0
18. Wait up to 20 seconds for the logs the next steps expect.
19. Expect "Render3D.log" contains "settings: applied Modules.overlays=0"
20. Wait 3 seconds.
21. Take a screenshot called "showcase".
22. Expect the shot "showcase" matches the baseline "roundD-base-port-showcase-overlays0" within rmse 0.
23. Overlay --unset Render3D.Modules:overlays
24. Wait 3 seconds.
25. Take a screenshot called "showcase".
26. Expect the shot "showcase" matches the baseline "roundD-base-port-showcase" within rmse 0.
27. Overlay Render3D.Modules:world=0
28. Wait up to 20 seconds for the logs the next steps expect.
29. Expect "Render3D.log" contains "settings: applied Modules.world=0"
30. Wait 3 seconds.
31. Take a screenshot called "showcase".
32. Expect the shot "showcase" matches the baseline "roundD-base-port-showcase-world0" within rmse 0.
33. Overlay --unset Render3D.Modules:world
34. Wait 3 seconds.
35. Take a screenshot called "showcase".
36. Expect the shot "showcase" matches the baseline "roundD-base-port-showcase" within rmse 0.
37. Overlay Render3D.World:Planets=0
38. Wait up to 20 seconds for the logs the next steps expect.
39. Expect "Render3D.log" contains "World.Planets=0"
40. Wait 3 seconds.
41. Overlay --unset Render3D.World:Planets
42. Wait 3 seconds.
43. Take a screenshot called "showcase".
44. Expect the shot "showcase" matches the baseline "roundD-base-port-showcase" within rmse 0.
45. Overlay Render3D.World:Rocks=0
46. Wait up to 20 seconds for the logs the next steps expect.
47. Expect "Render3D.log" contains "World.Rocks=0"
48. Wait 3 seconds.
49. Overlay --unset Render3D.World:Rocks
50. Wait 3 seconds.
51. Take a screenshot called "showcase".
52. Expect the shot "showcase" matches the baseline "roundD-base-port-showcase" within rmse 0.
53. Overlay Render3D.World:Nebulae=0
54. Wait up to 20 seconds for the logs the next steps expect.
55. Expect "Render3D.log" contains "World.Nebulae=0"
56. Wait 3 seconds.
57. Overlay --unset Render3D.World:Nebulae
58. Wait 3 seconds.
59. Take a screenshot called "showcase".
60. Expect the shot "showcase" matches the baseline "roundD-base-port-showcase" within rmse 0.
61. Overlay Render3D.Overlays:Fog=0 Render3D.Overlays:Grid=0 Render3D.Overlays:Selection=0 Render3D.Overlays:Ghosts=0
62. Wait up to 20 seconds for the logs the next steps expect.
63. Expect "Render3D.log" contains "Overlays.Selection=0"
64. Wait 3 seconds.
65. Overlay --unset Render3D.Overlays:Fog Render3D.Overlays:Grid Render3D.Overlays:Selection Render3D.Overlays:Ghosts
66. Wait 3 seconds.
67. Take a screenshot called "showcase".
68. Expect the shot "showcase" matches the baseline "roundD-base-port-showcase" within rmse 0.
69. Expect no crash.
