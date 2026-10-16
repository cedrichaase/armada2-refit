# The menu scene: a fleet in orbit behind the main menu and the campaign selection

Resolutions: 1280x720
Mod: refit
Timeout: 10 min

`ui/MENUS.md`, "The menu scene"; `render3d/README.md`, "The menu scene". Run it with
`--install .` (`./a2test run ui-menus-scene --install . -j 1`): the scene needs this
checkout's `Render3D.asi` (its `Render3D_MenuScene` export) and `UI.asi` together.

The main menu stands over render3d's 3D scene: a Federation fleet led by a Galaxy class,
front three-quarter, over a class M planet, under the column of entries at the left;
nothing of the stock screen's art. The campaign selection has the same scene seen from
further back, its four cards in a row along the foot. The column's places are the old
layout's, so `Click at 228,258` still opens Single Player; the named targets click the
UI's labels (`ui.json`, `label`) while UI.log says the scene is on.

1. Launch the game.
2. Wait for the main menu.
3. Expect "Render3D.log" contains "ships, 1400 stars".
4. Expect "UI.log" contains "menus: scene main on".
5. Wait 2 seconds.
6. Take a screenshot called "main menu".
7. Check that the screenshot shows several Starfleet ships (one large, the Galaxy class, near the centre) flying over a lit planet in the lower right against a dark blue starry sky, with STAR TREK ARMADA II and a column of menu entries (SINGLE PLAYER to EXIT) as plain text at the left, and no flat 2D menu art.
8. Move the mouse to 200,302.
9. Wait 1 second.
10. Take a screenshot called "hover".
11. Check that INSTANT ACTION is lit, with an amber bar at its left and a glow behind it.
12. Click the single player emblem.
13. Wait 3 seconds.
14. Expect "UI.log" contains "menus: scene single on".
15. Expect "Federation Campaign" is visible.
16. Take a screenshot called "campaign selection".
17. Check that four campaign cards (Tutorials, Federation, Klingon, Borg), each with its stock picture, stand in a row across the lower part of the screen over the same 3D scene of a fleet and a planet, with Previous Menu at the bottom left and Load Game at the bottom right.
18. Click the previous menu.
19. Wait 3 seconds.
20. Expect "SINGLE PLAYER" is visible.
