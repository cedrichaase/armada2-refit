# Test bench — the game, headless, end to end

`./a2test` runs Armada II on a private, invisible display at any resolution. It drives
the game with a real (virtual) mouse and keyboard, and checks plain-text scenarios
against it. Each run leaves a report with screenshots, logs and every verdict.

    ./a2test run main-menu hud font          # scenarios in testbench/scenarios/
    ./a2test run main-menu --res 16:9,21:9   # override the resolutions
    ./a2test run admirals-log --vnc          # ... and watch it (view-only VNC)
    ./a2test watch                           # every live session, tiled in one window
    ./a2test run hud --jobs 1                # one game at a time (default: 5 at once)
    ./a2test check hud                       # how each step would run, no launch
    ./a2test list

    ./a2test session start --res 16:10       # a live session to drive by hand; prints its ID
    ./a2test session start --install .       # this checkout's ./install, from stock
    ./a2test session start --res 800x600 --mod stock --stock-shell embed   # the stock baseline, menus clickable
    ./a2test session start --assets none --install .   # as a release installs it: no textures, plates, movies
    A2_DATA=$(mktemp -d) ./a2test session start --install .   # ... as without any assets
    ./a2test session start --clock bench     # animations on the sim time: "A frame that repeats"
    ./a2test session start --mute music,voice   # the game's own music and voice volumes at 0
    ./a2test session start --stems music,voice  # music and voice each on a silent sink of their own
    ./a2test drive --session ID shot menu    #   prints the PNG path
    ./a2test drive --session ID click-text "Single Player"
    ./a2test drive --session ID step 'Wait for the single player screen'
    ./a2test session stop ID                 #   ends the game, gathers logs, writes the report
    ./a2test session list                    # the active sessions and their IDs
    ./a2test baseline compare NAME --from ID # pixel diff against a baseline ("Baselines")

### Which session a command acts on

Several jobs share the bench, and every case of an `a2test run` is a session too. So
a session is named, not guessed:

- **`session start` prints the session's ID** (`session 20261006-142501-48213`,
  date, time and the starting process) together with the exact `drive` and `stop`
  commands for it. The ID is the name of its state directory,
  `~/.cache/a2test/sessions/<ID>/`.
- **`session stop` requires the ID** (`session stop ID`, `--session ID`, or
  `A2TEST_SESSION`). There is no default, so a stop can only end a session someone
  named.
- **`drive` takes `--session ID`** anywhere on its line, or `A2TEST_SESSION`. Without
  either, it drives the one active session if there is exactly one, and refuses, listing
  them, if there are several.
- `A2TEST_SESSION` takes the ID or the `state:` path `session start` prints; agent steps
  get it set for them. An ID that has ended or does not exist is an error, never a
  fall-back to another session.

**An agent that starts a session keeps track of its ID and passes it on every
command** -- `drive --session ID ...` and `session stop ID` -- and stops only the
sessions it started itself. It never stops a session it did not start: one in `session
list` that it does not recognise belongs to another job, however stale it looks. Each
tool call is a fresh shell, so an `export A2TEST_SESSION=...` does not carry over; write
the ID into the command, or into the script that runs the steps. With two sessions of
its own (a host and a joiner, red and green), it keeps both IDs and names the right one
each time.

Reports go to `~/.local/share/a2test/results/<run>/`, with `latest` pointing at the
newest. Each holds `index.html` (look at it), `report.md` (read or paste it),
`results.json`, and per case `log.txt`, `shots/` and `logs/`.

**Cases run in parallel**, up to `--jobs N` games at once (default 5, or
`A2TEST_JOBS`). Each case already has its own clone, prefix (and so wineserver),
headless sway, Xwayland and `a2input`, so nothing is shared but the machine. A
scenario's reference case runs first, and its other cases join the queue once it is
done. Console lines are prefixed with their case. Ctrl-C or SIGTERM stops every running
game and waits for each case to tear down. Five fit this machine (12 cores, 15 GB,
one GPU): 18 cases of five scenarios at five at once, with the user's own game also
running, left at least 5.5 GB available and swap untouched, at a peak load of 17
(run 20260926-142801). Past that, fixed waits such as "Wait 45 seconds" risk turning
flaky.

Since testbench 3.31.0 `--jobs` above the slot count queues: see "Game slots" below. The
measurement above was taken before slots; two stock probes died at four games at once
under load (2026-10-17), which is what the slots are for.

**A case can run several games** (`Players: host, joiner`), each with all of the above
to itself, steps addressed by player name (`scenarios/README.md`, "Several players").
`multiplayer-two-players` hosts, joins and plays a match that way in under four
minutes; it counts as one job but runs two games.

**The user's install is never written.** Every case runs on a reflink clone of the
game directory and the prefix, so a run is free to re-tune, break or `a2mod stock` its
copy, and can run while the game is being played.

