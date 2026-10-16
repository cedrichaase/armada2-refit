# A planet can be clicked, and a colony ship sent to colonise it (render3d, UI.asi)

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh ui-colonize
Timeout: 6 min

Run it on a checkout: `./a2test run ui-hud-colonize --install .`, so that checkout's
render3d draws the planet. render3d draws planets in the engine's place; before render3d
0.7.1 it did not mark them on screen (the byte `Entity::Render` writes), and the
engine's pointer pick (`DisplayInterface::s_PrecomputeMouseOver`) passes over any
object without it: a planet could be selected only with a drag box, and a colony
ship could not be right-clicked onto it, which left the campaigns unplayable
(render3d/README.md, world). The scene (`testbench/scene/scenes/ui-colonize.ini`) is
the player's colony ship and a Galaxy class beside an unclaimed class M planet, the
camera on the colony ship: the planet's middle is at 658,190 at 1280x720.

1. Launch the game.
2. Wait 50 seconds.
3. Expect "Scene.log" contains "scene ready".
4. Click at 658,190.
5. Wait 1 second.
6. Expect scene "selection" answers "selected 1: planet".
7. Scene "select colony".
8. Wait 1 second.
9. Move the mouse to 658,190.
10. Wait 1 second.
11. Expect "UI.log" contains "draws c_colonize".
12. Right-click at 658,190.
13. Wait 15 seconds.
14. Expect scene "query" not answer "team 0 population 0 ".
15. Take a screenshot called "colonising".
16. Check that the small colony ship has left its place below the planet and is now beside the planet, at about the planet's height.
17. Expect the game is still running.
18. Quit the game.
19. Expect no crash.
