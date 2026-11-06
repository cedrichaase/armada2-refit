# Fleet Operations: Roots with Render3D: the object step and the hull claim are installed, and nothing stands down

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d
Setup: display/bench-mode.sh stub Backend=vulkan
Timeout: 10 min

Run with `a2test --game fo-roots` and `--install .` (`fo-display8` has the setup):
`./install --game fo-roots` puts Display8, the ASI loader and Render3D into the clone's
`Data/`. `bench-mode.sh` sets `Backend=vulkan` and, on Vulkan, `Validate=1`: the game
draws on the GPU with the validation layer, so a bad call is a VUID in Display8's log
and not a hang. FO-5 (`PLAN-FLEETOPS.md`, "FO-5: render3d on Roots, the design") keeps
render3d's frame and steps on Roots and changes two places:

- the object step calls through the loop's own draw site (0x598297, which FleetOpsHook
  has redirected: `frame.obj_draw`, chained), so each object goes through Roots'
  per-object draw and its wrappers;
- hulls are claimed at the instance's database draw (0x4d5a11, `hulls.instance_draw`,
  the profile's `hl_claim_instance` = 1), not at the object, so what FleetOpsHook adds to
  a ship (range rings, overlay models, node and texture swaps) keeps running.

The checks are the log lines that say both are in, which the profile's identity lines
precede (the executable's file CRCs and the companion module's `.text` CRC, which every
resolver of a plugin checks against `profiles/fo-roots-5.0Z11/build.toml`), that the
race-keyed look has read the ODFs out of `ODF.fpq`, and that the modules which draw on
Roots did not stand down. The sky stands down on Roots on purpose (the engine's reads
there are not stock, `engine: render3d.sky.reads stands down`), so it is not checked.
The skirmish is the fragment's: Federation against Klingon, so the race is known.

1. Include "_fo-render3d-skirmish".
2. Take a screenshot called "skirmish".
3. Check that a space map is shown from above with ships and a station of the player's, resource counters along the top, a minimap at the bottom left, and the ships and stations textured (not white), and that the picture is not white.
4. Wait up to 20 seconds for the logs the next steps expect.
5. Expect "Render3D.log" contains "engine: build armada2/fo-roots-5.0Z11".
6. Expect "Render3D.log" contains "engine: ArmadaL.exe file crc: .text 0xef690537, .test 0xe7a4061b".
7. Expect "Render3D.log" contains "engine: companion FleetOpsHook.dll .text crc 0x28c90d19 matches the profile".
8. Expect "Render3D.log" contains "engine: hulls symbols: 85/85 resolved; 0 hooks blocked".
9. Expect "Render3D.log" contains "frame: the object step calls through the loop's draw site 0x598297".
10. Expect "Render3D.log" contains "hulls: hulls are claimed at the instance's database draw".
11. Expect "Render3D.log" contains "(claimed at the instance draw)".
12. Expect "Render3D.log" contains "is of race 'unitedfederation'".
13. Expect "Render3D.log" contains "files: ODF.fpq mounted at odf".
14. Expect "Render3D.log" contains "module hulls: on".
15. Expect "Render3D.log" contains "module effects: on".
16. Expect "Render3D.log" contains "module overlays: on".
17. Expect "Render3D.log" contains "module world: on".
18. Expect "Render3D.log" not contain "module hulls: stands down".
19. Expect "Render3D.log" not contain "module effects: stands down".
20. Expect "Render3D.log" not contain "module overlays: stands down".
21. Expect "Render3D.log" not contain "module world: stands down".
22. Expect "Display8.log" contains "engine: build armada2/fo-roots-5.0Z11".
23. Expect "Display8.log" not contain "VUID".
24. Expect "Display8.log" not contain "device removed".
25. Expect "Display8.log" not contain "bail-out".
26. Quit the game.
27. Expect the game to have exited.
28. Expect no crash.
