# The UI's own resources, top-right buttons, pause box and chat ([HUD], a match)

Resolutions: 1280x720
Mod: refit
Setup: ui/bench-asi.sh --no-grid HUD.Resources=1 HUD.Buttons=1 HUD.Messages=1 HUD.Pause=1 HUD.Replay=1 HUD.Overlay=1 HUD.Chat=1
Timeout: 12 min

`ui/HUD.md`, "The resources, the buttons, the boxes, the overlay and the chat". With
these `[HUD]` keys the UI draws the resource strip (top centre), the top-right
buttons, the pause box and the chat; the stock panels keep running underneath and
neither draw nor take the mouse. The match is `_enter-instant-action.md`'s (Borg,
Warzone). `UI.log` names what each panel found ("panels: ResourcePanel ours",
"resources: Crew=... Officers=16/600(Power Nodes) ...", "buttons: comm idle view
objectives(off) more") and each press ("panels: click more"); the boxes when they
show ("pause: shown"), and the chat ("chat: entry active ...", "chat: line ...").
At 1280x720 the top-right buttons are 44 units apart from the right: with "show
more" off, more at 1252,27 and idle at 1155,27; with it on, menu at 1024,27.
The briefing and objectives box is `ui-hud-panels-objectives.md`.

1. Include "_enter-instant-action".
2. Expect "UI.log" contains "panels: ResourcePanel ours".
3. Expect "UI.log" contains "panels: ButtonPanel ours".
4. Expect "UI.log" contains "resources: Crew=".
5. Expect "UI.log" contains "buttons: comm".
6. Move the mouse to 1252,27.
7. Wait 3 seconds.
8. Take a screenshot called "top bar".
9. Check that along the top of the screen there is one dark panel in the middle with four resource amounts (large numbers with small capital names above them, one of them reading like 16/600), a row of five square icon buttons in its own panel in the top-right corner with a tooltip under the rightmost one, and that the stock top bar's own art (the race's panel sheet) does not show.
10. Click at 1252,27.
11. Wait 2 seconds.
12. Expect "UI.log" contains "panels: click more".
13. Expect "UI.log" contains "buttons: menu help palette comm idle view".
14. Click at 1155,27.
15. Wait 2 seconds.
16. Expect "UI.log" contains "panels: click idle".
17. Move the mouse to 700,400.
18. Press Pause.
19. Wait 2 seconds.
20. Expect "UI.log" contains "pause: shown".
21. Take a screenshot called "paused".
22. Check that the screen is dimmed, a small panel in the middle reads GAME PAUSED with one button under it reading CONTINUE, and a line of chat in the top left says the game was paused.
23. Click at 640,384.
24. Wait 2 seconds.
25. Expect "UI.log" contains "panels: click CONTINUE".
26. Expect "UI.log" contains "unpaused the game".
27. Press Return.
28. Wait 1 second.
29. Type "hello there".
30. Wait 1 second.
31. Expect "UI.log" contains "chat: entry active".
32. Take a screenshot called "typing".
33. Check that the chat is drawn only at the top left, in the UI's own style: a text field with a rounded border in which a name in the accent colour is followed by "hello there", and above it a dark rounded box holding the earlier chat line about the game being unpaused (both are ours, in the race theme's colours); and that no chat text appears anywhere else on the screen.
34. Press Return.
35. Wait 1 second.
36. Expect "UI.log" contains "chat: entry closed".
37. Expect the game is still running.
