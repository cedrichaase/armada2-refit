# Films — storyboards shot on the test bench

A film is written as a **storyboard**, a Markdown file (or several, see "A board over
several files") that is both the script a person reads and the recipe `./a2film` shoots from: what each shot shows, how the camera moves,
and how the shot is used in the cut (layout, transition, titles, music). The footage is
filmed on the test bench (`testbench/README.md`), several games at once, and the cut is
made from the storyboard's edit direction (today a rough cut by `a2film animatic`; later
a real editor, fed by `a2film manifest`).

    ./a2film check  films/whats-new            # parse it, list the shots, timeline length
    ./a2film probe  films/whats-new            # every shot, 1280x720, 4 games at once
    ./a2film probe  films/whats-new g03 g04    # ... just these
    ./a2film sheet  films/whats-new            # one contact sheet: every shot, in order
    ./a2film animatic films/whats-new          # a rough cut from the probe takes, kept as rc<N>
    ./a2film animatic films/whats-new --section "Act 1"      # just a part, quickly
    ./a2film animatic films/whats-new --from a01 --to b05    # ... or a range of shots
    ./a2film review films/whats-new            # a first pass over the takes: sheets, flags, motion
    ./a2film review films/whats-new g03 g04    # ... just these
    ./a2film take   films/whats-new            # the final takes, 4K, one game at a time
    ./a2film manifest films/whats-new          # the edit as JSON, for an editor
    ./a2film status films/whats-new            # which takes are current, and why not
    ./a2film marks  films/whats-new e01        # what happened in a take, and when
    ./a2film probe  films/whats-new --ignore-code   # re-film changed specs only, keep the rest
    ./a2film adopt  films/beams-scout a01 films/promo   # take over a scratch board's takes of a01

Like the texture targets, a storyboard is a **recipe** and is committed. Its footage is
an asset and lives in `$A2_DATA/films/<film>/`, never in a checkout (hard rule 8):

    $A2_DATA/films/<film>/
      probe/<shot>/<variant>.mkv   .wav  .events.json  .marks.json  .spec.json  .sheet.png
                   <variant>.effects.wav  .music.wav  .voice.wav   with Stems: (below)
      final/<shot>/<variant>.mkv   ...
      probe/<shot>/<variant>.review.png   `review`: a contact sheet and curves per take
      probe/<shot>/variants.review.png    `review`: how two variants differ over time
      review-probe.html  .json      `review`: the overview of the board
      sheet-probe.png               every shot in timeline order
      animatic-probe-rc<N>.mp4      every rough cut, numbered; .shots.json beside each
      animatic-probe.mp4            the latest of them (a copy), with its .shots.json
      animatic-probe-part-<label>.mp4   a partial cut, not numbered
      cuts.json                     the log of every cut
      manifest.json

