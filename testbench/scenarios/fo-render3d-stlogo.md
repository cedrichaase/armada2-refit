# Fleet Operations: Roots with Render3D: a Federation ship's logo node is drawn with the texture the database gives it

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d
Setup: display/bench-mode.sh stub Backend=vulkan
Timeout: 10 min

FleetOpsHook is made to write each ship's own logo into the shared database before the draw render3d
claims (`CraftInstance::RenderInternal`, just before `ST3D_Instance::RenderInternal`): the
`stlogo` node's mesh gets the ship's logo texture, or is hidden. The claim honours the
database for every node of the SOD (`hulls::claim::honour_database`, FO-5 step 5, decision
21), so a mesh whose texture is not its SOD's is drawn with the database's. Render3D logs
the first eight such nodes: `hulls: <model> node stlogo drawn with <texture> (the database's
for this instance; the SOD has <texture>)`.

This is a close-up for the eye, not a log check, **and the log check cannot be written for
Roots 5.0Z11**: FleetOpsHook's logo (`CraftClass_InitLogo`) looks the class's database up
for a node named `stlogo` and gives it one of the textures the ODF's `logoFileNames`
lists, but none of the 626 SODs of the install (`Data\sod`, `Data\addon`) has a node
of that name, and none of the `logo_*` textures 63 ODFs list is in the install. So no
instance gets a logo, nothing is written for render3d to honour, and no `node ... drawn
with` line can come (none was seen on the construction ships, the station or four Sabers).
The lookup itself is shown by the claim's one line, `hulls: <model>: N of N shown nodes
found by name in the instance's database`, which this scenario expects. When a Roots
release or mod ships a SOD with an `stlogo` node, stage that ship and add
`Expect "Render3D.log" contains " node stlogo drawn with "`. The mouse wheel zooms the
camera towards the pointer; the Federation ship's hull is then judged textured, with its
markings.

1. Include "_fo-render3d-skirmish".
2. Take a screenshot called "skirmish".
3. Move the mouse to 433,590.
4. Wait 1 second.
5. Turn the wheel up 6 times.
6. Wait 2 seconds.
7. Move the mouse to 900,720.
8. Wait 2 seconds.
9. Take a screenshot called "close-up".
10. Check that Federation ships are shown (closer than in the "skirmish" shot if the wheel took), their hulls textured (not white or flat grey).
11. Expect "Render3D.log" contains " shown nodes found by name in the instance's database".
12. Expect "Display8.log" not contain "VUID".
13. Quit the game.
14. Expect the game to have exited.
15. Expect no crash.
