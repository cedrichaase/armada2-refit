---
name: film
description: Direct a video shot in the Armada II engine — write or revise a storyboard (films/<name>/storyboard.md), probe it on the test bench at low res in parallel, review the stills and the rough cut, iterate, then film the 4K takes and write the edit manifest. Use for any showcase, trailer, comparison or promo footage, "film this", "shoot a video", "storyboard", "animatic", "re-take".
---

# Directing a film with a2film

The format, the commands and the traps are in `films/README.md`. Read it first; this is
the procedure.

## The loop

1. **Board.** Edit `films/<name>/storyboard.md`. New film: copy the shape of
   `films/whats-new/storyboard.md`. Every section says in prose what it argues; every
   shot says what the eye should land on. Pick scenes from `testbench/scene/README.md`
   ("The scenes", "Commands") and object names from `testbench/scene/scenes/*.ini`. A
   feature with no scene gets a scene file beside the board (`Scene: my.ini`).
   When other sessions work on the same film, write your part in a file of its own
   that the board includes (`Include: beams.md`, "A board over several files") and
   leave the others' files alone. A shot that moves from a scratch board keeps its
   takes with `./a2film adopt`. A scouted moment goes into `films/reference/` and is
   cut in with `From: ref:<id>`.
2. **Check.** `./a2film check films/<name>` must print no `!` lines.
3. **Probe.** `./a2film probe films/<name> [ids]` (1280x720, 4 at once; only takes
   that are not current are re-filmed: the shot's spec changed, or the code it shows
   did since the take's commit. `./a2film status` says which and why. `--ignore-code`
   keeps takes the code has moved past; use it only when the user asks). Run it in the background; a take is about 2–3 minutes, mostly
   the game starting. A `FAILED` take has its log at
   `$A2_DATA/films/<name>/probe/<id>/<variant>.log`.
4. **Look.** Read each `<variant>.sheet.png` of the shots you changed, then
   `./a2film sheet films/<name>` (the whole film) and `./a2film animatic films/<name>`
   (pull a few frames with ffmpeg to look at). Judge against the house style below and
   against the shot's own prose. Fix framing in the board and probe again. Never mark a
   shot `approved` or `final` yourself: statuses are the user's call; you may set
   `probed` once a probe exists.
5. **Show the user.** The sheet and the animatic's path, and per shot what you changed
   and what you are unsure of. Put the user's notes into the board as edits, not as
   chat.
6. **Final takes** only for shots the user approved: `./a2film take films/<name> [ids]`
   (3840x2160, one at a time). A shot's `Tier:` (`1080`, or `final` for its 4K take in
   a probe run) is the same call: set it only where the user asked for it. Then `./a2film animatic --tier final` and
   `./a2film manifest films/<name> --tier final`; the manifest is the handoff to the
   editor.

## House style (the user's, from the October showcase)

- Close-ups: a hull fills a good part of the frame. Wide only when the wide thing is the
  subject (a fleet, a terminator, a nebula's extent).
- Slightly low angles: pitch -3 to -12, looking up.
- Wipe for one large centred subject; dissolve for spread-out scenes and UI.
- Keep out of frame: shields-down rings, transporters, damage plumes, hover gauges,
  tooltips, unless they are the subject.
- No game music in any take; beds per section in the cut, matched to the faction;
  never two music sources at once (a bed ends before a segment with its own score).
- A UI feature: close crop, held long enough to follow (about 18 s for the grid
  hotkeys), a key hint per press.
- Blacks must be black: judge a dark region of a decoded RGB frame (about 3), never
  the container's tags.

## Traps worth remembering

- 960x540 crashes the game at start; probe at 1280x720.
- Don't `orbit` an object the shot kills; orbit its position.
- Random content (asteroid rocks) differs between variants: dissolve or refit only.
- The HUD's action camera is a second view: a shot with the HUD on shows render
  plugins' per-view bugs that `Hud=0` scenes hide.
- Bench etiquette: `a2film` stops every session it starts; never stop sessions you
  did not start (`testbench/README.md`, "Which session a command acts on").