**Scenes on demand** (`scene/`): `Scene.asi` turns a directly launched, empty map into
a test scene, so renderer work need not reach a model, an angle or a weapon through
the menus. Fog, HUD, grid, cursor and notices are off, and the objects come from a
scene file (`SCENE=planet|nebula|firing`). A free camera can be moved while the game
runs: `a2test drive scene "orbit ship 120 20 250"`. `--install testbench/scene`; see
`scene/README.md`.

**Sound by category** (`--mute`, `--stems`; "Audio stems" below): a session can set
the game's music, voice, effects and interface volumes to 0 in its clone, and can play
music, voice and the interface on silent sinks of their own, so a take records each as
a separate track and the mix is made in the edit (`films/README.md`, "Sound").

## How a case runs

| Stage | What | Why this way |
|---|---|---|
| clone | `cp -a --reflink=always` of the game (4.5 GB) and the prefix (558 MB) into `~/.cache/a2test/sessions/<id>/` | btrfs: 0.7 s and no space. Must be the same filesystem as the game |
| prepare | `ARMADA.PRF` line 5 set to the resolution, then `ui/install.sh` with `A2_GAME=<clone>`: this checkout's `UI.asi` in, and what it supersedes taken out (`HUD.asi`, `GridLayout.asi`, `Menus.asi`, the file-based HUD fixes: `ui/README.md`, "Installing"). For `Mod: stock`, `a2mod stock` on the clone instead. With `--install PATH` (repeatable), `a2mod stock` and then each `PATH/install` in order, so the case shows exactly those checkouts; the log's `installs=` names each one's commit (`-dirty` if uncommitted) | `UI.asi` takes no resolution, so every case runs the same install: a pass at every aspect is what it claims |
| display | `sway` with `WLR_BACKENDS=headless`, one output at exactly the resolution, `xwayland force` | the game gets the same Xwayland path it has under Hyprland, and nothing appears on the desktop |
| input | `input/a2input`: a wlr virtual pointer and virtual keyboard on sway's seat, fed through a FIFO | see "Input" |
| game | `umu_run.py` with Heroic's environment, the clone as `WINEPREFIX`/`STEAM_COMPAT_DATA_PATH`; silent: `winepulse.drv=d;winealsa.drv=d` | see "Traps" |
| capture | `grim -o HEADLESS-1` | what the compositor actually shows, scaling included |
| teardown | Proton's `wineserver -k` on the clone, the launcher's process group, sway; logs gathered; clone deleted | also when `session start` fails part-way (an install, the display, a launch the audio guard stops): before 2.12.1 such a session kept its sway, input and VNC running and stayed in `session list` with no game |

The launch step also proves isolation: it scans `/proc/<pid>/maps` and `fd` for the
real install's path, and fails the case if the game has it open.

Logs gathered per case: `wine.log` (umu, Proton, Wine; `A2TEST_WINEDEBUG` sets
channels), `Menus.log`, `Render3D.log`, `BinkProxy.log`, `D3DTrace.log` (`d3dtrace/`), `D3D9Probe.log` (`d3d9probe/`), `Armada2_d3d9.log` and
`xalia_dxgi.log` (DXVK), `sway.log`, `input.log`, `prepare.log`,
`launch-env.txt` (the exact environment and argv), `Scene.log` and `Scene.marks`
(`scene/README.md`, "Marks"), `Stems.log` and `Stems.ini`, the clone's `ARMADA.PRF`,
`Menus.ini`, `Render3D.ini` and `dxvk.conf`, `exception.txt` if the game crashed, and
anything new in the game's `Logs/`.

## Audio stems

The game plays four categories of sound, each with its own volume in the options and in
the clone's `ARMADA.PRF`, whose third line opens with them in this order (0 silent, 20
full; the rest of the line is not volumes):

| Category | What it is (eAudioVolume) | `--mute` | `--stems` |
|---|---|---|---|
| `music` | the jukebox's tracks and a mission's scripted music (1) | yes | yes |
| `voice` | mission narration, and the speech `AudioQueue` plays (2) | yes | yes |
| `effects` | the 3D sounds of the world: weapons, explosions, engines (3) | yes | no: the rest |
| `interface` | the button bar's sounds (4) | yes | yes |

**`--mute music,voice`** sets those volumes to 0 in the clone's `ARMADA.PRF` before the
game starts; the user's install keeps its own. This is what the user did by hand on
2026-10-17 to film mission intros without their narration and music, as a bench option.

