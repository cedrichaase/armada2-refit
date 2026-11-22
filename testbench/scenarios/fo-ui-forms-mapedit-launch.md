# Fleet Operations: Roots with the UI: Edit Map in our map editor list opens Roots' editor

Resolutions: 1600x900
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh Menus.Forms=1 Menus.Scene=1
Timeout: 15 min

Run with `a2test --game fo-roots` and `--install .` (decision 280). Our map list
(`menus/roots/mapedit.rs`) presses Roots' Edit Map button: Roots' own action opens the
chosen map in its editor and closes the form; the load then ends in the editor's licence dialog (one of
Roots' nine dialogs, drawn by `roots_dialogs`), which the scenario stops at.

1. Launch the game.
2. Wait up to 180 seconds for "UI.log" to say "forms: TMainForm is ours".
3. Wait 2 seconds.
4. Click "MAP EDITOR".
5. Wait up to 20 seconds for "UI.log" to say "forms: TMapEditForm is ours".
6. Wait 3 seconds.
7. Click "A Good Day to Die".
8. Wait up to 10 seconds for "UI.log" to say "chosen".
9. Wait 2 seconds.
10. Click "Edit Map".
11. Wait up to 20 seconds for "UI.log" to say "press on buttonEditMap".
12. Wait 30 seconds.
13. Take a screenshot called "editor".
14. Check that the screen shows the editor's Software Licensing Agreement dialog over the mission scene, with ACCEPT and CANCEL under the text.
15. Expect "UI.log" contains "Armada II Editor".
16. Expect "UI.log" not contain "PANIC".
17. Stop the game.
18. Expect no crash.
