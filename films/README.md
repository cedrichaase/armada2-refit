# Films — storyboards shot on the test bench

A film is written as a **storyboard**, one Markdown file that is both the script a person
reads and the recipe `./a2film` shoots from: what each shot shows, how the camera moves,
and how the shot is used in the cut (layout, transition, titles, music). The footage is
filmed on the test bench (`testbench/README.md`), several games at once, and the cut is
made from the storyboard's edit direction (today a rough cut by `a2film animatic`; later
a real editor, fed by `a2film manifest`).

    ./a2film check  films/whats-new            # parse it, list the shots, timeline length
    ./a2film probe  films/whats-new            # every shot, 1280x720, 4 games at once
    ./a2film probe  films/whats-new g03 g04    # ... just these
    ./a2film sheet  films/whats-new            # one contact sheet: every shot, in order
    ./a2film animatic films/whats-new          # a rough cut from the probe takes
    ./a2film take   films/whats-new            # the final takes, 4K, one game at a time
    ./a2film manifest films/whats-new          # the edit as JSON, for an editor
    ./a2film status films/whats-new

Like the texture targets, a storyboard is a **recipe** and is committed. Its footage is
an asset and lives in `$A2_DATA/films/<film>/`, never in a checkout (hard rule 8):

    $A2_DATA/films/<film>/
      probe/<shot>/<variant>.mkv   .wav  .events.json  .spec.json  .sheet.png
      final/<shot>/<variant>.mkv   ...
      sheet-probe.png               every shot in timeline order
      animatic-probe.mp4            the rough cut
      manifest.json

## The process

1. **Write the board.** Sections, then shots: what each shows and why, camera, layout,
   titles. Prose is welcome anywhere: what the tool does not read is direction for the
   person (or agent) who cuts the film.
2. **Probe.** `a2film probe` films every changed shot at 1280x720, several games in
   parallel (`--jobs`, default 4), and makes a contact sheet per take. Probes are cheap
   and exist to get the *direction* right: framing, angle, timing, what is in the shot.
3. **Review.** `a2film sheet` and `a2film animatic` show the whole film at once. Notes go
   back into the board, and each shot's `Status:` moves on.
4. **Repeat** 2–3. A shot is only re-filmed when its spec changed (its setup, action,
   scene file or variant), so iterating on one shot costs one shot.
5. **Final takes.** `a2film take` films the shots at 3840x2160, one game at a time by
   default (`--jobs`), because a recording game under load draws fewer frames and the
   take shows it. The final takes record the commit they were installed from.
6. **Cut.** `a2film manifest` writes the edit — order, in and out points, layouts,
   transitions, titles, key hints, music — with the path and length of every take.

Statuses: `draft` (written, not filmed) → `probed` → `approved` (direction signed off;
the probe is the reference) → `final` (the 4K take is accepted). `a2film` never sets a
status: that is the reviewer's call, written into the board.

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

### Film fields (under the H1)