**`--stems music,voice`** records them apart. The engine plays every sound through one
DirectSound device, and Wine mixes all of a device's buffers into one output stream, so
the bench could only ever record the whole mix: no setting of the game or of PipeWire
can split it. `stems/Stems.asi` (in the clone only) opens one DirectSound device more
per category, on a null sink the session makes for it (`a2test-<id>-<category>`, beside
the session's own), and moves every 2D sound of that category onto it as it starts.
What stays on the session's sink is the `effects` stem: the 3D effects, plus any
category not named. A stem's volume is set to 20 in the clone, so each stem carries its
sound at full level; categories not named keep the user's setting. `--mute` and
`--stems` cannot name the same category, and `--stems` cannot go with `--audio`.

How `Stems.asi` finds the sound (each finding is used by its one hook, `stems.c`):

- Every `AudioSound` keeps its category as the int at +0xc, where `AudioSound::mSetVolume`
  (0x45fa60) reads it to pick the player's volume, and its file name at +4. The
  constructors are handed the category as a constant: 1 by `AudioMusic2D`, 2 by
  `AudioQueue::Play`, 3 by every `AudioSound3D`, 4 by `PopupPaletteImp`'s keyboard
  handler. `AudioSettings` writes the volumes for 1 to 4, in that order, as the third
  line of `ARMADA.PRF`; the user's change (music and voice to 0) is exactly its first
  two fields.
- A 2D sound gets its buffer from `SoundBuffer::RequestBuffer`, which loads the whole
  wave (or duplicates a cached one) and calls back at once, synchronously; the callback
  hands it to `AudioSound2D::mInit` (0x45fb90), which stores it at +0x10 and plays it.
  There is no streaming: music and narration are loaded whole too. So the hook is that
  one call (at 0x45fb7d): for a category with a device of its own, the buffer is copied
  there (format, flags, every byte), the game's copy released, and the copy passed on.
  The sound owns it and releases it when it ends, as before.
- 3D buffers are never moved: their listener lives on the game's device. That is why
  the effects are the remainder and not a stem of their own.
- Each new device's primary buffer takes the game's own format
  (`AudioDefault::s_sampleRate` and `s_bitsPerSample`, 22050 Hz and 16 bits).

The plugin finds its device among DirectSound's by the sink's name (Wine lists a sink by
its description, `Speakers (a2test-<id>-music)`); a category whose device cannot be
found or opened stays in the game's mix, and `Stems.log` (gathered with the logs) says
so. A device is opened only for a sink found by name, so nothing can reach a real
output; the audio guard accepts any `a2test-` sink. `a2film` records each sink's monitor
with its own `parec` and cuts every recording from its end, as it does the picture.

Measured on 2026-10-17 (`a2_fed01.bzn`, its intro, stock, 45 s at 1280x720): the music
stem held `Mfed300.wav`, the voice stem `a2_fed01_intro.wav`, and the effects stem
(-42.4 dB mean) matched a take of the same shot with `--mute music,voice` (-42.1 dB).

## Traps, each measured here

- **A run must be silent, and three obvious ways to do it fail.** Each was measured.
  `PULSE_SINK` at a null sink: winepulse ignores it and plays to the server's default,
  and the user heard the menu music and the Borg cutscene. Disabling `winepulse.drv` and
  `winealsa.drv`: Proton-CachyOS's **`winepipewire.drv`**, which this prefix uses, played
  the stock case's menu music. Disabling every driver (or `Audio=""`): Armada2.exe dies at
  start-up without an audio device (`virtual_setup_exception stack overflow`). What works:
  each session loads its own null sink on the real server (`a2test-<session>`, unloaded in
  teardown). In the clone's `user.reg` it is pre-registered under a GUID of ours
  (`Drivers\\<drv>\\devices\\0,<sink>`) and named as `DefaultOutput` and
  `DefaultVoiceOutput` for `winepipewire.drv` and `winepulse.drv`. mmdevapi reads those
  as `{0.0.0.00000000}.{GUID}`. A watchdog follows `pactl subscribe` for the whole case.
  It recognises the game's streams by pid (host or container, from `NSpid`), by an
  `a2test.session` marker, or by their being on the null sink. It logs where the audio
  went, and any stream of ours elsewhere is moved to the null sink, muted, and the game
  stopped. The last part is tested with a marked stream at volume 0.
  **Open: the routing is not airtight.** Besides its main stream (`Star Trek Armada 2`,
  pid -1, which has always landed on the null sink) the game sometimes opens a second
  one named `Armada2.exe`, with its real pid. In the logs of ~170 launches on
  2026-09-26 that one reached the null sink five times and the default output twice
  (run `session-20260926-173731`: `alsa_output…analog-stereo`, within the first 6 s).
  The watchdog muted and moved it and stopped the game each time, but for that moment it
  was on the user's output. Why the pre-registered default is not taken by that stream
  is not known.
- **XTest input does not work.** In headless sway the seat has no devices. `xdotool`
  moves Xwayland's core pointer, and Wine even logs the `ButtonPress`, but the frame
  stays pixel-identical: with no keyboard on the seat nothing holds focus, and no
  release arrives. A virtual pointer and keyboard (`a2input`) are real seat devices,
  and clicks work. The wlr protocol XMLs are vendored in `input/` because
  `wlr-protocols` is not installed; `a2input` is rebuilt whenever its source changes.
- **The game must go through umu, as Heroic launches it.** With Proton's `wine` run
  bare, button releases are lost ("XInput2 not supported, refusing to clip", every
  frame, from the game's `ClipCursor`), so no click completes. Through umu and the
  Steam runtime the same clicks work.
- **Which process is the game.** `umu_run.py`, the proton script and `umu.exe` all
  carry `Armada2.exe` as an argument and exit within seconds. The real game process,
  argv[0] `X:\…\Armada2.exe`, appears about 4 s after launch. Matching on "the command
  line contains Armada2.exe" latches onto a short-lived one, reports the game dead
  2–8 s in, and leaves the real game running unwatched past its teardown. Only argv[0]
  identifies it. The Wine errors in `wine.log` (`failed to update … wine.inf`) are a
  red herring, not the cause.
- **The game ignores WM_CLOSE.** It is still up 20 s after one. `Quit the game`
  therefore goes through its menus: in a mission Esc, "Exit to Windows", "Yes"; on the
  main menu the Exit panel, then "Yes". `session stop` does not bother: the clone is
  thrown away and every log is already on disk, so it terminates the game (two
  sessions in 3 s); `--graceful` quits through the menus.
- **Menu buttons are the pictures.** The labels ("SINGLE PLAYER") are painted into the
  background and take no clicks; the emblem above them does. Named targets in
  `ui.json` are the pictures, in 800x600 design space. They are mapped to the screen
  with the transform Menus.asi logs (`MoveWindow 800x600 @0,0 -> WxH @X,Y`).
- **OCR is a scene-text model, not tesseract.** Tesseract reads documents, dark on
  light; game text is every colour on every ground. It needed three preprocessing
  passes (as-is, background subtracted, adaptive threshold) and upscaling tuned per
  height, and still read the 21:9 mission list's "Werewolf Pack" (dark on a bright blue
  button) as "B er If Pack". Scored on 547 phrase instances in 86 bench frames across
  every screen the scenarios use, at 800x600 to 3440x1440 (labelled from tesseract's
  own readings, which favours it): tesseract's three passes found 93.6%, the best five
  of ten preprocessings 97.3%, **PaddleOCR's PP-OCRv3 100%**, in 0.8 s a frame (1.2 s
  max), with no upscaling and no false hits. It runs on `cv2.dnn`, so it needs no
  package the bench did not already have; `bench/ppocr.py` fetches its two ONNX models
  (Apache-2.0) into `~/.cache/a2test/models` on first use, pinned by revision and
  sha256. Tesseract is not kept as a fallback: it found nothing PP-OCR missed. (If it
  is ever run in parallel again for a comparison, set `OMP_THREAD_LIMIT=1`: it starts an
  OpenMP thread per core in every process, and sixteen at once finished 18 of 1720
  reads in ten minutes.)
