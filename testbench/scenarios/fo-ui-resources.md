# Fleet Operations: Roots with the UI: the resources panel is ours, and Roots' own is hidden where it draws

Resolutions: 1280x960
Launch: -nointro tutorialgameplay.bzn
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh HUD.Enable=1 HUD.Resources=1 HUD.Buttons=0 HUD.Pause=0 HUD.Chat=0 HUD.Selection=0 HUD.FrameRate=0 HUD.Overview=0 HUD.LayoutLog=1
Timeout: 14 min

Run with `a2test --game fo-roots` and `--install .`. FO-6 step 4 (`PLAN-FLEETOPS.md`, "FO-6:
the UI on Roots", "Step 4: the panels Roots changes least") takes the resources over through
the panel switch (`hud.panels`, verified: `DisplayInterface::RenderAll`'s entry is
FleetOpsHook's trampoline and the Render call at +0x47 behind it is stock's). Roots shows one
to three resources by race and FleetOpsHook draws each `ResourceComponent`; the panel of ours
reads the amounts through `Team::GetResource` for the components the render list names, so
the cells are the ones Roots keeps. The scenario checks that the panel is claimed, that
Roots' own strip is not drawn under ours, and that the amounts agree with the game's own
(the log names each cell's resource and amount).

1. Launch the game.
2. Move the mouse into the bottom right corner.
3. Wait up to 120 seconds for "UI.log" to say "resources:".
4. Wait 3 seconds.
5. Take a screenshot called "resources ours".
6. Check that the top left of the screen, where Fleet Operations' own resource strip sits, shows a strip of resource cells drawn by the UI (a dark rounded panel with one to three cells, each with Roots' own icon for the resource, a name and an amount: the dilithium crystal, the tritanium rock, the red supply crate), that the strip starts flush in the top left corner of the screen, a small margin in from both edges, that the names are Roots' (Dilithium, Tritanium, Supply) and that Fleet Operations' own resource counters are not drawn a second time under or beside it.
7. Move the mouse to 150,40.
8. Wait 2 seconds.
9. Take a screenshot called "resources hover".
10. Expect "UI.log" contains "panels: ResourcePanel ours".
11. Expect "UI.log" contains "fontglyph: FontFinal4_16.spr read".
12. Expect "UI.log" contains "resources:".
13. Expect "UI.log" contains "Tritanium=".
14. Expect "UI.log" contains "Supply=".
15. Expect "UI.log" not contain "(Metal)".
16. Expect "UI.log" not contain "the HUD stays stock".
17. Expect "UI.log" not contain "PANIC".
18. Expect "Display8.log" not contain "VUID".
19. Expect "Display8.log" not contain "device removed".
20. Quit the game.
21. Expect the game to have exited.
22. Expect no crash.
