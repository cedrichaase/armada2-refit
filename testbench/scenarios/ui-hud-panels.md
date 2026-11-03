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
"resources: Dilithium=... Officers=16/600(Power Nodes)", "buttons: menu comm idle view
objectives(off)") and each press ("panels: click idle"); the boxes when they
show ("pause: shown"), and the chat ("chat: entry active ...", "chat: line ...").
At 1280x720 the five top-right buttons (help, palette and "show more" are gone,
`hud/buttons.rs`) are centred at y 27: menu 1123, comm 1155, idle 1189, view 1220,
objectives 1253.
The briefing and objectives box is `ui-hud-panels-objectives.md`.

1. Include "_enter-instant-action".
2. Expect "UI.log" contains "panels: ResourcePanel ours".
3. Expect "UI.log" contains "panels: ButtonPanel ours".
4. Expect "UI.log" contains "resources: Dilithium=".
5. Expect "UI.log" contains "buttons: menu comm idle view objectives(off)".
6. Move the mouse to 1252,27.
7. Wait 3 seconds.
8. Take a screenshot called "top bar".
9. Check that along the top of the screen there is one dark panel in the middle with four resource amounts (large numbers with small capital names above them, one of them reading like 16/600), a row of five square icon buttons in its own panel in the top-right corner with a tooltip under the rightmost one, and that the stock top bar's own art (the race's panel sheet) does not show.
10. Click at 1189,27.
11. Wait 2 seconds.
12. Expect "UI.log" contains "panels: click idle".
13. Move the mouse to 700,400.
14. Press Pause.
15. Wait 2 seconds.
16. Expect "UI.log" contains "pause: shown".
17. Take a screenshot called "paused".
18. Check that the screen is dimmed, a small panel in the middle reads GAME PAUSED with one button under it reading CONTINUE, and a line of chat in the top left says the game was paused.
19. Click at 640,384.
20. Wait 2 seconds.
21. Expect "UI.log" contains "panels: click CONTINUE".
22. Expect "UI.log" contains "unpaused the game".
23. Press Return.
24. Wait 1 second.
25. Type "hello there".
26. Wait 1 second.
27. Expect "UI.log" contains "chat: entry active".
28. Take a screenshot called "typing".
29. Check that the chat is drawn only at the top left, in the UI's own style: a text field with a rounded border in which a name in the accent colour is followed by "hello there", and above it a dark rounded box holding the earlier chat line about the game being unpaused (both are ours, in the race theme's colours); and that no chat text appears anywhere else on the screen.
30. Press Return.
31. Wait 1 second.
32. Expect "UI.log" contains "chat: entry closed".
33. Expect the game is still running.
