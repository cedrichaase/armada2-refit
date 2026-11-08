# Fleet Operations: Roots with the UI: the mission timer and the status line under the pointer keep apart

Resolutions: 1280x960
Launch: -nointro PoliticsByOtherMeans.bzn
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan OneWindow=0 ui/bench-asi.sh HUD.Minimap=1 HUD.Info=1 HUD.UnitView=1 HUD.Tooltips=1 HUD.Bar=1 HUD.Hover=1 HUD.Overlay=1 HUD.Messages=1 HUD.LayoutLog=1
Timeout: 16 min

The user's report of 2026-11-07 (Roots, 3440x1440): "28 Minutes Remaining" at the top centre and
the status line under the pointer (`[HUD] Hover=1`) drawn on top of each other. The timer is the
script's `AddMessage` at (50,85) on the canvas, which falls under the resource strip of ours and is
therefore centred across the screen at its height (`hud/overlay_text.rs`, decision 326); the status
line is Roots' own place for it, canvas `625..974 x 50..117` at 4:3 and `669..930` at 21:9. The
overlay now keeps that band clear whenever the hover line is ours, so the timer sits below it
from the start and does not jump when the line shows (PLAN-FLEETOPS.md, decision 470).
*Politics by Other Means* adds the countdown (`PbOM_DominionMonitor.lua`, every few seconds) after
its opening cinematic, which Space skips, and the briefing, which OK closes.

Run with `a2test --game fo-roots` and `--install .`. The pointer steps need the pointer to work in a
direct launch (`OneWindow=0` in the Setup, decision 348); where they do not, the log checks (the message moved below the band) still decide.

1. Launch the game.
2. Wait up to 240 seconds for "UI.log" to say "overlay: #0".
3. Press space 3 times.
4. Wait up to 60 seconds for "UI.log" to say "objectives: ".
5. Wait 2 seconds.
6. Click "OK".
7. Wait up to 120 seconds for "UI.log" to say "Minutes Remaining".
8. Move the mouse into the bottom right corner.
9. Move the mouse to 1100,200.
10. Wait 1 second.
11. Move the mouse to 1110,205.
12. Wait 2 seconds.
13. Take a screenshot called "timer and pointer on the station".
14. Check that "30 Minutes Remaining" (or a lower number) is drawn at the top centre, clear of the resource strip at the top left and the round buttons at the top right, and that no other text overlaps it: when the pointer is on the station at the right, Roots' status line (class, hull, shields, crew) is above the timer and not on it.
15. Expect "UI.log" contains "overlay: #8 moved clear of the panels of ours, to 535,107".
16. Expect "UI.log" not contain "PANIC".
17. Expect "Display8.log" not contain "VUID".
18. Expect "Display8.log" not contain "device removed".
19. Quit the game.
20. Expect the game to have exited.
21. Expect no crash.
