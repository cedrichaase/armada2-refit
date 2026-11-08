# Fleet Operations: Roots with the UI: the campaign selection after a mission won

Resolutions: 1600x900
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh Menus.Forms=1 Menus.Scene=1 Menus.Enable=1 Menus.Credits=1
Timeout: 12 min

Run with `a2test --game fo-roots` and `--install .`. The user's report (2026-11-07): after a
mission won, the campaign selection was drawn over the mission's last frame (its HUD and chat
line too) as the 2x2 cards of the screen without the scene, the cards' art cut wrongly,
instead of the side column over the menu scene that the main menu's route shows. Roots' win
cheat (`kobayashimaru` typed into the chat) wins the first allied tutorial at once
(PLAN-FLEETOPS.md, decisions 316 and 317). The campaign selection is shot first from the
main menu, then after the win; both are judged against the same description, and the
window manager's events are watched across the win (decision 314: one window).

1. Launch the game.
2. Wait up to 180 seconds for "UI.log" to say "forms: TMainForm is ours".
3. Wait 3 seconds.
4. Move the mouse to 700,300.
5. Wait 1 second.
6. Click "Single Player".
7. Wait up to 30 seconds for "UI.log" to say "single player drawn by its spec".
8. Wait 4 seconds.
9. Move the mouse to 1100,800.
10. Wait 1 second.
11. Take a screenshot called "campaigns from the menu".
12. Check that the screen shows "SINGLE PLAYER" at the left over a column of four campaigns (Allied Campaign, Dominion Campaign, Species 8472 vs Borg War, Free Borg Campaign), each with a small square picture before its name, then Load Game and Previous Menu, over a 3D space scene with ships; no HUD of a mission and no 2x2 grid of large campaign cards.
13. Move the mouse to 300,336.
14. Wait 1 second.
15. Click at 300,336.
16. Wait up to 20 seconds for "UI.log" to say "mission select drawn by its spec".
17. Wait 2 seconds.
18. Click "TUTORIAL: Gameplay".
19. Wait 1 second.
20. Click "OK".
21. Wait up to 180 seconds for "Render3D.log" to say "fed_sovereignent at".
22. Wait 8 seconds.
23. Watch the window manager.
24. Move the mouse to 700,300.
25. Press Return.
26. Wait 1 second.
27. Type "kobayashimaru".
28. Wait 1 second.
29. Press Return.
30. Wait up to 90 seconds for "UI.log" to say "single player drawn by its spec".
31. Wait 4 seconds.
32. Move the mouse to 1100,800.
33. Wait 1 second.
34. Take a screenshot called "campaigns after the win".
35. Check that the screen shows "SINGLE PLAYER" at the left over a column of four campaigns (Allied Campaign, Dominion Campaign, Species 8472 vs Borg War, Free Borg Campaign), each with a small square picture before its name, then Load Game and Previous Menu, over a 3D space scene with ships; no HUD of a mission and no 2x2 grid of large campaign cards.
36. Expect "UI.log" contains "is the front end's single player: not over the mission that just ended".
37. Expect the window manager to have seen no other window, and the game's window kept.
38. Expect "UI.log" not contain "PANIC".
39. Stop the game.
40. Expect no crash.
