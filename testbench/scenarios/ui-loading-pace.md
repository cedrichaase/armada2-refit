# How often the loading screen can be redrawn: the still stretches of a campaign load

Resolutions: 1280x720
Mod: refit
Setup: ui/bench-asi.sh Loading.Enable=1
Timeout: 12 min

A measurement, run with `--install .` so the load is a refit install's (its textures,
render3d). `ui/LOADING.md`, "Frames of ours" and "Frames inside a step": frames of ours
come between the engine's loading steps and after each SOD and texture loads, so only
the engine's own work in between keeps the screen still. With no `Hold`, `UI.log` names
every stretch of 100 ms or more with no frame (`loading: still`), with its step and what
loaded in it, and the step counter lines give the longest wait.
The first Federation mission is loaded from the campaign selection, as a player starts
one. The mission list is the UI's, so its row and O.K. are clicked by their labels
(`_enter-federation-mission` clicks the stock list's points, which miss it).

1. Launch the game.
2. Wait for the main menu.
3. Click the single player emblem.
4. Wait 3 seconds.
5. Click "Federation Campaign".
6. Wait 3 seconds.
7. Click "Invasion".
8. Wait 1 second.
9. Click "O.K.".
10. Wait up to 300 seconds for the briefing.
11. Expect "UI.log" contains "first frame of ours".
12. Take a screenshot called "briefing".
13. Quit the game.
14. Expect no crash.
