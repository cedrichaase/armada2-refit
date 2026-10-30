# The last leftovers: systems' icons, shot scenes, the minimap's cursor, the frame rate

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Setup: display/bench-mode.sh forward ui/bench-asi.sh --no-grid --rts FRAMERATE_DISPLAY=1 --scene fog
Timeout: 12 min

`ui/HUD.md`, "What the stock overlay and minimap drew that ours left out, and the
frame rate" (ui 0.25.0). Run it on a checkout, with Display8 counting the engine's
draws: `./a2test run ui-leftovers-2 --install . -j 1`. `ui/bench-asi.sh --rts`
sets `FRAMERATE_DISPLAY = 1` in the clone's `RTS_CFG.h`, so the engine would print its
frame rate at the top left: with `[HUD] FrameRate=1` ours draws it and the engine
draws nothing in the mission.

The scene is `fog`: three of the player's Federation ships, not healed, their weapons
switched off and `ship1` immortal, so that a Klingon battleship attacking it keeps
fighting and its systems are hit. `ship1` is selected, so (with `SelectionGauges=1`)
the overlay shows its damaged systems under its brackets (`[HUD] SystemIcons=1`), as
it does for a pointed unit. With `ship2` and `scout` selected, the cursor over the
minimap is the warp cursor (`MinimapCursor=1`). In the tactical view (Caps Lock) the
minimap rings the fight, the shot director's scene (`ShotScenes=1`). The tactical view
is a display context of its own and is not checked for engine draws here: the
tactical camera's pivot marker there (`TacticalCamera::Render`, the object at `+0xe0`)
is none of these four; render3d draws it, which `render3d-pivot` checks.

At 1280x720 the minimap's square spans about 24..224 x 500..700.

1. Launch the game.
2. Wait up to 50 seconds for the logs the next steps expect.
3. Expect "Scene.log" contains "scene ready".
4. Expect "UI.log" contains "selection: ours".
5. Expect "UI.log" contains "minimap: ours".
6. Expect "UI.log" contains "leftovers: frame rate".
7. Mark the display context "mission".
8. Scene "weapons ship1 off; weapons ship2 off; weapons scout off; immortal ship1 on; cursor on".
9. Scene "spawn bad kbattle 2600 0 2600 0 2".
10. Wait 2 seconds.
11. Scene "attack bad ship1".
12. Scene "center ship1".
13. Scene "select ship1".
14. Move the mouse into the bottom right corner.
15. Wait up to 120 seconds for "UI.log" to say "overlay: systems of Defiant Class".
16. Take a screenshot called "systems".
17. Expect the engine to draw nothing in "mission".
18. Scene "select ship2 scout".
19. Wait 2 seconds.
20. Move the mouse to 120,600.
21. Wait up to 5 seconds for "UI.log" to say "cursor: Mission draws c_warp".
22. Take a screenshot called "minimap cursor".
23. Move the mouse to 640,200.
24. Mark the display context "tactical".
25. Press Caps_Lock.
26. Wait up to 30 seconds for "UI.log" to say "shot scenes ringed".
27. Take a screenshot called "tactical".
28. Press Caps_Lock.
29. Wait 2 seconds.
30. Mark the display context "mission again".
31. Wait 4 seconds.
32. Expect the engine to draw nothing in "mission again".
33. Quit the game.
34. Expect no crash.
