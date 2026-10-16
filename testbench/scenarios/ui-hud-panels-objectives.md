# The UI's own briefing and objectives box and overlay messages ([HUD], a tutorial)

Resolutions: 1280x720
Mod: refit
Setup: ui/bench-asi.sh --no-grid HUD.Resources=1 HUD.Buttons=1 HUD.Messages=1 HUD.Pause=1 HUD.Overlay=1 HUD.Chat=1
Timeout: 12 min

`ui/HUD.md`, "Messages and objectives". The Starfleet Academy tutorial "User
Interface" opens its objectives box at the start and advances its lessons on Enter,
so it shows both that the box is ours and that the keys stay the engine's: Enter
closes the box through the stock box's own key step, and a second Enter later opens
the next lesson's box. The intro's centre lines ("Retinal Authorization Status: ...")
are the overlay messages. The Academy is the top-left panel of the single player
screen; its first mission button is at 621,158 and O.K. at 555,580 (1280x720).

1. Launch the game.
2. Wait for the main menu.
3. Click the single player emblem.
4. Wait for the single player screen.
5. Click at 300,110.
6. Wait 4 seconds.
7. Click at 621,158.
8. Wait 1 second.
9. Click at 555,580.
10. Wait up to 300 seconds for the mission to start.
11. Wait 3 seconds.
12. Expect "UI.log" contains "overlay: #".
13. Expect "UI.log" contains "objectives: \"Tutorial: User Interface\"".
14. Take a screenshot called "objectives".
15. Check that a window titled TUTORIAL: USER INTERFACE is in the middle of a dimmed screen, with a BRIEFING SUMMARY heading and text, an OBJECTIVES heading with three lines each beginning with an empty check box, and an OK button at the bottom right, drawn in the UI's flat style, not the stock grey metal panel.
16. Move the mouse to 640,600.
17. Press Return.
18. Wait 2 seconds.
19. Take a screenshot called "closed".
20. Check that no objectives window is shown any more.
21. Press Return.
22. Wait 3 seconds.
23. Press Return.
24. Wait 3 seconds.
25. Expect "UI.log" contains "Select the Enterprise".
26. Click at 1220,27.
27. Wait 2 seconds.
28. Take a screenshot called "lesson 2".
29. Check that the objectives window is shown with the objective "Select the Enterprise".
30. Click "OK".
31. Wait 2 seconds.
32. Expect "UI.log" contains "panels: click OK".
33. Press Alt+o.
34. Wait 2 seconds.
35. Expect the game is still running.
