# The keys that moved out of UI.ini are carried to Display8.ini, once (ui 1.0.0, display 0.18.0)

Resolutions: 1280x720
Mod: refit
Setup: display/bench-moved.sh
Timeout: 10 min

`tools/settings/README.md`, "Moved keys" (C2), and `display/README.md`, "Window and
display mode". `[UI] Native` and `[UI] KeepClip` became `[Window] Native` and `KeepClip`
of `Display8.ini`, and Display8's table (`display/settings.toml`) says so with
`moved_from`. The clone starts with an old `UI.ini` that has `Native=0`, and an overlay
that has `[UI.UI] KeepClip=0`. `display/bench-moved.sh` runs `tools/settings/move-keys.py`
over the clone, which is what `./install` does: `Native=0` goes to `Display8.ini` and
leaves `UI.ini`. At its first frame the UI's hub moves the overlay's key to
`[Display8.Window]`. Run with `--install .`.

1. Launch the game.
2. Wait for the main menu.
3. Wait up to 30 seconds for the logs the next steps expect.
4. Expect "Display8.ini" contains "Native=0".
5. Expect "UI.ini" not contains "Native".
6. Expect "Display8.log" contains "window: Native=0: the game's mode is as set".
7. Expect "UI.log" contains "settings: RefitPlayer.ini [UI.UI] KeepClip moved to [Display8.Window] KeepClip".
8. Expect "RefitPlayer.ini" contains "[Display8.Window]".
9. Expect "RefitPlayer.ini" not contains "[UI.UI]".
10. Quit the game.
11. Expect no crash.

The overlay's key is moved after Display8 has read its keys (the hub runs at the UI's
first frame), so a player who had `KeepClip=0` there gets the shipped value for that one
launch and the moved one from the next.