Scouted moments worth cutting in as they are (a mission's intro cinematic, a scratch
board's take) are catalogued in `films/reference/` (its `README.md`): an id, where the
footage is, its in and out, its sound and its quality. A footage shot uses one with
`From: ref:<id>`.

## The process

1. **Write the board.** Sections, then shots: what each shows and why, camera, layout,
   titles. Prose is welcome anywhere: what the tool does not read is direction for the
   person (or agent) who cuts the film.
2. **Probe.** `a2film probe` films every shot without a current take at 1280x720, several games in
   parallel (`--jobs`, default 4; each game waits for a machine-wide game slot, so above
   the slot count they queue: `./a2film slots`, testbench README "Game slots"), and makes a contact sheet per take. Probes are cheap
   and exist to get the *direction* right: framing, angle, timing, what is in the shot.
   A shot with a `Tier:` of its own is filmed at that tier in the same run (see "A shot's
   tier").
3. **Review.** `a2film review` makes a first pass over the takes (below); `a2film sheet`
   and `a2film animatic` show the whole film at once. Notes go
   back into the board, and each shot's `Status:` moves on.
4. **Repeat** 2–3. A take is re-filmed when it is no longer current: its spec changed
   (its setup, action, scene file or variant), or the code it shows did (below). So
   iterating on one shot costs one shot, and after the game's look changes the takes
   that show it are filmed again.
5. **Final takes.** `a2film take` films the shots at 3840x2160, one game at a time by
   default (`--jobs`), because a recording game under load draws fewer frames and the
   take shows it.
6. **Cut.** `a2film manifest` writes the edit — order, in and out points, layouts,
   transitions, titles, key hints, music — with the path and length of every take.

Statuses: `draft` (written, not filmed) → `probed` → `approved` (direction signed off;
the probe is the reference) → `final` (the 4K take is accepted). `a2film` never sets a
status: that is the reviewer's call, written into the board.

A take can also be **suspect**: current by its spec and code, but its marks do not meet
its shot's `Expect:` (a recording that was to select twenty ships and selected none).
`status` says which expectation failed, `probe` and `take` film it again, and the
manifest counts it as not current. `Expect:` is not part of the spec, so changing it
re-judges the takes there are without filming them.

### When a take is current

A take is current when it still shows what filming it again would show. Two things
decide that, and `status`, `probe`, `take` and `manifest` say which one failed:

- **Its spec**: everything in the board that decides what the take looks like (film,
  shot, variant, scene file, setup and action, the variant's installs, resolution). The
  edit fields (`Layout`, `Title`, `In` ...) are not in it, and nor is the file or the
  section the shot is written in: a shot moved to another file or section of its film
  keeps its takes. Stale: `spec changed`.
- **The code it shows.** The game's look changes from week to week (render3d's nebula
  rework landed a day after the 2026-10-15 probes), so a take is as old as the checkout
  it was installed from. Every take records, as its session starts, the commit
  (`commit`), whether the tree had uncommitted changes (`dirty`), which paths it depends
  on (`scope`) and a hash of the uncommitted changes to them (`code`), in its
  `.spec.json`. It stays current while the diff from that commit to today's working tree,
  over those paths, hashes the same: nothing changed since, or the same uncommitted
  change was committed as it was. Stale: `code changed since 139e18a (render3d, menus)`,
  or `uncommitted code changes since ...`.

  The paths a take depends on are the whole repository, less what cannot change a take:
  `films/` (the boards and this tool), every `*.md`, `.gitignore` and `LICENSE`,
  `.claude/`, `.github/`, `publish/`, `testbench/scenarios/` and `render3d/perf/`. A
  variant whose installs are all folders of this repository other than its root (the
  stock variant, `--install testbench/scene`) depends only on them and on the bench
  (`testbench/`, `a2test`, `a2mod`, `a2env.*`), so a render3d change keeps its stock
  takes. A desktop shot, or one with `Prepare:`, depends on the whole repository.

  A take filmed before films 1.1.0 recorded only `git describe --dirty`: a clean one is
  judged as above; one from a dirty tree is `unknown` (which changes it had was not
  recorded), as is a take whose commit is not in this repository, and either is
  re-filmed.

**To keep takes the code has moved past** (a look change that does not touch them, a
deadline), pass `--ignore-code` to `probe` or `take`: only a changed spec re-films. It
works on `status` and `manifest` too, which then count such takes as current. `--force`
is the other way: re-film even what is current. `python3 films/film/selftest.py` pins
these rules on a throw-away repository, without a game, with the board's own (included
files, `Tier:`, the take a cut picks, `adopt`, references); run it after changing any of
them.

### Adopting takes from another board

Takes are kept by film and shot id (`$A2_DATA/films/<film>/<tier>/<shot>/`), so a shot
that moves to another film, or is renamed, has none there. When it is the same shot,
`a2film adopt` takes the old takes over instead of filming it again:

    ./a2film adopt films/beams-scout a01 films/promo            # beams-scout's a01 as promo's a01
    ./a2film adopt films/beams-scout a01 films/promo --as b03   # ... as promo's b03
    ./a2film adopt beams-scout a01 a02 films/promo -n           # by film name; say what it would do

The source is a board, or only a film's name when its board is on another branch. For
each tier and each variant of the target shot, a take is adopted only if the spec it was
filmed from is the target's spec, less the film's and the shot's names (a scene file is
compared by its text, not its path); otherwise `adopt` names the keys that differ and
leaves it. An adopted take is a copy (a reflink where the file system has them, so it
costs nothing on btrfs) with its `.spec.json` rewritten for the target and an `adopted`
key saying where it came from. Its commit and code are kept, so it is exactly as current
in its new film as it was in the old one. A target take that is current is kept unless
`--force`; `--tier` limits it to one tier.

## Review

`a2film review <board> [shots...]` (`--tier probe|final`, `--jobs 4`) is what is done by
hand with ffmpeg before a shot is approved, made automatic. It films nothing and changes
no take or board. Each take is decoded once, small (256 px wide, 15 frames a second, at
most 120 s) through ffmpeg into numpy, with Pillow drawing the pictures; 43 probe takes
take about 25 s. It writes, next to the takes:

- `<shot>/<variant>.review.png`: twelve frames across the shot's `In` .. `In`+`Length`
  window and a second either side (the margin dimmed), each with its timecode in the
  take; below, the motion curve (mean frame difference, peaks in amber, where it settles
  in green) and the luminance, the window shaded, flagged stretches marked on both.
- `<shot>/variants.review.png`: for a shot with two variants, their difference over the
  window (mean pixel difference and luma-histogram distance).
- `review-<tier>.html` (and `.json`): one page for the board with the table, every
  flag with its timestamp and every sheet. Running on a few shots writes the page for
  those only.

The table printed to the terminal has a row per take (shot, variant, window, flags,
suggested `In` / `Length`) and a row per pair of variants.

| Flag | Means |
|---|---|
| `black a-b` | at least 0.4 s of frames that are flat and near black (mean luma under 6, nothing brighter than the mean + 10). A dark sky with a ship in it is not black |
| `frozen a-b` | at least 1.5 s with no frame-to-frame change, apart from black and `desktop` shots. Advisory: a menu waiting for a click is frozen too |
| `flash t` | the luminance jumps up by 40 and falls back within 0.25 s |
| `white t` | a new, solid block of saturated white, less than a fifth of the frame: the artefact, not an explosion's core |
| `ends Ns early` | the take is shorter than `In` + `Length` |
| `silent`, `silent gap`, `clips t` | with `Audio: game`: no sound in the window, 2 s without any, or samples at full scale |

Only flags that overlap the window are in the table; the others are on the sheet and in the
page, marked "outside the window". The sound check reads the take's own sound, and a scene
with `Music=0` and nothing firing is silent, so read `silent` as a question.

Between two variants the page gives the mean difference over the window. Below 1.5 of 255
the pair "barely differ" (a before/after that shows nothing); above 30 the halves of a
`wipe` will not line up ("the variants differ everywhere": camera, scene or random
objects). It also looks for a constant offset of up to one second that makes the variants
agree better and says which `In:` it would take.

