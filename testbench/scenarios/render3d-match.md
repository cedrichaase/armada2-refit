# A real match on render3d, with a battle staged in it

Resolutions: 16:9
Mod: refit
Setup: testbench/scene/bench-setup.sh match
Timeout: 15 min

`render3d/README.md`. Run it on a checkout: `./a2test run render3d-match --install .`.
An Instant Action match on Warzone as the Borg against an easy AI (`_enter-instant-action`),
drawn by `Render3D.asi` with every module on (`./install` takes the plugins they replaced
out). The easy AI leaves the player alone, so `Scene.asi` (scene `match`: it builds and
hides nothing) stages a battle beside the Borg start on Warzone (about 16900, 0, 9000): two Federation ships and two Klingon
battleships attack each other, mortal, so ships die, burn and break up. The fog of war is off (it has its own scenario, `fog-edges`);
the HUD and its action camera, the minimap and the cursor are the game's own. The log
checks that each module is on and nothing panicked; the shots are for judging the
whole picture: hulls, weapons, shields, explosions, debris, vents of damaged ships, the
sky, fog and selection, in the main view and the inset.

1. Include "_enter-instant-action".
2. Wait 20 seconds.
3. Expect "Render3D.log" contains "module world: on".
4. Expect "Render3D.log" contains "module hulls: on".
5. Expect "Render3D.log" contains "module overlays: on".
6. Take a screenshot called "base".
7. Scene "spawn g1 fgalaxy 17300 0 9300 90 1; spawn g2 fbattle 17250 0 9050 90 1".
8. Scene "spawn k1 kbattle 18050 0 9250 270 2; spawn k2 kbattle 18100 0 9000 270 2".
9. Scene "attack g1 k1; attack g2 k2; attack k1 g1; attack k2 g2".
10. Scene "hud on; orbit g1 200 30 900".
11. Wait 25 seconds.
12. Take a screenshot called "battle".
13. Scene "orbit g1 200 25 500".
14. Wait 20 seconds.
15. Take a screenshot called "battle-close".
16. Wait 60 seconds.
17. Take a screenshot called "aftermath".
18. Scene "camera rts".
19. Wait 30 seconds.
20. Take a screenshot called "base-later".
21. Expect "Render3D.log" not contains "PANIC".
22. Expect the game is still running.
23. Quit the game.
24. Expect no crash.
