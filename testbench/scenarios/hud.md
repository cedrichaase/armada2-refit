# The in-mission HUD and font keep their shape at every aspect ratio

Resolutions: 4:3, 16:10, 16:9, 21:9
Mod: remastered
Reference: 4:3 stock
Stock shell: embed
Timeout: 12 min

Measured at the first Borg mission's briefing, where the faction (and so the HUD art)
is always the same. See `_enter-borg-mission.md` for why a direct map launch will not do.
The HUD panels stay visible around the briefing, and the briefing is in the in-game font.

`hud/ui-widescreen.py` re-declares the layout canvas per aspect. `font/ui-font-condense.py`
condenses the font for the resolution. The bench re-runs both for each case.

The baseline is **stock at 4:3**, the shape the HUD and font were drawn for, and not
the remastered 4:3 case. So the remastered 4:3 case is measured too, and any change
the mod makes at 4:3 counts against it like a stretch at 21:9 would. Regions are in 4:3
reference pixels (1600x1200): the start of the resource bar, the inside of the left
(minimap) panel, and the inside of the right panel. The template match finds each one
wherever it sits at the other aspects.

1. Include "_enter-borg-mission".
2. Take a screenshot called "briefing".
3. Check that the hud is not stretched compared with stock 4:3 in region 0,0,300,45.
4. Check that the hud is not stretched compared with stock 4:3 in region 0,830,160,370.
5. Check that the hud is not stretched compared with stock 4:3 in region 1440,830,160,370.
6. Check that "BRIEFING SUMMARY" is not stretched compared with stock 4:3.
7. Check that "OBJECTIVES" is not stretched compared with stock 4:3.
8. Check that "Werewolf Pack" is not stretched compared with stock 4:3.
9. Check that the HUD panels and the briefing keep the same proportions as in the stock 4:3 reference: round and square elements stay round and square, and nothing looks horizontally stretched or squeezed compared with it.
10. Quit the game.
11. Expect no crash.
