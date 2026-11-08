# Fleet Operations: Roots with the UI's forms on and their hook forced down: Roots' own front end

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan tools/engine/bench-mismatch.sh ui.forms.vcl ui/bench-asi.sh Menus.Forms=1 Loading.Enable=0 UI.RefitKey=0 HUD.Enable=0
Timeout: 10 min

`[Menus] Forms=1` with the forms' hook (`ui.forms.vcl`) forced to stand down through
`A2EngineTest.ini` (`tools/engine/bench-mismatch.sh`), as a FleetOpsHook outside the
measured family or a build without the profile's forms would make it: the UI hooks nothing
of the forms and Roots draws its own main menu, which must match the baseline
`fo-ui-standdown` (recorded by `fo-ui-standdown-ref`, Display8 and Render3D without the UI)
as `fo-ui-standdown` does (`PLAN-FLEETOPS.md`, FO-6 decision 93; option C, step 7). Run
with `a2test --game fo-roots` and `--install .`.

1. Launch the game.
2. Wait for the main menu.
3. Wait 5 seconds.
4. Take a screenshot called "menu".
5. Expect the screenshot "menu" matches baseline "fo-ui-standdown" with at most 0.05% of pixels over 4.
6. Wait up to 20 seconds for the logs the next steps expect.
7. Expect "UI.log" contains "forms: standing down".
8. Expect "UI.log" not contain "forms: TMainForm is ours".
9. Expect "UI.log" not contain "PANIC".
10. Stop the game.
11. Expect no crash.
