# Fleet Operations: Roots with Render3D: a dying craft's debris goes through the callee hook

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d
Setup: display/bench-mode.sh stub Backend=vulkan
Timeout: 20 min

On Roots FleetOpsHook owns the call of `ST3D_Instance::Fragment` that `Craft::Explode`
makes for a dying craft (`PLAN-FLEETOPS.md`, "The rules of (c)", rule 4), so render3d
cannot hook the call site: `effects.debris_callee` takes the function at its entry and
runs ours only while the return address lies in FleetOpsHook's module; the cancelled
construction's and the Evolver's cocoon's calls (the exe's) stay the engine's chunks. The
profile chooses it (`ef_debris_at_callee` = 1). Render3D logs the hook when it is
installed and, since FO-5 step 8, the first craft it takes at the callee, once:
`effects: debris: first dying craft taken at the callee`, followed by the chunks'
own line, `effects: debris: <odf> broke into N chunks`.

The kill is the AI's (`_fo-render3d-battle`): the Klingon AI's fleet reaches the base
within minutes and ships die on both sides. The wait is for the log line, up to ten
minutes of the game's time at its top speed, and the screenshots follow it.

1. Include "_fo-render3d-skirmish".
2. Expect "Render3D.log" contains "effects: debris: Fragment hooked at its entry, ours only while the companion module calls it".
3. Include "_fo-render3d-battle".
4. Wait up to 600 seconds for "Render3D.log" to say "effects: debris: first dying craft taken at the callee".
5. Wait 1 second.
6. Take a screenshot called "debris".
7. Wait up to 20 seconds for the logs the next steps expect.
8. Expect "Render3D.log" contains "effects: debris: first dying craft taken at the callee".
9. Expect "Render3D.log" contains " broke into ".
10. Wait 2 seconds.
11. Take a screenshot called "debris later".
12. Expect "Display8.log" not contain "VUID".
13. Expect "Display8.log" not contain "device removed".
14. Quit the game.
15. Expect the game to have exited.
16. Expect no crash.
