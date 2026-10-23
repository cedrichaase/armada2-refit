# Ships blow up as fireballs of gas and shock rings (Render3D.asi)

Resolutions: 16:9
Mod: refit
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh explosion
Timeout: 8 min

`render3d/README.md`, "effects" (explosions). Run it on a checkout: `./a2test run explosions --install .`
(the scene comes from `Setup:`). The scene (`testbench/scene/scenes/explosion.ini`) is a
Galaxy class beside a destroyer (`small`), a cruiser (`mid`) and a battleship (`big`);
`kill` makes one die at once. The cruiser's fireball and its shock ring are drawn by the
effects module.

1. Launch the game.
2. Wait up to 50 seconds for the logs the next steps expect.
3. Expect "Render3D.log" contains "module effects: on".
4. Expect "Render3D.log" not contain "FAILED: no fireballs".
5. Expect "Scene.log" contains "scene ready".
6. Scene "orbit 2045 0 2190 60 25 520".
7. Wait 2 seconds.
8. Scene "kill mid".
9. Wait 1 seconds.
10. Scene "pause".
11. Take a screenshot called "fireball".
12. Expect "Render3D.log" contains "effects: fireball at".
13. Expect a turbulent orange fireball of gas, not a cluster of flat sprites, where the cruiser was, and a broad orange band of fire around it, dark inside, as stock's shockwave disc.
14. Scene "resume".
15. Scene "kill small".
16. Scene "kill big".
17. Wait 6 seconds.
18. Take a screenshot called "after".
19. Expect the game is still running.
20. Expect no crash.
