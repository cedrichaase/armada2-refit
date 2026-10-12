# Shields down and repair welding, drawn by Shields.asi and Repair.asi

Resolutions: 16:9
Mod: refit
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh shields
Timeout: 8 min

`shields/README.md` ("Shields down") and `repair/README.md`. Run it on a checkout:
`./a2test run shields-down-repair --install .`. The `shields` scene, with the target's
healing switched off: under the Galaxy's phasers and the Klingon's torpedoes its shields
run out (the shields-down arcs), then the attackers stop firing and its damaged systems
are repaired (the welding). The log lines check the hooks and that both effects were
drawn and lit; the shots are for judging the look: jagged blue-white arcs crackling over
the shield ellipsoid where stock has a turning band of lightning texture, and blue-white
arc flares with rays and sparks on the hull where stock has white star sprites.

1. Launch the game.
2. Wait 50 seconds.
3. Expect "Scene.log" contains "scene ready".
4. Expect "Shields.log" contains "Ordnance::PreRenderAll patched".
5. Expect "Repair.log" contains "AddParticle, DrawScaled3D and RenderChunks".
6. Scene "heal target off".
7. Scene "orbit target 150 25 150".
8. Wait 45 seconds.
9. Expect "Shields.log" contains "shields down on fgalaxy.odf".
10. Expect "Shields.log" contains "first shields-down arcs drawn".
11. Expect "Shields.log" contains "first shields-down light".
12. Take a screenshot called "shields-down-close".
13. Scene "orbit target 150 -25 150".
14. Wait 2 seconds.
15. Take a screenshot called "shields-down-below".
16. Scene "orbit target 150 35 420".
17. Wait 2 seconds.
18. Take a screenshot called "shields-down-far".
19. Scene "weapons shooter off".
20. Scene "weapons klingon off".
21. Scene "orbit target 140 30 95".
22. Wait 20 seconds.
23. Expect "Repair.log" contains "weld on fgalaxy.odf".
24. Expect "Repair.log" contains "first welds drawn".
25. Expect "Repair.log" contains "first arc light".
26. Take a screenshot called "welds-1".
27. Wait 1 second.
28. Take a screenshot called "welds-2".
29. Wait 1 second.
30. Take a screenshot called "welds-3".
31. Expect the game is still running.
32. Quit the game.
33. Expect no crash.