- **In a mission the pointer moves by relative motion only.** The game keeps its own
  cursor there and moves it by (absolute pointer position - a reference point it fixes
  once and then tries to warp back to). Xwayland cannot warp a virtual pointer, so each
  absolute move arrives as a jump from that stale point: moving +50 across four times
  draws the cursor +200, +250, +300, +350 across and +100 down each time, and the
  cursor ends up in the corner at 0,0 — where it looks as if the game *hid* it, since
  a frame difference mistakes it for the animated resource-bar icon. `a2input rel`
  (relative motion) moves it exactly, so `Session.move` sends the difference from the
  last known position; only a session's first move is absolute. Wine's
  `MouseWarpOverride=disable` changes nothing, so the warp is not DirectInput's.
- **OCR boxes cannot measure shape.** The same word's box can come back 28 px tall in
  one pass and 36 in another: a phantom 30% stretch. Text is only *located* by OCR in
  the reference shot. Its shape is measured by the template match, like any patch.
- **Dark art is not a pillarbox.** The main menu's plate fades to near-black space at
  21:9, and a "mostly dark column" test called that 267 px of black bar. A bar is a
  column with *no* pixel above 4/255.
- **A campaign mission's opening cinematic skips with Space**, not Esc, straight to the
  briefing, where Space does nothing: `Wait for the briefing` presses Space at every
  look (`ui.json`'s `skip`), so the briefing is up seconds after the load instead of
  ~70 s later. For anything that only needs *a map*, launch
  straight into one (`Launch: -nointro a2_borg01`): a bare argument is a mission name
  (see `menus/run-wine.sh`), and the HUD is up in ~30 s.
- **A direct map launch deals a random faction.** One run drew the Federation,
  Cardassian and Romulan HUDs across its cases, and their panels have different shapes
  (the Federation's minimap panel is about half as wide as the Romulan's). A comparison
  across resolutions then measures different sprites. Anything compared across
  resolutions goes in through the Borg campaign
  (`Include "_enter-borg-mission"`), which is always Borg. The Admiral's Log and the
  in-mission menu are shell UI and faction-independent, so they keep the fast launch.
- **Fog of war is flat grey.** A map opens scrolled to its top-left corner, so its
  unexplored area fills part of the 3D view with flat grey, and so does the minimap.
  The flat-area check will flag it. Don't use that check on a map.
- **`a2test drive` used to drive the newest active session on the machine**, and every
  case of an `a2test run` counts as one. With another job running scenarios, a `drive`
  after your own `session start` went to *that* job's game (2026-09-27: an Escape and two
  clicks landed in a `hud-mode-switch` case), and a bare `session stop` terminated a
  parallel job's session (2026-10-05). That is why `session stop` now requires an ID and
  `drive` refuses to choose among several ("Which session a command acts on").
