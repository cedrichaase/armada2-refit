---
ui: minor
qol: minor
root: patch
---

### Added
- Fleet Operations: Roots: with [HUD] Bar=1 the bar of ours is the 5x3 grid of position keys (the stock buttons moved onto its cells after each layout, hook ui.hud.bar_grid, so FleetOpsHook's range ring, the tooltip and the click follow them; buttons past fifteen in keyless columns beside it); QOL's BarKeys work on Roots (qol.bar_keys verified; bk_key_cells leaves W without a key there); on Roots the HUD's icons are Roots' own art first (the bar, the top-right buttons; sprites the engine never drew take their art from Roots' .spr lines), and sprites on Roots' DDS sheets load (the cycle_support and Wgesconv buttons showed their names).
