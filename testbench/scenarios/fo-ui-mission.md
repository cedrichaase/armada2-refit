# Fleet Operations: Roots with the UI: the mission frame and input are the UI's, and a click on the Refit screen does not reach the game

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh
Timeout: 14 min

Run with `a2test --game fo-roots` and `--install .`: `./install --game fo-roots` puts
Display8, the ASI loader, Render3D and `UI.asi` into the clone's `Data/`, with `UI.ini` as
Roots has it by default (`ui/defaults/fo-roots.ini`: the loading screen and the Refit
screen's key on, every HUD panel, the menus and the cursor off). FO-6 step 3
(`PLAN-FLEETOPS.md`, "FO-6: the UI on Roots") verified `core.mission` beside FleetOpsHook's
trampoline of `ChatDisplay::Render` and its edit of `Armada_RenderFrame` (+0x245): the call at
0x598a16 is stock's in the file and in memory, so the UI draws after the chat as on stock.

Roots' game window is FleetOpsHook's own form, whose window procedure calls
`ProcessKeyboardMessages` and `ProcessMouseMessages` itself: the engine's `WindowProc` never
runs. The profile's `core_input_at_callee` = 1 takes the messages at the two functions'
entries (`core.input_callee`); UI.log names who calls them (`input: ... called from`) and
lists the thread's windows at the first loading and mission frames (class, window
procedure, which one the engine names).

With no HUD panel of ours on Roots, the element of ours clicked is the Refit screen, opened
over the mission by its key (Ctrl+F12, `[UI] RefitKey=123`; decision 86): a click on its
`Interface` tab is kept from the engine (`input: a left button down kept from the engine`),
and Esc closes the modal without reaching Roots (no escape menu). The render lists are
logged once (`hud: render lists`), with the name the info panel answers to (decision 77).

1. Launch the game.
2. Wait for the main menu.
3. Include "_fo-sides".
4. Click "LAUNCH".
5. Wait up to 300 seconds for "Render3D.log" to say "(claimed at the instance draw)".
6. Wait 15 seconds.
7. Take a screenshot called "match".
8. Check that the match has started: the 3D view of a space map with ships and Fleet Operations' own HUD (its resource counters, minimap and info strip), with no panel of the UI's over it.
9. Press ctrl+F12.
10. Wait up to 20 seconds for "UI.log" to say "refit: opened in a mission, by its key".
11. Wait 2 seconds.
12. Take a screenshot called "refit".
13. Check that a settings window titled REFIT with tabs (Graphics, Interface) and rows of switches is drawn in the middle of the screen over a dimmed mission.
14. Click "Interface".
15. Wait up to 10 seconds for "UI.log" to say "input: a left button down kept from the engine".
16. Wait 2 seconds.
17. Take a screenshot called "interface tab".
18. Check that the settings window now shows its Interface page (rows such as Theme or Loading screen), still over the dimmed mission.
19. Press Escape.
20. Wait up to 10 seconds for "UI.log" to say "refit: closed".
21. Wait 3 seconds.
22. Take a screenshot called "closed".
23. Check that the settings window is gone and the mission is shown with Fleet Operations' own HUD, and no in-game menu (no Return to Game or Quit buttons) has opened.
24. Wait up to 30 seconds for the logs the next steps expect.
25. Expect "UI.log" contains "engine: build armada2/fo-roots-5.0Z11".
26. Expect "UI.log" contains "input: taken at the entries of ProcessKeyboardMessages".
27. Expect "UI.log" contains "input: captured".
28. Expect "UI.log" contains "input: ProcessMouseMessages called from".
29. Expect "UI.log" contains "input: ProcessKeyboardMessages called from".
30. Expect "UI.log" contains "mission: hooked".
31. Expect "UI.log" contains "UI frames in".
32. Expect "UI.log" contains "windows at the first mission frame".
33. Expect "UI.log" contains "hud: render lists".
34. Expect "UI.log" not contain "mission: standing down".
35. Expect "UI.log" not contain "input: standing down".
36. Expect "UI.log" not contain "PANIC".
37. Expect "Display8.log" not contain "VUID".
38. Expect "Display8.log" not contain "device removed".
39. Quit the game.
40. Expect the game to have exited.
41. Expect no crash.
