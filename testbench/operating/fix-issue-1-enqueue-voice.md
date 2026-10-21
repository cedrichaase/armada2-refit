# Operating the bench: the advisor's lines, resource refusals (agent "fix-issue-1-enqueue-voice")

Tested 2026-10-21, 1280x720, Instant Action and `a2_borg01` with `SCENE=stations`.

## Hearing the advisor without audio
- Start the session with `--stems voice`: `Stems.log` then names every voice line the
  game plays (`Voice: gvofcv20.wav, ...`), in order. It has no timestamps: poll it and
  note the time yourself to measure a cadence.
- Which file is which event: `strings -n 3 "$A2_GAME/events.dat"`, a block per event
  (`InsufficientCrew{`, `Sound <federation> "gvofcv20.wav"`, its `Text`). Federation:
  `gvofcv21` dilithium, `gvofcv20` crew, `gvofcv22` officers, `fedev_no_lat` latinum,
  `fedev_no_metal` metal; `gvofcv31` construction begun, `gvofcv30` complete,
  `gvofcv09` enemy engaged.
- The event also writes its text in the chat area top left ("Insufficient crew to
  build. Colonize a planet."), which OCR finds for a few seconds after.
- **The scene map (`a2_borg01`, `SCENE=stations`) plays no advisor line at all**, in
  stock either (a stalled build there left `Stems.log` empty). Use Instant Action.

## Instant Action as the Federation, at 1280x720
- Main menu -> `step "Click the instant action cube"` -> `wait-text GAME SETUP` ->
  `click-text "Computer AI:Medium"`, then `"Computer AI:Easy"` -> `click 389 122`
  (player 1's Side) -> `click 396 147` (Federation) -> `click-text LAUNCH`, ~12 s.
- Resources default to Normal: 3500 dilithium, ~1250 crew, 600 officers. The camera
  starts on the starbase at about 735,305; click it, `q` opens its build menu, and `q`
  again queues a construction ship (250 dilithium, 150 crew, 2 officers).
- Ten fast `q`s: about eight are paid and the rest refused for crew. Crew comes back
  at a few per second, so a crew refusal is easy and a dilithium one needs the bank
  spent first: wait for the queue to drain (each ship ~40 s, `gvofcv30` in `Stems.log`
  counts them), let crew recover for a minute or so, and queue again.
- The easy AI rarely runs short; its orders are logged in `QOLRules.log` too, without an
  `advisor:` line unless they are refused.

## A stock comparison with the plugin installed
- `session start --no-launch`, set `PayOnQueue=0` in the clone's `QOLRules.ini`, then
  `drive launch -nointro`. **`drive launch` needs the same `A2_PROTON`/`A2_UMU` as
  `session start`**; without them the launcher exits at once ("exited (2) before the
  game started").
- With `PayOnQueue=0` the bank is charged at each start, so ten queued ships build in
  ~7 minutes without running out of crew. Spend the dilithium instead (queue ships until
  the bank is below one ship's cost): the next start stalls and speaks once.
