# Fleet Operations: Roots with Render3D: a Federation ship's logo node is drawn with the texture the database gives it

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d
Setup: display/bench-mode.sh stub Backend=vulkan
Timeout: 10 min

FleetOpsHook writes each ship's own logo into the shared database before the draw render3d
claims (`CraftInstance::RenderInternal`, just before `ST3D_Instance::RenderInternal`): the
`stlogo` node's mesh gets the ship's logo texture, or is hidden. The claim honours the
database for every node of the SOD (`hulls::claim::honour_database`, FO-5 step 5, decision
21), so a mesh whose texture is not its SOD's is drawn with the database's. Render3D logs
the first eight such nodes: `hulls: <model> node stlogo drawn with <texture> (the database's
for this instance; the SOD has <texture>)`.

The check is the line, then a close-up of the Federation ships at the start (the mouse wheel
zooms the camera towards the pointer) for the eye: the saucer of a Federation ship carries
its logo, which the SOD's own texture alone would not give.

1. Include "_fo-render3d-skirmish".
2. Take a screenshot called "skirmish".
3. Wait up to 20 seconds for "Render3D.log" to say " node stlogo drawn with ".
4. Expect "Render3D.log" contains " node stlogo drawn with ".
5. Move the mouse to 433,590.
6. Wait 1 second.
7. Turn the wheel up 6 times.
8. Wait 3 seconds.
9. Take a screenshot called "close-up".
10. Check that a Federation ship is shown close up, its hull textured (not white or flat grey) with a visible logo or marking on the saucer.
11. Expect "Display8.log" not contain "VUID".
12. Quit the game.
13. Expect the game to have exited.
14. Expect no crash.
