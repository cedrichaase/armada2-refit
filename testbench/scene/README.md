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
set `Scene.ini` is used (one Galaxy class). `SCENE_RACE=federation` (or any `Race=` value) pins
the player's race, and so the HUD's look, for whichever scene is installed: without it
the bench draws a different race on every launch. `install` refuses any game directory that is
not an a2test clone: this is a test tool and never goes into the player's install. The
bench gathers `Scene.log` and `Scene.ini` with the other logs.

## The scenes

| `SCENE=` | What | Seen on the bench |
|---|---|---|
| `zoom` | the player's Galaxy class (`mine`) and two destroyers, the HUD and cursor on, the camera centred on `mine` at zoom level 0.37: the view the old workaround of 150 `ctrl+Down` then 35 `ctrl+Up` presses reaches, in none of its 28 s | 2026-10-17 |
| `planet` | a Galaxy class beside a class M planet (`pb_clssm`) | 2026-10-04 |
| `nebula` | a Galaxy class at the edge of the Mutara nebula (`mnebula8`) | 2026-10-04 |
| `firing` | a Galaxy class firing at a Borg cube (`bbattle1`) without moving, for as long as the session runs: its engines are off; the cube cannot die, is healed every tick and has its weapons off | 2026-10-04, still firing after a minute |
| `colony` | a fully developed class M planet (`Population=full`), its night side lit by cities, with four Federation ships (`fente`, `fbattle`, `fgalaxy`, `fcruise1`) parked off that side, engines on and no orders; the camera looks from the night side (`orbit planet 45 8 1100`). For the city lights (`render3d/README.md`, "world") | 2026-10-07 |
| `factions` | one torpedo or pulse ship of each playable faction, each firing at its own Borg cube as in `firing`: Federation `fed` (`fgalaxy`), Klingon `kli` (`kbattle`), Borg `borg` (`bbattle1`), Cardassian `card` (`cbattle`), Romulan `rom` (`rbattle`), Species 8472 `sp` (`8472_mothership`); each target is `<name>_t`. For the weapons' light colours (`render3d/README.md`, "ordnance") | 2026-10-07 |
| `impacts` | a Klingon flagship (`kli`, `kbattle`: torpedoes and phasers) firing at a Galaxy class (`tgt`), and a Species 8472 mothership (`sp`, pulses) at another (`tgt2`), 800 apart; targets healed, immortal and unarmed, no shield flash (`ShieldFx=0`), so every hit lands on the shield until `heal tgt off`. For the impacts' light (`render3d/README.md`, "ordnance") | 2026-10-12 |
| `stations` | the player's own: three shipyards (`yard1..3`), an advanced shipyard (`adv`), a research station (`lab`) and two ships, with the HUD on, for the control-group keys and the build menu (`scenarios/qol-station-groups.md`) | 2026-10-05 |
| `showcase` | a fleet action for footage: the Enterprise-E (`fente`), a Sovereign (`fbattle`), a Galaxy and two Klingon flagships (`kmartok`, `kbattle`) firing at two Borg cubes that do not fire back (a Borg attack beams boarding parties over), in front of a class M planet; everyone immortal and healed, engines off | 2026-10-07 |
| `warp` | three of the player's Federation destroyers in open space at 3000,0,3000, with the HUD and cursor on, for long moves on the map going to warp (`scenarios/qol-warp.md`) | 2026-10-07 |
| `ui-colonize` | the player's colony ship (`colony`, `fcolony`, engines on) and a Galaxy class (`mine`) beside an unclaimed class M planet (`planet`), with the HUD, cursor, tooltips and hover on, the camera on the colony ship: a left click on the planet selects it, a right click with the colony ship selected colonises it (`scenarios/ui-hud-colonize.md`) | 2026-10-16 |
| `ui-hud-layout` | thirty of the player's Federation destroyers (`s1`..`s30`), engines and weapons off, six by five about the camera's start, with the HUD, cursor, tooltips and hover on, for the HUD's layout with a big selection (`scenarios/ui-hud-layout.md`) | 2026-10-17 |
| `explosion` | a Galaxy class beside a destroyer (`small`), a cruiser (`mid`) and a battleship (`big`), none firing, to blow up with `kill`, for the explosions (`render3d/README.md`, "effects"; `scenarios/explosions.md`) | 2026-10-12 |
| `shields` | a Galaxy class (phasers) and a Klingon battleship (`kbattle`, torpedoes) firing at a second Galaxy (`target`) as in `firing`: the target healed, its shields holding. For the shield hits (`render3d/README.md`, "effects"; `scenarios/shields-hits.md`) | 2026-10-12 |
| `fog-planet` | a class M planet (`planet`) at 3000,0,3000 with the player's scout beside it, the map's fog and shroud on and the HUD on; the camera looks down on the planet. `goto scout 6500 0 6500` leaves the planet as a fog-of-war ghost. For render3d's planet ghosts (`render3d/README.md`, "overlays"; `scenarios/render3d-planet-ghost.md`) | 2026-10-17 |
| `fog-blackhole` | a black hole (`hole`, `mblkhole`, 70 below the grid as its ODF has it) at 3000,0,3000 and a quantum singularity (`rift`) at 3500,0,3450, both neutral, with the player's scout beside them, the map's fog and shroud on and the HUD on; the camera looks down on the black hole. `goto scout 6500 0 6500` leaves both as fog-of-war ghosts. For render3d's ghosts of additive meshes (`render3d/README.md`, "overlays"; `scenarios/fog-blackhole.md`, issue #3) | 2026-10-21 |
| `fog` | three of the player's ships (`ship1`, `ship2`, `scout`) at 3000,0,3000 with the map's fog and shroud on (`Fog=1`) and the HUD on for the minimap; `goto scout ...` leaves fog of war behind. For the fog's and shroud's edges (`render3d/README.md`, "overlays"; `scenarios/fog-edges.md`) | 2026-10-12 |
| `asteroids` | an asteroid field (`field`, `mafield`: 4–8 rocks) and a big one (`big`, `mafield2`: 3–6 larger rocks), 500 apart, beside a Galaxy class with its engines off; the rocks are drawn at random from the field's models, so a session shows some of them, and `spawn` adds more fields. For the asteroids' surfaces and smoothing (`render3d/README.md`, "world"; `models/README.md`, "The asteroids") | 2026-10-13 |
| `venting` | a Galaxy class (`galaxy`) and a Klingon battleship (`klingon`) parked 300 apart, healed and immortal, every system's damage node showing as if damaged (`Vent=all`): the Galaxy's nacelle vents plasma, its hull gas. Nothing fires. For the venting plasma and gas (`render3d/README.md`, "particles") | 2026-10-13 |
| `assimilation` | the Borg technology assimilation beam: a Borg cube (`borg`, `bbattle1`, the player's) 220 from an enemy Sovereign (`tgt`), engines and the cube's weapons off, special energy full, both healed, the Sovereign unarmed, its corbomite reflector made available to its team (`Tech=gcorbom`), shield flashes and hover gauges off. `weapons borg on`, then a moment later (not on the same tick) `special borg gTechAssimBeam tgt`: the beam reaches the Sovereign about 1 s later and holds until about 7 s after firing, when the cube has taken the weapon; a cube takes one (`maxExtraWeapons`). Weapons off ends the beam; as they come on, and after the beam, the cube fires its phasers. For render3d's assimilation cone (`render3d/README.md`, "Ordnance"; `films/assimilation`) | 2026-10-17 |
| `assimilation-fusion` | as `assimilation`, from a Borg Fusion Cube (`borg`, `bbattle3`, about twice the cube's size) 300 from the Sovereign: `weapons borg on`, then `special borg gTechAssimBeam tgt`; its phasers fire as the weapons come on. For footage (`films/beams-scout`): the darkest sky behind the beam is seen from above, pitch +15 to +30 at yaw 60 to 100 | 2026-10-17, stock |
| `assimilator` | the Borg Assimilator (`assim`, `bcruise2`, the player's) 300 from an enemy Sovereign (`tgt`), both healed, engines off, special energy full: `weapons assim on`, `special assim gborgbor tgt`, then `weapons assim off` at once; the auto-assimilator holds with the weapons off (its 30 s) and the torpedoes stay quiet. The healed Sovereign keeps its crew, so it is never taken. For footage (`films/beams-scout`) | 2026-10-17, stock |
| `specials` | an enemy Sovereign (`tgt`) that cannot die, with its weapons and engines off and not healed, and a Galaxy class (`warper`) at 1500,0,1500 for the warp flash; shooters come with `spawn` and fire with `special`. For the special weapons and the warp flash (`render3d/README.md`, "ordnance"; `scenarios/specials.md`) | 2026-10-13 |
| `cloak` | the player's own Romulan warbird (`warbird`, `rbattle`) and Klingon Bird of Prey (`bop`, `kdestroy`) in front of a class M planet and a Galaxy class, engines and weapons off; `cloak warbird on`. For cloaked ships drawn as glass (`render3d/README.md`, "hulls"; `scenarios/cloak.md`) | 2026-10-13 |
| `mining` | a dilithium moon (`moon`, `mdmoon`), the player's mining station (`station`) behind it, a Federation and a Borg freighter (`freighter`, `borg`) and a Galaxy class, engines off. The freighters do not mine on their own (a moon the scene builds has no resource): `special freighter gmineDilithium moon` (or `gmineLatinum`; the Borg's `gmineDilithiumBorg`, `gmineMetalBorg`) fires the beam for 1.25 s, `pause` holds it. For the mining beams (`render3d/README.md`, "Ordnance") | 2026-10-15 |
| `ordmodels` | an enemy Sovereign (`tgt`) and about it the player's shooters of the special weapons that draw a model, engines off, special energy full: `fd` (`gamine`), `kd` (`ggrvmine`), `kc` (`gkcteam tgt`), `rc` (`gmyotron tgt`), `cc` (`gfluxwave`), `fc` (`gchainp tgt`), `bd` (`gtwarpd`). Mines are laid at the layer's position, inside its hull: `goto` it first and lay them under way; within 600 of an enemy they chase it. For the ordnance models (`render3d/README.md`, "Ordnance", `scenarios/render3d-ordmodels.md`) | 2026-10-15 |
| `construction` | the player's Federation construction ship (`rig`, `fconst`), a finished starbase (`base`) and a Galaxy class (`galaxy`); `build rig fyard 2300 0 2000` (or `fbase`) has the rig build a station there: worker bees, welding, the station rising face by face. For construction (`render3d/README.md`, "effects"; `scenarios/render3d-construction.md`) | 2026-10-15 |
| `sky` | nothing: no objects, the free camera at the middle of the map looking out (`orbit 3000 0 3000 0 0 1`), for judging the skies on their own; `Load=` in `Render3D.ini`'s `[Sky]` steps through the backgrounds (`render3d/README.md`, "sky") | |

## Scene files

`[Scene]`:

| Key | Default | |
|---|---|---|
| `Enable` | 1 | 0 patches nothing but the music, captions and race switches below (`Enable=0` with `Music=0`: a plain game with no music, for footage of the real campaign; with `Captions=0`, its cinematics without their text) |
| `Delay` | 30 | mission ticks before the scene is built |
| `Fog` | 0 | 0: fog and shroud off for good, the map fully explored; 1: as the map has them |
| `Hud`, `Grid`, `Cursor`, `Notices`, `Tooltips` | 0 | 0 hides them; 1 leaves them as the game has them. Notices are the game's events: "Enemy engaged." and the like, their voice and minimap marker. Tooltips are the rollover boxes, an object's name and description, that the pointer brings up over the 3D view even when the cursor is not drawn |
| `Hover` | 1 | 0 removes what the object under the pointer gets drawn over it: its hull, shield and special-energy gauges, its system icons and the hover ring, which come up even with the HUD and the cursor hidden. 1 is stock. Selection circles and group numbers stay |
| `ShieldFx` | 1 | 0 removes the flash on a craft's shields when a weapon hits them (green on most). The shields still take the damage. 1 is stock. The shields-down effect (a blue electric ring when they run out) is separate and stays; `Heal=1` prevents it |
| `Music` | 1 | 0 stops the game starting any music track (`StartMusic` and the jukebox's `mStartNewTrack` return at once); sound effects and voices stay. `SCENE_MUSIC=0` at install puts it into the installed copy of any scene file, so footage can be laid over its own soundtrack |
| `Captions` | 1 | 0 removes a mission script's captions: the subtitles and stardate lower thirds burned into the intro cinematics and the mission's title (`ScriptInterfaceImp::ShowTitle`, `ShowSubtitle`; "How it works"). Everything else in the cinematic, its timing, sound and voices, stays, and so do the game's own overlay messages. Set at start-up, so it holds with `Enable=0`, in stock and refit alike (`films/intros-scout`) |
| `Race` | | the player's race, which decides the HUD (stock's skin, and the UI's theme with `[UI] Theme=auto`): `federation`, `klingon`, `romulan`, `cardassian`, `borg` or `species8472` (races.odf's `name`s). Empty: as the game sets it, which on `a2_borg01` is a different race on every launch. Set before the mission loads, so it holds in stock and refit alike, and with `Enable=0`. `SCENE_RACE=<race>` at install puts it into the installed copy of any scene file, ahead of the file's own |
| `Anchor` | `camera` | `world`: object positions are world coordinates; `camera`: offsets from the RTS camera's interest point |
| `Center` | | centre the RTS camera on this object |
| `Zoom` | | the RTS camera's zoom level, 0 (closest) to 1 (farthest), set once the scene is built (`zoom`, below; "The zoom"). Unset leaves the camera as the game started it |
| `Camera` | | a `camera` or `orbit` command (below), run once the scene is built |

`[Marks]`, the event marks (below, "Marks"); every scene has them without the section:

| Key | Default | |
|---|---|---|
| `Enable` | 1 | 0 writes no `Scene.marks` |
| `FireGap` | 1.0 | seconds without a shooter's ordnance of a weapon that end its burst (`fire.end`) |
| `Still` | 0.3 | seconds without the camera moving that settle it (`camera.still`) |

`[Object.<name>]`, one per object, built in file order:

| Key | Default | |
|---|---|---|
| `Odf` | | the ODF name under `odf/` (`fgalaxy`, `bbattle1`, `pb_clssm`, `mnebula8`, ...) |
| `Team` | 1 | planets and nebulae take 0 (the editor forces them neutral) |
| `X`, `Y`, `Z` | 0 | position. Y is up |
| `Heading` | 0 | degrees about the up axis; 0 faces +z |
| `Immortal` | 1 | the object cannot die (craft only) |
| `Heal` | 0 | 1: hull, shields, crew and every system topped up to full every tick, so it never shows damage: its shields never run out (no shields-down effect, the electric ring) and no damaged-system effect (the plasma plume, venting gas) ever shows |
| `Energy` | 0 | 1: the craft's special energy topped up to full every tick, so a special weapon that runs on it (a tractor beam, a holding beam) keeps running |
| `Tech` | | tech-tree items (`gcorbom`, `fedpod1` ...), space-separated, made available to the object's team as if researched, as `tech` (below) does |
| `Engines`, `Weapons` | 1 | 0 disables them: an attacker with no engines fires without moving. A system switched off here is not drawn as damaged (no plume) |
| `Vent` | | `all`, `none` or one system (`shields`, `engines`, `weapons`, `sensors`, `life`): its damage node shows as a damaged system's does -- the plume, the venting gas -- healed or not, and the system keeps working |
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
| `zoom [<level>]` | the RTS camera's zoom level, 0 (closest) to 1 (farthest), clamped. With no level it only reads. It answers with the level the camera has now, the camera in use, its raw value and its limits: `zoom 0.37 (parabolic camera, height 809.81, limits 10.00 to 2200.00)`. `query` ends with the same line |
| `spawn <name> <odf> <x> <y> <z> [heading] [team]` | build an object (anchored like the scene file's positions) |
| `attack <name> <target>` | order an attack |
| `succeed [seconds]`, `fail [seconds]` | the mission ends, won or lost, after the delay, as its script ends it (`ScriptInterfaceImp::SucceedMission`, 0x457d70, and `FailMission`, 0x457d50); for the screens that follow a mission (`scenarios/ui-menus-success.md`) |
| `kill <name>` | the craft dies at once, as a mission script blows one up (`ScriptInterfaceImp::ExplodeNow`, 0x456cf0); its healing and immortality are switched off first |
| `special <name> <weapon> [<target>]` | fire a craft's special weapon, by its weapon ODF's name (`gholdbm`, `gcorbom`), at a target or none: its special energy is filled first, the weapon's slot in the craft's carrier found (`WeaponClass::Find`), and `Craft::FireAtSpecial` (0x4c3820) called as a player's order does. Answers `slot N, can fire, fired` (or `cannot fire now`, `NOT fired`). The craft's weapons must be on (`Carrier::TriggerSelected` refuses while they are off) and the target within the weapon's range |
| `energy <name> on\|off` | as `Energy=` |
| `tech <team \| object> <item> [on\|off]` | a tech-tree item (an ODF name as the `techtree/*.tt` files spell it: `gcorbom`, `fedpod1`) made available to a team (an object's, given its name) as if researched, or not (`off`); answers what it was. Only the item changes, not what depends on it. A map's tree makes some items impossible for every team (`-1`; on `a2_borg01` `fedpod1` and with it the Sovereign's `gcorbom`): `on` makes the item possible too. For the Borg technology assimilation beam, which only holds on a ship that carries a special weapon its team has available ("How it works", "Tech tree") |
| `cloak <name> on\|off\|now` | `on`: the craft starts to cloak, `off`: to decloak, `now`: fully cloaked at once (`GameObject::ImmediateCloak`, 0x4d03e0). Its special energy is filled and kept full while it is on. It sets the request byte of the craft's cloaking device (+0x108, byte +0x2c) that the player's button leaves for `CloakingDevice::Simulate`, rather than calling `ScriptInterfaceImp::Cloak`: that asks `AbleToCloak`, which wants the team to have researched the device, and it refused the bench's ships. `query` shows each craft's state (`cloak visible`, `cloaking`, `cloaked`, `decloaking`) |
| `goto <name>[,<name>...] <object \| x y z> [warp]` | order a move, as a player's right-click does (AiCommand `GO`; `warp`: `GO_WARP`), and switch the engines on. To a craft, the ship follows it; to any other object (a nebula, a planet), it goes to where the object is. A point is anchored like the scene file's positions. Several names, comma-separated, keep their places: each goes to the point plus its own offset from the group's centre. It ends an `attack`, though a ship still fires at what comes in range |
| `build <rig> <odf> <x y z \| object>` | order a construction ship to build a station (an ODF name, `fyard`, `fbase`...) at a point (anchored like the scene file's) or where an object is, as a player's build order (AiCommand `BUILD` with the class from `GameObjectClass::Find` and the point, through `GameObject::SetCommand`, 0x4d1cb0, as `ScriptInterfaceImp::SetCommandWhatWhoWhereVO` gives it). The rig's team gets every resource topped up first (`Team::AddResource`, clamped to its maximum) and its engines are switched on. The rig flies there and opens; its bees start the station at that point (fbase took about 2 minutes to start on the bench, fyard less) |
| `stop <name>` | order a stop (AiCommand `STOP`, as `ScriptInterfaceImp::Stop` gives) |
| `heal`, `engines`, `weapons`, `immortal` `<name> on\|off` | as the keys above |
| `vent <name> <system \| all> on\|off` | as the `Vent` key, one system at a time (or all five) |
| `colonize <planet> [<population> \| full \| off] [team]` | make a planet a grown colony of a team (default `full` and team 1), without a colony ship: it changes hands, gets a full garrison, and its cities show at once at that population, clamped to the class's maximum. The cities are the team's race's (`cityTextureName`: `ECFR` for the Federation and Romulans, `ECNA`, `BORG`); team 1 on `a2_borg01` is the Federation. `off` makes it neutral again with no population. The planet stays colonised (seen on the bench for several minutes); a population set below the maximum grows as any colony's does |
| `center <name>` | centre the RTS camera on an object |
| `pause`, `resume` | the game's own pause (`PauseSimulation`) |
| `hud`, `grid`, `cursor`, `notices`, `tooltips`, `hover`, `shieldfx` `on\|off` | as the keys above. `tooltips off` also clears a box already showing; `shieldfx off` lets flashes already showing run out |
| `query` | every object's handle and position (and a producer's build queue, a planet's team, population and the population its cities are drawn at), and the camera's eye, front and up |
| `select <name> [<name> ...]` | select the first as a click does and add the rest as Shift-clicks do (`cOverViewImp::Select`), up to 39 names (a command has at most 40 words); answers as `selection` |
| `selection` | what is selected, each one as `name[g<group> q<queue> c<class>]` (`q` for producers only; group -1 for none), then every control group that is not empty, ships' and stations' |
| `marks` | how many marks `Scene.marks` has, and the last 16, each as written (`Expect scene "marks" answers "cloak.done obj=warbird"`) |

## The zoom

`Zoom=` and `zoom` set the RTS camera's zoom as a **level from 0 (closest) to 1
(farthest)**, whichever camera the game is using. The camera in use is the first field
of `gCameraManager` (0x763370), and on the bench it is a `ParabolicCamera`: its zoom is
a height above the map, 10 to 2200 (`OVERVIEW_MIN_HEIGHT`, `OVERVIEW_MAX_HEIGHT` as
the game has them at run time), and the game starts it at 150 (level 0.06). A
`TacticalCamera` keeps a distance factor from 1 to 20 instead (start 10); the plugin
handles it the same way, but it has not been seen in use. Both implement
`SetZoomLevel(t)`, a straight interpolation between their limits and the call
`ScriptInterfaceImp::SetCameraZoom` (0x4551d0) makes, so the level is the same on both.
The plugin refuses if the camera in use is neither class.

- **Level to height** (parabolic): `10 + level * 2190`. Level 0.06 is the start, 0.37 is
  a player's-eye view of a few ships, 1 shows a patch of the map.
- **Measuring a manual setup**: put the camera where it should be with the wheel and
  `ctrl+Up`/`ctrl+Down`, then `zoom` (or `query`) reads the level to put in `Zoom=`.
- **The keys are not repeatable.** Each press changes the height by a factor that
  depends on the frame time (`ctrl+Down` x5 took 1105 to 1273, `ctrl+Up` x5 then took
  it back only to 1113). 150 `ctrl+Down` end at the limit (1.00) whatever the frame
  time, and 35 `ctrl+Up` after them reach **0.36 to 0.37** (heights 806 and 810, level 0.365, two
  runs at 1920x1080 and 0.15 s a press). **`Zoom=0.37` is the workaround's view**.
- **Free camera modes.** The free camera (`camera`, `orbit`, and so `glide` and `spin`)
  replaces the transform the game's camera computes, so zoom does not move it: its
  distance is the `orbit` command's own. `zoom` still sets the RTS camera underneath
  and says so; the zoom shows after `camera rts`. A `Zoom=` and a `Camera=` orbit in
  one scene file therefore do not fight: the scene file sets the zoom, then the orbit
  takes over the picture. A scene that keeps the game's camera sets `Zoom=` and no `Camera=`.
- **Anything that moves the game's camera afterwards** wins over a zoom set earlier:
  the wheel and `ctrl+Up`/`ctrl+Down`. `Zoom=` is applied once, after
  `Center=` and before `Camera=`, when the scene is ready.
- The line `zoom as the game started it:` in `Scene.log` shows the zoom before `Zoom=`
  applied it.

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
  read `Scene.cmd`. One more hook, in a vtable, hides the damage effects of systems the
  scene switched off ("Damaged-system effects" below).
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
  cursor off, the draw is skipped. Clicks still land where the pointer is. `UI.asi`
  wraps the same call when it loads, behind Scene.asi's wrap, and draws no cursor of
  its own while the draw does not reach it (ui/README.md, "The cursor").
- **Notices: `GameEvent::TriggerEvent`.** "Enemy engaged." is the event
  `EVENTS_ENEMY_ENGAGED` from `events.dat` (text, voice, minimap marker). Events are
  fired through three entry points: `TriggerEvent()` 0x479880,
  `TriggerEvent(const Vector3 &, int, const Race *)` 0x4799a0, which the `GameObject`
  overload calls, and `TriggerEvent(const Race *)` 0x479bb0. With notices off, each
  entry returns false at once (`xor eax,eax; ret N`). The original bytes are kept and
  put back by `notices on`.
- **Tooltips: the one `DisplayInterface::TooltipOn()` call in
  `SelectionDisplay::AlwaysSimulate`** (0x508070). Each tick that function asks
  `TooltipOn()` (0x51b610, the object `s_PrecomputeMouseOver` found under the
  pointer) and, with an object, hands it to the cursor's
  `CursorInterface::DoTooltip` (0x505a80), which shows the box once the pointer has
  rested for `s_tooltipDelay`; with none it calls `ClearTooltips` (0x505d40). With
  tooltips off the call becomes `xor eax,eax` plus three `nop`s, "no object", so the
  box is cleared and never comes up. The original five bytes are kept and put back by
  `tooltips on`. Nothing else asks `TooltipOn()`. Clicks, orders and the cursor's
  shape go by `DisplayInterface::MouseOn()` (0x51b5e0, `s_mouseOverObjectHandle`),
  which this leaves alone, so the pointer still hovers and clicks as before. The
  `stations` scene, which films the HUD with the cursor on, gets tooltips off too
  unless it sets `Tooltips=1`. The gauges drawn over an object the pointer rests on
  do not come from `TooltipOn()`; `hover` (below) removes them.
- **Hover: the two `DisplayInterface::MouseOn()` calls in `SelectionDisplay`.**
  `SelectionDisplay::Render` (0x509540) walks every object and draws its gauges
  (`mDrawHullGauge`, `mDrawShieldGauge`, `mDrawSpecialEnergyGauge`) and system icons
  when it is the *hover object*, `SelectionDisplay`+0x7c, or when it is selected and
  on the hover object's team, +0x88; an unselected object other than the hover object
  is skipped. Both fields are written in `SelectionDisplay::AlwaysSimulate` from the
  one `MouseOn()` call at 0x5080fc (+0x88 is the hover object's team, +0x8c a fade-in
  time). The hover ring is separate: `SelectionDisplay::PreRender` asks `MouseOn()`
  itself, at 0x508e72, and moves selection effect 4 (`AddSelectionEffect` /
  `RemoveSelectionEffect`) to that object. With hover off both calls become
  `xor eax,eax` plus three `nop`s, so both functions see what they see with the
  pointer over empty space: no hover gauges, no ring, and a selected object's gauges
  as they are then. Selection circles and group numbers come from elsewhere in
  `Render` and `PreRender` and stay. Clicks and orders ask `MouseOn()` themselves and
  still work. Bench, `showcase`, the pointer over a cube: with hover on, a gauge above
  it and a row of system icons below; with hover off, neither.
  **Dead end:** blanking only the `PreRender` call (an earlier try) removes the ring
  but not the gauges, because `Render` reads +0x7c, which only `AlwaysSimulate`
  writes.
- **Shield hits: the four weapon calls of `ShieldEffect::CreateShieldHit`.** The flash
  on a craft's shields where a weapon strikes them is a `ShieldHit`, a model instance
  that the static, cdecl `ShieldEffect::CreateShieldHit(GameObject *, const Matrix34 &,
  ShieldType, float, int)` (0x4743b0) builds and lists in `m_shieldList`; it returns
  the hit's id, or -1 when the shield type has no model. The weapons call it after
  dealing their damage (a virtual call on the target just before): `Beam::Simulate`
  at 0x58bb95 (which keeps the id at +0xe8 to move the effect along the beam),
  `Bullet::Simulate` at 0x58cad8, `Mine::mMoveTowardsTarget` at 0x58d502 and
  `Missile::Simulate` at 0x58dd65, each with the type and duration from the weapon's
  class. With shield effects off each of those calls becomes `or eax,-1` plus two
  `nop`s, the "no effect" answer the weapons already handle: `ShieldEffect::ShieldUpdate`
  (0x4747d0) and `ShieldStop` (0x474770) find nothing under -1. Bench, `showcase`:
  with shield effects on the struck cube sits in a green shell; with them off, none,
  and the beams still end at the shield; with `heal cube1 off` its shields ran out
  within 40 s (the shields-down ring came up and the beams reached the hull), so the
  damage still lands.
  Left alone: **the shields-down ring**, the blue electric effect, is a lasting
  `ShieldHit` of type 1 (duration -1) that `Craft::ShieldsDown` (0x4c74f0),
  `ShieldCollapse`, `CheckDerelict`, `DamageAlloc`, `mInitShipsSystems` and `Init`
  create through the same function and keep at `Craft`+0x208; `Heal=1` keeps shields
  from running out, so it never comes up. The special weapons that make shield
  effects of their own (`ShieldInversion`, `EnergyShieldConverter`, `ReflectWeapon`,
  `ClairvoyantLink`, `TimedInvincible`, ...) call it from elsewhere and are untouched.
- **Heal: hull, shields and crew.** Hull through `ScriptInterfaceImp::SetCurrentHealth`
  as before; shields through `ScriptInterfaceImp::SetShieldPercent(int, float)`
  (0x455eb0), whose float is a fraction (0..1) of the maximum: it stores
  `max * f` into the craft's shields (+0x1c8, the maximum at +0x1cc, as
  `GetShieldPercent` 0x455e70 reads them); crew through `ScriptInterfaceImp::SetCrew
  (int, float)` (0x456060), which calls `Craft::SetCrew` (0x4c83e0), clamped to the
  maximum crew at +0x1c4 (the crew is at +0x1dc). `ScriptInterfaceImp::GetMaxCrew` is
  a stub that returns a global, so the maximum is read from the craft. Both are only
  written when below the maximum, because `Craft::SetCrew` also recomputes the craft's
  state. Bench, `showcase`, 30 s of fire: with only the hull healed both cubes wore
  the blue electric shields-down ring; with shields healed, none (the green flash of
  shields taking hits remains; `shieldfx off` removes it).
- **Damaged-system effects: the model's damage nodes.** The orange plume a Galaxy
  streamed from a nacelle with nothing shooting it was the engines' damage effect,
  shown because `Engines=0` had switched the engines off. A craft's model carries a
  node per system (`Shield`, `Engines`, `Target`, `Sensors` and `Life` `Damage`;
  `CraftClass::InitializeDamageNodes` 0x4bfc70 finds them), each with an emitter from
  `Sprites/emitter.spr` -- the Galaxy's `Engines` node vents `plasmalrg`, its `Life`
  node `steamsml`. `CraftInstance::Update` (0x4cb390) reads each of the craft's five
  systems (`Craft`+0x1e0, 0x30 bytes each: shields, engines, weapons, life support,
  sensors) into a byte at +0x9c..+0xa0 of the render instance, set while the system is
  online, and `CraftInstance::RenderInternal` (0x4cb0e0) turns each into its node's
  hidden bit. A system off for any reason is drawn as damaged, and
  `ScriptInterfaceImp::DisableEngines` ends in `Craft::PermanentDisableEngines`
  (0x4ca1a0), which takes the engines offline and marks them held off (byte +1 of the
  system). The plugin replaces `Update` in `CraftInstance`'s vtable (slot at
  0x6b3e64) with a wrapper that calls it and then, for the scene's own objects, sets
  the byte of every system held off -- and of all five on a healed craft -- so its
  node stays hidden. `Heal=1` also mends real damage: whenever a system that is not held
  off is offline or short of hit points (+0x18, a double, against +0x4), it calls
  `Craft::RepairAllSystemsComplete` (0x4c8be0), which restores every such system and
  leaves held-off ones alone, so `Engines=0` survives healing. The crew node (`crew1`..
  `crew5` fire sprites) follows the crew fraction, which `Heal=1` already keeps full.
  Bench, `nebula-close` (one Galaxy, `Engines=0`) and `showcase`: the plume on every
  Federation and Klingon ship before, none after, and the Galaxy still in place.
  `Vent=` and the `vent` command go the other way: the wrapper clears the byte of each
  system named, so its node shows on a healed craft, and nothing else is touched. Bench,
  `SCENE=venting`, 2026-10-13: the Galaxy's nacelle plume and its hull's gas, the
  Klingon's two, for as long as the session runs.
- **Move orders: `GameObject::SetCommand`.** The script interface's own orders end in
  `GameObject::SetCommand` overloads: `(AiCommand, const GameObject *, long, bool)`
  (0x4d1af0), `(AiCommand, const Vector3 &, long, bool)` (0x4d1b50) and
  `(AiCommand, long, bool, bool)` (0x4d1a40), all thiscall on the object. `goto` calls
  the first or second with `GO` (4) or `GO_WARP` (0x2b), and `stop` the third with
  `STOP` (3), as `ScriptInterfaceImp::Stop` (0x453590) does. The names come from
  `AiCommandToName` (0x44e1c0) and its table. A craft is flag 8 of the flags at +0x14,
  and +0x113 marks one that is dying; both are checked first, as the script interface
  does. `ScriptInterfaceImp::Goto(int, int, bool, int)` (0x453520) was tried first and
  did nothing for a Galaxy ordered to the nebula; it also checks a byte at +0x1bc of
  the ship. `GO` with the nebula as its object did nothing either, which is why only a
  craft is gone to as an object. Bench: a Galaxy ordered 590 units away arrived in
  under 15 s, turning and accelerating as a player's ship does; three ships ordered as
  a group kept their spacing; a Klingon flagship ordered to a Borg cube flew to it and
  stopped beside it.
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
- **Tech tree: `TechnologyTree::mFind(const char *)`** (0x4999e0, thiscall) on
  `g_technologyTreeArray` (0x739000, one `TechnologyTree` of 0x98 bytes per team, 9 of
  them) finds a `TechItem` by name (case-insensitive). `TechnologyTree::Available`
  (0x498d60), which `Weapon::IsAvailable` asks with the weapon class's id (+0x208),
  answers the item's +0x4 when its +0xc ("possible") is set; `Created` sets +0x4 of an
  item's children once all its parents exist, and a tree file's `-1` clears +0xc for good.
  `tech` sets both bytes, or clears +0x4.
  **Why the assimilation beam needs it.** `TechAssimBeam::Simulate` (0x586720) ends the
  beam on the tick its owner has no weapon to take from the target: a special weapon of
  the target's (+0x20c of its class) that `Weapon::IsAvailable` says its team has, that
  the owner does not carry, while the owner holds fewer extra weapons than its class
  allows (`maxExtraWeapons`, 1 on a cube). When the beam has held for
  `assimilationTime` the weapon is the owner's and the beam ends. On `a2_borg01` the tree
  (`techtree/a2_borg01.tt`) marks `fedpod1` impossible, so the Sovereign's `gcorbom` is
  never available and the beam ended on the tick it was fired (the 2026-10-16 note in
  `scenarios/specials.md`); a research station and a `fedpod1` spawned for the
  Sovereign's team could not change that. Bench, `SCENE=assimilation`, 2026-10-17:
  `tech team 2 gcorbom: was not available, now available`, then a green cone from the
  cube onto the saucer for about 6 s; a second cube spawned in its place fires it again.
- **The player's race: the two `Race::Find` calls in `SetupDefaultGame`** (0x489590).
  A map named on the command line (`-nointro a2_borg01`) is set up by
  `SetupDefaultGame`, from `Program::MainLoop`: it gives slot 0, the player's, the race
  of the map's first start location, `Race::Find(name)` (0x48b100, cdecl, an exact match
  on races.odf's `name`) at 0x4897ad or 0x489872 depending on the branch, then
  `GameSetup::setRace` with the race's index (`Race`+0x44c). When `Find` returns NULL
  the slot keeps the "random" race its constructor gave it, and
  `Program::GameOpenPreLoad` (0x4846a0) draws a random race for every playing slot
  still random (`GameSetup::RaceRandom`, retried until `CanRacePlayCurrentGameType`)
  before `DisplayInterface::PostLoadAll` loads that race's HUD into
  `Race::s_missionRace`. `a2_borg01` names no race for its start location, so the
  bench's HUD changed race from launch to launch; nothing in `ARMADA.PRF`, the command
  line or the clone decides it. With `Race=` set, both calls go to the plugin, which
  returns `Find(Race)`, logs it with what the map named (`race: federation for the
  player (the map's first start location names ""; ...)`), and falls back to the map's
  own for a name `Find` does not know. Both call sites are checked to be `call
  Race::Find` first, and `Find` against its first bytes; the hook goes in at start-up,
  outside `Enable`, so it is in place before the mission is set up. Bench, 2026-10-17,
  `showcase` with `Hud=1` and `Race=federation`: the Federation HUD on three launches
  in stock and three in refit (the log: the map names `""`); `SCENE_RACE=klingon` over
  it gave the Klingon theme.
- **A mission's captions: the two `OverlayText::AddMessage` calls in the script
  interface.** The text burned into the intro cinematics -- the Federation intro's
  subtitles at the top ("These are the voyages of the starship Enterprise..."), the
  lower thirds (`USS Enterprise / Stardate 54522.2 / Location: Alpha Quadrant`, and
  the Klingon and Borg intros' stardates) and the mission's title ("Invasion") -- is
  not a game option and not part of the scene: the mission script asks for each one
  through `ScriptInterfaceImp::ShowTitle` (0x454ff0) or `ShowSubtitle` (0x4550c0).
  Each looks the text up (`MISSION_*_SUBTITLE*` in `Dynamic_Localized_Strings.h`),
  makes one call to `OverlayText::AddMessage` (0x5061e0, thiscall, seven arguments,
  `ret 0x1c`) on `overlayText` (0x764488), at 0x455081 and 0x45510c, and returns the
  key it gives. The script later passes that key to `RemoveTitle` / `RemoveSubtitle`,
  which hand it to `FadeMessage` (0x506400) or `RemoveMessage` (0x506460); both look
  it up in the overlay's map and do nothing when it is not there. With `Captions=0`
  both calls go to the plugin's `caption_add`, which takes the same arguments, adds
  nothing and returns -1, a key `AddMessage` never gives (a counter from 0, 0x7644bc).
  No message exists, so neither stock's `OverlayText::Render` nor `UI.asi`'s overlay,
  which reads the same map (`ui/HUD.md`, "Overlay messages"), has anything to draw.
  `Scene.log` names each caption it hid (`caption hidden: "..."`). The game's own
  callers of `AddMessage` (`GameTypeGeneral::SetGameOverMessage` and the network
  transport's) are left alone. The hook goes in at start-up, outside `Enable`, as
  `Race=` does; the two call sites are checked to be `call AddMessage` first, and
  `AddMessage` against its first bytes.
- **Where the camera looks: `gTacticalCamera`** (0x763650). Its interest point, the map
  position the RTS camera looks at, is the `Vector3` at +0x98
  (`TacticalCamera::GetInterest`). **That camera is not the one in use on the bench**
  (bench, 2026-10-17): `gCameraManager`'s first field (0x763370) held
  `gParabolicCamera` (0x763470), and `query`'s `rts interest` stayed at 0,0,0 while
  `Center=` moved the view. The zoom is set through that field; see "The zoom".

## Marks

From the moment the scene is built, `Scene.asi` writes **`Scene.marks`** beside
`Scene.log`: timestamped events for the films (`films/README.md`, "Marks", has the
kinds and what `a2film` does with them). It is a file of its own so that a command's
answer in `Scene.log`, which `drive scene` reads, never has marks in it; it is deleted
when the game starts, and the bench gathers it with the logs.

    epoch 0x01dd3f6a 0x8e0c1b40 74712093
    74712093 ready objects=4
    74713511 cmd text=select warbird bop
    74713528 select n=2 names=warbird,bop
    74717262 cloak.start obj=warbird
    74719944 cloak.done obj=warbird

Every line starts with `GetTickCount()` at the moment it marks, then its kind and its
fields as `key=value`, no spaces in a value except `text`, which is last and runs to the
end of the line. An `epoch <high> <low> <tick>` line pairs a tick with
`GetSystemTimeAsFileTime()` (a FILETIME's two halves, in hex): under Wine that is the
host's clock, the one the film runner starts its action by, so a reader places every
mark on the take's clock without a handshake. It is written as the scene is built and
again before any mark more than 10 s after the last, so the two clocks cannot drift
apart over a take. The plugin never divides a 64-bit number (no CRT to do it): the
reader does the arithmetic.

**Every mark is read, never patched**: once a frame, from the camera hook
(`camera_update`, after the camera is set), `marks_poll` compares what it reads with
the frame before. What it reads, and why it is safe to read there (`s_UpdateMainCamera`
runs between ticks, on the game's thread, as the renderers' walks do):

- **The camera**: the main `ST3D_Camera`'s camera-to-world matrix (+0xc0), as `query`
  reads it. An axis that changes by more than 2e-5 or an eye by more than 0.01 units is
  motion; none for `Still` seconds settles it, marked at the last motion.
- **The selection**: `GetSelectNum` and `GetSelectList` (vtable +0x94, +0x98), as
  `selection` reads it, so `QOL.asi`'s larger selections are seen too.
- **The scene's crafts**: the cloaking device's state (+0x108, then +0x3c), as `query`
  reads it; the dying byte +0x113 that the move orders check; `Entity::Get` returning
  nothing once an object is gone.
- **Fire: `Ordnance::ordnanceList`**, the `std::list` whose head node's pointer is at
  0x771fac (each node's next at +0, its ordnance at +8), walked as render3d walks it
  (`render3d/src/mods/ordnance/eng.rs`). An ordnance's shooter is the handle at +0x5c
  and its target the one at +0x60; its ODF is named by its class (+0x34), whose cPrjID
  (+0x150) `cPrjID::GetOdfName` (0x6593a0, checked by its first bytes) turns into
  the ODF's name, as render3d names a special by it. Ordnance are counted per shooter and
  ODF, and an ordnance pointer not there the frame before is a new shot.
- **Explosions: `Explosion::explosionList`**, the `std::list` at 0x7367c4, as
  render3d's explosions module walks it. A fireball has the vtable 0x6afbd4, a
  shockwave 0x6afdac; +0x34 is the radius it was built with; its place is the
  transform `Entity::GetTransform` (0x4cfd50) returns, at +0x24.
- **Commands and specials** are marked where they are run, `glide.end` from the glide's
  own start and length, so it is exact rather than the frame that saw it.

Bench, 2026-10-17, `films/marks` (stock, 1280x720): a selection, a glide, a cloak and
decloak, a Galaxy's fire and a battleship blown up were each marked, the cloak's steps
3.0 s apart; a mark leads the first frame of the video that shows it by about 0.05 s.
**`kill` removes a craft within the frame**: +0x113 is never seen set, so `destroyed` is
marked when a craft is found gone without it (`how=removed`), and an explosion's `near`
uses where each object was last seen. The same take showed `attack galaxy big` leaving
the Galaxy firing at `small` (every burst's `target=small`).

## Where this is going

- **A `Scene:` block in scenarios**, so a scenario names its scene and the bench
  installs it, and a first lighting regression scenario.
- **Screen-space boxes in `query`** (`ST3D_Camera::ProjectPoint`), so a step can
  measure or judge a crop of one object.
- **A second map**, once a scene needs more room than `a2_borg01`'s, with its bounds
  read rather than assumed.
