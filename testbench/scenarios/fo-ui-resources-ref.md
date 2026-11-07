# Fleet Operations: Roots' own resources panel, the reference for fo-ui-resources

Resolutions: 1280x960
Launch: -nointro tutorialgameplay.bzn
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh HUD.Enable=1 HUD.Resources=0 HUD.Buttons=0 HUD.Pause=0 HUD.Chat=0 HUD.Selection=0 HUD.FrameRate=0 HUD.Overview=0 HUD.LayoutLog=1
Timeout: 14 min


Run with `a2test --game fo-roots` and `--install .`. The A side of `fo-ui-resources`: the UI on, its resources row off, so the shot shows Roots' own strip.

1. Launch the game.
2. Move the mouse into the bottom right corner.
3. Wait 40 seconds.
4. Take a screenshot called "resources roots".
5. Quit the game.
6. Expect the game to have exited.
7. Expect no crash.
