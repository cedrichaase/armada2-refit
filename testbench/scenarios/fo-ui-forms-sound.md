# Fleet Operations: Roots with the UI: the sound screen as Armada II's, over FleetOpsHook's form

Resolutions: 1600x900
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh Menus.Forms=1 Menus.Scene=1
Timeout: 14 min

Run with `a2test --game fo-roots` and `--install .` (`PLAN-FLEETOPS.md`, FO-6 step 7,
option C; `ui/MENUS.md`, "Roots' forms"). Roots' Sound Settings form has a spec
(`menus/roots/sound.rs`): the four volumes and the channel count as sliders of ours at the
values Roots' sliders hold, unit speech and hardware acceleration as check boxes, the sound
device and the speaker set-up as dropdowns. A slider is dragged on the control's own thumb
and read back, a check box is clicked through the library's own hit test.

1. Launch the game.
2. Wait up to 180 seconds for "UI.log" to say "forms: TMainForm is ours".
3. Wait 2 seconds.
4. Click "OPTIONS".
5. Wait up to 20 seconds for "UI.log" to say "forms: TOptionsMenuForm is ours".
6. Wait 2 seconds.
7. Click "Sound Settings".
8. Wait up to 20 seconds for "UI.log" to say "forms: TSoundSettingsForm is ours".
9. Wait 3 seconds.
10. Take a screenshot called "sound".
11. Check that a window of ours titled "SOUND", laid out as Armada II's settings screens, shows Music Volume, Voice Volume, Effects Volume and Button Volume over four sliders at the left, and at the right a ticked Unit Speech check box, two dropdowns, a ticked Hardware Acceleration check box and Sound Channels 32 over a slider, with a Previous Menu button at the bottom left.
12. Click at 700,326.
13. Wait up to 10 seconds for "UI.log" to say "sldrMusicVolume of TSoundSettingsForm now at".
14. Click "Unit Speech".
15. Wait up to 10 seconds for "UI.log" to say "chkUnitSpeech of TSoundSettingsForm now unchecked".
16. Click at 1100,486.
17. Wait up to 10 seconds for "UI.log" to say "sldrSoundChannels of TSoundSettingsForm now at".
18. Wait 2 seconds.
19. Take a screenshot called "changed".
20. Check that the Music Volume slider's knob is well right of the middle of its track, Unit Speech is not ticked, and the Sound Channels slider's knob is right of the middle with its count above 32.
21. Click "Previous Menu".
22. Wait up to 20 seconds for "UI.log" to say "press on buttonBack".
23. Wait 3 seconds.
24. Expect "UI.log" not contain "PANIC".
25. Expect "Display8.log" not contain "VUID".
26. Stop the game.
27. Expect no crash.
