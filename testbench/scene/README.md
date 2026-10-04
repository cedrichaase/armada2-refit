# Scene builder — a test scene inside a running mission

End-to-end scenarios reach a scene the way a player does, which makes renderer work
hard to reproduce: one model at one angle, one weapon firing, one ship beside one
nebula. `Scene.asi` builds the scene instead. The bench launches straight into a map
(`-nointro a2_borg01`, about 30 s to the first tick, with no units on it), and once the
mission has run `Delay=` ticks the plugin turns that empty stage into what `Scene.ini`
describes.

    ./a2test session start --install . --install testbench/scene --args '-nointro a2_borg01'
    SCENE_INI=my-scene.ini ./a2test session start ...   # a scene of your own

`install` refuses any game directory that is not an a2test clone: this is a test tool
and never goes into the player's install. The bench gathers `Scene.log` and `Scene.ini`
with the other logs.

**Status: a spike.** One object from `Scene.ini`, fog, HUD and grid off, and the camera
centred on it, seen on the bench 2026-10-04 at 1920x1080. "Where this is going" below
lists the rest.

## Scene.ini

| Key | Default | |
|---|---|---|
| `Enable` | 1 | 0 patches nothing |
| `Delay` | 30 | mission ticks before the scene is built |
| `Fog` | 0 | 0: fog and shroud off for good, the map fully explored; 1: as the map has them |
| `Hud` | 0 | 0: no HUD at all; 1: the HUD as usual |
| `Grid` | 0 | 0: no map grid; 1: as usual |
| `Odf` | `fgalaxy` | the object, by its ODF name under `odf/` (`fgalaxy` is the Galaxy class) |
| `Team` | 1 | |
| `Anchor` | `camera` | `camera`: `X`/`Y`/`Z` are an offset from the RTS camera's interest point; `world`: absolute |
| `X`, `Y`, `Z` | 0 | position |
| `Heading` | 0 | degrees about the up axis; 0 faces +z |
| `Immortal` | 1 | the object cannot die |
| `Center` | 1 | centre the RTS camera on the object afterwards |

On `a2_borg01` the camera's interest point starts at 0,0,0, which is the map's corner:
half the view is off the map, and with `Fog=1` that half is flat grey.

## How it works, and what it relies on in `Armada2.exe`

Addresses are from `armada2.map` (GOG patch 1.1). Every entry point is checked against
its first bytes before anything is patched or called. A different exe leaves the plugin
inert, and `Scene.log` says so.

- **The hook: one call site in the game tick.** `Simulate` (0x483290) calls
  `GameObject_UpdateRange()` at 0x483351 on every tick, *also while the simulation is
  paused*, unlike the object simulation beside it, which a flag at `[0x76b5ac]+0x84`
  skips. That call is pointed at the plugin, which calls the original first. A live
  scene (pause, then move the camera) needs a hook that keeps running, which is why
  this one was chosen.
- **Building an object: `BuildObject(char *odf, int team, const Matrix34 &)`**
  (0x451990, cdecl), a free function. The mission scripts' own
  `ScriptInterfaceImp::BuildObject` cannot start an empty scene: its third argument is
  an existing object's handle, the position is taken relative to that object, and it
  returns 0 without one. The free function takes a whole transform, so position and
  orientation are both free. A `Matrix34` is three axis rows (right, up, front) and
  then the position. The new object's handle is the int at +0x28.
- **The script interface needs no mission DLL.** The mission scripts (`missions/*.dsl`,
  `*.drl`, ordinary Win32 DLLs) reach the engine through the global
  `g_pScriptInterface` (0x735c70). A static initialiser points it at a static
  `ScriptInterfaceImp` (0x735c40) at start-up, so its methods can be called on a map
  with no script. `CenterCamera(int handle)` (0x455190) and `CraftCannotDie(int, bool)`
  (0x457590) are called that way. Neither reads `this`.
  `ScriptInterfaceImp::GridVisible(bool)` is an empty stub, so it cannot hide the grid.
- **Fog of war: `Scanner::ForceFogAndShroud(bool)`** (0x4935d0) writes the game setup's
  fog and shroud flags. `Scanner::IsFogged` / `IsShrouded` read them on every query, so
  `false` is the map as with fog and shroud off in the setup screen: explored, and never
  re-fogged. `Scanner::ForceUpdate()` (0x493600) makes the scanner recompute.
  `ScriptInterfaceImp::ClearFog` alone would not last, because fog grows back wherever
  no unit can see. The control on the bench: `Fog=1` leaves the off-map half of the view
  and the minimap flat grey, and `Fog=0` clears both.
- **HUD: `DisplayInterface::SetInterfaceState(mode)`** (0x51a460, cdecl). The
  `toggle_interface` binding (Ctrl+I in `Input.map`) steps a mode kept at
  `[0x76b5ac]+0x78` through 0..3 and applies it with this function. 0 is the full HUD,
  1 drops the tactical camera view (the 3D portrait of the selection), and 3 shows no
  HUD at all (seen on the bench, one Ctrl+I at a time). The plugin writes 3 and applies
  it. The cursor stays.
- **Grid: `GridRenderState`**, three ints per view at 0x768e18, `{mode, ?, visible}`.
  The `grid_toggle` binding (Alt+G) cycles `mode` and `GridRenderState::Update`
  (0x528080) derives the other two from it: mode 2 sets both to 0. `GridVisible()`
  (0x51e180), which the grid renderer asks, returns `visible`. The plugin writes
  `{2, 0, 0}` for views 0 and 1, the two the toggle serves. Alt+G sent through the
  bench's virtual keyboard did nothing visible, so the toggle is set, not pressed.
- **Where the camera looks: `gTacticalCamera`** (0x763650). Its interest point, the map
  position the RTS camera looks at, is the `Vector3` at +0x98
  (`TacticalCamera::GetInterest`).

## Where this is going

The plan (agreed 2026-10-04): scenes described in a scenario and built on demand, with
live control for agents.

1. **Several objects and a scene format**: named objects, a position and orientation
   each, an ODF, and later timed or triggered steps (`at 5s: ship attacks target`).
   It is interpreted, not compiled to a mission DLL, but designed so that it could be.
2. **A free camera**: `CameraManager::SetCamera(GameCamera *)` takes any camera, and a
   camera writes its view in its virtual `UpdateCamera(ST3D_Camera *)`. One of our
   own (eye, target, up, fov) keeps the engine's culling consistent with what is drawn,
   unlike a view matrix overridden at the Direct3D level.
3. **A live channel**: a loopback port per session and `a2test drive spawn` / `camera` /
   `attack` / `pause` / `tick N` / `query`. `query` returns each object's screen-space
   box, so a step can measure or judge a crop of one object.
4. **A `Scene:` block in scenarios**, and a first lighting regression scenario.

Open: whether `BuildObject` places nebulae, which may be map-grid features rather than
objects, and hiding the cursor (moving it off the view would do).
