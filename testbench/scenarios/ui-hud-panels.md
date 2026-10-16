# The UI's own resources, top-right buttons, pause box, objectives and chat ([HUD])

Resolutions: 1280x720
Mod: refit
Setup: ui/bench-asi.sh --no-grid HUD.Resources=1 HUD.Buttons=1 HUD.Messages=1 HUD.Pause=1 HUD.Replay=1 HUD.Overlay=1 HUD.Chat=1
Timeout: 12 min

`ui/HUD.md`, "The resources, the buttons, the boxes, the overlay and the chat". With
these `[HUD]` keys the UI draws the resource strip (top centre), the top-right
buttons, the pause box, the objectives box, the overlay messages and the chat; the
stock panels keep running underneath and neither draw nor take the mouse. The match
is `_enter-instant-action.md`'s (Borg, Warzone, Resources: Unlimited, so the amounts
read 999999). `UI.log` names what each panel found ("panels: ResourcePanel ours",
"resources: Crew=...", "buttons: comm(off) idle view objectives more") and each press
("panels: click objectives"), and the boxes when they show ("pause: shown",
"objectives: ...").

1. Include "_enter-instant-action".
2. Expect "UI.log" contains "panels: ResourcePanel ours".
3. Expect "UI.log" contains "panels: ButtonPanel ours".
4. Expect "UI.log" contains "resources: Crew=".
5. Expect "UI.log" contains "buttons:".
6. Move the mouse to 640,30.
7. Wait 2 seconds.
8. Take a screenshot called "top bar".
9. Check that along the top of the screen there is one dark panel in the middle with several labelled resource amounts (large numbers with small capital names above them), and a row of square icon buttons in its own panel in the top-right corner, and that the stock top bar's own art (the race's panel sheet with its numbers) does not show.
10. Press Pause.
11. Wait 2 seconds.
12. Expect "UI.log" contains "pause: shown".
13. Take a screenshot called "paused".
14. Check that the screen is dimmed and a small panel in the middle shows a large word for paused and one button under it.
15. Press Pause.
16. Wait 2 seconds.
17. Expect the game is still running.
