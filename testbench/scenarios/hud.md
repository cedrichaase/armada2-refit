# The in-mission HUD and font keep their shape at every aspect ratio

Resolutions: 4:3, 16:10, 16:9, 21:9
Mod: remastered
Reference: 800x600 stock
Stock shell: embed
Timeout: 12 min

Measured at the first Federation mission's briefing, where the faction (and so the HUD
art) is always the same, reopened with the camera over unexplored space
(`_enter-federation-mission.md`). The HUD panels stay visible around the briefing, the
briefing is in the in-game font, and behind both is the flat grey of the fog, where a
line along a sprite's edge shows plainly. The Federation HUD is the plainest of the four
for the same reason. (Until testbench 1.7.0 this ran at the Borg briefing, in front of
space: the lines hud 2.1.0 fixes were there and could hardly be seen.)

`HUD.asi` re-declares the layout canvas and condenses the font at run time, for the
mode the game is in. Every remastered case runs the same install (`hud/install.sh`,
which takes no resolution), so a pass at every aspect is the claim being tested.

The baseline is **stock at 800x600**, the resolution the game was designed around:
the front end runs in its own 800x600 mode, so nothing is stretched and clicks map 1:1.
It is not the remastered 4:3 case, so that case is measured too, and any change the mod
makes at 4:3 counts against it like a stretch at 21:9 would. Regions are in reference
pixels (800x600): the start of the resource bar, the inside of the left (minimap) panel,
and the command bar at the top right (the right panel is mostly the ship portrait, which
moves, so it is not used). The template match finds each one wherever it sits at the other
resolutions, scaled by H/600. The pointer is parked in the fog before the shot: it is
left on the command bar's checkmark, whose hover highlight would spoil that template.

The cursor is measured on its own: the pointer is parked at HUD point 60,400 (30,200
in the stock 800x600 frame) and the region around it is matched like a HUD panel. The
game draws this cursor itself (the synchronous path, under DXVK), so it is in the
screenshot; stock at 21:9 draws it ~1.79x wide. Until testbench 1.6.1 the pointer never
got there (`README.md`, "Traps"): the check matched empty background and meant nothing.
It is allowed ±10%: the stock arrow is 19x23 px at 800x600, so one pixel is ~5%, and the
first honest run measured 0.94–1.02 where the arrows themselves, measured directly, keep
one width-to-height ratio to 0.6% (0.782–0.787 at 4:3, 16:10, 16:9 and 21:9). An
uncorrected cursor is 1.20x wide at 16:10, 1.33x at 16:9 and 1.79x at 21:9.

Then the action bar: the command buttons shown for a selected unit are sprites of their
own (no flag 0x80, so the engine never snapped them), and they had the same lines until
hud 2.1.0 snapped every 2D sprite. The briefing is closed, the minimap is clicked at the
fleet's corner (a left-anchored HUD point, so the same map point at every aspect), a box
is dragged over most of the 3D view to select whatever of the fleet is in it — a fixed
click on one ship missed, since the camera does not land the same way twice — and the
minimap is clicked into the fog again; the selection stays.

The thin-line checks are judged, and they are the weakest ones here: the lines hud 2.1.0 removed
were one pixel wide and ~7 levels off the fog, and the judge reads the shot at its own
size. The measurement behind that fix is in `hud/README.md`, "Seams between tiles".

The font is allowed ±10% where the HUD gets ±5%. The condense (1.25·H/W) aims
for the glyphs' own 1280x1024 shape, which is 0.9375 of stock's width at 4:3, and it
measured 0.91–0.94 at every aspect ratio. The user judged it fine in game
(2026-09-26). ±10% still catches the font drawn stretched (stock at 21:9 is ~1.79x).

1. Include "_enter-federation-mission".
2. Move the mouse to hud 60,400.
3. Wait 1 second.
4. Take a screenshot called "briefing".
5. Check that the hud is not stretched compared with stock 800x600 in region 0,0,150,23.
6. Check that the hud is not stretched compared with stock 800x600 in region 0,415,80,185.
7. Check that the hud is not stretched compared with stock 800x600 in region 660,0,140,28.
8. Check that "BRIEFING SUMMARY" is not stretched compared with stock 800x600 within 10%.
9. Check that "OBJECTIVES" is not stretched compared with stock 800x600 within 10%.
10. Check that "Invasion" is not stretched compared with stock 800x600 within 10%.
11. Check that the HUD panels and the briefing keep the same proportions as in the stock 800x600 reference: round and square elements stay round and square, and nothing looks horizontally stretched or squeezed compared with it.
12. Check that no thin lines are visible against the flat grey background: none along the outer edges of the HUD panels, the minimap frame or the briefing panel, and none running through the briefing panel where its tiles meet. The white trapezoid inside the minimap is the camera's view outline and belongs there.
13. Click "OK".
14. Wait 2 seconds.
15. Click at hud 25,1130.
16. Wait 2 seconds.
17. Drag from hud 250,60 to hud-right 1500,800.
18. Wait 2 seconds.
19. Click the minimap fog.
20. Wait 2 seconds.
21. Move the mouse to hud 60,400.
22. Wait 2 seconds.
23. Take a screenshot called "action bar".
24. Check that a row of square command buttons is shown above the unit panel at the bottom, and that no thin lines are visible against the flat grey background between those buttons, along their sides or under them.
25. Take a screenshot called "cursor".
26. Check that the hud is not stretched compared with stock 800x600 in region 24,194,44,44 within 10%.
27. Quit the game.
28. Expect no crash.
