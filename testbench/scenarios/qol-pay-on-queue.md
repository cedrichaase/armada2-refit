# Pay when queuing, refund on cancel (QOLRules.asi, PayOnQueue=)

Resolutions: 16:9
Mod: refit
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh stations
Timeout: 8 min

`qol/README.md`, QOL-7. Run it on a checkout: `./a2test run qol-pay-on-queue --install .`.
The scene is the `stations` one (`yard1`, a shipyard, with the bank the mission starts
with: 3500 dilithium, 750 crew). Press `q` opens the yard's build menu; in it `x` queues the
Federation's big ship (225 dilithium, 250 crew, 8 officers) and `t` cancels the last
waiting item. `QOLRules.log` writes every payment, refusal and refund with the bank
after it, which is what the steps read.

1. Launch the game.
2. Wait 50 seconds.
3. Expect "QOLRules.log" contains "items are paid when queued".
4. Expect "Scene.log" contains "scene ready".
5. Note "three orders for a ship the bank can cover: each leaves the bank at once, the start takes nothing more".
6. Scene "select yard1".
7. Press q.
8. Press x 3 times.
9. Expect "QOLRules.log" contains "paid dil 225 met 0 lat 0 bio 0 crew 250 off 8; bank dil 3275 crew 500".
10. Expect "QOLRules.log" contains "started dil 225 met 0 lat 0 bio 0 crew 250 off 8; bank dil 3275 crew 500".
11. Expect "QOLRules.log" contains "paid dil 225 met 0 lat 0 bio 0 crew 250 off 8; bank dil 2825 crew 0".
12. Expect scene "selection" answers "yard1[g-1 q3".
13. Note "a fourth is refused: the crew is spent. Nothing is queued and the bank stays".
14. Press x.
15. Expect "QOLRules.log" contains "refused dil 225 met 0 lat 0 bio 0 crew 250 off 8; bank dil 2825 crew 0".
16. Expect scene "selection" answers "yard1[g-1 q3".
17. Note "a cancel takes the item in progress first, as stock does, and gives it back whole: the crew too, which stock's own refund clamps to a cap".
18. Press t.
19. Expect scene "selection" answers "yard1[g-1 q2".
20. Expect "QOLRules.log" contains "cancelled in progress dil 225 met 0 lat 0 bio 0 crew 250 off 8; bank dil 3050 crew 250 officers free 540".
21. Note "the next item has started by now (a start takes nothing more), so this cancel is an in-progress one too. A waiting item, cancelled before its start, logs plain `cancelled`".
22. Press t.
23. Expect "QOLRules.log" contains "started dil 225 met 0 lat 0 bio 0 crew 250 off 8; bank dil 3050 crew 250 officers free 540".
24. Expect "QOLRules.log" contains "cancelled in progress dil 225 met 0 lat 0 bio 0 crew 250 off 8; bank dil 3275 crew 500 officers free 548".
25. Expect scene "selection" answers "yard1[g-1 q1".
26. Note "the last one is in progress by now; cancelling it leaves the bank as it was".
27. Press t.
28. Expect "QOLRules.log" contains "cancelled in progress dil 225 met 0 lat 0 bio 0 crew 250 off 8; bank dil 3500 crew 750 officers free 556".
29. Expect scene "selection" answers "yard1[g-1 q0".
30. Expect the game is still running.
31. Quit the game.
32. Expect no crash.
