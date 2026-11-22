# Fleet Operations: Roots with the UI: the tutorial's briefing and objectives in a box of ours at its own proportions

Resolutions: 2150x900
Launch: -nointro tutorialgameplay.bzn
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh HUD.Minimap=1 HUD.Info=1 HUD.UnitView=1 HUD.Tooltips=1 HUD.Bar=1 HUD.BarLabels=1 HUD.InfoCamera=1 HUD.Messages=1 HUD.LayoutLog=1
Timeout: 16 min

The B side of `fo-ui-briefing` (PLAN-FLEETOPS.md, decisions 322 onward): with `[HUD]
Messages=1`, Roots' default since then, the briefing and objectives and the script's
message boxes are ours (`hud/objectives.rs`, `hud/messages.rs`), laid out in the UI's own
units and so keep their proportions on a wide screen, over a dimmed screen that takes the mouse;
the stock box is shown and hidden by the engine as before and only not drawn. The panels
of the player's `RefitPlayer.ini` are on, as in the user's report. The game starts straight in the
mission (`Launch: -nointro tutorialgameplay.bzn`; `fo-ui-briefing` keeps the menu walk). The tutorial's script raises the briefing by itself about 25 seconds
in; the objectives key then opens it again, Return closes it, and sending the Enterprise
through the wormhole ticks the first objective (the script shows the box again, ticked),
and after OK the script adds the second objective and shows the box once more. This
tutorial raises no message box (`MessageDisplay`): its lessons are objectives.

1. Launch the game.
2. Wait up to 180 seconds for "Render3D.log" to say "fed_sovereignent at".
3. Wait 8 seconds.
4. Wait up to 60 seconds for "UI.log" to say "objectives: ".
5. Wait 2 seconds.
6. Take a screenshot called "briefing".
7. Check that the briefing is a window of ours in the middle of a dimmed screen, not Roots' metal frame: the title "TUTORIAL: GAMEPLAY", a briefing summary header and its text ("This tutorial will teach you core gameplay elements"), an objectives header, the objective "Send the Enterprise through the blue wormhole" with an unticked check box, and an OK button; its letters keep their natural proportions.
8. Expect "UI.log" contains "panels: ObjectivesDisplay ours".
9. Expect "UI.log" contains "[ ] Send the Enterprise through".
10. Expect "UI.log" contains "OK found".
11. Expect "UI.log" contains "panels: the game paused".
12. Click at 300,300.
13. Wait 2 seconds.
14. Take a screenshot called "clicked outside".
15. Check that the briefing box of ours is still shown: a click on the dimmed map beside it did not close it.
16. Click "OK".
17. Wait 3 seconds.
18. Take a screenshot called "closed".
19. Check that the briefing box is gone and the panels of ours (the resource strip, the minimap, the top-right buttons) are drawn.
20. Expect "UI.log" contains "panels: the game runs".
21. Press Alt+o.
22. Wait 3 seconds.
23. Take a screenshot called "objectives key".
24. Check that the briefing box of ours is shown again (the objectives key opens it).
25. Press Return.
26. Wait 3 seconds.
27. Take a screenshot called "return".
28. Check that the briefing box is gone (Return closed it, as in stock) and no chat entry line is open.
29. Drag from 1040,240 to 1190,380.
30. Wait 2 seconds.
31. Right-click at 1093,485.
32. Wait up to 180 seconds for "UI.log" to say "[X] Send the Enterprise".
33. Wait 2 seconds.
34. Take a screenshot called "objective done".
35. Check that the box of ours shows the objectives again with "Send the Enterprise through the blue wormhole" ticked (its check box on).
36. Expect "UI.log" contains "[X] Send the Enterprise".
37. Click "OK".
38. Wait up to 120 seconds for "UI.log" to say "2 objectives:".
39. Wait 2 seconds.
40. Take a screenshot called "next objective".
41. Check that the box of ours shows two objectives: "Send the Enterprise through the blue wormhole" ticked and "Explore the nebulas, then send the Enterprise back through the wormhole" not ticked.
42. Click "OK".
43. Wait 3 seconds.
44. Take a screenshot called "later".
45. Expect "UI.log" not contain "PANIC".
46. Expect "Display8.log" not contain "VUID".
47. Expect "Display8.log" not contain "device removed".
48. Quit the game.
49. Expect the game to have exited.
50. Expect no crash.
