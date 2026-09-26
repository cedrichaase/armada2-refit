# Launch the first Borg mission from the menus, then quit

Resolutions: 16:9
Mod: remastered
Timeout: 12 min

The whole front-end path a player takes into a campaign, in deterministic steps: every
click is a named target in `testbench/ui.json` or text found by OCR. The mission opens
with a ~70 s cinematic before its briefing; the briefing proves the mission loaded.

1. Launch the game.
2. Wait for the main menu.
3. Click the single player emblem.
4. Wait for the single player screen.
5. Click the Borg campaign.
6. Wait for "Werewolf Pack".
7. Take a screenshot called "mission list".
8. Click the first mission.
9. Click the mission OK button.
10. Wait up to 300 seconds for the briefing.
11. Take a screenshot called "briefing".
12. Expect "Werewolf Pack" is visible.
13. Expect "Borg Queen" is visible.
14. Expect the game is still running.
15. Quit the game.
16. Expect the game to have exited.
17. Expect no crash.
