# Scene builder — test scenes inside a running mission

End-to-end scenarios reach a scene the way a player does, which makes renderer work
hard to reproduce: one model at one angle, one weapon firing, one ship beside one
planet. `Scene.asi` builds the scene instead. The bench launches straight into a map
(`-nointro a2_borg01`, about 30 s to the first tick, with no units on it). Once the
mission has run `Delay=` ticks, the plugin turns that empty stage into what a scene
file describes. A free camera and other commands then change the scene while it runs.

    SCENE=firing ./a2test session start --install . --install testbench/scene --args '-nointro a2_borg01'
    ./a2test drive --session ID scene "orbit shooter 120 20 250"   # look from another side
    ./a2test drive --session ID scene pause "orbit shooter 240 -30 250"
    ./a2test drive --session ID shot underside
    ./a2test session stop ID

A scenario puts the plugin in with `Setup: testbench/scene/bench-setup.sh <name>` and
drives it with `Scene "CMD"` and `Expect scene "CMD" answers "TEXT"` steps
(`scenarios/README.md`).

`SCENE=<name>` picks `scenes/<name>.ini`, `SCENE_INI=<file>` any file, and with neither
set `Scene.ini` is used (one Galaxy class). `install` refuses any game directory that is
not an a2test clone: this is a test tool and never goes into the player's install. The
bench gathers `Scene.log` and `Scene.ini` with the other logs.

## The scenes

| `SCENE=` | What | Seen on the bench |
|---|---|---|
| `planet` | a Galaxy class beside a class M planet (`pb_clssm`) | 2026-10-04 |
| `nebula` | a Galaxy class at the edge of the Mutara nebula (`mnebula8`) | 2026-10-04 |
| `firing` | a Galaxy class firing at a Borg cube (`bbattle1`) without moving, for as long as the session runs: its engines are off; the cube cannot die, is healed every tick and has its weapons off | 2026-10-04, still firing after a minute |
| `colony` | a fully developed class M planet (`Population=full`), its night side lit by cities, with four Federation ships (`fente`, `fbattle`, `fgalaxy`, `fcruise1`) parked off that side, engines on and no orders; the camera looks from the night side (`orbit planet 45 8 1100`). For the city lights (`lighting/README.md`, "Planets on the GPU") | 2026-10-07 |
| `factions` | one torpedo or pulse ship of each playable faction, each firing at its own Borg cube as in `firing`: Federation `fed` (`fgalaxy`), Klingon `kli` (`kbattle`), Borg `borg` (`bbattle1`), Cardassian `card` (`cbattle`), Romulan `rom` (`rbattle`), Species 8472 `sp` (`8472_mothership`); each target is `<name>_t`. For the weapons' light colours (`lighting/README.md`, "Torpedoes and pulses") | 2026-10-07 |
| `stations` | the player's own: three shipyards (`yard1..3`), an advanced shipyard (`adv`), a research station (`lab`) and two ships, with the HUD on, for the control-group keys and the build menu (`scenarios/qol-station-groups.md`) | 2026-10-05 |
| `showcase` | a fleet action for footage: the Enterprise-E (`fente`), a Sovereign (`fbattle`), a Galaxy and two Klingon flagships (`kmartok`, `kbattle`) firing at two Borg cubes that do not fire back (a Borg attack beams boarding parties over), in front of a class M planet; everyone immortal and healed, engines off | 2026-10-07 |
| `warp` | three of the player's Federation destroyers in open space at 3000,0,3000, with the HUD and cursor on, for long moves on the map going to warp (`scenarios/qol-warp.md`) | 2026-10-07 |

## Scene files

`[Scene]`:

| Key | Default | |
|---|---|---|
| `Enable` | 1 | 0 patches nothing |
| `Delay` | 30 | mission ticks before the scene is built |
| `Fog` | 0 | 0: fog and shroud off for good, the map fully explored; 1: as the map has them |
| `Hud`, `Grid`, `Cursor`, `Notices` | 0 | 0 hides them; 1 leaves them as the game has them. Notices are the game's events: "Enemy engaged." and the like, their voice and minimap marker |
| `Anchor` | `camera` | `world`: object positions are world coordinates; `camera`: offsets from the RTS camera's interest point |
| `Center` | | centre the RTS camera on this object |
| `Camera` | | a `camera` or `orbit` command (below), run once the scene is built |

`[Object.<name>]`, one per object, built in file order:

| Key | Default | |
|---|---|---|
| `Odf` | | the ODF name under `odf/` (`fgalaxy`, `bbattle1`, `pb_clssm`, `mnebula8`, ...) |
| `Team` | 1 | planets and nebulae take 0 (the editor forces them neutral) |
| `X`, `Y`, `Z` | 0 | position. Y is up |
| `Heading` | 0 | degrees about the up axis; 0 faces +z |
| `Immortal` | 1 | the object cannot die (craft only) |
| `Heal` | 0 | 1: health topped up to full every tick, so it never shows damage |
| `Engines`, `Weapons` | 1 | 0 disables them: an attacker with no engines fires without moving |
| `Attack` | | the name of an object to attack, ordered once everything is built |
| `Population` | | planets: colonised at once, as `colonize` (below) does: a number, `full`, or nothing for none |
| `Colonist` | 1 | the team that holds a planet with `Population=`, and so whose race's cities it shows |

**Where to put things.** On `a2_borg01` the map is the quadrant x > 0, z > 0, and the
RTS camera starts looking at its corner, 0,0,0. **A planet outside the map is built
but never drawn** (bench: one at x = −900 was invisible from any distance, one at
1500,0,1500 drew). The scenes therefore use `Anchor=world` around 2000,0,2000.
Scale: a Galaxy class is roughly 100 units long, and fills a 1920x1080 frame from
about 100 units away. A class M planet is roughly 450 units across.

**An ODF without a model draws as a placeholder.** A ship or station draws
`SOD/<odf>.sod`. Some ODFs are templates other ODFs include (`bbattle` is the base of
`bbattle1`..`4`) and have no SOD. They are still built, and drawn as the engine's
placeholder: a small cube with a red bug on every face. `spawn` logs a note when
there is no SOD.

## Commands

`./a2test drive scene "CMD" ["CMD" ...]` (or `;` between commands) writes them to
`Scene.cmd` in the clone. The plugin reads it on its next tick, or its next frame
while paused, and logs each command (`> ...`) and its answer. `drive scene` prints that
answer and fails if a command did.

