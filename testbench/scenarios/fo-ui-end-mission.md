# Fleet Operations: Roots with the UI: End Mission holds the picture until the next screen

Resolutions: 1600x900
Mod: refit
Layers: display asiloader render3d ui
Launch: -nointro tutorialgameplay.bzn
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh Menus.Forms=1 Menus.Scene=1 Menus.Enable=1 Menus.Credits=1
Timeout: 8 min

Run with `a2test --game fo-roots` and `--install .`. The quick loop for `fo-ui-ingame-window`'s
last switch (PLAN-FLEETOPS.md, decisions 318 and 400): the first allied tutorial straight from
the command line, Esc, End Mission, Yes, and a series of shots from Yes until the next screen
of ours, none of which may be black (the in-mission menu's last picture is held while the
engine presents frames with nothing of ours on them and the next screen's first frame is
drawn); the window manager sees no window come or go.

1. Launch the game.
2. Wait up to 180 seconds for "Render3D.log" to say "fed_sovereignent at".
3. Wait 8 seconds.
4. Move the mouse into the bottom right corner.
5. Move the mouse to 700,300.
6. Watch the window manager.
7. Press Escape.
8. Wait up to 20 seconds for "UI.log" to say "forms: TOptionsMenuForm is ours".
9. Wait 3 seconds.
10. Click "End Mission".
11. Wait up to 20 seconds for "UI.log" to say "forms: TYesNoForm is ours".
12. Wait 2 seconds.
13. Click "Yes".
14. Take 16 screenshots called "end" every 0.25 seconds.
15. Expect no shot of the series "end" to be black.
16. Wait 4 seconds.
17. Take a screenshot called "after".
18. Expect the window manager to have seen no other window, and the game's window kept.
19. Expect "UI.log" not contain "PANIC".
20. Stop the game.
21. Expect no crash.
