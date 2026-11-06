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

This is a close-up for the eye, not yet a log check: on the start (the two construction
ships and the station) and on four Sabers in a fight, no node was logged as drawn with a
texture other than its SOD's (no `node ... drawn with` line), so the override has not
been seen to fire on Roots with anything the bench can stage cheaply. The mouse wheel
zooms the camera towards the pointer; the Federation ship's hull is then judged textured,
with its markings. When a ship that carries Roots' logo node is staged, add the wait
and `Expect "Render3D.log" contains " node stlogo drawn with "` here.

1. Include "_fo-render3d-skirmish".
2. Take a screenshot called "skirmish".
3. Move the mouse to 433,590.
4. Wait 1 second.
5. Turn the wheel up 6 times.
6. Wait 2 seconds.
7. Move the mouse to 900,720.
8. Wait 2 seconds.
9. Take a screenshot called "close-up".
10. Check that a Federation ship is shown close up, its hull textured (not white or flat grey) with a visible logo or marking on the saucer.
11. Expect "Display8.log" not contain "VUID".
12. Quit the game.
13. Expect the game to have exited.
14. Expect no crash.