| Command | |
|---|---|
| `orbit <object \| x y z> <yaw> <pitch> <distance>` | free camera on a sphere about a point or an object, which it follows. Yaw 0 looks from +z, 90 from +x; pitch is up from the horizon (±89) |
| `glide <seconds> <yaw> <pitch> <distance>` | after an `orbit`: move the camera to a new yaw, pitch and distance about the same point, eased in and out over that time. Yaw may pass 360 for a longer sweep |
| `spin <degrees per second>` | after an `orbit`: turn the yaw steadily; `spin 0` stops. A new `orbit` ends a glide but not a spin |
| `camera <ex> <ey> <ez> <object \| tx ty tz>` | free camera at an eye, looking at a point or an object |
| `camera rts` | back to the game's own camera |
| `spawn <name> <odf> <x> <y> <z> [heading] [team]` | build an object (anchored like the scene file's positions) |
| `attack <name> <target>` | order an attack |
| `heal`, `engines`, `weapons`, `immortal` `<name> on\|off` | as the keys above |
| `colonize <planet> [<population> \| full \| off] [team]` | make a planet a grown colony of a team (default `full` and team 1), without a colony ship: it changes hands, gets a full garrison, and its cities show at once at that population, clamped to the class's maximum. The cities are the team's race's (`cityTextureName`: `ECFR` for the Federation and Romulans, `ECNA`, `BORG`); team 1 on `a2_borg01` is the Federation. `off` makes it neutral again with no population. The planet stays colonised (seen on the bench for several minutes); a population set below the maximum grows as any colony's does |
| `center <name>` | centre the RTS camera on an object |
| `pause`, `resume` | the game's own pause (`PauseSimulation`) |
| `hud`, `grid`, `cursor`, `notices` `on\|off` | as the keys above |
| `query` | every object's handle and position (and a producer's build queue, a planet's team, population and the population its cities are drawn at), and the camera's eye, front and up |
| `select <name> [<name> ...]` | select the first as a click does and add the rest as Shift-clicks do (`cOverViewImp::Select`); answers as `selection` |
| `selection` | what is selected, each one as `name[g<group> q<queue> c<class>]` (`q` for producers only; group -1 for none), then every control group that is not empty, ships' and stations' |

**`glide` and `spin` run on wall-clock time, per frame** (`GetTickCount`, in the camera
hook), not per tick, so a recording plays them smoothly whatever the tick rate. Don't
use them while paused: the objects stay where the last simulated frame drew them.

**Moving the camera while paused runs the simulation for 3 ticks, then pauses again.**
While the simulation is paused, a moved camera draws the skybox from the new eye and
every object as from the last simulated frame's camera: the ship lands off-centre,
seen from the wrong side (seen on the bench, and gone after one resumed frame). The
cost: a beam or torpedo advances by 3 ticks with each camera move.

## How it works, and what it relies on in `Armada2.exe`

Addresses are from `armada2.map` (GOG patch 1.1). Every entry point is checked against
its first bytes before anything is patched or called. A different exe leaves the plugin
inert, and `Scene.log` says so.

- **The tick: one call site.** `Simulate` (0x483290) calls `GameObject_UpdateRange()`
  at 0x483351 on every tick. The plugin wraps that call to build the scene, heal, and
  read `Scene.cmd`.
- **Building an object: `BuildObject(char *odf, int team, const Matrix34 &)`**
  (0x451990, cdecl), a free function. The mission scripts' own
  `ScriptInterfaceImp::BuildObject` cannot start an empty scene: its third argument is
  an existing object's handle, the position is taken relative to that object, and it
  returns 0 without one. The free function takes a whole transform, so position and
  heading are both free. A `Matrix34` is three axis rows (right, up, front) and then the
  position. The new object's handle is the int at +0x28.
- **The script interface needs no mission DLL.** The mission scripts (`missions/*.dsl`,
  `*.drl`, ordinary Win32 DLLs) reach the engine through `g_pScriptInterface`
  (0x735c70). A static initialiser points it at a static `ScriptInterfaceImp`
  (0x735c40) at start-up, so its methods work on a map with no script. None of those
  used reads `this`:
  - `Attack(int, int, int)` 0x452be0: the attacker must be a craft with weapons; the
    target is any object.
  - `GetLocation(int)` 0x453140
  - `PauseSimulation()` 0x454cc0 / `UnpauseSimulation()` 0x454cf0
  - `CenterCamera(int)` 0x455190
  - `DisableEngines(int, bool)` 0x456960 / `DisableWeapons(int, bool)` 0x456a60
  - `SetCurrentHealth(int, float)` 0x456d20 / `GetMaxHealth(int)` 0x456da0
  - `CraftCannotDie(int, bool)` 0x457590

  `ScriptInterfaceImp::GridVisible(bool)` is an empty stub, so it cannot hide the grid.
- **Fog of war: `Scanner::ForceFogAndShroud(bool)`** (0x4935d0) writes the game setup's
  fog and shroud flags. `Scanner::IsFogged` / `IsShrouded` read them on every query, so
  `false` is the map as with fog and shroud off in the setup screen: explored, and never
  re-fogged. `Scanner::ForceUpdate()` (0x493600) makes the scanner recompute.
  `ScriptInterfaceImp::ClearFog` alone would not last, because fog grows back wherever
  no unit can see. Control on the bench: with `Fog=1` the shroud covers the map flat
  grey, and with `Fog=0` it is gone.
- **HUD: `DisplayInterface::SetInterfaceState(mode)`** (0x51a460, cdecl). The
  `toggle_interface` binding (Ctrl+I) steps a mode kept at `[0x76b5ac]+0x78` through
  0..3. 0 is the full HUD, 1 drops the tactical camera view (the 3D portrait of the
  selection), and 3 shows no HUD at all (seen on the bench, one Ctrl+I at a time).
- **Grid: `GridRenderState`**, three ints per view at 0x768e18, `{mode, ?, visible}`.
  The `grid_toggle` binding (Alt+G) cycles `mode`, and `GridRenderState::Update`
  (0x528080) derives the other two from it: mode 2 sets both to 0. `GridVisible()`
  (0x51e180), which the grid renderer asks, returns `visible`. The plugin writes
  `{2, 0, 0}` for views 0 and 1. Alt+G sent through the bench's virtual keyboard did
  nothing visible, so the toggle is set, not pressed.
- **The free camera: the one call in `s_UpdateMainCamera`.** `s_UpdateMainCamera`
  (0x53ed90) updates the main `ST3D_Camera` through a single virtual call,
  `call *0x88(%eax)` at 0x53edac, on the view object. That is
  `cOverViewImp::UpdateCamera`, which passes it on to `gCameraManager`'s current camera,
  whatever its class. The call becomes `call camera_update; nop`. The wrapper makes the
  same virtual call, then, when the free camera is on, calls the `ST3D_Camera`'s
  virtual `SetTransform` (slot 5) with its own camera-to-world matrix. `SetTransform`
  derives the rest itself (world-to-camera matrix, frustum). The camera-to-world matrix
  is at `ST3D_Camera`+0xc0 (`GetCameraToWorldTransform`), which `query` reads.
  A dead end that shaped this: patching `TacticalCamera::UpdateCamera`'s slot in
  `TacticalCamera`'s vtable did nothing, because the camera in use is not that class.
- **Cursor: the one `ST3D_Sprite::DrawScaled2D` call in `RefreshDisplay`** (0x6246fa),
  which draws the cursor under DXVK. `HUD.asi` wraps the same call ("Cursors" in
  `hud/README.md`). So `Scene.asi` patches it only once the scene is built, by when
  `HUD.asi` has long since loaded, and chains to whatever the call went to. With the
  cursor off, the draw is skipped. Clicks still land where the pointer is.
- **Notices: `GameEvent::TriggerEvent`.** "Enemy engaged." is the event
  `EVENTS_ENEMY_ENGAGED` from `events.dat` (text, voice, minimap marker). Events are
  fired through three entry points: `TriggerEvent()` 0x479880,
  `TriggerEvent(const Vector3 &, int, const Race *)` 0x4799a0, which the `GameObject`
  overload calls, and `TriggerEvent(const Race *)` 0x479bb0. With notices off, each
  entry returns false at once (`xor eax,eax; ret N`). The original bytes are kept and
  put back by `notices on`.
- **A colony without a colony ship: `Planet::StartWithColony(int team)`** (0x4b5660,
  thiscall), what a map that starts with a colony uses. It sets the population to the
  medium level, a garrison of 100 and the team (`SetTeam`, virtual), so the planet is
  the team's. `colonize` then sets the population asked for with
  `Planet::SetPopulation(float)` (0x4b5970), which also sets the population level and
  the maximum garrison; `Planet::GetMaxPopulation()` (0x4b5550) is the class's
  `maxPopulation` level, read from `RTS_CFG.h`'s `cfgPOP_*` (heavy, a class M planet's,
  is 5000), and is what `full` means. `Craft::SetCrew(float)` (0x4c83e0) fills the
  garrison, clamped to that maximum: `Planet::Simulate` neutralises a colonised planet
  whose garrison is 0. `off` is `Planet::NeutralizePlanet()` (0x4b5150) and a
  population of 0. A `Planet` is told from other objects by its vtable (0x6b2b3c);
  the team (+0xec) and population (+0x2ac) are what `query` reads.
  **What is drawn is a second, eased population.** `PlanetInstance::Update` copies the
  planet's +0x2c4 (the race whose cities are drawn) and +0x2c8 (the population they are
  drawn at), and `mSetupHemisphere` paints the population map from
  `pop / cfgPOP_HEAVY` against each race's `CityAllocArray` and binds the race's
  development texture (`PD_<cityTextureName><hemisphere>`). `Planet::Simulate` sets
  +0x2c4 to the team's race (`Team::GetTeam(int)` 0x496340, +0x244) and moves +0x2c8
  toward the population at 200 a second, so a planet colonised from nothing would take
  25 s to show a heavy planet's cities. `colonize` writes both itself.
  **`ScriptInterfaceImp::Colonize(int, int)` (0x452ca0) is not this**: it orders a
  colony ship (its first argument, a craft able to colonise) to colonise the planet
  (its second), and does nothing for any other first argument. That is why it did
  nothing when tried from this plugin with the planet alone.
- **Where the camera looks: `gTacticalCamera`** (0x763650). Its interest point, the map
  position the RTS camera looks at, is the `Vector3` at +0x98
  (`TacticalCamera::GetInterest`).

## Where this is going

- **A `Scene:` block in scenarios**, so a scenario names its scene and the bench
  installs it, and a first lighting regression scenario.
- **Screen-space boxes in `query`** (`ST3D_Camera::ProjectPoint`), so a step can
  measure or judge a crop of one object.
- **A second map**, once a scene needs more room than `a2_borg01`'s, with its bounds
  read rather than assumed.
