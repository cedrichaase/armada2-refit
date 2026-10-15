# render3d: mining beams, mines and the special weapons that draw a model

Resolutions: 16:9
Mod: refit
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh specials
Timeout: 8 min

`render3d/README.md`, "Ordnance" ("The models", the mining beams). Run it on a checkout:
`./a2test run render3d-ordmodels --res 1280x720 --install .`. The scene is
`specials` (an enemy Sovereign, `tgt`, that cannot die and does not fire) with the
player's shooters spawned about it, as `scenes/ordmodels.ini` has them, engines off and
special energy kept full. A freighter fires
its dilithium beam at a moon, a Federation and a Klingon destroyer lay an antimatter and
a gravity mine while under way, a Klingon cruiser launches its commando team and a
Romulan cruiser a myotron at the Sovereign, a Federation cruiser a chain pulsar. The log
lines check that each was drawn by render3d (the beam by its recipe, the models through
the vtable slot the module takes; a model is logged when it is first drawn, so the camera
looks at it first); `special` answers `cannot fire now, fired` for a mine it does lay,
and `NOT fired` when it does not. The shots are for judging the look against stock
(`--install` of a checkout without this work): a blue beam flowing back to the freighter
and lighting the moon where stock has two thin lines; the mines' models with a pulsing
glow at the heart; the shuttle's rocket trails and the myotron's contrails drawn with
every other particle.

1. Launch the game.
2. Wait 50 seconds.
3. Expect "Scene.log" contains "scene ready".
4. Scene "spawn fd fdestroy 3000 0 3400 180 1".
5. Scene "spawn kd kdestroy 3200 0 3350 210 1".
6. Scene "spawn kc kcruise2 2800 0 3350 150 1".
7. Scene "spawn rc rcruise2 3350 0 3150 240 1".
8. Scene "spawn fc fcruise1 3350 0 2750 300 1".
9. Scene "energy fd on".
10. Scene "energy kd on".
11. Scene "energy kc on".
12. Scene "energy rc on".
13. Scene "energy fc on".
14. Scene "engines kc off".
15. Scene "engines rc off".
16. Scene "engines fc off".
17. Scene "spawn moon mdmoon 3000 0 4300 0 0".
18. Scene "spawn fr ffreight 3100 0 4150 0 1".
19. Scene "engines fr off".
20. Scene "energy fr on".
21. Scene "camera 3200 90 4040 3030 0 4260".
22. Wait 10 seconds.
23. Expect scene "special fr gmineDilithium moon" answers ", fired".
24. Scene "pause".
25. Take a screenshot called "mining-beam".
26. Scene "resume".
27. Expect "Render3D.log" contains "drawing special minedilithium (Beam)".
28. Scene "goto fd 3000 0 4900".
29. Scene "goto kd 3600 0 3900".
30. Wait 3 seconds.
31. Expect scene "special fd gamine" answers ", fired".
32. Expect scene "special kd ggrvmine" answers ", fired".
33. Scene "camera 3300 450 3200 3200 0 3700".
34. Wait 4 seconds.
35. Take a screenshot called "mines".
36. Scene "pause".
37. Scene "camera 3387 45 3550 3387 3 3610".
38. Take a screenshot called "gravity-mine".
39. Scene "camera 2947 45 3583 2947 0 3643".
40. Take a screenshot called "antimatter-mine".
41. Scene "resume".
42. Expect "Render3D.log" contains "slot 11 (Entity::Render) of vtable 0x6b78bc (amine.odf) is ours".
43. Expect "Render3D.log" contains "drawing the model of amine.odf".
44. Expect "Render3D.log" contains "drawing the model of grvmine.odf".
45. Expect scene "special kc gkcteam tgt" answers ", fired".
46. Expect scene "special rc gmyotron tgt" answers ", fired".
47. Expect scene "special fc gchainp tgt" answers ", fired".
48. Wait 1 second.
49. Scene "pause".
50. Scene "camera 3000 700 2500 3000 0 3100".
51. Take a screenshot called "shots".
52. Scene "resume".
53. Expect "Render3D.log" contains "drawing the model of kcteam.odf".
54. Expect "Render3D.log" contains "drawing the model of myotron.odf".
55. Expect "Render3D.log" contains "chainp.odf (vtable 0x6b7c74, Submit 0x574f20): Stock".
56. Scene "hud on".
57. Scene "camera rts".
58. Scene "center tgt".
59. Expect scene "special kc gkcteam tgt" answers ", fired".
60. Wait 5 seconds.
61. Expect "Render3D.log" contains "inset view".
62. Take a screenshot called "hud-and-inset".
63. Wait 30 seconds.
64. Expect the game is still running.
65. Quit the game.
66. Expect no crash.
