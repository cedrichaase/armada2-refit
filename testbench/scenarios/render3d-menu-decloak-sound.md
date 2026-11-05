# The menu scene: the Klingon group's decloak sound plays once per arrival

Resolutions: 1280x720
Mod: refit
Setup: ui/bench-asi.sh --menus
Timeout: 12 min

`render3d/README.md`, "The menu scene"; issue #12. The campaign menu's Klingon group
decloaks its ships one after another as the camera arrives; its `CloakSound` is heard
once per arrival, not once per ship. Nothing else shows the sound, so `Render3D.log`
carries one line per sound played, numbered across the run: `menu: decloak sound N
<wav> (group <name>)`. The first arrival must write number 1 and no number 2; leaving
the view and coming back writes number 2 and no number 3. Whether it is *heard* is for a
person: the bench has no audio.

1. Launch the game.
2. Wait for the main menu.
3. Expect "Render3D.log" contains "menu: map (built in)".
4. Expect "UI.log" contains "menus: scene main on".
5. Click the single player emblem.
6. Wait 3 seconds.
7. Expect "UI.log" contains "menus: campaign emblems from allrace.tga".
8. Move the mouse to 220,374.
9. Wait up to 20 seconds for the logs the next steps expect.
10. Expect "Render3D.log" contains "menu: view klingon".
11. Expect "Render3D.log" contains "menu: decloak sound 1 "
12. Wait 6 seconds.
13. Expect "Render3D.log" not contains "menu: decloak sound 2 "
14. Move the mouse to 220,322.
15. Wait up to 8 seconds for the logs the next steps expect.
16. Expect "Render3D.log" contains "menu: view federation"
17. Wait 8 seconds.
18. Move the mouse to 220,374.
19. Wait up to 20 seconds for the logs the next steps expect.
20. Expect "Render3D.log" contains "menu: view klingon"
21. Expect "Render3D.log" contains "menu: decloak sound 2 "
22. Wait 6 seconds.
23. Expect "Render3D.log" not contains "menu: decloak sound 3 "
24. Click the previous menu.
25. Wait 3 seconds.
26. Expect the game is still running.
27. Quit the game.
28. Expect no crash.
