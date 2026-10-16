# The menu scene comes back after a mission is aborted

Resolutions: 1280x720
Mod: refit
Timeout: 12 min

`ui/MENUS.md`, "The menu scene"; `ui/src/menus/backdrop.rs`. Run it with `--install .`
(`./a2test run ui-menus-scene-abort --install . -j 1`). Aborting a mission goes Options,
the admiral's log, the campaign selection, each opened within a moment of the last
closing; the picture of the mission the in-mission menu stood over was handed from each
to the next, so the campaign selection still counted as over a mission and drew without
the scene, its stock look. The Borg campaign is not pointed at before the mission, so
render3d's `menu: view borg` can only come from the campaign selection after it.

1. Launch the game.
2. Wait for the main menu.
3. Expect "UI.log" contains "menus: scene main on".
4. Click the single player emblem.
5. Wait 3 seconds.
6. Click "Federation Campaign".
7. Wait 3 seconds.
8. Click "Invasion".
9. Wait 1 second.
10. Click "O.K.".
11. Wait up to 300 seconds for the briefing.
12. Wait 3 seconds.
13. Click "OK".
14. Wait 3 seconds.
15. Press Escape.
16. Wait 3 seconds.
17. Expect "UI.log" contains "over the mission's last frame".
18. Click "Abort Mission".
19. Wait 2 seconds.
20. Click "Yes".
21. Wait 5 seconds.
22. Take a screenshot called "after the abort".
23. Click "Done".
24. Wait 4 seconds.
25. Move the mouse to 220,426.
26. Wait 5 seconds.
27. Expect "Render3D.log" contains "menu: view borg".
28. Take a screenshot called "campaign selection after the abort".
29. Check that the screenshot shows a column at the left under SINGLE PLAYER of four campaigns each with a white emblem (Borg Campaign lit), then LOAD GAME and PREVIOUS MENU, over a 3D scene of a dark Borg cube in a purple nebula, and not four picture cards in a 2x2 grid.
