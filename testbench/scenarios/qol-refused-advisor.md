# A refused order speaks the advisor's line (QOLRules.asi, issue #1)

Resolutions: 1280x720
Mod: refit
Timeout: 8 min

`qol/README.md`, QOL-7, "The advisor on a refused order". Run it on a checkout:
`./a2test run qol-refused-advisor --install . -j 1`. Under `PayOnQueue=1` an order the bank
cannot cover is dropped where it is queued, so stock's `StartBuild` never fails and never
speaks; the plugin makes stock's own advisor call instead. This plays an Instant Action
match as the Federation (resources Normal: 3500 dilithium, about 1250 crew), selects the
starbase and queues construction ships (250 dilithium, 150 crew, 2 officers) until the
crew runs out. The refusal must log `refused (crew)` and an `advisor:` line for the local
team, and the event's chat line ("Insufficient crew to build. Colonize a planet.") must
show. To hear it, or to see `gvofcv20.wav` in `Stems.log`, run the same steps in a session
started with `--stems voice` (`testbench/operating/fix-issue-1-enqueue-voice.md`).

The starbase is at the centre of the screen after the launch (the camera starts on it) at
1280x720; the side list is opened by clicking player 1's Side field.

1. Launch the game.
2. Wait for the main menu.
3. Click the instant action cube.
4. Wait for "GAME SETUP".
5. Click "Computer AI:Medium".
6. Wait 1 second.
7. Click "Computer AI:Easy".
8. Wait 1 second.
9. Click at 389,122.
10. Wait 1 second.
11. Click "Federation".
12. Wait 1 second.
13. Expect "Federation" is visible.
14. Click "LAUNCH".
15. Wait 12 seconds.
16. Expect "QOLRules.log" contains "a refused order is announced".
17. Click at 735,305.
18. Wait 1 second.
19. Press q.
20. Wait 1 second.
21. Note "ten orders: about eight are paid, the rest are refused for crew".
22. Press q 10 times.
23. Wait 3 seconds.
24. Expect "QOLRules.log" contains "refused (crew) dil 250 met 0 lat 0 bio 0 crew 150 off 2".
25. Expect "QOLRules.log" contains "advisor: topic 10 for the local team".
26. Expect "Insufficient crew" is visible.
27. Take a screenshot called "refused".
28. Expect the game is still running.
29. Quit the game.
30. Expect no crash.
