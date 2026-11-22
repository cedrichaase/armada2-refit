# Fleet Operations: Roots as it draws itself: the windows of the in-mission menu (a reference)

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d
Setup: display/bench-mode.sh stub Backend=vulkan
Timeout: 10 min

The reference for `fo-ui-ingame-window` (PLAN-FLEETOPS.md, decisions 310 to 313): Roots with
Display8 and Render3D and no UI, so its forms are its own windows. The first allied tutorial
(as `fo-render3d-tutorial` reaches it), Esc opens Roots' Options, Esc again goes back. The
window manager's events are recorded across both: Roots' Options is a window of its own (a
floating dialog), so the window manager sees another window, and the check's detail counts how
often FleetOpsHook replaced its device window, which happens with or without the UI.

1. Launch the game.
2. Wait for the main menu.
3. Move the mouse to 640,393.
4. Wait 3 seconds.
5. Click at 640,393.
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
20. Take a screenshot called "mission selected".
21. Click "OK".
22. Wait up to 180 seconds for "Render3D.log" to say "fed_sovereignent at".
23. Wait 8 seconds.
24. Wait 3 seconds.
25. Watch the window manager.
26. Move the mouse to 700,300.
27. Press Escape.
28. Wait 6 seconds.
29. Take a screenshot called "options".
30. Press Escape.
31. Wait 8 seconds.
32. Take a screenshot called "mission".
33. Expect the window manager to have seen another window.
34. Expect "Display8.log" not contain "device removed".
35. Stop the game.
36. Expect no crash.
