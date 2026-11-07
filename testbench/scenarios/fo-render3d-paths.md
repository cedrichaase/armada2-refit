# Fleet Operations: Roots with Render3D: a move of one ship, and of several, draws its path

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d
Setup: display/bench-mode.sh stub Backend=vulkan
Timeout: 12 min

Roots has no QOL, so the move paths come from the engine's own orders: the fleetops set's
`EngineMoves = all` (`render3d/README.md`, "Move paths", "Every engine move"). A move of one
ship is a GO the paths read directly; a move of several ships gives the group's leader the
move and every other ship FOLLOW (command 11), which the paths read through the leader
(`PLAN-FLEETOPS.md`, "FO-B: the formation move's path", defaults 53 to 57). The log line is
`overlays: move paths: N selected ship(s) on a path, M point(s) ahead (engine)`; N is the
selected ships on a path, so a group of two or more reads N of its size, and 0 once they
have arrived.

The bench cannot hold Ctrl (`testbench/README.md`, "A key cannot be held"), so only the
formation move without Ctrl is checked here: with Ctrl each follower flies a copy of the
leader's path, drawn as a move of its own, which only a player's hand can stage.

The player's start is whatever Instant Action deals; a drag over the whole map selects the
ships in view (the station is not a ship and takes no part), and a right-click far away
sends them. Judge the shots: a fine dotted line from the leader to the clicked point, one leg
and a dot per follower to its place, nothing running to the map's origin, nothing left after
arrival.

1. Include "_fo-render3d-skirmish".
2. Expect "Render3D.log" contains "overlays: move paths 1".
3. Expect "Render3D.log" contains "overlays: move paths: QOL's queue not found".
4. Drag from 100,120 to 1180,700.
5. Wait 1 second.
6. Right-click at 640,200.
7. Wait up to 20 seconds for "Render3D.log" to say "point(s) ahead (engine)".
8. Wait 1 second.
9. Take a screenshot called "move".
10. Check that a fine dotted line runs from each moving ship towards the clicked point (the leader's to the point, a follower's to its place beside it), none runs to a corner of the screen, and none is drawn for ships that were not selected.
11. Wait up to 120 seconds for "Render3D.log" to say "0 selected ship(s) on a path".
12. Wait 1 second.
13. Take a screenshot called "arrived".
14. Check that no dotted line is left on the map.
15. Expect "Display8.log" not contain "VUID".
16. Expect "Display8.log" not contain "device removed".
17. Quit the game.
18. Expect the game to have exited.
19. Expect no crash.
