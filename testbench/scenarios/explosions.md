# Ships blow up as fireballs of gas and shock rings (Explosions.asi)

Resolutions: 16:9
Mod: refit
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh explosion
Timeout: 8 min

`explosions/README.md`. Run it on a checkout: `./a2test run explosions --install .`
(the scene comes from `Setup:`). The scene (`testbench/scene/scenes/explosion.ini`) is a
Galaxy class beside a destroyer (`small`), a cruiser (`mid`) and a battleship (`big`);
`kill` makes one die at once. The cruiser's fireball and its shock ring are drawn by the
plugin; `Lighting.asi`'s explosion light, which wraps two other slots of the same vtable,
must still be patched beside it.

1. Launch the game.
2. Wait 50 seconds.
3. Expect "Explosions.log" contains "FireballExplosion::Submit and RenderChunks patched, ShockwaveExplosion::Submit too".
4. Expect "Lighting.log" not contain "NOT PATCHED: FireballExplosion".
5. Expect "Scene.log" contains "scene ready".
6. Scene "orbit 2045 0 2190 60 25 520".
7. Wait 2 seconds.
8. Scene "kill mid".
9. Wait 1 seconds.
10. Scene "pause".
11. Take a screenshot called "fireball".
12. Expect "Explosions.log" contains "first fireball".
13. Expect "Explosions.log" contains "first ring".
14. Expect a turbulent orange fireball of gas, not a cluster of flat sprites, where the cruiser was, and a thin ring of light around it.
15. Scene "resume".
16. Scene "kill small".
17. Scene "kill big".
18. Wait 6 seconds.
19. Take a screenshot called "after".
20. Expect the game is still running.
21. Expect no crash.
