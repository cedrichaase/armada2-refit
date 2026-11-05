# Locked campaigns on the campaign selection, and the Klingons opening as the Federation campaign is won

Resolutions: 1280x720
Mod: refit
Setup: ui/bench-progress.sh 1=9 --scene ending
Timeout: 15 min

`ui/MENUS.md`, "Locked campaigns" (issue #13); `render3d/README.md`, "The menu scene".
Run it with `--install .`: it needs this checkout's `UI.asi` and `Render3D.asi`
(`Render3D_MenuLocked`). The clone's `save\shell.set` is a new profile's with the
Federation campaign at its last mission (`ui/bench-progress.sh 1=9`): the Klingon and
Borg campaigns are locked, their rows greyed with a padlock, the Klingon ships stay
cloaked and the Borg sit down in the gas; a click on a locked row reaches the stock
panel, which does nothing. The last Federation mission, "There and Back Again", is
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
10. Check that in the column at the left the Klingon Campaign and Borg Campaign rows are grey with a small padlock after the name, the pointed-at Klingon row has a grey bar and the line "Finish the Federation Campaign to unlock" under its name, and the scene shows a red nebula with no ships in it.
11. Move the mouse to 220,426.
12. Wait 6 seconds.
13. Take a screenshot called "borg locked".
14. Check that the Borg Campaign row is grey with a padlock and the line "Finish the Klingon Campaign to unlock", and the scene shows a dark Borg cube half hidden in purple gas, with no large ship close to the camera.
15. Click at 220,374.
16. Wait 3 seconds.
17. Expect "SELECT MISSION" is not visible.

## The Federation campaign won
18. Click at 220,322.
19. Wait 3 seconds.
20. Click "There and Back Again".
21. Wait 1 second.
22. Click "O.K.".
23. Wait up to 300 seconds for the briefing.
24. Wait 3 seconds.
25. Click "OK".
26. Wait up to 10 seconds for the logs the next steps expect.
27. Expect "UI.log" contains "a2_fed10".
28. Scene "succeed".
29. Wait up to 60 seconds for "ADMIRAL'S LOG".
30. Click "Done".
31. Wait up to 60 seconds for "Klingon Campaign".
32. Wait up to 10 seconds for the logs the next steps expect.
33. Expect "UI.log" contains "complete [0, 1, 0, 0]; locked: borg (waits for klingon)".
34. Expect "Render3D.log" contains "menu: locked views: borg".

## Open
35. Move the mouse to 220,374.
36. Wait 6 seconds.
37. Take a screenshot called "klingon open".
38. Check that the Klingon Campaign row is lit in white with an amber bar and no padlock, the Borg Campaign row is still grey with a padlock, and the scene shows dark green Klingon warships (a large one near the centre, Birds of Prey) in front of a red nebula.
39. Click at 220,374.
40. Wait 3 seconds.
41. Expect "A Dagger At Zarush" is visible.
42. Click "Cancel".
43. Wait 2 seconds.
44. Click "Previous Menu".
45. Wait for the main menu.
46. Quit the game.
47. Expect no crash.
