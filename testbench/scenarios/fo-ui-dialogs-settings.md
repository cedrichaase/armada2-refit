# Fleet Operations: Roots with the UI: Game Settings as a screen of ours

Resolutions: 1600x900
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh Menus.Forms=1 Menus.Scene=1 Menus.Enable=1 Menus.Credits=1
Timeout: 10 min

Run with `a2test --game fo-roots` and `--install .`. Roots' Game Settings (`GameSettingsDlgProc`,
opened from Options) is adopted and drawn by its spec of ours (PLAN-FLEETOPS.md, decisions
255 onwards; ui/MENUS.md, "Roots' dialogs, laid out"): Armada II's settings form, the
sliders and the cycle buttons the stock ones, every change a click on the stock control. At
1600x900 the Mouse Hold Level slider's track runs from x 318 to 782 at y 415 (`game_settings.rs`'s
window at a scale of 900/1080).

1. Launch the game.
2. Wait up to 180 seconds for "UI.log" to say "forms: TMainForm is ours".
3. Wait 3 seconds.
4. Move the mouse to 700,300.
5. Wait 1 second.
6. Click "Options".
7. Wait up to 20 seconds for "UI.log" to say "TOptionsMenuForm is ours".
8. Wait 2 seconds.
9. Click "Game Settings".
10. Wait up to 20 seconds for "UI.log" to say "game settings drawn by its spec".
11. Wait 3 seconds.
12. Take a screenshot called "settings".
13. Expect the window manager to see one window.
14. Check that a window of ours titled "Game Settings" shows six sliders captioned Game Speed, Mouse Scroll Speed, Keyboard Scroll Speed, Mouse Hold Level, AI Build Cost Handicap and AI Build Time Handicap (the last two with Hard and Easy at their ends) on the left, Game Difficulty with a "Medium" button and Interface Mouse Mode with a "Right Click" button on the right, and "Previous Menu" at the foot.
15. Click "Medium".
16. Wait 2 seconds.
17. Take a screenshot called "difficulty".
18. Check that the Game Difficulty button no longer says "Medium" (Roots steps it to the next difficulty).
19. Move the mouse to 630,415.
20. Wait 1 second.
21. Click at 640,415.
22. Wait up to 10 seconds for "UI.log" to say "scroll bar read back".
23. Wait 1 second.
24. Take a screenshot called "hold level".
25. Check that the Mouse Hold Level slider's knob stands at about seven tenths of its track (near x 640), not near its left end where it started.
26. Expect "UI.log" not contain "PANIC".
27. Stop the game.
28. Expect no crash.
