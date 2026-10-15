# Venting plasma and gas, drawn by Render3D.asi

Resolutions: 16:9
Mod: refit
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh venting
Timeout: 6 min

`render3d/README.md`, "particles" (vents). Run it on a checkout: `./a2test run venting --install .`. The
`venting` scene: a Galaxy class and a Klingon battleship, healed and immortal, with
every system's damage node showing (`Vent=all`), so the Galaxy's nacelle vents plasma
and its hull gas, as a damaged ship's do. The log lines check that the module is on and that vents
were found; the shots are for judging the look: a flame-coloured plume with
billows, white-hot where it leaves the nacelle and cooling to orange and red, a glow at
the vent and the hull about it lit warm, where stock has a stream of smoke sprites; a
blue vapour jet where stock has blue-grey puffs; and on the moving ship one unbroken
trail from each vent.

1. Launch the game.
2. Wait 50 seconds.
3. Expect "Scene.log" contains "scene ready".
4. Expect "Render3D.log" contains "module particles: on".
5. Scene "orbit galaxy 200 15 120".
6. Wait 3 seconds.
7. Take a screenshot called "galaxy-parked".
8. Scene "orbit galaxy 230 -10 110".
9. Wait 2 seconds.
10. Take a screenshot called "galaxy-below".
11. Scene "orbit klingon 200 15 160".
12. Wait 2 seconds.
13. Take a screenshot called "klingon-parked".
14. Scene "goto galaxy 2000 0 3500".
15. Scene "orbit galaxy 120 20 150".
16. Wait 5 seconds.
17. Take a screenshot called "galaxy-moving".
18. Scene "orbit galaxy 200 45 700".
19. Wait 2 seconds.
20. Take a screenshot called "far".
21. Expect the game is still running.
22. Quit the game.
23. Expect no crash.
