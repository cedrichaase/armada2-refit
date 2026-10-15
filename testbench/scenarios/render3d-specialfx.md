# render3d draws the special-object effects (Step::Specials)

Resolutions: 16:9
Mod: refit
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh specials
Timeout: 10 min

`render3d/README.md`, "specials". Run it on a checkout: `./a2test run render3d-specialfx
--install . --res 1280x720`. The three special weapons whose effects stock draws in
`SpecialObjectEffect::RenderAll` are fired at the scene's Sovereign (`tgt`) one after
another: the Borg Computer Override, the Klingon Energy Dissipator and Species 8472's
Psionic Disruption. The log checks that each effect's emitter SOD was found from its
weapon's ODF (`particleArt`) and its cloud handed to the particles module, and that a
layer was drawn over the hull. The shots are for judging the look: stock's cloud of
coloured stars or spores, and over the Sovereign green circuitry creeping from the Borg
ship's side, a pink film of veins, and violet ripples; then the HUD's action camera on.

1. Launch the game.
2. Wait 50 seconds.
3. Expect "Scene.log" contains "scene ready".
4. Expect "Render3D.log" contains "module specials: on".
5. Scene "camera 3150 90 3120 3000 0 3080".
6. Scene "spawn ovr bspecial 3000 0 3300 180 1".
7. Scene "engines ovr off".
8. Scene "energy ovr on".
9. Wait 2 seconds.
10. Expect scene "special ovr goverrid tgt" answers "fired".
11. Wait 1 second.
12. Scene "weapons ovr off".
13. Wait 3 seconds.
14. Expect "Render3D.log" contains "of 'Computer Override' (ordnance overrid): wovremit".
15. Expect "Render3D.log" contains "first layer drawn: Override over fbattle".
16. Take a screenshot called "override".
17. Check that the Sovereign carries thin glowing green lines (a circuit pattern) over its hull, with a few coloured star sprites near it, and that nothing is garbled.
18. Wait 3 seconds.
19. Take a screenshot called "override-later".
20. Scene "kill ovr".
21. Wait 20 seconds.
22. Scene "spawn mic kspecial 3000 0 3300 180 1".
23. Scene "engines mic off".
24. Scene "energy mic on".
25. Wait 2 seconds.
26. Expect scene "special mic gmicro tgt" answers "fired".
27. Wait 1 second.
28. Scene "weapons mic off".
29. Wait 4 seconds.
30. Expect "Render3D.log" contains "(ordnance micro): wmicremit".
31. Take a screenshot called "dissipator".
32. Scene "kill mic".
33. Wait 12 seconds.
34. Scene "spawn psi 8472_cruise1 3000 0 3300 180 1".
35. Scene "engines psi off".
36. Scene "energy psi on".
37. Wait 2 seconds.
38. Expect scene "special psi gpsionicDisruption tgt" answers "fired".
39. Wait 1 second.
40. Wait 2 seconds.
41. Expect "Render3D.log" contains "(ordnance psionicdisruption): psionicdisruption".
42. Take a screenshot called "psionic".
43. Scene "hud on".
44. Wait 5 seconds.
45. Take a screenshot called "psionic-hud".
46. Scene "weapons psi off".
47. Wait 60 seconds.
48. Expect the game is still running.
49. Expect "Render3D.log" not contains "PANIC".
50. Expect "Render3D.log" not contains "the engine draws it".
51. Quit the game.
52. Expect no crash.
