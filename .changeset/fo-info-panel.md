---
layers: [ui]
bump: minor
games: [fo-roots]
---

### Added
- On Fleet Operations: Roots, HUD.Info=1 draws Roots' info panel in Roots' own layout (the fleetops set's [InfoPanel]: the ship image, FleetOpsHook's weapon and buff icons, the subsystems, the queue on Roots' plates, the groups), with the UI's fonts, bars and frame, over the panel's Render run unseen (hook hud.info_fo), so Roots' tooltips, clicks and range rings on hover stay; new [HUD] keys BottomRow (stacked, an order of minimap/info/bar, auto; Roots: auto), InfoWidth and BarCellSize put the grid bar in the bottom row beside a resizable info panel
