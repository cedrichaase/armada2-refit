# The crash catcher: a fault on purpose in a mission is recorded and the game exits

Resolutions: 1280x720
Mod: refit
Setup: display/bench-mode.sh stub TestFrames=3000
Timeout: 15 min

Run with `--install .`. `[Crash] TestFrames=3000` (read in a bench clone only) makes
Display8 write to an unmapped address at the 3000th presented frame, which falls in the
first mission. The engine's exception handler runs, as in the white screen of 2026-11-06,
and where it used to walk the call stack and hang, `display/src/crash.rs` has written the
record (the exception, the registers, the stack, the address space) into `Display8.log`
and ended the process. The checks are the record and that the game has exited.

1. Launch the game.
2. Wait for the main menu.
3. Click the single player emblem.
4. Wait for the single player screen.
5. Click "Federation Campaign".
6. Wait 3 seconds.
7. Click "Lifelines".
8. Wait 1 second.
9. Click "O.K.".
10. Wait up to 300 seconds for the logs the next steps expect.
11. Expect "Display8.log" contains "crash: exception 1 (first chance): code 0xc0000005".
12. Expect "Display8.log" contains "address space (crash)".
13. Expect "Display8.log" contains "the engine's exception handler is at its call-stack dump".
14. Wait 5 seconds.
15. Expect the game to have exited.
