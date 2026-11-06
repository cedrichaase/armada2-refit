# Fragment: Fleet Operations: Roots, Instant Action with the sides picked: Federation against the Klingon AI

From the main menu to the setup screen with both sides set and read back, for the
Fleet Operations: Roots scenarios (`a2test --game fo-roots`). Instant Action deals the sides
at random unless they are picked, and a wait on the HUD's "6000" depended on the race. The
player is Federation and the AI Klingon (the setup screen's Side lists, at 1280x960, where
the 1200x900 shell sits at the top left); the scenario then clicks LAUNCH and waits on what
suits it (`_fo-render3d-skirmish` waits on render3d's log; the others on the HUD's
counters, which are Federation's "6000").

Roots' shell takes a click only after the pointer has been over the spot (a click straight
after a jump is lost): each click is preceded by a move and a pause, and a list gets two
seconds to open. The sides are checked by OCR before LAUNCH, so a click that went astray
fails there and not as a Borg game three minutes on.

1. Move the mouse to 640,520.
2. Wait 1 second.
3. Click "Instant Action".
4. Wait up to 30 seconds for "LAUNCH".
5. Wait 3 seconds.
6. Move the mouse to 270,114.
7. Wait 1 second.
8. Click at 278,114.
9. Wait 2 seconds.
10. Move the mouse to 278,177.
11. Wait 1 second.
12. Click at 278,177.
13. Wait 2 seconds.
14. Move the mouse to 270,144.
15. Wait 1 second.
16. Click at 278,144.
17. Wait 2 seconds.
18. Move the mouse to 278,228.
19. Wait 1 second.
20. Click at 278,228.
21. Wait 2 seconds.
22. Take a screenshot called "instant action setup".
23. Expect "Federation" is visible.
24. Expect "Klingon" is visible.
