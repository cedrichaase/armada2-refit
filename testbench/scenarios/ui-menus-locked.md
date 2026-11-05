# Locked campaigns on the campaign selection, and the Klingons opening as the Federation campaign is won

Resolutions: 1280x720
Mod: refit
Setup: ui/bench-progress.sh 1=9 --scene ending
Timeout: 15 min

`ui/MENUS.md`, "Locked campaigns" (issue #13); `render3d/README.md`, "The menu scene".
Run it with `--install .`: it needs this checkout's `UI.asi` and `Render3D.asi`
(`Render3D_MenuLocked`). The clone's `save\shell.set` is a new profile's with the
Federation campaign at its last mission (`ui/bench-progress.sh 1=9`): the Klingon and
Borg campaigns are locked, their rows greyed with a padlock, the Klingon ships are held
cloaked (drawn in the cloak's glass, never decloaking, so no decloak sound) and the Borg
sit down in the gas, the cube turned a corner towards the camera (`LockedYaw=`); a
click on a locked row plays `[Menus] LockedSound` (logged: the bench has no audio) and
reaches the stock panel, which does nothing. The last Federation mission, "There and Back Again", is
then won by `Scene.asi`'s `succeed`: the game marks the campaign complete in memory
(the file is written only when the game exits), the UI reads it there, and back on the
campaign selection the Klingon row is open, its ships decloak and a click opens its
mission list. The Borg stay locked.

Clicks by place (1280x720 only): the main menu's Single Player and the column's rows
(Federation 322, Klingon 374, Borg 426).

## Locked
1. Launch the game.
2. Wait for the main menu.
3. Click at 228,258.
4. Wait up to 10 seconds for the logs the next steps expect.
5. Expect "UI.log" contains "menus: campaign progress from memory: reached [0, 9, 0, 0], complete [0, 0, 0, 0]; locked: klingon (waits for federation) borg (waits for klingon)".
6. Expect "Render3D.log" contains "menu: locked views: klingon,borg".
7. Move the mouse to 220,374.
8. Wait 6 seconds.
9. Take a screenshot called "klingon locked".
10. Check that in the column at the left the Klingon Campaign and Borg Campaign rows are grey with a small padlock after the name, the pointed-at Klingon row has a grey bar and the line "Complete the Federation Campaign to unlock" under its name, and the scene shows a red nebula with the Klingon warships in it as cloaked glass: faint shapes that bend and colour-fringe the gas behind them, with a thin bright rim, not solid dark green hulls and not an empty nebula.
11. Expect "Render3D.log" not contains "menu: decloak sound".
12. Move the mouse to 220,426.
13. Wait 6 seconds.
14. Take a screenshot called "borg locked".
15. Check that the Borg Campaign row is grey with a padlock and the line "Complete the Klingon Campaign to unlock", and the scene shows a dark Borg cube half hidden in purple gas, turned a corner towards the camera (two faces meet down its middle, the left one darker, rather than one flat square face), with no large ship close to the camera.
16. Click at 220,374.
17. Wait 3 seconds.
18. Expect "SELECT MISSION" is not visible.
19. Expect "UI.log" contains "menus: locked campaign klingon clicked: sound mnu_back.wav".

## The Federation campaign won
20. Click at 220,322.
21. Wait 3 seconds.
22. Click "There and Back Again".
23. Wait 1 second.
24. Click "O.K.".
25. Wait up to 300 seconds for the briefing.
26. Wait 3 seconds.
27. Click "OK".
28. Wait up to 10 seconds for the logs the next steps expect.
29. Expect "UI.log" contains "a2_fed10".
30. Scene "succeed".
31. Wait up to 60 seconds for "ADMIRAL'S LOG".
32. Click "Done".
33. Wait up to 60 seconds for "Klingon Campaign".
34. Wait up to 10 seconds for the logs the next steps expect.
35. Expect "UI.log" contains "complete [0, 1, 0, 0]; locked: borg (waits for klingon)".
36. Expect "Render3D.log" contains "menu: locked views: borg".

## Open
37. Move the mouse to 220,374.
38. Wait 6 seconds.
39. Take a screenshot called "klingon open".
40. Check that the Klingon Campaign row is lit in white with an amber bar and no padlock, the Borg Campaign row is still grey with a padlock, and the scene shows dark green Klingon warships (a large one near the centre, Birds of Prey) in front of a red nebula.
41. Click at 220,374.
42. Wait 3 seconds.
43. Expect "A Dagger At Zarush" is visible.
44. Click "Cancel".
45. Wait 2 seconds.
46. Click "Previous Menu".
47. Wait for the main menu.
48. Quit the game.
49. Expect no crash.
