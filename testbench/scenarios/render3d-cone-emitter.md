# render3d: a cone beam's emitter flare on Vulkan, no black square

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Setup: display/bench-mode.sh stub assimilator
Timeout: 8 min

The Assimilator's auto-assimilator (`borgbor`, a `Kind=cone` recipe with `Emitter=5`)
on Display8's stub with `Backend=auto`, so Vulkan: run it with
`./a2test run render3d-cone-emitter --install .` (on the GPU by default).
Until render3d 0.26.1 the emitter flare drew as an opaque black square there (and only
there: Direct3D 9 drew it right), because the fx pixel shader weighted every kind by 0
or 1 and summed them, and the cone's kind, run on the flare's parameters, reached an
infinity: 0 x infinity is NaN on SPIR-V and DXIL (`render3d/README.md`, "One source, two
compilers"). The shots are judged: a small white-blue point where the
beam leaves the Assimilator's hull, no square around it.

1. Launch the game.
2. Wait up to 240 seconds for "Scene.log" to say "scene ready".
3. Expect "Display8.log" contains "stub: the backend is Vulkan".
4. Expect "Render3D.log" contains "module ordnance: on".
5. Scene "weapons assim on".
6. Wait 2 seconds.
7. Expect scene "special assim gborgbor tgt" answers ", fired".
8. Scene "weapons assim off".
9. Wait up to 20 seconds for "Render3D.log" to say "ordnance: drawing special borgbor (Cone)".
10. Wait 1 second.
11. Take a screenshot called "emitter".
12. Scene "orbit assim 200 20 260".
13. Wait 1 second.
14. Take a screenshot called "emitter-close".
15. Expect the game is still running.
16. Quit the game.
17. Expect no crash.