| Key | Default | |
|---|---|---|
| `Film` | the folder's name | the name of the footage folder in `$A2_DATA/films/` |
| `Final` | `3840x2160` | the final takes' resolution |
| `Probe` | `1280x720` | the probes' resolution. Keep the aspect of `Final`: positions in `Crop:` are fractions of the frame, so they carry over. Not 960x540: the game dies at start there (a Wine stack overflow) |
| `Fps` | `30` | the cut's frame rate. Takes are variable-rate (real time); never fixed at capture |
| `Args` | `-nointro a2_borg01` | the game's arguments, for every shot that names no `Args` of its own |
| `Variant <name>` | `refit`, `stock` | the bench `--install`s that make a variant; `refit`: `--install . --install testbench/scene`; `stock`: `--install testbench/scene`. Add your own (a branch's checkout, a setting) |
| `Music` | | `Music=0` in every scene: `0` (the default) or `1` |

### Section fields (under an H2)

| Key | |
|---|---|
| `Music` | the bed under this section: `<file> [from S] [gain dB] [until <shot>] [fade S]`. `file` is under the game's `sounds/music/` or a path. A section that carries its own score (a cinematic, a movie) says `Music: none`, and the bed before it must end before it begins |

### Shot fields (under an H3)

| Key | Default | |
|---|---|---|
| `Kind` | `scene` | `scene`: filmed on the bench with a `Scene.asi` scene. `card`: a title card from a ```` ```card ```` block, nothing filmed. `footage`: another shot's take (`From:`) or a file (`File:`), not filmed. `mission`: a mission played from its start (`Args:` names it, `Scene: none` or a scene file with `Enable=0`) |
| `Shows` | | the features or layers this shot is for, comma-separated (`explosions`, `lighting: shadows`). Prose for the reader, and what `check` counts |
| `Scene` | | a scene in `testbench/scene/scenes/` by name, or a `.ini` path relative to the board |
| `Args` | the film's | the game's arguments for this shot |
| `Variants` | `refit` | which variants to film, in the order the layout reads them: the first is the one a wipe or dissolve starts from |
| `Layout` | `full` | how the variants fill the frame: `full [<variant>] [label]` (one variant, a corner label if `label`); `wipe [<mid> <end>]` (the first variant on the left; a divider comes in from the right, eases to rest at `mid` of the width, then on to `end`, revealing the second; default `0.5 1.0`, as the October showcase); `dissolve [hold fade]` (the first variant held, then dissolved into the second; for subjects spread over the frame); `split` (side by side, each cropped to its centre half); `pip <variant> <corner>` (that variant small in a corner, `tl`, `tr`, `bl` or `br`, over the other) |
| `Crop` | | `x y w h`, fractions of the frame, applied to every variant before the layout (the HUD's grid: `0.15 0.64 0.43 0.36`) |
| `Transition` | `cut` | how the shot comes in: `cut`, `dissolve S`, `fade S` (from black), `dip S` (through black) |
| `Title`, `Sub` | | a lower-third title and its line below; `Caption: top` moves them to the top (HUD footage) |
| `In` | `0` | seconds into the take where the cut starts; per variant: `In: stock 3.3, refit 4.0` |
| `Length` | | seconds on screen. `check` warns if the action's waits are shorter than `In` + `Length` x `Speed` |
| `Speed` | `1` | playback speed in the cut |
| `Audio` | `game` | `game` (the take's own sound), `none`, or `vo` (voice-over: lift it over the bed) |
| `Keys` | `auto` | key hints: `auto` shows each `press` of the action block for 1.2 s; or `T text; T text`, times from `In`; `none` |
| `From` | | `footage` only: `<shot> [variant]`, that shot's take at the tier being cut (its last variant by default). For teasers and montages that reuse a take |
| `File` | | `footage` only: a file, a path relative to `$A2_DATA` |
| `Background` | | `card` only: a shot id whose take is blurred behind the card |
| `Status` | `draft` | `draft`, `probed`, `approved`, `final` |

### Blocks

- **```` ```setup ````** — run once the scene is built, before recording: place the camera,
  give the first orders. Not recorded.
- **```` ```action ````** — recorded from its first line to its last. Its time line is the
  take's: `In:` and `Keys:` count from its start.
- **```` ```card ````** — a card's lines, top to bottom, each optionally styled:
  `[accent] KLINGON CAMPAIGN`, `[big] ARMADA II`, `[small] github.com/...`; a plain line
  is a heading.

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

Pointer coordinates are in the take's pixels, so a shot that uses them should be filmed at
one resolution; prefer Scene.asi's `select` and `goto`, which have none.

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
- **Shots run in a fresh session each**, so one shot's orders never leak into the next.
  A session's start costs about a minute; that is what `--jobs` buys back.

## The cut today, and where this is going

`a2film animatic` is a rough cut, there to judge structure and timing: every layout,
caption, key hint, transition and music bed of the board, at the probe tier, with a grey
slate for any shot not yet filmed. It approximates: a `dip` fades the shot before out
and this one in, a `dissolve` is the only overlap, `vo` is a gain with no ducking, and
the font is the system's.

The finished cut is meant to be made elsewhere, from `a2film manifest`: a
self-describing JSON (`"format": "a2film-manifest/1"`) with the film's sections and
music, every shot in timeline order with its start and end, layout, transition, titles,
key hints and card lines, its prose as `direction`, and per variant the take's path,
length, commit, in point and whether it is current. An editor driven over MCP reads
that and nothing else.

Not yet:

- **Montages** (a fast cut of several takes as one shot) are written as consecutive
  `footage` shots with `From:`.
- **A fixed-timestep film mode** in Scene.asi (the game clock advanced 1/60 s per drawn
  frame, every frame saved) would make final takes smooth whatever the load, at the
  cost of live sound. Measure the 4K takes first.
