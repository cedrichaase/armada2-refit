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

Where the start view puts the starbase is not fixed: the start position is dealt at
random and the camera starts wherever that leaves it, so the base has been at the
centre (735,305, where this clicked until 2026-10-23), at 788,262, and in the top
right corner close in. A click on a fixed point there selected nothing in some runs,
and the presses after it queued nothing. Home (`center_player` in `Input.map`, "Recenter
camera on mission start") sets both the camera's place and its zoom: after it the
starbase is at about 835,250 at 1280x720, whatever the start position and the zoom
before it (measured over three starts, zoomed in and out). Every step after the click
is tied to the selection and to the local player: `UI.log` names the selected builder
("info: Builder Starbase") and the bar's layout for it, then each key the bar takes
("bar: key Q=build", "bar: key Q=fconst", the construction ship); `QOLRules.log` writes
each payment with the bank after it, and the local bank is the one that goes from 3500
dilithium to 3250 and 3000 by construction ships; the advisor line names the local team,
and `UI.log` the event's chat line. The side list is opened by clicking player 1's Side field.

1. Launch the game.
2. Wait for the main menu.
3. Click the instant action cube.
4. Wait for "GAME SETUP".
5. Expect "QOLRules.log" contains "a refused order is announced".
6. Click "Computer AI:Medium".
7. Wait 1 second.
8. Click "Computer AI:Easy".
9. Wait 1 second.
10. Click at 389,122.
11. Wait 1 second.
12. Click "Federation".
13. Wait 1 second.
14. Expect "Federation" is visible.
15. Click "LAUNCH".
16. Wait up to 30 seconds for the logs the next steps expect.
17. Expect "UI.log" contains "race: the HUD's race is".
18. Expect "UI.log" contains "bar: shown as the player set it".
19. Wait 2 seconds.
20. Note "Home puts the camera on the base, the same view every time; select the starbase there".
21. Press Home.
22. Wait 2 seconds.
23. Click at 835,250.
24. Wait up to 5 seconds for the logs the next steps expect.
25. Expect "UI.log" contains "info: Builder Starbase".
26. Expect "UI.log" contains "bar: menu=0 mode=1 | Q=build W=order E=aiMenu A=recrew S=transport D=rally".
27. Take a screenshot called "selected".
28. Press q.
29. Wait up to 5 seconds for the logs the next steps expect.
30. Expect "UI.log" contains "bar: key Q=build".
31. Expect "UI.log" contains "bar: menu=2 mode=1 | Q=fconst".
32. Note "ten more q: about eight construction ships are paid, the rest are refused for crew".
33. Press q 10 times.
34. Wait up to 10 seconds for the logs the next steps expect.
35. Expect "UI.log" contains "bar: key Q=fconst".
36. Expect "QOLRules.log" contains "paid dil 250 met 0 lat 0 bio 0 crew 150 off 2; bank dil 3250".
37. Expect "QOLRules.log" contains "paid dil 250 met 0 lat 0 bio 0 crew 150 off 2; bank dil 3000".
38. Expect "QOLRules.log" contains "refused (crew) dil 250 met 0 lat 0 bio 0 crew 150 off 2".
39. Expect "QOLRules.log" contains "advisor: topic 10 for the local team".
40. Expect "UI.log" contains "Insufficient crew to build. Colonize a planet.".
41. Expect "Insufficient crew" is visible.
42. Take a screenshot called "refused".
43. Expect the game is still running.
44. Quit the game.
45. Expect no crash.
