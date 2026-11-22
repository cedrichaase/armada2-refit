# Fleet Operations: Roots without Render3D: the ice asteroid field of the first allied tutorial, the engine's own reference

Resolutions: 1280x960
Mod: refit
Layers: display asiloader
Setup: display/bench-mode.sh stub Backend=vulkan
Timeout: 12 min

The reference for `fo-render3d-asteroids`: the same view of Roots' first allied tutorial
with Display8 alone, so the engine draws everything through the passthrough and no
render3d frame is involved (no tone map, no bloom). The field shot is compared with
`fo-render3d-asteroids`' "field" and "field-engine" by eye and by the statistics in
PLAN-FLEETOPS.md (decision 354).

1. Launch the game.
2. Wait for the main menu.
3. Move the mouse to 640,393.
4. Wait 1 second.
5. Click "Single Player".
6. Wait 4 seconds.
7. Move the mouse to 235,225.
8. Wait 1 second.
9. Click at 237,225.
10. Wait up to 20 seconds for "TUTORIAL: Gameplay".
11. Wait 2 seconds.
12. Move the mouse to 640,233.
13. Wait 1 second.
14. Click "TUTORIAL: Gameplay".
15. Wait 1 second.
16. Double-click "TUTORIAL: Gameplay".
17. Wait 2 seconds.
18. Move the mouse to 540,780.
19. Wait 1 second.
20. Click "OK".
21. Wait up to 240 seconds for "BRIEFING SUMMARY".
22. Wait 2 seconds.
23. Click at 1235,912.
24. Move the mouse to 640,400.
25. Wait 3 seconds.
26. Take a screenshot called "field".
27. Press ctrl+Up 8 times.
28. Wait 3 seconds.
29. Take a screenshot called "zoom-in".
30. Press ctrl+Down 20 times.
31. Wait 3 seconds.
32. Take a screenshot called "zoom-out".
33. Expect "Display8.log" not contain "VUID".
34. Expect "Display8.log" not contain "device removed".
35. Quit the game.
36. Expect the game to have exited.
37. Expect no crash.
