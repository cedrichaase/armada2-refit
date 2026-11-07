# Fleet Operations: Roots with Render3D: the ice asteroid field of the first allied tutorial is drawn solid

Resolutions: 1280x960
Launch: -nointro tutorialgameplay.bzn
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan
Timeout: 14 min

The user's report of 2026-11-07 (3440x1440, Roots 5.0Z11): in Roots' first allied
tutorial (*TUTORIAL: Gameplay*) the ice asteroid field across the bottom of the opening
view looked washed out and see-through, while the brown field at the top right was
solid. Roots' asteroid sets (`map_asteroidset*`, `map_iceasteroidset*`) are latinum
nebulae (`classlabel = "latinum"`) whose SODs hold only sprite nodes: each field is a
handful of camera-facing quads painted with a sheet of rocks (`map_*astroid_field*`,
`@tmaterial=alpha`) over a haze of dust quads (`additive`). render3d's particles module
draws those puffs, and drew the `alpha` material alpha-tested at one half, as
`alphathreshold`: the ice rocks' alpha is soft (most of each rock under one half), so
only their highlights were left, over the haze; the brown rocks' alpha is a hard mask,
so they kept their shape. `alpha` (and `alpha_nof`, the same without filtering) is the
engine's alpha blend, and is now drawn blended (PLAN-FLEETOPS.md, decision 350).

The mission is launched straight, as in `fo-render3d-tutorial` (`Launch: -nointro
tutorialgameplay.bzn`). The opening view has the ice field
across its lower half. Each shot after the first hands one more part back to the
engine through the overlay, live: the particles module (the engine's own puffs, drawn by
Display8's passthrough into render3d's frame), then every module (the engine draws the
whole scene, the reference of stock Roots under Display8). The ice field in the first
shot should read as it does in the last two: whole pale rocks over the blue haze.

1. Launch the game.
2. Wait up to 180 seconds for "Render3D.log" to say "particles: nebula map_iceasteroidset".
3. Wait 10 seconds.
4. Take a screenshot called "field".
5. Check that the pale ice asteroids across the lower half of the view are drawn as whole rocks, each a solid pale shape over the blue haze, and not as scattered bright flecks with the haze showing through them.
6. Overlay Render3D.Modules:particles=0
7. Wait up to 20 seconds for the logs the next steps expect.
8. Expect "Render3D.log" contains "settings: applied Modules.particles=0"
9. Wait 3 seconds.
10. Take a screenshot called "field-engine-puffs".
11. Overlay Render3D.Modules:world=0 Render3D.Modules:hulls=0 Render3D.Modules:effects=0 Render3D.Modules:ordnance=0 Render3D.Modules:specials=0 Render3D.Modules:sky=0 Render3D.Modules:overlays=0
12. Wait 3 seconds.
13. Take a screenshot called "field-engine".
14. Expect "Render3D.log" contains "particles: nebula map_iceasteroidset"
15. Expect "Display8.log" not contain "VUID".
16. Expect "Display8.log" not contain "device removed".
17. Quit the game.
18. Expect the game to have exited.
19. Expect no crash.