A suggestion appears only where it is obvious: `In` after a black start, or where a
motion that peaks at the start of the window settles (a camera pan coming to rest);
`Length` where the take ends early, or black or frozen frames start inside the window.
They are proposals: `In` and `Length` are the reviewer's, and `Status` is only set by a
person. `python3 films/film/selftest.py` pins the flags on short clips made with ffmpeg.

## The storyboard

Headings give the structure; `Key: value` lines directly under a heading (no blank line
needed, any order) are the fields; fenced blocks carry the camera and the cards; every
other line is prose. Only the keys below are fields, so a sentence that happens to start
`Note:` stays prose.

    # What's new in the refit              <- the film (one H1)
    Film: whats-new
    Final: 3840x2160
    Variant refit: --install . --install testbench/scene

    ## Act 1 — Fire                         <- a section (sequence, act)
    Music: Mkli300.wav from 0 gain 0 until k01 fade 1.5

    ### e01 · Galaxy class explodes          <- a shot: its id, a dot, a name
    Shows: explosions
    Scene: explosion
    Variants: stock, refit
    Layout: wipe
    Transition: dissolve 0.6
    Title: Explosions
    Sub: Ray-marched fireballs in place of the flipbook sprites
    In: 1.0
    Length: 8
    Status: draft

    ```setup
    scene orbit big 30 -6 420
    ```
    ```action
    wait 1
    scene kill big; glide 8 70 -4 380
    wait 9
    ```

### A board over several files

A board can be split, so that several people (or agents) can each write their part of a
film at once without editing one file. A line `Include: <file>.md`, on its own and outside
a fenced block, splices that file in where it stands, as if its text were written there:

    # The promo                      storyboard.md
    Film: promo

    Include: intro.md               <- its sections and shots come here, in order

    ## Beams
    Music: Mkli300.wav

    Include: beams.md               <- its shots join the section Beams
    Include: beams-borg.md

    ### z01 · End card
    ...

    # Beams, part of films/promo     beams.md
    Shots for the Beams section; the board is storyboard.md.

    ### b01 · Phasers
    ...

- An included file is read from its first `##` section, `###` shot or `Include:`. What
  stands before it is the file's own preamble, for whoever opens that file: a title, a
  note on whose part it is. It sets no fields, and only the board itself has a `# Film`
  heading and film fields.
- Its paths are relative to the file that includes it, and it must lie in the film's
  folder; it may include others in turn, but not itself. A scene file (`Scene: x.ini`)
  in any of them is relative to the film's folder, as in the board.
- The combined board is the film: `check`, `status`, `probe`, `take`, `sheet`,
  `animatic` and `manifest` all see it whole. Shot ids are unique across all its files,
  and a second use is an error that says where the first is. `check` lists the included
  files and marks each shot written in one (`[beams.md]`); the manifest carries
  `includes` and each shot's `file`.
- Where a shot is written does not change its takes (its spec has no file in it), so
  moving one between the files of a film costs nothing. Between films, see "Adopting
  takes from another board".

### Film fields (under the H1)

| Key | Default | |
|---|---|---|
| `Film` | the folder's name | the name of the footage folder in `$A2_DATA/films/` |
| `Final` | `3840x2160` | the final takes' resolution |
| `Probe` | `1280x720` | the probes' resolution. Keep the aspect of `Final`: positions in `Crop:` are fractions of the frame, so they carry over. Not 960x540: the game dies at start there (a Wine stack overflow) |
| `Fps` | `30` | the cut's frame rate. Takes are variable-rate (real time); never fixed at capture |
| `Pointer` | `1920x1080` | the space the pointer commands' coordinates are written in, scaled to each take, so a probe and a final take share one action block |
| `Args` | `-nointro a2_borg01` | the game's arguments, for every shot that names no `Args` of its own |
| `Variant <name>` | `refit`, `stock` | the bench `--install`s that make a variant; `refit`: `--install . --install testbench/scene`; `stock`: `--install testbench/scene`. Add your own (a branch's checkout, a setting) |
| `Music` | | `Music=0` in every scene: `0` (the default) or `1` |
| `Mute`, `Stems` | | the sound of every take, as the section and shot fields below; the nearest of shot, section and film wins |

### Section fields (under an H2)

| Key | |
|---|---|
| `Music` | the bed under this section: `<file> [from S] [gain dB] [until <shot>] [fade S]`. `file` is under the game's `sounds/music/`, a path under its `sounds/` (`effects/Shellsd0.WAV`, the main menu's loop), or a path. A section that carries its own score (a cinematic, a movie) says `Music: none`, and the bed before it must end before it begins |
| `Mute`, `Stems` | the sound of this section's takes, as the shot fields; a shot's own wins |

### Shot fields (under an H3)

