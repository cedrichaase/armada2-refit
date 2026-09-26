# Control: stock is stretched at 21:9 (proves the measurements can fail)

Resolutions: 4:3, 21:9
Mod: stock
Stock shell: embed
Timeout: 12 min

A test that only ever passes proves nothing. This runs the game with every visual layer
put aside (`a2mod stock`, on the clone). There the HUD and the font are known to draw
~1.79x too wide at 21:9 (hud/README.md: 2.15 across against 1.20 down), and the 21:9
case must MEASURE that stretch. It uses the same Borg briefing and regions as `hud.md`,
so a faction mix-up cannot pass it by accident. In the 4:3 reference case the
comparisons are skipped.

1. Include "_enter-borg-mission".
2. Take a screenshot called "briefing".
3. Check that the hud is stretched compared with 4:3 in region 0,0,300,45.
4. Check that the hud is stretched compared with 4:3 in region 0,830,160,370.
5. Check that "BRIEFING SUMMARY" is stretched compared with 4:3.
6. Quit the game.
