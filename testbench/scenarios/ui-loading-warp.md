# A mission loaded from the campaign selection departs from the menu scene into warp

Resolutions: 1280x720
Mod: refit
Setup: ui/bench-asi.sh Loading.Enable=1 Loading.Hold=12
Timeout: 15 min

Run with `--install .` (render3d's `Render3D_MenuWarp`, ui's departure). `ui/MENUS.md`,
"The menu scene", and `render3d/README.md`, "The menu scene": when a mission loads
while the menu scene is up, the loading screen draws over the scene, whose ships go to
warp front first with the camera swinging behind them and following, and the warp's
tunnel comes in once they have gone. `Hold=12` keeps the load at half-way for 12
seconds, frames of ours still drawn, so the departure plays out to be photographed.
The Federation campaign's first mission ("Invasion") is started from the campaign
selection, its list's row and O.K. clicked by their labels.

1. Launch the game.
2. Wait for the main menu.
3. Click the single player emblem.
4. Wait 3 seconds.
5. Click "Federation Campaign".
6. Wait 3 seconds.
7. Click "Invasion".
8. Wait 1 second.
9. Click "O.K.".
10. Wait 1 second.
11. Take a screenshot called "departure 1".
12. Wait 1 second.
13. Take a screenshot called "departure 2".
14. Wait 1 second.
15. Take a screenshot called "departure 3".
16. Wait 1 second.
17. Take a screenshot called "departure 4".
18. Wait 2 seconds.
19. Take a screenshot called "tunnel".
20. Expect "UI.log" contains "the load departs from the scene federation".
21. Expect "Render3D.log" contains "menu: departure from federation".
22. Check that the "tunnel" screenshot shows the loading screen (the mission's title INVASION at the top left, a progress bar along the foot) over a warp tunnel: streaks of light radiating from a point near the centre over a darkened scene, no menu entries.
23. Wait up to 300 seconds for the briefing.
24. Take a screenshot called "briefing".
25. Quit the game.
26. Expect no crash.
