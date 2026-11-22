---
layers: [ui]
bump: minor
games: [fo-roots]
---

### Changed
- On Fleet Operations: Roots the info panel of ours keeps Roots' own proportions (unstretched at 21:9) and the pointer follows it wherever it is drawn (the panel's FocusSimulate sees the mouse mapped into its own rectangle, so tooltips, clicks and range rings stay Roots'); BottomRow=auto puts the minimap bottom left, the panel centred and the grid as tall as the minimap in the bottom right corner where they fit (the row with the grid in the corner, else stacked); the weapon icons in the look FleetOpsHook gives them (grey until researched, reload and energy fill, the race's own colours), the buff icons by FleetOpsHook's live count, and the builder face's own subsystem icons
