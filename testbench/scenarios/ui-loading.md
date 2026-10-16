# The loading screen is the UI's: the map's sky, the mission, the players, the progress

Resolutions: 1280x720
Mod: refit
Setup: ui/bench-asi.sh Loading.Enable=1 Loading.Hold=25
Timeout: 15 min

`ui/README.md`, "The loading screen". With `[Loading] Enable=1` the screen of ours
covers the engine's logo scene and bar. `Hold=25` makes each load wait 25 seconds at its
half-way point, the screen still drawn by frames of ours, so a screenshot can be taken
of it (a test aid; never set in play). An Instant Action load (map Warzone, the default,
as `_enter-instant-action.md` sets it up: the Borg against one easy computer player)
shows the kicker INSTANT ACTION, the map's title, the players panel with both slots, and
the bar near 50%. The campaign's first Federation mission ("Invasion") is then loaded
from the main menu: its briefing and objectives come from `objectives/a2_fed01_A.txt`,
and the theme is the Federation's. `UI.log` names the map, the sky face it chose, and
counts the frames of ours, which must outnumber the engine's own (one each 5%).

The campaign is reached from Instant Action's match by Esc, Exit to the main menu.

1. Include "_enter-instant-action"
2. Expect "UI.log" contains "loading: a screen of ours".
3. Expect "UI.log" contains "holding at".
4. Take a screenshot called "instant action load".
5. Expect "INSTANT ACTION" is visible.
6. Expect "Players" is visible.
7. Check that the screen is a loading screen drawn edge to edge: a starry nebula sky behind, a mission title at the top left, a players panel at the right listing two players with races, a progress bar across the bottom near half full, and no 3D logo or stock loading bar anywhere.
8. Wait 2 seconds.
9. Take a screenshot called "instant action load later".
10. Check that the ring of dots at the left of the status line has turned since the previous screenshot (the frames between the engine's are animated).
11. Wait 25 seconds.
12. Expect "UI.log" contains "hold over".
13. Expect "UI.log" contains "first frame of ours".
14. Take a screenshot called "match".
15. Check that the match has started: the 3D view and the in-mission HUD are up and no loading screen is left over them.
16. Quit the game.
17. Expect no crash.