| Key | Default | |
|---|---|---|
| `Kind` | `scene` | `scene`: filmed on the bench with a `Scene.asi` scene. `card`: a title card from a ```` ```card ```` block, nothing filmed. `footage`: another shot's take (`From:`), a scouted reference (`From: ref:<id>`) or a file (`File:`), not filmed. `mission`: a mission played from its start (`Args:` names it, `Scene: none` or a scene file with `Enable=0`). `desktop`: no game, a bare desktop running a program (`Run:`), for the GUI installer; see "Desktop shots" |
| `Shows` | | the features or layers this shot is for, comma-separated (`explosions`, `lighting: shadows`). Prose for the reader, and what `check` counts |
| `Scene` | | a scene in `testbench/scene/scenes/` by name, or a `.ini` path relative to the board |
| `Args` | the film's | the game's arguments for this shot |
| `Variants` | `refit` | which variants to film, in the order the layout reads them: the first is the one a wipe or dissolve starts from |
| `Players` | | several games in one shot, filmed together: `host, joiner`. Each player is a take named after it, installed as the shot's first `Variants:` (refit by default), and the layout reads the players as its variants; see "Two games in one shot" |
| `Prepare` | | `<script> [args]`, a script of this repository run with each take's session state file after its clone is made and before the game starts (`online/bench-asi.sh --server`), as a scenario's `Setup:` |
| `Layout` | `full` | how the variants fill the frame: `full [<variant>] [label]` (one variant, a corner label if `label`); `wipe [<mid> <end>]` (the first variant on the left; a divider comes in from the right, eases to rest at `mid` of the width, then on to `end`, revealing the second; default `0.5 1.0`, as the October showcase); `sweep [hold cross]` (the first variant held `hold` seconds, then the divider crosses the whole frame in `cross`, revealing the second; default `2.2 1.6`; a wipe without the split held in the middle); `dissolve [hold fade]` (the first variant held, then dissolved into the second; for subjects spread over the frame); `split [w]` (side by side, each variant's centre `w` of its width, 0.5 by default, fitted into half the frame; `0.75` is a 4:3 menu whole, letterboxed); `pip <variant> <corner>` (that variant small in a corner, `tl`, `tr`, `bl` or `br`, over the other) |
| `Crop` | | `x y w h`, fractions of the frame, applied to every variant before the layout (the HUD's grid: `0.15 0.64 0.43 0.36`) |
| `Transition` | `cut` | how the shot comes in: `cut`, `dissolve S`, `fade S` (from black), `dip S` (through black) |
| `Title`, `Sub` | | a lower-third title and its line below; `Caption: top` moves them to the top (HUD footage) |
| `In` | `0` | seconds into the take where the cut starts; per variant: `In: stock 3.3, refit 4.0` |
| `Length` | | seconds on screen. `check` warns if the action's waits are shorter than `In` + `Length` x `Speed` |
| `Cut` | | `from <time> to <time> [pad S]`, the `In` and `Length` two marks give (see "Marks"); `check` and `status` print them for each tier with takes, to be written into `In:` and `Length:` |
| `Expect` | | what the take must show, checked against its marks after filming (`selected >= 20 by 3s; decloak.done before 8s`; see "Marks"). A take that misses it is `suspect` |
| `Speed` | `1` | playback speed in the cut |
| `Audio` | `game` | `game` (the take's own sound), `none`, `vo` (voice-over: lift it over the bed), or a mix of the take's stems, `<stem> [dB] + ...`: `effects + voice -6` (see "Sound") |
| `Mute` | | game sound categories at volume 0 in the take: `music`, `voice`, `effects`, `interface`; `none` undoes a section's. Changes the take (it is in the spec) |
| `Stems` | | categories recorded as stems of their own: `music`, `voice`, `interface`; the rest is the `effects` stem. Not with the same category in `Mute`. Changes the take |
| `Labels` | | the text a layout's corner labels show in place of the variants' (or players') names: `host Player 1; joiner Player 2`. An edit field: it never re-films |
| `Keys` | `auto` | key hints: `auto` shows each `press` of the action block for 1.2 s; or `T text; T text`, times from `In`; `none` |
| `From` | | `footage` only: `<shot> [variant]`, that shot's take at the tier being cut (its last variant by default; the best one, as "The cut today" says). For teasers and montages that reuse a take. Or `ref:<id>`, a moment from the reference catalogue (`films/reference/README.md`): its file, with the shot's `In:` counted from the entry's `In`, and running to its `Out` without a `Length:` |
| `Tier` | `probe` | the resolution this shot is probed at, when it is not the film's `Probe`: a height (`1080`, `1440`, `2160` or `4k`) at the probes' aspect, or `WxH`; or `final` (or a size equal to `Final`), which films its final take in the probe run. See "A shot's tier" |
| `File` | | `footage` only: a file, a path relative to `$A2_DATA` |
| `Background` | | `card` only: a shot id whose take is blurred behind the card |
| `Status` | `draft` | `draft`, `probed`, `approved`, `final` |
| `Run` | | `desktop` only: the program the `run` command starts. Leading `NAME=value` words are its environment; `{repo}` is this checkout |
| `Wallpaper` | black | `desktop` only: `<shot> [variant] [t]`, a still at `t` s (1) from that shot's take at the same tier, or `#rrggbb` |

### A shot's tier

The film's `Probe` and `Final` apply to a whole run: `probe` films at the one, `take` at
the other. A shot's `Tier:` lets the money shots be filmed larger in the same probe run
while the rest stay cheap:

| `Tier:` | `probe` films it | into | `take` films it |
|---|---|---|---|
| (none), `probe` | at the film's `Probe` | `probe/` | at `Final` |
| `1080`, `1440`, `2160`, `4k`, `WxH` | at that size, the probe's quick encoding | `probe/` | at `Final` |
| `final`, or a size equal to `Final` (`2160` or `4k` at a 3840x2160 `Final`) | its final take: at `Final`, the final encoding | `final/` | at `Final`, unless that take is current |

