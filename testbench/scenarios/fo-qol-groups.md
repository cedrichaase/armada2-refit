# Fleet Operations: Roots with QOL: Shift+number adds to a control group, beside Roots' own Alt+number

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d qol
Setup: display/bench-mode.sh stub Backend=vulkan
Timeout: 14 min

FO-7 round two (`PLAN-FLEETOPS.md`, "FO-7: QOL on Roots", decision 108). `qol.shift_group` is
verified on this build: `cOverViewImp::mCheckGroupSelect` is stock's in every byte QOL uses, but
Roots jumps out of it before the loop over the ten group keys to take Alt+number itself (select
the group, centre on a second tap), and `mBindGroup` is stock's at its entry with Roots' own
selection array and group cap of 30 behind it. So `ShiftAddsToGroup=1` (QOL.ini's value; the
Roots overlay no longer sets it) has to leave Roots' Alt+number alone and make Shift+number add.
There is no scene harness on Roots, so the ships are picked with the mouse in an Instant Action
game (`_fo-sides`): the player starts with two Mediterranean class construction ships
(`USS Hafner`, one at the left, 335,640, and one below it, 465,760), the Mandril miner
`USS Nome` (880,605), the science ship `USS Shield` (790,725) and a starbase. Selected ships
show a health bar above them.

1. Launch the game.
2. Wait up to 50 seconds for the logs the next steps expect.
3. Expect "QOL.log" contains "build armada2/fo-roots-5.0Z11".
4. Expect "QOL.log" contains "Shift+N adds to group N, Alt+N selects and centres, patched".
5. Expect "QOL.log" not contain "patch refused".
6. Expect "QOL.log" not contain "qol.shift_group stands down".
7. Expect "QOL.log" contains "MaxSelection=16".
8. Wait for the main menu.
9. Include "_fo-sides".
10. Click "LAUNCH".
11. Wait up to 180 seconds for "6000".
12. Wait 5 seconds.
13. Take a screenshot called "start".
14. Note "Ctrl+1 binds the left Hafner; Shift+1 adds the Nome; selecting the other Hafner and pressing 1 recalls both".
15. Click at 335,640.
16. Press ctrl+1.
17. Click at 880,605.
18. Press shift+1.
19. Click at 465,760.
20. Wait 1 second.
21. Take a screenshot called "other ship selected".
22. Press 1.
23. Wait 1 second.
24. Take a screenshot called "group 1 recalled".
25. Check that the ships at about 335,640 (left) and 880,605 (right) each have a health bar above them and the ship at about 465,760 has none, with the HUD showing a selection of more than one ship (not a single ship's name).
26. Note "Alt+4 is Roots' own: it recalls the station group, and empties the selection first (QOL leaves it alone)".
27. Click at 830,380.
28. Press ctrl+4.
29. Click at 790,725.
30. Wait 1 second.
31. Press alt+4.
32. Wait 1 second.
33. Take a screenshot called "alt 4".
34. Check that the starbase (the large round station at about 830,380) is selected, with a health bar above it, and the HUD's selection panel at the bottom names a "Starbase" and not a ship.
35. Note "Shift+2 with the Shield alone binds it; recall brings it back".
36. Click at 790,725.
37. Press shift+2.
38. Click at 465,760.
39. Wait 1 second.
40. Press 2.
41. Wait 1 second.
42. Take a screenshot called "group 2 recalled".
43. Check that the ship at about 790,725 has a health bar above it and the ship at about 465,760 has none.
44. Note "Ctrl+1 replaces: bind the Shield alone, recall".
45. Click at 790,725.
46. Press ctrl+1.
47. Click at 465,760.
48. Wait 1 second.
49. Press 1.
50. Wait 1 second.
51. Take a screenshot called "group 1 replaced".
52. Check that only the ship at about 790,725 has a health bar above it.
53. Expect "QOL.log" not contain "patch refused".
54. Wait 5 seconds.
55. Quit the game.
56. Expect the game to have exited.
57. Expect no crash.
