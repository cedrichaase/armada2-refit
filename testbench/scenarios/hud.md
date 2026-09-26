# The in-mission HUD and font keep their shape at every aspect ratio

Resolutions: 4:3, 16:10, 16:9, 21:9
Mod: remastered
Reference: 800x600 stock
Stock shell: embed
Timeout: 12 min

Measured at the first Borg mission's briefing, where the faction (and so the HUD art)
is always the same. See `_enter-borg-mission.md` for why a direct map launch will not do.
The HUD panels stay visible around the briefing, and the briefing is in the in-game font.

`hud/ui-widescreen.py` re-declares the layout canvas per aspect. `font/ui-font-condense.py`
condenses the font for the resolution. The bench re-runs both for each case.

The baseline is **stock at 800x600**, the resolution the game was designed around:
the front end runs in its own 800x600 mode, so nothing is stretched and clicks map 1:1.
It is not the remastered 4:3 case, so that case is measured too, and any change the mod
makes at 4:3 counts against it like a stretch at 21:9 would. Regions are in reference
pixels (800x600): the start of the resource bar, the inside of the left (minimap) panel,
and the inside of the right panel. The template match finds each one wherever it sits
at the other resolutions, scaled by H/600.

1. Include "_enter-borg-mission".
2. Take a screenshot called "briefing".
3. Check that the hud is not stretched compared with stock 800x600 in region 0,0,150,23.
4. Check that the hud is not stretched compared with stock 800x600 in region 0,415,80,185.
5. Check that the hud is not stretched compared with stock 800x600 in region 720,415,80,185.
6. Check that "BRIEFING SUMMARY" is not stretched compared with stock 800x600.
7. Check that "OBJECTIVES" is not stretched compared with stock 800x600.
8. Check that "Werewolf Pack" is not stretched compared with stock 800x600.
9. Check that the HUD panels and the briefing keep the same proportions as in the stock 800x600 reference: round and square elements stay round and square, and nothing looks horizontally stretched or squeezed compared with it.
10. Quit the game.
11. Expect no crash.