- The resolution is part of the spec, so changing a shot's `Tier:` makes its probe stale
  and the next `probe` films it again; a shot without one keeps the spec (and the takes)
  it had before `Tier:` existed. Final takes do not depend on it. A `Tier: final` shot's
  final take is current or stale like any other (`--force`, `--ignore-code` as usual).
- A numeric `Tier:` that comes to the film's `Final` (a height at the probes' aspect, or a
  `WxH`) means `final`: a 4K probe would only be filmed again by `take`, so the shot's
  final take is filmed instead. Smaller sizes stay larger probes.
- In a probe run the takes larger than the probes are filmed after the others, one at a
  time, whatever `--jobs` says: several large recording games at once is what died at
  1080p ("Traps").
- `status` shows both tiers; a `Tier: final` shot has no probe, only its final take.

### Blocks

- **```` ```setup ````** — run once the scene is built, before recording: place the camera,
  give the first orders. Not recorded.
- **```` ```action ````** — recorded from its first line to its last. Its time line is the
  take's: `In:` and `Keys:` count from its start.
- **```` ```card ````** — a card's lines, top to bottom, each optionally styled:
  `[accent] KLINGON CAMPAIGN`, `[big] ARMADA II`, `[mark] REFIT` (big, in the accent colour),
  `[small] github.com/...`; a plain line is a heading.

The setup and action blocks take one command per line (`#` comments):

| Command | |
|---|---|
| `scene CMD[; CMD]` | Scene.asi commands (`testbench/scene/README.md`, "Commands"): `orbit`, `glide`, `spin`, `goto`, `attack`, `kill`, `cloak`, `special`, `hud on` ... |
| `wait S` | wait S seconds |
| `key KEY`, `click X Y`, `step TEXT`, `shot NAME` | as `a2test drive` |
| `ptr X Y [S]` | move the pointer there, eased, over S seconds (0.6) |
| `drag X0 Y0 X1 Y1 [S]` | press, drag over S seconds (1.0), release |
| `lclick [X Y]` | a left click, after moving there |
| `press COMBO` | a key press at human speed (`shift+1`); with `Keys: auto` it becomes a key hint |
| `run` | `desktop` only: start the shot's `Run:` program |
| `typelog PLAYER FILE MARKER` | `Players:` only: type the word after `MARKER` on the last line of `FILE` in that player's game directory that has it, waiting up to 30 s (`typelog host Online.log "join code"`) |
| `PLAYER: CMD` | `Players:` only: the command goes to that player's game (`host: step Click "Create Game"`); a line without a player goes to each in turn, and a bare `wait` is waited once |

Pointer coordinates (`ptr`, `drag`, `lclick`, `click`) are in the film's `Pointer` space,
1920x1080 unless it says otherwise, and scaled to each take: a 1280x720 probe and a 4K
take of one action block put the pointer on the same thing. In the game that holds for
the HUD and the map, which scale with the screen at one aspect ratio. Where an order can
go without the pointer, Scene.asi's `select` and `goto` are surer.

### Desktop shots

A `desktop` shot films a program, not the game: the GUI installer, in this repository.
Its take is a stock bench session started with `--no-launch` (`a2test session start
--mod stock --no-launch`): a headless desktop and a clone of the stock game, with no
game running. The runner then

- scales the output to 900 logical rows (a 1600x900 desktop at every tier), so a window
  fills the same part of the frame in a probe and a 4K take, and the `Pointer` space
  maps onto it;
- floats every window in the middle, over the wallpaper, which is an `mpv` window tiled
  underneath (there is no `swaybg` here);
- makes a scratch `HOME` in which Heroic (`gog_store/installed.json`, `GamesConfig`)
  knows the game at `~/Games/Heroic/Star Trek Armada II`, with the user's own vkBasalt
  layer copied in, and the clone's record of an earlier release removed: the installer
  finds a fresh stock game and really installs into it;
- runs the program in user, mount and PID namespaces of its own: the clone is bind-mounted
  at that path (the installer resolves links, and would show the bench's), and it sees
  none of the bench's other games or the user's Heroic, which it would warn about. Its
  environment has no D-Bus session, so nothing it does reaches the user's desktop;
- records with `wf-recorder -D`: a still desktop sends no damage, so without it a take
  has a handful of frames.

There are no variants: one take, named `desktop`.

### Two games in one shot

A shot with `Players: host, joiner` films two games together, for online play: each
player is a session of its own, both started together (eight seconds apart), set up and
recorded over one action block, so their time lines agree and `Layout: split` can put
them side by side (`split 0.75` for the menus, which fill the 4:3 middle). Every line
runs in order, whichever player it is for, so a `host:` step that waits for text holds
the joiner too: write the action as the story the two screens tell together.

- `Prepare: online/bench-asi.sh --server` installs `Online.asi` in each clone and starts
  a local `a2online-server`; with `Prepare:` a session starts without the game
  (`--no-launch`), runs the script, then launches it with the shot's `Args:`.
- The steps are the online scenarios' (`testbench/scenarios/_online-setup.md`,
  `_online-play.md`), as `step` lines: they find buttons by OCR, which is surer than
  pointer coordinates across tiers. The joiner types the host's join code with
  `typelog host Online.log "join code"`.
- A menu shot's scene file has `Enable=0` (no scene to wait for: the take starts once
  the game runs) and `Music=0`.
- `check` cannot time an action that waits for text on screen, so it does not warn
  about `Length:` there; read the takes' length off the probe.
- Two players are two games but one job of `--jobs`: probing it beside other shots,
  lower `--jobs` so the bench is not running more games than it should.

## Marks

