# The in-mission HUD and font keep their shape at every aspect ratio

Resolutions: 4:3, 16:10, 16:9, 21:9
Mod: remastered
Reference: 800x600 stock
Stock shell: embed
Timeout: 12 min

Measured at the first Borg mission's briefing, where the faction (and so the HUD art)
is always the same. See `_enter-borg-mission.md` for why a direct map launch will not do.
The HUD panels stay visible around the briefing, and the briefing is in the in-game font.

`HUD.asi` re-declares the layout canvas and condenses the font at run time, for the
mode the game is in. Every remastered case runs the same install (`hud/install.sh`,
which takes no resolution), so a pass at every aspect is the claim being tested.

The baseline is **stock at 800x600**, the resolution the game was designed around:
the front end runs in its own 800x600 mode, so nothing is stretched and clicks map 1:1.
It is not the remastered 4:3 case, so that case is measured too, and any change the mod
makes at 4:3 counts against it like a stretch at 21:9 would. Regions are in reference
pixels (800x600): the start of the resource bar, the inside of the left (minimap) panel,
and the inside of the right panel. The template match finds each one wherever it sits
at the other resolutions, scaled by H/600.

The cursor is measured on its own: the pointer is parked at HUD point 60,400 (30,200
in the stock 800x600 frame) and the region around it is matched like a HUD panel. The
game draws this cursor itself (the synchronous path, under DXVK), so it is in the
screenshot; stock at 21:9 draws it ~1.79x wide. Until testbench 1.6.1 the pointer never
got there (`README.md`, "Traps"): the check matched empty background and meant nothing.
It is allowed ±10%: the stock arrow is 19x23 px at 800x600, so one pixel is ~5%, and the
first honest run measured 0.94–1.02 where the arrows themselves, measured directly, keep
one width-to-height ratio to 0.6% (0.782–0.787 at 4:3, 16:10, 16:9 and 21:9). An
uncorrected cursor is 1.20x wide at 16:10, 1.33x at 16:9 and 1.79x at 21:9.

The font is allowed ±10% where the HUD gets ±5%. The condense (1.25·H/W) aims
for the glyphs' own 1280x1024 shape, which is 0.9375 of stock's width at 4:3, and it
measured 0.91–0.94 at every aspect ratio. The user judged it fine in game
(2026-09-26). ±10% still catches the font drawn stretched (stock at 21:9 is ~1.79x).

1. Include "_enter-borg-mission".
2. Take a screenshot called "briefing".
3. Check that the hud is not stretched compared with stock 800x600 in region 0,0,150,23.
4. Check that the hud is not stretched compared with stock 800x600 in region 0,415,80,185.
5. Check that the hud is not stretched compared with stock 800x600 in region 720,415,80,185.
6. Check that "BRIEFING SUMMARY" is not stretched compared with stock 800x600 within 10%.
7. Check that "OBJECTIVES" is not stretched compared with stock 800x600 within 10%.
8. Check that "Werewolf Pack" is not stretched compared with stock 800x600 within 10%.
9. Check that the HUD panels and the briefing keep the same proportions as in the stock 800x600 reference: round and square elements stay round and square, and nothing looks horizontally stretched or squeezed compared with it.
10. Move the mouse to hud 60,400.
11. Wait 2 seconds.
12. Take a screenshot called "cursor".
13. Check that the hud is not stretched compared with stock 800x600 in region 24,194,44,44 within 10%.
14. Quit the game.
15. Expect no crash.
