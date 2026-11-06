# Fleet Operations: Roots with the UI: the chat lines and the text entry are ours, typing reaches Roots' chat

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh HUD.Enable=1 HUD.Resources=0 HUD.Buttons=0 HUD.Pause=0 HUD.Chat=1 HUD.Selection=0 HUD.FrameRate=0 HUD.Overview=0 HUD.LayoutLog=1
Timeout: 14 min

Run with `a2test --game fo-roots` and `--install .` (FO-6 step 4, `PLAN-FLEETOPS.md`, "Step 4: the panels
Roots changes least"). The match is `_fo-render3d-skirmish`'s: Federation against Klingon,
Instant Action, `first`. The chat's text input is FleetOpsHook's (`GameKeysProcessInput`); ours draws the entry and the lines from the
engine's chat object and leaves the keys to the engine.

1. Include "_fo-render3d-skirmish".
2. Wait 5 seconds.
3. Move the mouse to 700,300.
4. Press Return.
5. Wait 1 second.
6. Type "hello roots".
7. Wait 1 second.
8. Expect "UI.log" contains "chat: entry active".
9. Take a screenshot called "typing".
10. Check that a text field with a rounded border is drawn at the top left, a name followed by "hello roots", and that Fleet Operations' own chat entry is not drawn a second time.
11. Press Return.
12. Wait up to 10 seconds for "UI.log" to say "chat: entry closed".
13. Wait 2 seconds.
14. Take a screenshot called "sent".
15. Press Pause.
16. Wait up to 15 seconds for "UI.log" to say "chat: line".
17. Take a screenshot called "chat line".
18. Check that a line of chat text in the UI's own style (small, white, with a dark rounded backdrop) at the top left says the game was paused, and that Fleet Operations' own chat lines are not drawn a second time.
19. Press Pause.
20. Wait 2 seconds.
21. Expect "UI.log" contains "panels: chat lines and text entry ours".
22. Expect "UI.log" contains "chat: line".
23. Expect "UI.log" not contain "PANIC".
24. Expect "Display8.log" not contain "VUID".
25. Expect "Display8.log" not contain "device removed".
26. Quit the game.
27. Expect the game to have exited.
28. Expect no crash.