Every take records **event marks**: what happened in it and when, on the take's own
clock (0 is the first line of the action block, the video's time 0; the setup's marks
are negative). They replace measuring a moment from frame differences (when a pan
settles, when a cloak is done) and finding a take that went wrong only by watching it.

    ./a2film marks films/promo b07                  # every mark of b07's takes
    ./a2film marks films/promo b07 --kind 'cloak.*' --tier final
    ./a2film marks films/promo b07 --setup --json   # with the setup's marks, as JSON

They come from three places, put on one clock:

- **Scene.asi** writes `Scene.marks` in the game directory (`testbench/scene/README.md`,
  "Marks"). Each line carries the game's `GetTickCount()`, and its `epoch` lines pair a
  tick with `GetSystemTimeAsFileTime()`, which under Wine is the host's own clock, the
  one the runner starts the action by: a mark lands on the take's clock without a
  handshake. On the bench (`films/marks`, 2026-10-17) every `cmd` mark fell between the
  runner sending its line and reading the answer, and **a mark leads the first frame
  that shows it by about 0.05 s** (a glide's first motion 0.05 s, an explosion 0.06 s:
  three or four frames at 60 fps, the time from the game's frame to the recording).
- **The action runner** marks each line of the action block as it starts
  (`action.<op>`) and, except `wait`, as it is done (`action.<op>.done`):
  `action.step.done` is when an OCR wait found its text, `action.press` a key.
- **UI.log**, followed from the setup on, gives `menu.open` (and `menu.pane`) for each
  shell dialog the UI hosts, named as `UI.log` names it, as its line appears (within
  0.05 s; what the log held before is not marked). Only takes with `UI.asi` have these:
  a stock take's menus are not marked.

| Kind | Fields | When |
|---|---|---|
| `ready` | `objects` | the scene is built |
| `cmd` | `text` | Scene.asi takes a command (the frame or tick it reads `Scene.cmd`) |
| `camera.move`, `camera.still` | `eye`, `front` | the view starts to move; it has not moved for `[Marks] Still` (0.3 s), marked at its last motion: a pan, a glide or a followed ship settled |
| `glide.start`, `glide.end` | `secs`, `yaw`, `pitch`, `dist` | a `glide` command, and the moment it ends |
| `spin.start`, `spin.stop` | `rate` | |
| `select` | `n`, `names` | the selection changes (and once at `ready`): how many and which, by scene name or handle |
| `cloak.start`, `cloak.done`, `decloak.start`, `decloak.done` | `obj` | each step of a scene craft's cloak |
| `fire.start`, `fire.end` | `obj`, `weapon`, `target`, `shots`, `secs` | a burst: the first ordnance of a shooter and weapon ODF, and the last one, once none has been alive for `[Marks] FireGap` (1 s). Beams (phasers, the tractor, holding and assimilation beams) are ordnance while drawn, torpedoes and pulses while in flight |
| `special` | `obj`, `weapon`, `target`, `can`, `fired` | a `special` command's answer |
| `explosion` | `type` (`fireball`, `shockwave`), `r`, `at`, `near`, `d` | a new explosion, with the scene object nearest it |
| `destroyed`, `removed` | `obj` (`how`) | a scene craft starts dying (or, `how=removed`, is gone without being seen dying, as `kill` does it); an object is gone |
| `action.<op>`, `action.<op>.done` | `arg` (`player`) | the runner's lines |
| `menu.open`, `menu.pane` | `name`, `template` | UI.log |

`<take>.marks.json` (`"format": "a2film-marks/1"`) holds `start` (the action's start,
Unix time), `duration`, `sources` (which of `scene`, `ui`, `action` the take had), the
`marks` in time order, each `{"t", "kind", "src", ...fields}`, and, for a shot with
`Expect:`, the verdicts as filmed. The manifest carries each take's marks, its `cut` and
its `expect`.

**A mark is named** `kind[:subject][#n]`. The kind and the subject may be globs; the
subject matches any field (`fire.start:galaxy` by shooter, `fire.*:gholdbm` by weapon,
`select:warbird` among the names); `#2` is the second, `#-1` (`#last`) the last. Kinds
may be written with spaces (`decloak done:bop`).

**`Cut: from <time> to <time> [pad S]`**: a time is a mark (its first, unless `#n`)
with an optional offset (`fire.end:big +0.5`), seconds (`2.5`), `start` or `end`;
`pad` widens both ends. It gives the `In` and the `Length` (at the shot's `Speed`), per
variant, which `check` and `status` print as the lines to write:

    m01      probe: Cut suggests In: 2.9   Length: 9.4   (the board: In: 0   Length: 10)

It does not set them: `In:` and `Length:` stay what the board says, so the cut never
moves because a take was filmed again.

**`Expect:`** is `;`-separated, each one of:

| Form | Holds when |
|---|---|
| `selected <op> N [by T \| at T]` | the selection's count reaches it at some moment up to `T` (the take's end), or is it at `T`; op is `>=`, `<=`, `==`, `!=`, `>`, `<` |
| `<mark> [before T \| by T \| after T \| between A and B]` | a mark of that name falls in the take, then |
| `no <mark> [...]` | none does |
| `<mark> count <op> N [...]` | so many do |

`T` is a time as in `Cut:`, so `cloak.done after glide.end +1` works. Only marks inside
the take count (the setup's do not, but the selection at the start counts what the
setup selected). An expectation a take cannot check (a `select` with no Scene.asi, a
`menu.open` without UI.asi) is reported as not checked and does not make it suspect.
`python3 films/film/selftest_marks.py` pins all of this without a game, and
`./a2film probe films/marks --jobs 1` checks it on the bench.

What the marks cannot see: anything of objects the scene did not build except fire and
explosions (cloaks and deaths are of scene objects); the menus without `UI.asi`; a
special weapon that makes no ordnance (a cloak, a shield effect); and the last frames:
a take is cut from the end of its recording, so a mark is as exact as the take's own
time line is (`.events.json` has the same error).

## Sound

A take records the game's sound live, from the bench session's silent sink. The game
has four categories of sound, each with its own volume (`testbench/README.md`, "Audio
stems"): `music`, `voice` (mission narration and speech), `effects` (the 3D sounds of
the world) and `interface` (the button bar). Two fields decide how they reach a take,
on a shot, its section or the whole film:

- **`Mute: music, voice`** sets those volumes to 0 in the take's clone of the game
  (`a2test session start --mute`), so they are never in it: a mission intro filmed
  without its narration and score, to be laid under a voice-over or a bed of the
  film's own. Mutes are part of the take's spec: changing them re-films the take.
- **`Stems: music, voice`** records them as separate tracks
  (`a2test session start --stems`): each plays on a silent sink of its own and is
  recorded beside the take as `<variant>.music.wav` and `<variant>.voice.wav`, and
  whatever is left (the effects, the interface, unless it is a stem too) as
  `<variant>.effects.wav`. Every stem is cut from its end like the picture, so all
  share the take's time line, and each is recorded at full volume. The take's own
  track (`<variant>.mkv`) stays the whole of its sound: the stems summed, so
  `Audio: game` means what it always did.

The mix is then the edit's: `Audio: effects + voice -6` plays the effects stem and the
voice 6 dB down in the animatic; `Audio: game` the whole; a term `game` is the take's
own track. `check` refuses a mix that names a stem the shot does not record, and the
animatic leaves out a stem it cannot find (a take filmed before `Stems:`), saying so.
The manifest gives each shot's `mute`, `stems` and `mix`, and per take the stems'
paths (`stems`), so an editor can mix them itself.

**For a music stem, the film needs `Music: 1`.** The default `Music: 0` puts `Music=0`
into every take's scene file, and Scene.asi then never starts a track: the music stem
would be silence. A mission shot with its own scene file (`Enable=0`, `Music=1`) is
treated the same way.

The bench measured the effects stem of a Federation intro at the level of the same
shot filmed with `Mute: music, voice` (`testbench/README.md`, "Audio stems"). The stems
are recorded from separate Wine devices, each with its own output buffer, and by
separate `parec`s stopped together, so they are aligned to within that buffering, not
to the sample; the offset has not been measured yet. Judge a hard sync point (a line
on a cut) by ear before relying on it. Mute and stems apply
to the game's own sound only; a Bink movie (`cutscenes/`) has its own track and is not
split.

## What the board should say

The board is the brief for the cut as much as for the camera. Beyond the fields, say in
prose what each section is *for* (the argument it makes), what the eye should land on in
each shot, and anything the editor should know that no field holds: a sound that must
land on a cut, a shot that must not be trimmed past a moment, an alternative angle.
House style for showcase footage (from the October showcase):

- **Close-ups.** A hull fills a good part of the frame; wide only when the subject is the
  wide thing (a fleet, a terminator).
- **Slightly low angles**: pitch about -3 to -12, looking up at the subject.
- **Wipe for one large centred subject; dissolve for spread-out scenes and UI.**
- **Keep out**: shields-down rings, transporters, damage plumes, hover gauges, tooltips —
  unless they are the subject (Scene.asi switches each off).
- **No game music in takes** (`Music=0`); beds are laid in the cut, matched to the
  section, and **never two music sources at once**.
- A UI feature gets a close crop held long enough to follow, with a key hint per press
  (about 18 s for the grid hotkeys; 5 s was far too short).

## Game slots and retries

A take starts its session in a game slot (`testbench/bench/slots.py`): at most `A2_SLOTS`
games run on the machine at once, whoever started them, and a take waits for a free one.
A shot with `Players` takes the slots of all its players in one request, so two such shots
cannot deadlock. `./a2film slots` shows who holds them.

A take whose game dies with it (crashed, killed under load, never launched, gone before the
scene was built or at the end of the action) is filmed once more from a fresh session; a
shot with `Players` is filmed again as a whole. A second death fails the take. Any other
failure is not retried.

## Traps

- **Never record at a fixed frame rate** (`wf-recorder -r`): the frames are stamped at
  that rate and the video drifts from the sound when the game draws fewer. Takes are
  variable-rate and are converted to `Fps` in the cut.
- **wf-recorder writes limited-range pixels but flags them full-range.** Every take is
  re-flagged on the way in (no re-encode), or every later step lifts black to grey. Judge
  black by decoding to RGB (a dark region's mean is about 3), never by the container's
  tags.
- **The hidden pointer still raises tooltips.** Scenes turn them off (`tooltips off`);
  a take with the HUD on should be checked.
- **Several recording games at once is for probes.** At 1080p two recording sessions once
  both died (DXVK allocation failure). Four at 1280x720 held 59–60 fps each
  (2026-10-14); final takes run `--jobs 1` unless measured otherwise.
- **wf-recorder takes seconds to deliver its first frame.** The runner waits until it
  writes, and cuts each take from its *end*, where both recordings stopped together.
- **Don't `orbit` an object the shot kills**: when it dies the camera jumps to the
  origin and the frame is empty sky. Orbit a fixed point at its position.
- **Things drawn at random differ between variants** (an asteroid field's rocks are
  picked per session): a wipe across them shows two different fields. Prefer `dissolve`,
  or `full refit label`.
- **A real install takes about 40 s** from the cache-less GUI installer, most of it on
  "Rewriting the models…" (2026-10-14). Film it in one take and cut its start and its
  end as two shots (`From:` the same take).
- **Tk is not installed here** (`libtk8.6.so`), so the Windows/macOS window
  (`A2_INSTALLER_TK=1`) cannot be filmed until `pacman -S tk`.
- **A take's currency sees committed and uncommitted changes to tracked files, nothing
  else.** A new file not yet added to git, an asset rebuilt in `$A2_DATA` (a texture, a
  plate, a movie) and another checkout a variant installs (`--install ../other`) do not
  make a take stale; a variant that installs another checkout is judged on all of this
  one. After rebuilding assets, re-film with `--force`.
- **Run `a2film` from the checkout whose code you mean to film.** Takes record the
  checkout that ran `a2film`; judged from a worktree whose branch is behind or ahead of
  main, main's takes are stale there and current again on main.
- **Shots run in a fresh session each**, so one shot's orders never leak into the next.
  A session's start costs about a minute; that is what `--jobs` buys back.

## Cuts

`a2film animatic` keeps every cut. A whole-film cut is written as
`animatic-<tier>-rc<N>.mp4`, `N` the next free number (cuts made at the same moment never
share one), and copied to `animatic-<tier>.mp4`, which is always the latest: players and
the review page can keep pointing at it. It is a copy, not a link, because an older
`a2film` writes that file in place. `--name <label>` writes `animatic-<tier>-<label>.mp4`
in place of `-rc<N>` (it never overwrites one); it is still a whole cut and becomes the
latest.

A **partial cut** is for judging a few shots without waiting for the film:

| Option | Cuts |
|---|---|
| `--only a01,b03` (or positional ids) | these shots, in the order given |
| `--section "Act 1"` | one section, by its title or a part of it |
| `--from a01 --to b05` | the shots between, in film order, either end optional |

Several can be given and all apply. A partial cut goes to
`animatic-<tier>-part-<label>.mp4` (the label from the options, or `--name`), is not
numbered and does not replace the latest; positional ids keep their old file name
`animatic-<tier>-<ids>.mp4`. It uses the same layouts, titles, transitions and beds, with
the segments rendered in a temporary folder and a faster encode. A section's bed starts at
the cut's first shot of that section, from its `from` point, and ends at its `until` shot
or the last shot in the cut, so a partial cut's music is a sketch.

**The shot map.** Next to every cut, `<cut>.shots.json` (`"format": "a2film-shots/1"`)
lists the shots in cut order with `id`, `name`, `title`, `section`, `kind`, `status`,
`start` and `end` (the shot's own span; a dissolve overlaps the shot before),
`owns_start` and `owns_end` (the spans split without overlap, a dissolve at its middle),
`layout`, `transition`, `variants`, `used_variants` and `takes`: the path of each take the
layout shows, whether it is current, and its in point. Any time `t` in the cut is in the
one shot with `owns_start <= t < owns_end`. It also holds the cut's probed `duration`, the
board's path, hash and commit (`board_dirty` if the board has uncommitted changes).
`cuts.json` is the log: for each cut its file, `kind` (`rc`, `named` or `part`), `rc`,
date, duration, board hash and commit, and its shots with their takes (a partial cut also
its selection).

## The cut today, and where this is going

`a2film animatic` is a rough cut, there to judge structure and timing: every layout,
caption, key hint, transition and music bed of the board, at the probe tier, with a grey
slate for any shot not yet filmed.

**Each shot is cut from its best take**, scaled to the cut's resolution (the film's
`Probe`, or `Final` with `--tier final`). At the probe tier that is the shot's final take
or its probe, whichever is current; with both current, the larger. A stale take whose
spec still holds (its code changed) comes before one whose spec changed, and a stale take
before a slate. So a money shot with a 4K take, or a probe at `Tier: 1080`, shows at its
best in a 720p animatic. `--tier final` cuts from final takes only. `sheet` and
`manifest` pick the same takes (each manifest take says its `tier`), and a footage shot's
`From: <shot>` follows them. `--ignore-code` counts a take whose code changed as current
when picking. It approximates: a `dip` fades the shot before out
and this one in, a `dissolve` is the only overlap, `vo` is a gain with no ducking, and
the font is the system's.

The finished cut is meant to be made elsewhere, from `a2film manifest`: a
self-describing JSON (`"format": "a2film-manifest/1"`) with the film's sections and
music, every shot in timeline order with its start and end, layout, transition, titles,
key hints and card lines, its prose as `direction`, and per variant the take's path,
length, commit (`dirty` if uncommitted changes were installed), in point, whether it is
`current` and, if not, why (`stale`: "spec changed", "code changed since ..."). An editor driven over MCP reads
that and nothing else.

Not yet:

- **Montages** (a fast cut of several takes as one shot) are written as consecutive
  `footage` shots with `From:`.
- **A fixed-timestep film mode** in Scene.asi (the game clock advanced 1/60 s per drawn
  frame, every frame saved) would make final takes smooth whatever the load, at the
  cost of live sound. Measure the 4K takes first.
