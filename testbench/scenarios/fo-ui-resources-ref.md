# Fleet Operations: Roots' own resources panel, the reference for fo-ui-resources

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh Menus.Enable=0 Menus.Forms=0 HUD.Enable=1 HUD.Resources=0 HUD.Buttons=0 HUD.Pause=0 HUD.Chat=0 HUD.Selection=0 HUD.FrameRate=0 HUD.Overview=0 HUD.LayoutLog=1
Timeout: 14 min


Run with `a2test --game fo-roots` and `--install .`. The A side of `fo-ui-resources`: the UI on, its resources row off, so the shot shows Roots' own strip.

1. Include "_fo-render3d-skirmish".
2. Wait 20 seconds.
3. Take a screenshot called "resources roots".
4. Quit the game.
5. Expect the game to have exited.
6. Expect no crash.
