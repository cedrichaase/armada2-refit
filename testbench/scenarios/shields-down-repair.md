# Shields down and repair welding, drawn by Render3D.asi

Resolutions: 16:9
Mod: refit
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh shields
Timeout: 8 min

`render3d/README.md`, "effects" (shields, repair). Run it on a checkout:
`./a2test run shields-down-repair --install .`. The `shields` scene, with the target's
healing switched off: under the Galaxy's phasers and the Klingon's torpedoes its shields
run out (the shields-down arcs), then the attackers stop firing and its damaged systems
are repaired (the welding). The log lines check that the module is on and that both
effects were noticed and drawn; the shots are for judging the look: jagged blue-white arcs crackling over
the shield ellipsoid where stock has a turning band of lightning texture, and blue-white
arc flares with rays and sparks on the hull where stock has white star sprites.

1. Launch the game.
2. Wait up to 50 seconds for the logs the next steps expect.
3. Expect "Scene.log" contains "scene ready".
4. Expect "Render3D.log" contains "module effects: on".
5. Expect "Render3D.log" contains "effects: repair: AddParticle".
6. Scene "heal target off".
7. Scene "orbit target 150 25 150".
8. Wait up to 45 seconds for the logs the next steps expect.
9. Expect "Render3D.log" contains "effects: shields down on fgalaxy.odf".
10. Take a screenshot called "shields-down-close".
11. Scene "orbit target 150 -25 150".
12. Wait 2 seconds.
13. Take a screenshot called "shields-down-below".
14. Scene "orbit target 150 35 420".
15. Wait 2 seconds.
16. Take a screenshot called "shields-down-far".
17. Scene "weapons shooter off".
18. Scene "weapons klingon off".
19. Scene "orbit target 140 30 95".
20. Wait up to 20 seconds for the logs the next steps expect.
21. Expect "Render3D.log" contains "effects: weld on fgalaxy.odf".
22. Expect "Render3D.log" contains "effects: first welds drawn".
23. Take a screenshot called "welds-1".
24. Wait 1 second.
25. Take a screenshot called "welds-2".
26. Wait 1 second.
27. Take a screenshot called "welds-3".
28. Expect the game is still running.
29. Quit the game.
30. Expect no crash.
