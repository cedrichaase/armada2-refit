# Venting plasma and gas, drawn by Venting.asi

Resolutions: 16:9
Mod: refit
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh venting
Timeout: 6 min

`venting/README.md`. Run it on a checkout: `./a2test run venting --install .`. The
`venting` scene: a Galaxy class and a Klingon battleship, healed and immortal, with
every system's damage node showing (`Vent=all`), so the Galaxy's nacelle vents plasma
and its hull gas, as a damaged ship's do. The log lines check the hooks and that vents
were drawn and lit; the shots are for judging the look: a flame-coloured plume with
billows, white-hot where it leaves the nacelle and cooling to orange and red, a glow at
the vent and the hull about it lit warm, where stock has a stream of smoke sprites; a
blue vapour jet where stock has blue-grey puffs; and on the moving ship one unbroken
trail from each vent.

1. Launch the game.
2. Wait 50 seconds.
3. Expect "Scene.log" contains "scene ready".
4. Expect "Venting.log" contains "RenderParticle, the spray emitter's RenderInternal".
5. Scene "orbit galaxy 200 15 120".
6. Wait 3 seconds.
7. Expect "Venting.log" contains "plasma vents: sprite node".
8. Expect "Venting.log" contains "gas vents: sprite node".
9. Expect "Venting.log" contains "first vents drawn".
10. Expect "Venting.log" contains "first vent light".
11. Take a screenshot called "galaxy-parked".
12. Scene "orbit galaxy 230 -10 110".
13. Wait 2 seconds.
14. Take a screenshot called "galaxy-below".
15. Scene "orbit klingon 200 15 160".
16. Wait 2 seconds.
17. Take a screenshot called "klingon-parked".
18. Scene "goto galaxy 2000 0 3500".
19. Scene "orbit galaxy 120 20 150".
20. Wait 5 seconds.
21. Take a screenshot called "galaxy-moving".
22. Scene "orbit galaxy 200 45 700".
23. Wait 2 seconds.
24. Take a screenshot called "far".
25. Expect the game is still running.
26. Quit the game.
27. Expect no crash.
