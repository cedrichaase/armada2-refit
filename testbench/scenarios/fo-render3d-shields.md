# Fleet Operations: Roots with Render3D: shield hits come from RTS_CFG.h (the shell x2, white hits)

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d
Setup: display/bench-mode.sh stub Backend=vulkan
Timeout: 20 min

Roots' `RTS_CFG.h` has `SHIELD_EFFECT_RESCALE = 1` and `SHIELD_EFFECT_DAMAGE_COLORED = 0`,
which FleetOpsHook honours in `ShieldHit::DrawShield` (the shell scaled by its factor, a
hit not coloured by the shields' level). Stock reads neither, so render3d's shield hits
read them where the profile says the build follows the keys (`r3d_core::rtscfg`, FO-5
step 5; the factor, 200 per cent, is the profile's `ef_shield_rescale_pct`). It logs
both, once, when the first hit is drawn: `effects: shields: RTS_CFG.h: the shell scaled
x2.00, hits white`, after `effects: first shield hits drawn`. Since FO-5 step 8 it also
logs the first live hit it sees in the engine's list (`effects: shields: first live hit
in the engine's list`), which tells a hit that never reaches the module from one that
does and is not recognised.

The fight is the AI's, as in `fo-render3d-debris` (`_fo-render3d-battle`).

**The hits are render3d's.** On the branch the scenario was written on it failed (run
20261106-054918): the engine's list held live hits whose model was none of the shield
database's, so render3d drew none of them and Roots' own `DrawShield` drew them all. The
cause is FleetOpsHook's pick of a hit's model in `CreateShieldHit` (per ship class, from
its ODF's `shieldHit`, `shieldDown` and the like); render3d now chains that call
(`effects.shield_models`) and knows each model by the type it was picked for. The log says
so at the module's first frame (`the companion's pick of each hit's model chained`), the
first live hit is then `noted as type 0`, and the hits drawn are render3d's (`first
shield hits drawn`, `shield hit N on ...`), with the shell x2 and white. The check is the
log's: the camera stays on the start view, and the AI's attack (on the station, which the
shroud may hide) is seldom in frame, so the two shots are kept for the eye only. A click on
the minimap on the base does not do it: the base's corner differs from game to game (run
20261106-074128 had it bottom left, the one before bottom right), and nothing on Roots
finds it but the eye. The first live hit is noted as type 0 or 1 (a ship's shields down),
so the check reads "noted as type ".

1. Include "_fo-render3d-skirmish".
2. Include "_fo-render3d-battle".
3. Expect "Render3D.log" contains "effects: shields: the companion's pick of each hit's model chained".
4. Wait up to 600 seconds for "Render3D.log" to say "effects: first shield hits drawn".
5. Take a screenshot called "shield hits".
6. Wait up to 20 seconds for the logs the next steps expect.
7. Expect "Render3D.log" contains "noted as type ".
8. Expect "Render3D.log" contains "effects: shields: RTS_CFG.h: the shell scaled x2.00, hits white".
9. Expect "Render3D.log" contains "effects: shield hit ".
10. Wait 1 second.
11. Take a screenshot called "shield hits later".
12. Expect "Display8.log" not contain "VUID".
13. Expect "Display8.log" not contain "device removed".
14. Stop the game.
15. Expect the game to have exited.
16. Expect no crash.
