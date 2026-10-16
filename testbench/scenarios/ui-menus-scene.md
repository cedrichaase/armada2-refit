# The menu scene: the main menu and the campaign selection over render3d's map

Resolutions: 1280x720
Mod: refit
Timeout: 12 min

`ui/MENUS.md`, "The menu scene"; `render3d/README.md`, "The menu scene". Run it with
`--install .` (`./a2test run ui-menus-scene --install . -j 1`): the scene needs this
checkout's `Render3D.asi` (its `Render3D_MenuScene` export) and `UI.asi` together.

The main menu stands over render3d's map, its view `main`: a Federation fleet led by a
Galaxy class over a class M planet, under the column of entries at the left; nothing
of the stock screen's art. The campaign selection is a column too: each campaign its
emblem and its name, and pointing at one moves the camera to its scene on the same
map: the starbase (Tutorials), the fleet (Federation), Klingon ships decloaking before
a red Metreon nebula (Klingon), a Borg fusion cube casting its shadow through a purple
Mutara nebula (Borg). The column's places on the main menu are the old layout's, so
`Click at 228,258` still opens Single Player; the named targets click the UI's labels
(`ui.json`, `label`) while UI.log says the scene is on.

1. Launch the game.
2. Wait for the main menu.
3. Expect "Render3D.log" contains "menu: map (built in)".
4. Expect "UI.log" contains "menus: scene main on".
5. Wait 2 seconds.
6. Take a screenshot called "main menu".
7. Check that the screenshot shows several Starfleet ships (one large, the Galaxy class, near the centre) flying over a lit planet in the lower right against a dark blue starry sky, with STAR TREK ARMADA II and a column of menu entries (SINGLE PLAYER to EXIT) as plain text at the left, and no flat 2D menu art.
8. Click the single player emblem.
9. Wait 3 seconds.
10. Expect "UI.log" contains "menus: campaign emblems from allrace.tga".
11. Move the mouse to 220,270.
12. Wait 4 seconds.
13. Expect "Render3D.log" contains "menu: view tutorials".
14. Take a screenshot called "tutorials".
15. Check that a column at the left reads SINGLE PLAYER over four rows, each a white emblem and a name (Tutorials with a triangle, Federation Campaign, Klingon Campaign, Borg Campaign), then LOAD GAME and PREVIOUS MENU; Tutorials is lit with an amber bar; the scene shows a large round Federation starbase over a planet.
16. Move the mouse to 220,322.
17. Wait 4 seconds.
18. Expect "Render3D.log" contains "menu: view federation".
19. Take a screenshot called "federation".
20. Check that Federation Campaign is lit and the scene shows Starfleet ships over a planet.
21. Move the mouse to 220,374.
22. Wait 8 seconds.
23. Expect "Render3D.log" contains "menu: view klingon".
24. Expect "Render3D.log" contains "hulls: menu scene: kbattle".
25. Take a screenshot called "klingon".
26. Check that Klingon Campaign is lit and the scene shows several dark green Klingon warships (a large one near the centre, two Birds of Prey with their wings down) in front of a red, cloudy nebula.
27. Move the mouse to 220,426.
28. Wait 5 seconds.
29. Expect "Render3D.log" contains "menu: view borg".
30. Take a screenshot called "borg".
31. Check that Borg Campaign is lit and the scene shows a dark Borg cube floating in a purple, cloudy nebula, and to the right of and below the cube a darker region of gas bounded by a straight edge running from the cube down to the right (the cube's shadow).
32. Click the previous menu.
33. Wait 3 seconds.
34. Expect "SINGLE PLAYER" is visible.
