# The pointer and a desktop bar after the game loses the focus

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Setup: display/bench-mode.sh stub ui/bench-asi.sh UI.PointerLog=1
Timeout: 8 min

Run with `--install .`. A player on Hyprland (Omarchy) took a screenshot in a mission,
then moved the pointer into the top rows of the screen, where the desktop's bar is:
the desktop's own cursor showed and the game's stopped where it was (`ui/README.md`,
"Focus and the pointer"). In a mission Wine clips the pointer to the full-screen
window, and Xwayland, the cursor being hidden, locks the Wayland pointer to the game:
nothing else can have it. A screenshot's overlay takes the keyboard, Wine lets go of
its clip, and when the keyboard comes back Wine does not always clip again; an
unclipped pointer moves over the bar, which takes it from the game.

The bench's bar is `swaynag` on the overlay layer (sway draws its top layer under a
full-screen window; the player's bar, on Hyprland's top layer, took the pointer). It
never takes the keyboard. The screenshots are taken as Omarchy's Print key takes them,
a frozen screen under `slurp`. Each excursion pushes the pointer into a corner (the
compositor's and the game's pointers part at every screenshot), moves it up into the
bar, 600 pixels along it and down again: a pointer the bar took would come back to the
game 600 pixels from where the game's cursor stopped, which `PointerLog` logs as a jump.

With `[UI] KeepClip=1` (shipped) UI.asi asks Wine for its clip again while the pointer
moves; the scenario checks that it does and that the pointer stays the game's. The
bench does not show the escape itself (2026-10-23): sway gives an overlay bar the
pointer over a full-screen window, but in eleven such screenshots and two window round
trips Wine let its clip go for good only twice, and the pointer still did not reach the
bar then. A pass says the re-clip runs and breaks nothing here, not that it cures the
player's compositor.

1. Launch the game.
2. Wait up to 90 seconds for "UI.log" to say "UI frames".
3. Show a bar along the top of the screen.
4. Move the mouse to 300,300.
5. Right-drag from 600,300 to 700,350.
6. Note "screenshot 1, then up into the bar and along it".
7. Take the focus from the game with a screenshot for 1 second.
8. Move the mouse into the bottom right corner.
9. Move the mouse to 300,200.
10. Move the mouse to 300,4.
11. Move the mouse to 900,4.
12. Move the mouse to 900,300.
13. Note "screenshot 2, then up into the bar and along it".
14. Take the focus from the game with a screenshot for 1 second.
15. Move the mouse into the bottom right corner.
16. Move the mouse to 900,200.
17. Move the mouse to 900,4.
18. Move the mouse to 300,4.
19. Move the mouse to 300,300.
20. Note "screenshot 3, then up into the bar and along it".
21. Take the focus from the game with a screenshot for 1 second.
22. Move the mouse into the bottom right corner.
23. Move the mouse to 300,200.
24. Move the mouse to 300,4.
25. Move the mouse to 900,4.
26. Move the mouse to 900,300.
27. Take a screenshot called "after".
28. Hide the bar.
29. Expect "UI.log" contains "pointer: Wine's clip asked for again".
30. Expect "UI.log" not contain "pointer: jumped".
31. Expect the game is still running.
32. Check that the shot "after" shows one mouse cursor over the map, in the right half of the screen.
