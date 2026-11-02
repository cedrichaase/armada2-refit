# QOL.asi settings follow RefitPlayer.ini while the game runs

Resolutions: 16:9
Mod: refit
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh stations
Timeout: 10 min

`qol/README.md`, "Settings live". Run it on a checkout: `./a2test run qol-settings-live
--install .`. The scene is the one of `qol-station-groups` (shipyards `yard1..3`, ships
`ship1`, `ship2`). The patches stay installed behind flags, so a key of `QOL.ini` flipped
in `RefitPlayer.ini` (`[QOL.QOL]`, written here by `a2test drive overlay`) applies at
once: UI.asi's settings hub sees the overlay change, calls `QOL_SettingsChanged`, and
QOL.log says `settings: applied QOL.<Key>=<value>`. Checked by those lines and by what
the groups hold. `MaxSelection` is `restart` and is not flipped. The `Overlay` step is
`./a2test drive overlay` as a scenario step (`testbench/scenarios/README.md`); `Overlay
--unset Section.Name:Key` removes a key.

1. Launch the game.
2. Wait up to 50 seconds for the logs the next steps expect.
3. Expect "QOL.log" contains "Shift+N adds to group N, Alt+N selects and centres, patched".
4. Expect "QOL.log" contains "stations of one kind select and group together, patched".
5. Expect "Scene.log" contains "scene ready".
6. Note "ShiftAddsToGroup on: Shift+1 adds the second ship to group 1".
7. Scene "select ship1".
8. Press ctrl+1.
9. Scene "select ship2".
10. Press shift+1.
11. Expect scene "selection" answers "ship group 1 (2): ship1 ship2".
12. Note "ShiftAddsToGroup off, live: Shift+2 no longer binds".
13. Overlay QOL.QOL:ShiftAddsToGroup=0
14. Wait up to 10 seconds for the logs the next steps expect.
15. Expect "QOL.log" contains "settings: applied QOL.ShiftAddsToGroup=0".
16. Scene "select ship1".
17. Press ctrl+2.
18. Scene "select ship2".
19. Press shift+2.
20. Expect scene "selection" answers "ship group 2 (1): ship1".
21. Note "back on, live".
22. Overlay --unset QOL.QOL:ShiftAddsToGroup
23. Wait up to 10 seconds for the logs the next steps expect.
24. Expect "QOL.log" contains "settings: applied QOL.ShiftAddsToGroup=1".
25. Scene "select ship2".
26. Press shift+2.
27. Expect scene "selection" answers "ship group 2 (2): ship1 ship2".
28. Note "StationGroups on: two yards of one kind select together".
29. Scene "select yard1 yard2".
30. Expect scene "selection" answers "selected 2: yard1".
31. Note "StationGroups off, live: a second station replaces the first, as in stock".
32. Overlay QOL.QOL:StationGroups=0
33. Wait up to 10 seconds for the logs the next steps expect.
34. Expect "QOL.log" contains "settings: applied QOL.StationGroups=0".
35. Scene "select yard1 yard2".
36. Expect scene "selection" answers "selected 1: yard2".
37. Note "back on, live".
38. Overlay --unset QOL.QOL:StationGroups
39. Wait up to 10 seconds for the logs the next steps expect.
40. Expect "QOL.log" contains "settings: applied QOL.StationGroups=1".
41. Scene "select yard1 yard2".
42. Expect scene "selection" answers "selected 2: yard1".
43. Note "PanSpeed 1 (stock), live, then 4".
44. Overlay QOL.QOL:PanSpeed=1
45. Wait up to 10 seconds for the logs the next steps expect.
46. Expect "QOL.log" contains "settings: applied QOL.PanSpeed=1 (coefficient".
47. Overlay QOL.QOL:PanSpeed=4
48. Wait up to 10 seconds for the logs the next steps expect.
49. Expect "QOL.log" contains "settings: applied QOL.PanSpeed=4 (coefficient".
50. Note "WarpDistance 0 and ViewDistance 0, live".
51. Overlay QOL.QOL:WarpDistance=0
52. Overlay QOL.QOL:ViewDistance=0
53. Wait up to 10 seconds for the logs the next steps expect.
54. Expect "QOL.log" contains "QOL.WarpDistance=0".
55. Expect "QOL.log" contains "QOL.ViewDistance=0".
56. Expect "QOL.log" contains "settings: RefitPlayer.ini [QOL.QOL] PanSpeed=4 overrides QOL.ini's PanSpeed=2.5".
57. Expect the game is still running.
58. Quit the game.
59. Expect no crash.