- **The briefing's OK button did not take a click** in one session, though Esc did.
  Not investigated.
- **A clone starts with the player's own shell settings.** The game keeps the
  multiplayer name, and the rest of what the shell remembers, in `save/shell.set` in the
  game directory, in plain text at offset 0x10f. The clone copies it like everything
  else. So the Enter Name field opens with whatever the player last typed ("asdfasdfasdf"
  on this install, saved 2026-09-27), not with stock's "Player". In the 2026-10-02 sweep
  this looked like a name leaking from one session into a later clone, but it was never
  the bench's. A scenario must not expect the default name; `multiplayer-name.md` clears
  the field before it types.

## A frame that repeats

Two sessions of one scene, stopped at one moment, can draw the same picture pixel for
pixel, so that a port of the renderer can be checked against a baseline by a diff. It
takes three things, each off by default:

- **`--clock bench`** (`a2test session start`, `a2test run`; a scenario's `Clock: bench`;
  `A2TEST_CLOCK=bench`): `[Bench] Clock=bench` in the clone's `Render3D.ini` and
  `UI.ini`. Every animation of the plugins then runs on the simulation's time, which
  stands while the game is paused, cosmetic seeds are fixed, and what adds frames up
  (the nebulae's `Temporal=`) settles on one picture (`render3d/README.md` and
  `ui/README.md`, "The animation clock").
