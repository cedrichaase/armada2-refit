# The UI's Multiplayer Connection screen, alone: one game, no DirectPlay

Resolutions: 1280x720
Mod: refit
Setup: ui/bench-asi.sh --menus
Timeout: 8 min

`ui/MENUS.md`, "The setup and multiplayer specs". `ui-menus-multiplayer.md` reaches
the connection screen with two games; this one is the screen alone, so it can run beside
another game and be compared screenshot for screenshot across builds. The window is
laid out by `connection_layout` (`menus/screens/multiplayer.rs`), which Roots' form
shares: a change to it shows here. No judged step.

1. Launch the game.
2. Wait for the main menu.
3. Expect "UI.log" contains "menus: hosted by the UI".
4. Click at 228,346.
5. Wait up to 20 seconds for "Internet - Manual IP".
6. Expect "Internet - Online" is visible.
7. Take a screenshot called "connection".
8. Click at 640,271.
9. Press End.
10. Press BackSpace 32 times.
11. Type "host".
12. Wait 1 second.
13. Take a screenshot called "connection typed".
14. Stop the game.
15. Expect no crash.
