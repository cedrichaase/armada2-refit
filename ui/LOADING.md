# UI — the loading screen of ours

`[Loading] Enable=1` (default 0) puts a loading screen of ours in place of the engine's
logo scene and bar (`PLAN.md`, step 5): the map's sky, the mission's title, briefing and
objectives, the players, and the load's progress, animated between the engine's own
frames. The look is decided in `STYLE.md` ("Screens"); the hooks it draws from are in
`README.md`, "The loading screen" (the overlay hook) and below. Code:
`src/engine/loading.rs` (the hooks), `src/engine/setup.rs` (what is being loaded),
`src/loading.rs` (the screen).

## Over the engine's frame, and between them

The screen is drawn edge to edge and opaque, so whatever the engine drew under it
never shows: the skybox (`mbg02`), the logo model and the stock bar. **A player without
the widened `logo.SOD` (the models layer's `LOADING` target, `models/README.md`) gets
the same screen**, since nothing of the logo is seen. The engine still loads and draws
the logo scene at each 5% (its frames are the only ones it presents, and they come from
the same function that pumps the message queue and reports the load's progress to the
other players, so it is called, not replaced). With `[Loading] Enable=1` the `LOADING`
target and `logo-sod.py` therefore change nothing on screen; they still matter to a
player with the UI's loading screen off, and to the moment before the first frame of
ours (below).

**Frames of ours.** `LoadBarNext` calls `RenderLoopAssetLoadStatusUpdate` once per
loading step, with the step's text, and the engine draws only when the fraction has
risen another 5%. The hook takes `LoadBarNext`'s call of it (`0x6019eb`, a `call rel32`,
cdecl with the text as its one argument, checked to call stock) and makes the engine's
call first. When the engine drew no frame in it, and the last frame (the engine's or
ours) is a frame period old (30 a second), it presents one of ours: a scene of its own
on the device the engine's last loading frame used, cleared, `surface::frame(Context::
Loading)` drawn, `EndScene` and `Present` through the device (`engine/dev8.rs`, the
calls `run_screen` makes), the viewport put back as it was. Nothing is drawn before the
engine's first frame of the load, which is the one that puts the 3D display up. A step
is short except for a few long ones, so the frames come as often as the steps do:
measured below.

**When a load starts.** `Program::GameDoOne` puts the splash up through one call of
`RenderLoop_Show_Splash_Screen` (`0x4871ad`); the hook makes that call and then has the
screen read what it will show (`loading::begin`). It is the one place a mission's load
begins, and what it loads is the map `GameSetup::getMapName` names (its caller passes
exactly that).

**`[Loading] Hold=N`** is for the test bench only: at the load's half-way point the
status hook keeps presenting frames of ours for N seconds, draining the queue through
the engine's own `HandleMessageQueue` (`0x488730`) as its status update does, so a
screenshot can catch the screen. Never set in play (a network game's other players wait
for it).

## What is being loaded

`src/engine/setup.rs` reads the game setup the shell filled in, through the engine's own
getters on `g_transport`'s (`0x76b8d4`) `GameSetup` (`Transport::getGameSetup`,
`0x557940`), each checked by its first bytes:

| Getter | What the screen takes from it |
|---|---|
| `GameSetup::getMapName` (`0x5461f0`) | the map's file name (`a2_fed01`, `mp02eye`): the `.bzn`, the objectives and the preview are found by it |
| `GameSetup::getMapTitle` (`0x546240`) | the title, when the mission has no objectives text |
| `getLocalSlot` (`0x547470`), `getRace` (`0x546650`), `getTeam` (`0x546610`), `isVacant` (`0x546af0`), `isGamePlayer` (`0x546b90`), `getPlayerName` (`0x5466e0`) | the players panel; `getPlayerName` answers a `std::string` through a pointer it is given, made and freed with the game's own runtime (`engine/msvc.rs`, `with_string`) |
| `GameSetup::RaceRandom` (`0x547cd0`), `Race::FindInstantActionSlot` (`0x48b180`) | a slot's race index to a race (its name at `+0x14`); the "random" index stands for no race yet |
| `Transport` virtuals `IsInstantAction` (`+0x10`), `IsSinglePlayer` (`+0x18`) | the kicker (Instant Action, a campaign, Multiplayer) |
| `ShowLoadPercentages` (`+0x40`), `GetLoadPercentage(slot)` (`+0x44`), `IsSendWorthy(slot)` (`+0x5c`) | each network player's progress, as the stock `LoadPercentageBox` asks for it (HUD.md); the transport keeps the fraction each player last reported |

**The player's race** is decided as `DisplayInterface::PostLoadAll` decides the HUD's
(HUD.md, "The player's race"): in a game on this machine, the race a campaign mission's
details set (`Race::s_missionRace`, which `Map_Dialog::UpdateGameFromMapDetails` sets
when the mission is chosen) if there is one; otherwise the local slot's race. Instant
Action's "random" is dealt in `Program::GameOpenPreLoad`, after the splash is up, so the
race is asked again each frame until it is known; the screen is in the shell's theme
until then, and in the race's from that frame on (`[UI] Theme=` other than `auto` keeps
its theme).

`ShowLoadPercentages` on a network transport also remembers, once every player has
finished, that it has nothing more to show. The screen of ours asking it is what the
stock box would have done at the same moment.

**From the game's files** (every one is the player's own install, stock or refit, so
the screen needs no asset of this project):

- **The sky**: the background the map's `.bzn` names (the text after the first
  `02 00 00 00 64 00 00 00`, `textures/README.md`). A cube model (`mbgborg.sod`) has its
  faces at `Textures/RGB/<stem>.1.tga` .. `.6.tga`, or, when there are none of those,
  under a name the SOD itself carries (`MbgBlue.SOD` draws `MBG02`); a 2x2 atlas
  (`MBG02.tga`) is four faces; a prefix (`mbgkl`) has `<prefix>0.tga` .. `5.tga`. The
  face with the highest mean brightness on a 32x32 grid is the one with the most nebula
  in it, and is shown; reading all six costs measured below. A refit install's faces
  are the upscaled ones.
- **The briefing**: `objectives/<map>_a.txt` (or `_main`, `_primary`,
  `_main_objectives`, the names the first objectives of a campaign mission have): its
  first line is the title, then the briefing's paragraphs, the objectives (lines that
  start with `*`) and a hint. A mission can have several such files, which its script
  shows in turn; the screen shows the opening one. The tutorials' texts are steps, not
  briefings, and multiplayer maps have none.
- **The preview**: `bzn/<map>.bmp`, the 160-pixel minimap picture of a multiplayer map.

**What the load is doing**: the step's text is the engine's (`Constructing 'fgalaxy'`,
`Tech Tree Computed`, a sprite's name), so the screen says it in the player's words by
its first word or two (`src/loading.rs`, `doing`).

## Seen on the bench

Not yet: the screen builds, and `testbench/scenarios/ui-loading.md` is written, but it has
not been run (no bench slot came free before the job stopped). The measured facts this
file refers to (how often frames of ours come, what reading the sky costs) are still to
be taken from `UI.log`, which logs both.
