# The HUD follows a display mode changed in the middle of a mission

Resolutions: 4:3, 21:9
Mod: remastered
Reference: 4:3
Timeout: 14 min

Regression test for hud 2.2.0. Before it, `HUD.asi` fitted the layout canvas to the
display mode once, when the GUI config loaded at the start of a mission. A mode chosen
in Graphics Settings during a mission kept the old canvas until the next one: from
21:9 to 1600x1200 the game drew 4:3, pillarboxed, with every HUD panel squeezed to
1600/2867 = 0.56 of its width.

Both cases switch to 1600x1200x32 in the Federation mission's Graphics Settings. For
the 4:3 case, the reference, that keeps the same resolution. The 21:9 case lands in the
same mode, so after the switch its HUD, briefing and font must measure as the reference
does: 1.00, where the squeezed HUD reads ~0.56. Regions are in the reference's
1600x1200 pixels: the resource bar, the minimap panel and the command bar (`hud.md`'s,
doubled). Before the switch, the same regions check that the case started right.

The briefing stays open through the switch. The re-layout rebuilds the objectives
display, and its text coming back ("Invasion", the headings) is part of the claim; the
first version brought it back empty. So is the minimap, judged once the briefing is
closed: the re-layout rebuilds its panel but keeps the sensor grids, so the explored
area around the fleet still shows. That step is weak: with the camera over the fog, the
minimap shows little explored area in either frame, so both can read as uniform grey;
the grids are kept by construction (`hud/README.md`). After the switch the 21:9 case
draws into a pillarboxed 1920x1440, where the bench's HUD coordinates no longer hold,
so nothing after it clicks the HUD: keys and OCR only.

1. Include "_enter-federation-mission".
2. Move the mouse to hud 60,400.
3. Wait 1 second.
4. Take a screenshot called "before".
5. Check that the hud is not stretched compared with 4:3 in region 0,0,300,46.
6. Check that the hud is not stretched compared with 4:3 in region 0,830,160,370.
7. Check that the hud is not stretched compared with 4:3 in region 1320,0,280,56.
8. Press Escape.
9. Wait for the options menu.
10. Click "Graphics Settings".
11. Wait for "Display Mode".
12. Click the display mode box.
13. Wait 2 seconds.
14. Click "1600x1200x32".
15. Wait 3 seconds.
16. Click "Previous Menu".
17. Wait for the options menu.
18. Press Escape.
19. Wait 5 seconds.
20. Wait for the briefing.
21. Take a screenshot called "after".
22. Expect the game is still running.
23. Check that the hud is not stretched compared with 4:3 in region 0,0,300,46.
24. Check that the hud is not stretched compared with 4:3 in region 0,830,160,370.
25. Check that the hud is not stretched compared with 4:3 in region 1320,0,280,56.
26. Check that "BRIEFING SUMMARY" is not stretched compared with 4:3 within 10%.
27. Check that "OBJECTIVES" is not stretched compared with 4:3 within 10%.
28. Expect "Invasion" is visible.
29. Click "OK".
30. Wait 5 seconds.
31. Take a screenshot called "closed".
32. Expect the game is still running.
33. Check that the minimap in the bottom-left panel shows the same explored area around the fleet as in the reference, rather than being uniformly grey.
34. Quit the game.
35. Expect no crash.
