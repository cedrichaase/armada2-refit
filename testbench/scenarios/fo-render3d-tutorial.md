# Fleet Operations: Roots with Render3D: the Enterprise-E of the first allied tutorial is drawn

Resolutions: 1280x960
Launch: -nointro tutorialgameplay.bzn
Mod: refit
Layers: display asiloader render3d
Setup: display/bench-mode.sh stub Backend=vulkan
Timeout: 12 min

The user's report of 2026-11-06: in Roots' first allied tutorial (*TUTORIAL: Gameplay*)
the player's Enterprise-E (`fenterprise`, `fed_sovereignent.sod`) was not drawn at all,
with render3d's hulls claiming it at the instance draw. The claim honours what the engine
wrote into the class's shared database for the instance (`hulls::claim::honour_database`),
and the database marked the `lod*` level the module had picked as hidden: those flags are
the engine's own choice of level, which can differ from the module's, so the whole hull
went with the level (`hulls: fed_sovereignent: 4 of 4 shown nodes`: root, `damage`,
`geometry` and `lod1`, none of the meshes). The module now keeps its own pick for a
level of detail and logs the database's flag (`... a level of detail, the module's own
pick kept`); every other node's flags are honoured as before.

The game starts straight in the mission (`Launch: -nointro tutorialgameplay.bzn`, the map
named with its `.bzn`, `../README.md`, "Roots straight into a map, and scenes on Roots"):
the menu walk it took before (the single-player screen, the allied tile, the mission list,
OK) is `fo-ui-dialogs-single`'s to test. The wait is for
render3d's claim of the Enterprise, then the judged shot: the ship stands near the top
centre of the opening view, above the wormhole's blue glow.

1. Launch the game.
2. Wait up to 180 seconds for "Render3D.log" to say "fed_sovereignent at".
3. Wait 8 seconds.
4. Take a screenshot called "tutorial".
5. Check that a starship hull (a grey Federation ship with a saucer and two nacelles) is drawn in the upper middle of the view, above the blue glow, and not just a small yellow marker on empty space.
6. Expect "Render3D.log" contains "hulls: fenterprise is of race 'unitedfederation'".
7. Expect "Render3D.log" not contain "hulls: fed_sovereignent: 4 of 4 shown nodes".
8. Expect "Display8.log" not contain "VUID".
9. Expect "Display8.log" not contain "device removed".
10. Quit the game.
11. Expect the game to have exited.
12. Expect no crash.
