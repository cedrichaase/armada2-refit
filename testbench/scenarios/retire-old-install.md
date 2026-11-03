# An old install is retired by ./install and the game still starts

Resolutions: 16:9
Mod: refit
Assets: none
Setup: testbench/retire-old-install.py
Timeout: 15 min

`retire/README.md`. Run it on a checkout: `./a2test run retire-old-install --install . -j 1`.
Before 15.2.0 `./install` put `HUD.asi`, `GridLayout.asi` and `Menus.asi` (with their inis)
into the game, the hud scripts rewrote the `misc/` configs, the cursors and the font
atlases (each with a backup), the d3d8to9 and DXVK chain stood in the game directory beside
a marked `dxvk.conf`, twelve per-part plugins stood where render3d now is, and the
widened loading screen was installed. `testbench/retire-old-install.py` (the `Setup:`)
snapshots the clone as the refit install leaves it, puts that old install over it from the
last tag that has the layers (`testbench/bench/legacy.py`, cached in `$A2_DATA/bench/legacy/`),
runs the checkout's `./install` over it and compares every file by sha256: the clone must
come out exactly as it was (the chain class, `d3d8_real.dll`, `d3d9.dll` and `dxvk.conf`,
is taken out too: `./install` passes `retire.sh --chain` since 15.6.0). `Assets: none`, so no texture
target puts back what the retire restored. The report is `RetireCheck.log` in the clone.

1. Expect "RetireCheck.log" contains "retire-old-install: ok".
2. Expect "RetireCheck.log" not contains "FAIL".
3. Launch the game.
4. Wait for the main menu.
5. Expect "UI.log" contains "menus: hosted by the UI".
6. Expect "Render3D.log" not contains "PANIC".
7. Expect "UI.log" not contains "PANIC".
8. Quit the game.
9. Expect no crash.
