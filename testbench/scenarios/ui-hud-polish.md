# The UI's trade status, the shorter info panel, its icons and the minimap's filters

Resolutions: 1680x720
Mod: refit
Setup: ui/bench-asi.sh --no-grid
Timeout: 15 min

`ui/HUD.md`, "The trade status and the team message", "The info panel" (the queue
only when something is queued), "The minimap" (the filters) and `ui/STYLE.md`,
"Icons". The second Federation mission, *Lifelines*, is the one whose script shows
the trade status ("Sent to Team 2") from the start; a direct map launch has no
script, so it goes in through the campaign menu (the UI's menus, at 1680x720: Single
Player at 198,258, the Federation campaign at 260,322). The camera opens with
Starbase 27 at 890,172 and the Galaxy *USS Odyssey* at 655,387. `UI.log` names the
trade status once ("trade: Sent to Team 2: Dilithium: 0 Latinum: 0 Metal: 0 |"), the
selection ("info: Single ..."), and the bar's layout ("bar: menu=7 ... Q=galert").

1. Launch the game.
2. Wait 45 seconds.
3. Click at 198,258.
4. Wait 5 seconds.
5. Click at 260,322.
6. Wait 6 seconds.
7. Click "Lifelines".
8. Wait 2 seconds.
9. Click "O.K.".
10. Wait up to 300 seconds for the briefing.
11. Click "OK".
12. Wait 6 seconds.
13. Expect "UI.log" contains "panels: team texts ours".
14. Expect "UI.log" contains "trade: Sent to Team 2".
15. Click at 890,172.
16. Wait 3 seconds.
17. Expect "UI.log" contains "info: Single Starbase 27".
18. Take a screenshot called "starbase".
19. Check that under the row of buttons in the top-right corner there is a small panel of the UI's own titled SENT TO TEAM 2 with three rows (Dilithium, Latinum, Metal, each 0), and no stretched text in the default font anywhere near it; that the info panel at the bottom shows STARBASE 27 with two bars and a row of five system chips under them, and no row of empty round slots below those; that the button bar beside it shows flat single-colour icons (an I-beam, a chair, a brain), not bevelled metal tiles; and that above the minimap at the bottom left the two filters read SHIPS and STATIONS, each with four small dots.
20. Click at 655,387.
21. Wait 2 seconds.
22. Press E.
23. Wait 2 seconds.
24. Expect "UI.log" contains "Q=galert".
25. Take a screenshot called "ai menu".
26. Check that the button bar shows three round status lights (green, yellow, red) and three upward arrows, each with a row of three small bars under it lit one, two and three in green, yellow and red, and a back arrow in the right-hand column.
27. Press G.
28. Expect the game is still running.
