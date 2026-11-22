# Fleet Operations: Roots with the UI as Roots has it by default: the seven verified HUD panels together over Roots' own bar, info strip and minimap

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh --rts FRAMERATE_DISPLAY=1 HUD.LayoutLog=1
Timeout: 14 min

Run with `a2test --game fo-roots` and `--install .`. The keys are `ui/defaults/fo-roots.ini`'s (FO-6
step 4, `PLAN-FLEETOPS.md`, "Step 4: the panels Roots changes least"): the resources, the top-right
buttons, the pause box, the chat, the selection overlay, the frame-rate panel and the 3D view's
order ring, height posts and placement footprint are ours; the bar, the info strip, the minimap
and the message, objectives, replay and overlay-text boxes are Roots'. The per-panel scenarios
(`fo-ui-resources`, `-topbar`, `-pause`, `-chat`, `-selection`, `-frame-rate`, `-overview`) check
each alone; this one checks that they do not clash, and that the drag box stays the engine's.

1. Include "_fo-render3d-skirmish".
2. Wait up to 60 seconds for "UI.log" to say "resources:".
3. Wait 3 seconds.
4. Expect "UI.log" contains "panels: ResourcePanel ours".
5. Expect "UI.log" contains "panels: ButtonPanel ours".
6. Expect "UI.log" contains "panels: PauseBox ours".
7. Expect "UI.log" contains "panels: chat lines and text entry ours".
8. Expect "UI.log" contains "selection: ours".
9. Expect "UI.log" contains "leftovers: 5 of the engine's last draws are ours".
10. Move the mouse to 700,300.
11. Wait 1 second.
12. Drag from 300,420 to 1250,640.
13. Wait 3 seconds.
14. Take a screenshot called "all ours".
15. Check that the resource strip (top centre), the five round buttons (top right), a frame-rate panel (top left) and the selection brackets and gauges are the UI's, that Fleet Operations' own bar, info strip and minimap are drawn as before, and that nothing of Fleet Operations' own resource strip or top-right buttons shows under ours.
16. Move the mouse to 700,300.
17. Press Return.
18. Type "hello roots".
19. Wait 1 second.
20. Take a screenshot called "chat typing".
21. Press Return.
22. Wait up to 10 seconds for "UI.log" to say "chat: entry closed".
23. Press Pause.
24. Wait up to 15 seconds for "UI.log" to say "pause: shown".
25. Click at 647,513.
26. Wait up to 10 seconds for "UI.log" to say "panels: click Continue".
27. Wait 3 seconds.
28. Take a screenshot called "continued".
29. Expect "UI.log" not contain "the HUD stays stock".
30. Expect "UI.log" not contain "PANIC".
31. Expect "Display8.log" not contain "VUID".
32. Expect "Display8.log" not contain "device removed".
33. Quit the game.
34. Expect the game to have exited.
35. Expect no crash.
