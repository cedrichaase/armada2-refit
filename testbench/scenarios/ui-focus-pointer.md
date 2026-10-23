# The pointer after the game loses the focus and gets it back

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Setup: ui/bench-asi.sh UI.PointerLog=1
Timeout: 8 min

Run with `--install .`. A player saw the cursor jump about in a mission after a region
screenshot took the focus from the game (Hyprland, Display8 on Vulkan), until the
mission was left (`ui/README.md`, "Focus and the pointer"). In a mission Wine clips
the pointer to the full-screen window and the engine hides it, so Xwayland locks the
Wayland pointer and moves the X pointer by relative motion; a right-drag also has the
engine warp the pointer to the middle of the window at every read. Losing the X focus
undoes Wine's clip; getting it back may or may not redo it. The game itself is never
told: under Proton its window stays the foreground, active and focus window throughout.

`[UI] PointerLog=1` has UI.asi log each change of the foreground, active or focus
window and each jump of the pointer between two frames that the engine's pan did not
make. This walks both ways of losing the focus, a region screenshot's overlay
(`slurp`) and another window, with a right-drag before each and moves and a right-drag
after, and checks that the pointer never jumped and the engine was not left holding it
for a pan.

The cursor need not be where the steps put it after a round trip: the bench moves the
pointer by relative motion, and the motion made over the overlay reaches the
compositor's pointer but not the game's, so the game's ends up displaced (by about
+107,+62 after the overlay, -28,+27 after the window, on 2026-10-23). That displacement
is the gap between the two pointers that the player's jumps are thought to come from;
the checks ask only for one cursor in the right part of the screen.

Headless sway is not Hyprland: the bench has not shown the jumps themselves, so a pass
says the focus round trip, the engine's mouse state and the pointer are sound here,
not that the player's compositor behaves.

1. Launch the game.
2. Wait up to 90 seconds for "UI.log" to say "UI frames".
3. Expect "UI.log" contains "focus: foreground yes".
4. Note "a right-drag, then a region screenshot's overlay over the game".
5. Move the mouse to 400,300.
6. Right-drag from 600,300 to 700,350.
7. Take the focus from the game with an overlay for 1 second.
8. Move the mouse to 900,500.
9. Move the mouse to 300,200.
10. Take a screenshot called "after-overlay".
11. Note "a right-drag, then another window over the game, closed".
12. Right-drag from 600,300 to 500,250.
13. Take the focus from the game with a window for 1 second.
14. Wait 2 seconds.
15. Move the mouse to 800,450.
16. Move the mouse to 200,600.
17. Right-drag from 600,300 to 700,350.
18. Move the mouse to 640,200.
19. Take a screenshot called "after-window".
20. Expect "UI.log" not contain "pointer: jumped".
21. Expect "UI.log" not contain "pointer held for a pan".
22. Expect the game is still running.
23. Check that the shot "after-overlay" shows one mouse cursor over the map, in the upper left quarter of the screen.
24. Check that the shot "after-window" shows one mouse cursor over the map, in the upper half of the screen.
