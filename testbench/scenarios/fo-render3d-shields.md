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

1. Include "_fo-render3d-skirmish".
2. Include "_fo-render3d-battle".
3. Wait up to 600 seconds for "Render3D.log" to say "effects: first shield hits drawn".
4. Take a screenshot called "shield hits".
5. Wait up to 20 seconds for the logs the next steps expect.
6. Expect "Render3D.log" contains "effects: shields: RTS_CFG.h: the shell scaled x2.00, hits white".
7. Expect "Render3D.log" contains "effects: shield hit ".
8. Wait 1 second.
9. Take a screenshot called "shield hits later".
10. Expect "Display8.log" not contain "VUID".
11. Expect "Display8.log" not contain "device removed".
12. Quit the game.
13. Expect the game to have exited.
14. Expect no crash.
