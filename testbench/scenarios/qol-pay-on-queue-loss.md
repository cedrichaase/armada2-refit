# Pay when queuing: refunds when the building is lost, and across a save

Resolutions: 16:9
Mod: refit
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh stations
Timeout: 10 min

`rules/README.md`, QOL-7; the cases `qol-pay-on-queue` leaves open. Run it on a checkout:
`./a2test run qol-pay-on-queue-loss --install .`. The scene is `stations`: three shipyards
and the bank the mission starts with, 3500 dilithium and 750 crew. In a yard's build menu
(`q`) `x` queues the Federation's big ship (225 dilithium, 250 crew, 8 officers) and `t`
cancels the last waiting item. Three of them spend the crew, leaving 2825 dilithium and
no crew. After each loss the bank must be back at 3500 and 750, exactly once.

Destroyed: `damage` (`../scene/README.md`) is a hit on the hull through the engine's own
`DamageHull`, so the engine destroys the yard its own way.

Captured or assimilated: both end the same way for the yard, which changes team once the
boarding has won. `team` makes that change as mission scripts do (the object's own
`SetTeam`); the fight before it is not staged.

Save and load: through the in-mission menu, as a player does. Loading aborts the running
mission, whose yard gives its queue back into a bank the loaded game then replaces (the
three `lost` lines after the save); the loaded bank is the saved one.

1. Launch the game.
2. Wait 50 seconds.
3. Expect "QOLRules.log" contains "items are paid when queued".
4. Expect "Scene.log" contains "scene ready".
5. Note "destroyed: three ships in yard1, then a hit that takes its whole hull".
6. Scene "select yard1".
7. Press q.
8. Wait 1 second.
9. Press x 3 times.
10. Expect scene "selection" answers "yard1[g-1 q3".
11. Expect "QOLRules.log" contains "paid dil 225 met 0 lat 0 bio 0 crew 250 off 8; bank dil 2825 crew 0".
12. Scene "immortal yard1 off; damage yard1 100000".
13. Wait 5 seconds.
14. Expect "QOLRules.log" contains "lost dil 225 met 0 lat 0 bio 0 crew 250 off 8".
15. Expect "QOLRules.log" contains "; bank dil 3500 crew 750".
16. Note "a destroyed producer is refunded once, through ClearTeam; DestroyShip is not hooked, and nothing may give the ship in progress back a second time".
17. Wait 5 seconds.
18. Expect "QOLRules.log" not contain "bank dil 3725".
19. Note "captured or assimilated: three ships in yard2, then yard2 goes to team 2".
20. Scene "select yard2".
21. Press q.
22. Wait 1 second.
23. Press x 3 times.
24. Expect scene "selection" answers "yard2[g-1 q3".
25. Scene "team yard2 2".
26. Wait 3 seconds.
27. Expect "QOLRules.log" contains "lost dil 225 met 0 lat 0 bio 0 crew 250 off 8".
28. Expect "QOLRules.log" contains "; bank dil 3500 crew 750".
29. Expect "QOLRules.log" not contain "bank dil 3725".
30. Note "save and load: three ships in yard3, saved; loaded; all three cancelled must give back all three".
31. Scene "select yard3".
32. Press q.
33. Wait 1 second.
34. Press x 3 times.
35. Expect scene "selection" answers "yard3[g-1 q3".
36. Press Escape.
37. Wait for the options menu.
38. Click "Save Game".
39. Wait 2 seconds.
40. Click at design 400,518.
41. Type "paybench".
42. Click at design 400,579.
43. Wait for "Save Successful".
44. Click "O.K.".
45. Click "Previous Menu".
46. Wait for the options menu.
47. Click "Load Game".
48. Wait 2 seconds.
49. Click "paybench".
50. Click at design 400,579.
51. Wait for "abort the mission".
52. Click "Yes".
53. Wait 10 seconds.
54. Scene "select yard3".
55. Note "the loaded queue: three ships, the bank as saved".
56. Expect scene "selection" answers "yard3[g-1 q3".
57. Press q.
58. Wait 1 second.
59. Press t 3 times.
60. Wait 2 seconds.
61. Expect scene "selection" answers "yard3[g-1 q0".
62. Note "each cancel gives one ship back; the next then starts without a charge, which with no crew left only a paid item can".
63. Expect "QOLRules.log" contains "started dil 225 met 0 lat 0 bio 0 crew 250 off 8; bank dil 3050 crew 250".
64. Expect "QOLRules.log" contains "cancelled in progress dil 225 met 0 lat 0 bio 0 crew 250 off 8; bank dil 3500 crew 750".
65. Expect the game is still running.
66. Quit the game.
67. Expect no crash.