- **`Seed=`, `Step=` and `Pause=`** in the scene (`scene/README.md`, "A frame that
  repeats"): the engine's random sequences from a seed, every tick the same length, and
  the pause on a tick counted from the scene's build. `Setup:
  testbench/scene/bench-setup.sh showcase Seed=7 Step=16 Pause=200 Race=federation`.
- **Nothing by hand before the pause.** A command from `drive scene` lands on whatever
  tick is running; the camera and every order belong in the scene file.

`bench-determinism` is the regression scenario: a shot of the paused frame, another 3 s
later, then a second launch of the same clone and a third, compared with `Expect shots
"A" and "B" to match`.

Measured at 1280x720, two launches each, the shot 4 s after the pause, mean absolute
difference over every channel (0..255), the largest difference, and the share of pixels
that differ (2026-10-22):

| Scene | Before: wall clock, `scene pause` 15 s after the build | Seed, Pause, wall-length ticks | + `Step=16` | + per-object seeds from handles, the engine's sprite animation on the sim time: today |
|---|---|---|---|---|
| `showcase` | 3.516, 254, 15.8% | 0.557, 255, 9.5% | 0.159, 211, 2.9% | **0, 0, 0%** |
| `nebula` | 0.220, 253, 16.7% | | 0, 0, 0% | **0, 0, 0%** (`Step=16` run) |
| `firing` | 0.511, 254, 5.3% | | 0, 0, 0% | **0, 0, 0%** (`Step=16` run) |

What each step took out: before, the battle was simply at another moment, and the
stars of the sky stood elsewhere on each launch (the engine lays them out from a
sequence seeded by the wall clock as the mission loads). With a seed and a fixed tick
count the stars matched but torpedoes did not: the game's tick is as long as the frame
that ran it, so tick 200 was 4.3550 s of simulation on one launch and 4.3484 s on the
next. With `Step=16` both were 3.7 s exactly (`0x406c8b57`), and what was left was the
torpedoes' starbursts and the phasers' texture, animated by the engine's sprite clock
and seeded from heap addresses. Within one launch, before: `nebula`'s paused frame still
moved (0.145, 4, 29.6% of pixels, 3 s apart: the gas's frames adding up); with the bench
clock nothing moves. The clock's own share, `showcase` with everything of today but
`--clock wall`: 0.122, 200, 1.9% between launches (the shots' sprite animation and
address-seeded flicker), which is why `bench-determinism` allows a mean of 0.01 at most
between launches and 0 within one; it passed at 0 (run 20261022-225239, 123 s).

## Operating the game: what agents learned

Lessons from agents driving the game on the bench, kept so the next one need not
re-learn them. Recipes for staging particular things are in `operating/`, one file
per topic: `construction.md` (a station built by a construction ship, sparks),
`special-weapons-and-mining.md` (mines, model-drawn special weapons, mining beams),
`special-object-effects.md` (Computer Override, Energy Dissipator, Psionic
Disruption), `stations-ghosts-holograms-evolver.md` (the build ghost and height
posts, a hologram, the Evolver, a survey of what no module draws).

- **The bench's GPU is the user's.** A GPU hang in one bench game (2026-10-15) reset
  the GPU, killed every bench game and crashed the user's desktop compositor. Run one
  game at a time (`a2test run -j 1`; a scenario with a judged step also runs its 4:3
  reference case), stop each session as soon as you have what you need, run at 720p
  (`--res 1280x720`) unless fine detail is judged, and keep every new shader pass
  fixed-cost. A session whose game died shows "game not running" in `session list`;
  `session stop` still cleans it up (exit code 2 then).
- **Starting takes two to three minutes** (clone, both installs, the first tick), so
  `session start` can outlast a 120 s tool timeout: start it in the background and
  wait for `stop it:` in its output. The scene is built 30-60 s after that: wait for
  `scene ready` in the clone's `Scene.log`, not a fixed time.
- **Without a scene the 3D view is grey**, in stock too: a bare `-nointro a2_borg01`
  shows the HUD over nothing. Always stage with `SCENE=` (`scene/README.md`).
- **`Setup:` runs from the checkout `a2test` runs from**, not from `--install`: a scene
  or scene command that exists only in a copy is not found. Use an existing scene and
  `Scene "spawn ..."` steps, or a temporary copy of the scenario with an absolute
  `Setup:` path. `a2test run` takes a scenario by path.
- **`--install` runs that tree's `./install`, which builds into it**: for a stock
  comparison install a reflink copy of a pristine tree, never the tree itself.
- **A stock reference in the same setup** (`--install` cannot go with `--mod stock`):
  `session start --no-launch`, set `Enable=0` under `[Render3D]` in the clone's
  `Render3D.ini`, then `drive launch -nointro a2_borg01`.
- **A/B at the same moment**: a module key with `Reload=1` is read again while the game
  runs (read again every ~2 s); `scene pause`, shoot,
  switch the module off, wait for the log to say so, shoot again. Rendering continues
  while paused. Cloud particles drift while paused in stock, so that side moves.
- **Scenario pacing**: `spawn X; select X` on one line selects nothing (spawn, wait a
  second, then select); a `special` fired in the second its shooter spawned answers
  "fired" but does nothing (wait 2 s); button-bar clicks need a second between them.
  `answers "fired"` also matches "NOT fired": check `", fired"`.
- **Escape during placement opens the OPTIONS menu**, not a cancel, and `Scene.asi`
  stops reading commands until "Return to Game" is clicked.
- **Agent sandboxes** may refuse shell lines that mix variables with `sed`, loops of
  `drive` commands or heredocs: put the sequence in a small script and run that.

## The stretch measurement

Every aspect fix in this project keeps the vertical axis as stock draws it and corrects
the horizontal. Between the 4:3 reference and any other resolution, whatever is drawn
right has therefore scaled by exactly `H / H_ref` on both axes. `vision.measure_stretch`
fixes the vertical factor there and template-matches the reference patch across a
range of horizontal factors. It reports the best one: 1.00 is undistorted, and stock's
HUD at 21:9 should read ~1.79. It recovered a synthetic 1.344x stretch as 1.34, at a
correlation of 0.995. Text uses the same measurement, on the patch around the phrase
that OCR locates in the reference. `stock-control.md` is the control that shows the
measurement can fail.

**What the reference is.** For the HUD, cursors and font the baseline is **stock at
800x600**: the shape and resolution the original game was designed for. Refit 4:3
is not a baseline, because the mod changes things at 4:3 too (the font is condensed at
every resolution), and a comparison against it would pass anything the mod gets
consistently wrong. At 800x600 the front end's own 800x600 mode fills the screen, so
clicks map 1:1 and the game quits through its menus. At 1600x1200, stock did neither.
`hud.md` says `Reference: 800x600 stock` and `Stock shell: embed`.

## Baselines

A port of the renderer (render3d and ui onto a shared `gpu` crate, then Vulkan and
Direct3D 12) is right when it draws the same pixels. That is checked against
**baselines**: named sets of frames, kept with their provenance, and a pixel diff
against them (`bench/baseline.py`).

    ./a2test baseline record pre-gpu --from SESSION_ID --shot orbit --shot wide
    ./a2test baseline compare pre-gpu --from ~/.local/share/a2test/results/<run>/<scenario>/1280x720
    ./a2test baseline diff a.pfm b.pfm --gif        # any two frames, dirs, sessions or baselines
    ./a2test baseline list
    ./a2test baseline show pre-gpu

**Where they live.** `$A2_DATA/baselines/<NAME>/` (`A2_BASELINES` overrides): one file
per shot, copied as it was (PNG, PPM or PFM), and `meta.json`. Each shot there carries
its own provenance: when it was recorded, the bench's commit, the `installs=` commits
of the session or case it came from, its resolution, mod state, scene and session. A
frame is game-derived, so neither a baseline nor a comparison's output may go into a
checkout (hard rule 8): the tool refuses a path inside one. Comparisons write to
`~/.cache/a2test/baselines/<time>-<name>/` (or `--out`): a heat map and a side by side
per shot, `compare.json` and an `index.html`.

**Where frames come from** (`--from`, and both sides of `diff`): an image file, named
by its stem; a case directory of a run or a session, whose screenshots are named as the
step log names them (the last of each name) and whose `.ppm`/`.pfm` dumps anywhere
under it are named by stem; a scenario or run directory holding one case; any other
directory of images; a session ID, live or ended; or a baseline's name. `record` adds
to a baseline and refuses to replace a shot it holds unless given `--force`.

**What the numbers mean.** Every frame is compared as RGB; an alpha channel is dropped,
and the report says so.

| Number | What |
|---|---|
| `rmse` | root mean square of every channel's difference, in 1/255 |
| `max` | the largest single channel difference, in 1/255 |
| `over` | pixels where any channel differs by more than `--threshold` (default 2/255), as a count and a share of the frame. Strictly more: a difference of exactly the threshold is not over it |
| `psnr` | 20 log10(255 / rmse), in dB; `inf` when identical |

A PFM (float, the HDR scene target and the bloom levels) is compared in linear values:
`linear rmse` and `linear max` as stored. The other four are then measured after a fixed
tone map (Reinhard, x/(1+x) per channel, times 255), so the same `--threshold` and
tolerances read the same way for a float frame as for an 8-bit one. Frames of different
sizes, or an 8-bit frame against a float one, are not compared; that is a failure with
its reason, never a number. PFM rows are stored bottom to top and the sign of its scale
is the byte order (negative: little-endian); the reader undoes both, and the self-test
checks it against hand-made bytes and against OpenCV.

**The heat map** shows each pixel's largest channel difference (linear for a PFM) on a
square-root scale up to that frame's own maximum, so that one changed pixel still
shows. It is drawn over a dimmed grey copy of the baseline, and identical pixels stay
grey. The legend under it gives the scale. `--gif` adds a two-frame flicker GIF
(ImageMagick): a one-pixel shift that a heat map shows as an outline is obvious there.

**Pass or fail.** `--rmse X` and `--max-over N` (a count, or `P%`) are the tolerances.
With neither, no pixel may be over the threshold. `compare` and `diff` exit 0 when
every shot is within tolerance, 1 when any is not (or a shot is missing, or the sizes
differ), and 2 on a usage error such as a missing baseline.

**In a scenario** (`scenarios/README.md`, "Baselines"):

    1. Take a screenshot called "orbit".
    2. Expect the screenshot "orbit" matches baseline "pre-gpu" within RMSE 0.5.

A missing baseline fails the step and says how to record it. The report shows the
numbers and links the heat map and the side by side, kept in the case's `baseline/`.

**Why not the judge.** A judged step asks a model whether a picture looks right. A port
is right only when it draws the same pixels, and a model cannot see a half-texel shift,
a channel swapped in one material, or bloom 3% too strong. It passes all three, and
each costs money and time. The diff is exact, free and the same on every run. It
also gives a number to track: before/after a refactor, and backend against backend.

**The noise floor.** Measured on `showcase` at 1280x720, paused with `scene pause`
(session 20261022-214129-23171): shots taken 2 s and 5 s apart in one pause are
identical, rmse 0 and no pixel over 2/255. One second of simulation between two shots
(`scene resume`, then `pause`) gives rmse 15.4/255, max 254/255, 10.9% of pixels over
2/255. The beams, shields, sparks and the planet's clouds have moved, and the sky has
not. So a paused frame is stable within a launch. What is not yet measured is two
launches of the same scene, or two builds: the cosmetic simulation, its particles and
the march's thresholds run on the sim clock, so a baseline compares only frames taken
at the same tick, with the same camera. Until the render is deterministic across
launches, a tolerance has to sit above that floor. Once it is, the default (nothing
over 2/255) is the target.

`python3 testbench/bench/baseline_selftest.py` checks every number, the readers, the
exit codes and the steps on synthetic frames, with no game.

## Judged and agent steps

Steps that name no measurable check go to `claude -p` (`bench/judge.py`). A judged
assertion reads the screenshot and the same-named reference shot, and returns pass,
fail or inconclusive, with its reasoning and cost in the report. An agent step drives
the live session through `a2test drive` until it reports done or failed; every command
it runs is logged like any other. `--no-claude` turns judged steps into REVIEW, left
for a person, and makes agent steps fail. The model is **Sonnet** (`--model sonnet`) for judged and agent steps alike. `A2TEST_MODEL` picks another, and `A2TEST_MODEL=` (empty) leaves it to the `claude` CLI's default. The judge
prompt carries a short brief of what is *supposed* to look odd (pillarbox areas, fog of
war, the 21:9-tuned font). Add to it when it flags something known.

## Watching

**`./a2test watch`** shows every live session at once, view-only, in one window, whoever
started it and whether or not it was started with `--vnc`. For each session whose sway
is up, it starts its own `wayvnc --disable-input --render-cursor` on a loopback port
(so the virtual pointer is visible), and it serves a page on `127.0.0.1:5950` that tiles
them with noVNC, choosing the column count that makes the tiles largest. It opens that
page as a Chromium `--app` window with its own profile. Sessions that start later appear
within two seconds, and ended ones drop out. Ctrl-C, or closing the window, stops the
`wayvnc`s it started and nothing else: no session is touched, and input is disabled at
the server. `--no-open` only serves the page, `--port` moves it (`A2TEST_WATCH_PORT`),
and `?debug` on the URL turns on noVNC's console logging. Runtime files (control
sockets, logs, the browser profile) are in `~/.cache/a2test/watch/`.

Two things it works around, both measured:

- **`wayvnc --websocket` segfaults on the first client** (wayvnc 0.10.1 with neatvnc
  1.0.1: a call through a null pointer in `libneatvnc`). The watcher therefore bridges
  each tile's WebSocket (`/ws/<session id>`) to the plain RFB port itself
  (`bench/watch.py`, stdlib only). It answers as HTTP/1.1, because Chromium refuses an
  upgrade answered as 1.0.
- **noVNC's npm package is CommonJS**, and jsdelivr's `/+esm` fails to bundle it. The
  page imports noVNC's own ES-module source (`core/rfb.js`) from jsdelivr's GitHub mirror,
  pinned to v1.6.0. The page needs the network to load it.

`--vnc` (per case) or `session start --vnc` starts a view-only `wayvnc` on the headless
output: `vncviewer localhost:5910` (the port is printed; it counts up if busy). `--record`
writes each case to `run.mp4` with `wf-recorder`. All are optional and need `wayvnc`
(and `wf-recorder`, or a viewer such as `tigervnc`). Without them the bench warns and
carries on, and `watch` refuses.

## Game slots

Several agents run the game on the bench at once, from separate worktrees and processes
(`a2test run --jobs`, `a2film probe --jobs`, interactive sessions), and the machine
carries only so many games: four at once killed two stock probes. So every place that
starts a game takes a **slot** first, from one queue shared by the whole machine
(`bench/slots.py`). When all are taken it waits, and prints who holds them.

- **How.** `N` files `slot-0` ... under `$A2_SLOTS_DIR` (default `$XDG_RUNTIME_DIR/a2-slots`,
  else `$A2_DATA/locks`). A slot is taken while some process holds `flock` on its file. The
  kernel drops the lock when the last holder exits, a crash or `kill -9` included, so a
  slot never stays taken by a dead process. `slot-K.json` beside it says who has it (pid,
  worktree, label, since); it is information only, and the lock is the truth.
- **What holds a slot.** A bench session, from before its clone to its teardown: the file
  descriptor is inherited by the session's sway and game launcher, so the slot outlives
  the `a2test session start` that took it and is freed by `session stop`, or by the
  session's processes dying. `a2test run` takes one per case, `a2film` one per take.
- **Several games at once.** A case with `Players:` and an a2film shot with `Players`
  take all their slots in one request: all or none, and only the oldest waiter tries.
  Two such shots therefore cannot each hold one and wait for the other, and a shot that
  needs two slots is not overtaken for ever by single games. A request for more slots
  than exist is refused (raise `A2_SLOTS`). The cost is that a small request waits behind
  a large one in front of it.
- **How many.** `A2_SLOTS` in the environment, else `A2_SLOTS=3` in
  `~/.config/armada2-refit.conf` (the file `a2env.sh` reads), else a default from the
  machine: one game per 4 cores and per 5 GiB of memory, from 2 to 3 (3 on 12 cores and
  15 GiB). The GPU is not measured. `--jobs` above the count just queues. Every process
  should see the same count; one that is set lower or higher takes slots by the number it
  believes in.
- **Where it looks.** `A2_SLOTS_DIR` moves the files; `A2_SLOTS_POLL` (default 1 s) is how
  often a waiter looks.
- **`./a2test slots`** (and `./a2film slots`) lists the slots, their holders and the
  queue. A waiter prints the same holders when it starts waiting, and every 30 s.
- **Old code.** A checkout from before 3.31.0 does not know the slots and neither takes
  one nor is stopped by them; only processes running this code queue.

`python3 testbench/bench/slots_selftest.py` checks all of it with dummy processes
and no game.

## Requirements

`sway`, `grim`, `python-opencv` (numpy with it; its `dnn` module runs the OCR models),
`wayland-scanner`
and `cc` (for `a2input`), `pactl`, Heroic's Proton-CachyOS and umu, and `claude` for
judged and agent steps. Optional: `wayvnc` (for `watch` and `--vnc`, with Chromium for
`watch`), `wf-recorder`, `tigervnc`. The texture
pipeline's "no numpy" rule is about what that pipeline may assume. The bench is
separate and needs opencv.

## Further improvements

Not built yet. Each is recorded with what is known so far.

**Sonnet, checked against Opus.** The judge runs on Sonnet; Opus, the CLI's default
here, costs $0.11–0.21 per judged step. Haiku was considered, but the judgments that
matter are fine ones: a panel a few percent off, or a small copy of the log in a
corner. A false pass there costs more than the tokens saved. Still open:

- Re-judge stored shots from past Opus runs with Sonnet and compare the verdicts,
  including the known failures (stretched HUD at 16:9 and wider, pillarbox bars). A way
  to re-judge without launching the game may need adding.
- Agent steps (many turns of reading the screen and clicking, where a slip costs a
  two-minute launch) may want the stronger model back. That needs a second variable,
  e.g. `A2TEST_AGENT_MODEL`, since `A2TEST_MODEL` sets both today.
- Unmeasured: how much of each call's cost is the `claude` CLI's own start-up context
  rather than the judgment. If it is most of it, the switch saves less than list prices
  suggest.
