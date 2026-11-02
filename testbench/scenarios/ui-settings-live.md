# UI.asi's settings: a key in the overlay switches a panel, and the theme, without a restart (UI_SettingsChanged)

Resolutions: 1280x720
Mod: refit
Setup: ui/bench-asi.sh --no-grid
Overlay: UI.HUD:Minimap=0 UI.UI:Theme=klingon
Timeout: 15 min

`ui/README.md`, "Settings". The overlay (`RefitPlayer.ini`, `tools/settings/README.md`) is
written before launch with the minimap off and the Klingon theme. The UI then starts with
the stock minimap (the radar's click hook is in, so the key can come back while playing)
and the Klingon frame everywhere; the settings hub finds `UI_Settings` and
`UI_SettingsChanged` in UI.asi. The live flips are made in a session with
`./a2test drive --session ID overlay --unset UI.HUD:Minimap` (the log then says
`minimap: ours` and `settings: applied HUD.Minimap=1`), `overlay UI.UI:Theme=borg`,
and back; a scenario has no step that edits the overlay in a running game.

1. Include "_enter-instant-action".
2. Expect "UI.log" contains "settings: UI_Settings in UI.asi, 19 rows".
3. Expect "UI.log" contains "settings: RefitPlayer.ini [UI.HUD] Minimap=0 overrides UI.ini's Minimap=1".
4. Expect "UI.log" contains "settings: RefitPlayer.ini [UI.UI] Theme=klingon overrides UI.ini's Theme=auto".
5. Expect "UI.log" not contain "minimap: ours at".
6. Press Home.
7. Wait 3 seconds.
8. Take a screenshot called "off".
9. Check that at the bottom left there is the game's own minimap panel (a grey square under a small row of lights, in the game's metal frame), not a panel with a red angular frame holding SHIPS and STATIONS tabs, while the resources bar at the top has a red angular frame.
10. Quit the game.
11. Expect no crash.
