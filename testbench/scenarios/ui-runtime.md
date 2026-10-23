# The UI runtime draws and takes input in all three contexts (UI.asi)

Resolutions: 16:9
Mod: refit
Setup: ui/bench-asi.sh --demo shell,loading,mission
Timeout: 12 min

`ui/README.md`. `UI.asi` is not in `./install` yet; the setup puts it in the clone with
the demonstration panel on in every context. In the shell, the Credits are replaced
by a screen of ours (`[Demo] Shell=1`): a panel with a button that counts clicks, left
with Esc. The match is entered as `_enter-instant-action.md` does, from the main menu
the demo screen returns to (that fragment launches the game, so its steps are
written out here; Game Setup is the UI's, as there). The loading screen gets the panel over the logo, reading the load's progress,
and the match gets it over the HUD, top right, 96 units from the edges. The click count
is the demo's own and carries over from the shell to the match, so after one click in
each it reads "2 clicks". A click on the panel in the match is the UI's: the engine
must not see it (capture), so the Assembler selected before stays selected. `UI.log`
names each hook and counts the mission frames, which must be once per presented frame.

1. Launch the game.
2. Wait for the main menu.
3. Expect "UI.log" contains "shell: hooked".
4. Expect "UI.log" contains "mission: hooked".
5. Expect "UI.log" contains "loading: hooked".
6. Click the credits.
7. Wait 3 seconds.
8. Take a screenshot called "shell screen".
9. Expect "UI RUNTIME" is visible.
10. Expect "Context: Shell" is visible.
11. Click "Click me".
12. Wait 1 second.
13. Expect "1 clicks" is visible.
14. Press Escape.
15. Wait for the main menu.
16. Expect "UI.log" contains "demo screen left".
17. Click the instant action cube.
18. Wait for "GAME SETUP".
19. Click "Computer AI:Medium".
20. Wait 1 second.
21. Click "Computer AI:Easy".
22. Wait 1 second.
23. Click at hud 767,203.
24. Wait 1 second.
25. Click "Borg".
26. Wait 1 second.
27. Expect "Computer AI:Easy" is visible.
28. Expect "Borg" is visible.
29. Click "LAUNCH".
30. Wait up to 8 seconds for the logs the next steps expect.
31. Expect "UI.log" contains "loading: frame".
32. Take a screenshot called "mission panel".
33. Expect "Context: Mission" is visible.
34. Click at 960,388.
35. Wait 1 second.
36. Click "Click me".
37. Wait 1 second.
38. Expect "2 clicks" is visible.
39. Expect "UI.log" contains "UI frames in".
40. Take a screenshot called "after the panel click".
41. Check that the Assembler is still selected (its portrait and name in the info panel at the bottom) and that the UI RUNTIME panel at the top right shows "2 clicks".
42. Quit the game.
43. Expect no crash.
