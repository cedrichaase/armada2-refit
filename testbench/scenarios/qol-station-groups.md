# Stations in control groups, and the build menu for several (QOL.asi)

Resolutions: 16:9
Mod: refit
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh stations
Timeout: 10 min

`qol/README.md`, QOL-5 and QOL-6. Run it on a checkout: `./a2test run
qol-station-groups --install .` (the scene comes from `Setup:`). The scene
(`testbench/scene/scenes/stations.ini`) is three shipyards (`yard1..3`), an advanced
shipyard (`adv`), a research station and two ships, all the player's. `Scene "select …"`
selects as a click and Shift-clicks do (`cOverViewImp::Select`); the group keys are
real key presses. `selection` answers with what is selected, each one's group label
(`g`), queue (`q`) and class, and every group that is not empty.

1. Launch the game.
2. Wait 50 seconds.
3. Expect "QOL.log" contains "stations of one kind select and group together, patched".
4. Expect "Scene.log" contains "scene ready".
5. Scene "select yard1 yard2 yard3".
6. Expect scene "selection" answers "selected 3: yard1".
7. Scene "select yard1 adv".
8. Expect scene "selection" answers "selected 1: adv".
9. Scene "select ship1 yard1".
10. Expect scene "selection" answers "selected 1: ship1".
11. Note "Ctrl+1 on three yards; recall from a ship".
12. Scene "select yard1 yard2 yard3".
13. Press ctrl+1.
14. Scene "select ship2".
15. Press 1.
16. Expect scene "selection" answers "selected 3: yard1[g1".
17. Note "Ctrl+1 on a ship empties the yards' group and clears their labels".
18. Scene "select ship1".
19. Press ctrl+1.
20. Expect scene "selection" not answer "station group 1".
21. Expect scene "select yard1" answers "yard1[g-1".
22. Note "Shift+2 refuses a ship and another kind of station, and takes a third yard".
23. Scene "select yard1 yard2".
24. Press ctrl+2.
25. Scene "select ship2".
26. Press shift+2.
27. Scene "select adv".
28. Press shift+2.
29. Scene "select yard3".
30. Press shift+2.
31. Expect scene "selection" answers "station group 2 (3): yard1 yard2 yard3".
32. Note "2 twice centres the camera on the yard nearest the group's middle, yard2 at x 1700".
33. Scene "center ship1".
34. Press 2 2 times.
35. Wait 1 second.
36. Expect scene "query" answers "camera rts eye 1700.0".
37. Note "five orders with three yards selected go 2 + 2 + 1; a cancel goes to the longest queue".
38. Scene "select yard1 yard2 yard3".
39. Press q.
40. Press x 5 times.
41. Expect scene "selection" answers "yard1[g2 q2".
42. Expect scene "selection" answers "yard2[g2 q2".
43. Expect scene "selection" answers "yard3[g2 q1".
44. Press t.
45. Expect scene "selection" answers "yard1[g2 q1".
46. Expect scene "selection" answers "yard2[g2 q2".
47. Expect the game is still running.
48. Quit the game.
49. Expect no crash.
