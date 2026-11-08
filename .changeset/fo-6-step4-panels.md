---
layers: [ui, testbench, root]
bump: minor
games: [fo-roots]
status: Installed, not yet seen in game
---

### Added
- FO-6 step 4 on Roots: seven HUD panels are on by default there (`ui/defaults/fo-roots.ini`: the resources, the top-right buttons, the pause box, the chat, the selection overlay, the frame-rate panel, and the order ring, height posts and footprint), each with a scenario (`fo-ui-resources`, `-topbar`, `-pause`, `-chat`, `-selection`, `-frame-rate`, `-overview`, and `-hud` for the seven together); `hud.panels`, `hud.order_ring`, `hud.frame_rate`, `panels.presses`, `panels.chat` and `panels.trade` are verified in the Roots profile (root)
- UI.log's `buttons: panel` line names each of the top-right panel's eight buttons, its press-function match and its rectangle (ui)

### Changed
- The HUD part no longer requires the sound hook, and a profile value (`panels_sound_ours`, 0 on Roots) switches the UI's button sounds off where every audio class is the companion module's: the panels are silent there (ui)
- The pause box uses its own words when the game's string table gives none for its keys, and a resource's name comes from the content set when the component's tooltip is not a word (ui)
- The Roots scenarios of earlier steps carry `HUD.Enable=0` so that they still test what they did (testbench)
