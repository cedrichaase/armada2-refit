# 20 ships with special weapons selected in combat: no overrun, a group of 20 (QOL.asi, MaxSelection=)

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh qol-bigselect
Timeout: 10 min

`qol/README.md`, QOL-3. Run it on a checkout: `./a2test run qol-big-selection --install .`.
Stock held a selection of 16, and the special-weapon button gathers the selected ships that
can use each special weapon into 16-entry stack arrays: 17 Galaxy class ships overran them
(the frame never finished, or the heap was damaged). With `MaxSelection=40` (the shipped
default) the arrays move to buffers of ours (`qol.special_select`, `qol.selection`). Twenty
Galaxy class ships firing at a Borg cube, all twenty selected with the HUD on, a group
bound to them and recalled; the game must keep running and the selection must hold all twenty.

1. Launch the game.
2. Wait up to 50 seconds for the logs the next steps expect.
3. Expect "QOL.log" contains "selections and groups up to 40".
4. Expect "Scene.log" contains "scene ready".
5. Scene "select g1 g2 g3 g4 g5 g6 g7 g8 g9 g10 g11 g12 g13 g14 g15 g16 g17 g18 g19 g20".
6. Expect scene "selection" answers "selected 20".
7. Wait 8 seconds.
8. Take a screenshot called "bigselect".
9. Check that the screenshot shows a space battle with many Federation ships, the game's interface around the map view, and no error dialog.
10. Press ctrl+1.
11. Scene "select g1".
12. Press 1.
13. Expect scene "selection" answers "selected 20".
14. Wait 5 seconds.
15. Expect no crash.
