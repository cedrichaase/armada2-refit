# Fleet Operations: Roots with the UI: the tutorial's briefing and objectives in a box of ours at its own proportions

Resolutions: 2150x900
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh HUD.Minimap=1 HUD.Info=1 HUD.UnitView=1 HUD.Tooltips=1 HUD.Bar=1 HUD.BarLabels=1 HUD.InfoCamera=1 HUD.Messages=1 HUD.LayoutLog=1
Timeout: 16 min

The B side of `fo-ui-briefing` (PLAN-FLEETOPS.md, decisions 322 onward): with `[HUD]
Messages=1`, Roots' default since then, the briefing and objectives and the script's
message boxes are ours (`hud/objectives.rs`, `hud/messages.rs`), laid out in the UI's own
units and so keep their proportions on a wide screen, over a dimmed screen that takes the mouse;
the stock box is shown and hidden by the engine as before and only not drawn. The panels
of the player's `RefitPlayer.ini` are on, as in the user's report. The way in is
`fo-ui-briefing`'s. The tutorial's script raises the briefing by itself about 25 seconds
in; the objectives key then opens it again, Return closes it, and sending the Enterprise
through the wormhole ticks the first objective (the script shows the box again, ticked),
and after OK the script adds the second objective and shows the box once more. This
tutorial raises no message box (`MessageDisplay`): its lessons are objectives.

1. Launch the game.
2. Wait for the main menu.
3. Wait 5 seconds.
4. Move the mouse to 1075,361.
5. Wait 1 second.
6. Click at 1076,363.
7. Wait up to 15 seconds for "Previous Menu".
8. Wait 4 seconds.
9. Move the mouse to 235,225.
10. Wait 1 second.
11. Click at 237,225.
12. Wait up to 20 seconds for "TUTORIAL: Gameplay".
13. Wait 2 seconds.
14. Move the mouse to 640,233.
15. Wait 1 second.
16. Click "TUTORIAL: Gameplay".
17. Wait 1 second.
18. Double-click "TUTORIAL: Gameplay".
19. Wait 2 seconds.
20. Move the mouse to 540,780.
21. Wait 1 second.
22. Take a screenshot called "mission selected".
23. Click "OK".
24. Wait up to 180 seconds for "Render3D.log" to say "fed_sovereignent at".
25. Wait 8 seconds.
26. Wait up to 60 seconds for "UI.log" to say "objectives: ".
27. Wait 2 seconds.
28. Take a screenshot called "briefing".
29. Check that the briefing is a window of ours in the middle of a dimmed screen, not Roots' metal frame: the title "TUTORIAL: GAMEPLAY", a briefing summary header and its text ("This tutorial will teach you core gameplay elements"), an objectives header, the objective "Send the Enterprise through the blue wormhole" with an unticked check box, and an OK button; its letters keep their natural proportions.
30. Expect "UI.log" contains "panels: ObjectivesDisplay ours".
31. Expect "UI.log" contains "[ ] Send the Enterprise through".
32. Expect "UI.log" contains "OK found".
33. Expect "UI.log" contains "panels: the game paused".
34. Click at 300,300.
35. Wait 2 seconds.
36. Take a screenshot called "clicked outside".
37. Check that the briefing box of ours is still shown: a click on the dimmed map beside it did not close it.
38. Click "OK".
39. Wait 3 seconds.
40. Take a screenshot called "closed".
41. Check that the briefing box is gone and the panels of ours (the resource strip, the minimap, the top-right buttons) are drawn.
42. Expect "UI.log" contains "panels: the game runs".
43. Press Alt+o.
44. Wait 3 seconds.
45. Take a screenshot called "objectives key".
46. Check that the briefing box of ours is shown again (the objectives key opens it).
47. Press Return.
48. Wait 3 seconds.
49. Take a screenshot called "return".
50. Check that the briefing box is gone (Return closed it, as in stock) and no chat entry line is open.
51. Drag from 1040,240 to 1190,380.
52. Wait 2 seconds.
53. Right-click at 1093,485.
54. Wait up to 180 seconds for "UI.log" to say "[X] Send the Enterprise".
55. Wait 2 seconds.
56. Take a screenshot called "objective done".
57. Check that the box of ours shows the objectives again with "Send the Enterprise through the blue wormhole" ticked (its check box on).
58. Expect "UI.log" contains "[X] Send the Enterprise".
59. Click "OK".
60. Wait up to 120 seconds for "UI.log" to say "2 objectives:".
61. Wait 2 seconds.
62. Take a screenshot called "next objective".
63. Check that the box of ours shows two objectives: "Send the Enterprise through the blue wormhole" ticked and "Explore the nebulas, then send the Enterprise back through the wormhole" not ticked.
64. Click "OK".
65. Wait 3 seconds.
66. Take a screenshot called "later".
67. Expect "UI.log" not contain "PANIC".
68. Expect "Display8.log" not contain "VUID".
69. Expect "Display8.log" not contain "device removed".
70. Quit the game.
71. Expect the game to have exited.
72. Expect no crash.
