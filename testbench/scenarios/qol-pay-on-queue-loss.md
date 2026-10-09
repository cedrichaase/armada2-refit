# Pay when queuing: refunds when the building is lost, and across a save

Resolutions: 16:9
Mod: refit
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh stations
Timeout: 10 min

`qol/README.md`, QOL-7; the cases `qol-pay-on-queue` leaves open. Run it on a checkout:
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
14. Expect "QOLRules.log" contains "destroyed dil 225 met 0 lat 0 bio 0 crew 250 off 8; bank dil 3050 crew 250".
15. Expect "QOLRules.log" contains "destroyed dil 225 met 0 lat 0 bio 0 crew 250 off 8; bank dil 3500 crew 750".
16. Note "the destroyed yard's team is cleared after; that must not give the ship in progress back a second time (qol 1.6.1)".
17. Wait 5 seconds.
18. Expect "QOLRules.log" not contain "lost".
19. Expect "QOLRules.log" not contain "bank dil 3725".
20. Note "captured or assimilated: three ships in yard2, then yard2 goes to team 2".
21. Scene "select yard2".
22. Press q.
23. Wait 1 second.
24. Press x 3 times.
25. Expect scene "selection" answers "yard2[g-1 q3".
26. Scene "team yard2 2".
27. Wait 3 seconds.
28. Expect "QOLRules.log" contains "lost dil 225 met 0 lat 0 bio 0 crew 250 off 8; bank dil 3050 crew 250".
29. Expect "QOLRules.log" contains "lost dil 225 met 0 lat 0 bio 0 crew 250 off 8; bank dil 3500 crew 750".
30. Expect "QOLRules.log" not contain "bank dil 3725".
31. Note "save and load: three ships in yard3, saved; loaded; all three cancelled must give back all three".
32. Scene "select yard3".
33. Press q.
34. Wait 1 second.
35. Press x 3 times.
36. Expect scene "selection" answers "yard3[g-1 q3".
37. Press Escape.
38. Wait for the options menu.
39. Click "Save Game".
40. Wait 2 seconds.
41. Click at design 400,518.
42. Type "paybench".
43. Click at design 400,579.
44. Wait for "Save Successful".
45. Click "O.K.".
46. Click "Previous Menu".
47. Wait for the options menu.
48. Click "Load Game".
49. Wait 2 seconds.
50. Click "paybench".
51. Click at design 400,579.
52. Wait for "abort the mission".
53. Click "Yes".
54. Wait 10 seconds.
55. Scene "select yard3".
56. Note "the loaded queue: three ships, the bank as saved".
57. Expect scene "selection" answers "yard3[g-1 q3".
58. Press q.
59. Wait 1 second.
60. Press t 3 times.
61. Wait 2 seconds.
62. Expect scene "selection" answers "yard3[g-1 q0".
63. Note "each cancel gives one ship back; the next then starts without a charge, which with no crew left only a paid item can".
64. Expect "QOLRules.log" contains "started dil 225 met 0 lat 0 bio 0 crew 250 off 8; bank dil 3050 crew 250".
65. Expect "QOLRules.log" contains "cancelled in progress dil 225 met 0 lat 0 bio 0 crew 250 off 8; bank dil 3500 crew 750".
66. Expect the game is still running.
67. Quit the game.
68. Expect no crash.
