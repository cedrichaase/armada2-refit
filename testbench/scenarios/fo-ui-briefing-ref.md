# Fleet Operations: Roots with Display8 and Render3D alone: the tutorial's briefing as Roots lays it out, the reference for fo-ui-briefing

Resolutions: 2150x900
Mod: refit
Layers: display asiloader render3d
Setup: display/bench-mode.sh stub Backend=vulkan
Timeout: 14 min

The A side of `fo-ui-briefing` (PLAN-FLEETOPS.md, FO-6, decisions 320 onward): Roots'
briefing and objectives screen (the stock `ObjectivesDisplay`, Roots' art) in the first
allied tutorial, opened by the objectives key, with no `UI.asi` at all. Its shot is where
the frame sits and how wide it is when nothing of ours is drawn.

The way in is `fo-render3d-tutorial`'s, with Single Player clicked at its place five
seconds after the main menu shows: clicked by its label sooner, it was missed in two runs
of three on this branch.

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
26. Take a screenshot called "tutorial".
27. Press Alt+o.
28. Wait 3 seconds.
29. Take a screenshot called "briefing".
30. Check that the briefing screen (BRIEFING SUMMARY, OBJECTIVES, an OK button) is drawn in Roots' metal frame.
31. Expect "Display8.log" not contain "VUID".
32. Expect "Display8.log" not contain "device removed".
33. Quit the game.
34. Expect the game to have exited.
35. Expect no crash.
