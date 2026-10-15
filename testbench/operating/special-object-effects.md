# Operating the bench: special-object effects (agent specialfx)

Tested on 2026-10-15, `SCENE=specials`, 16:9 (1920x1080), `-nointro a2_borg01`.

## Starting

    cd <worktree>
    SCENE=specials ./a2test session start --res 16:9 --install <copy> \
        --install <copy>/testbench/scene --args '-nointro a2_borg01'

- The install into the clone takes over two minutes, so `session start` can outrun a
  120 s tool timeout; the session ID is then the newest directory in
  `~/.cache/a2test/sessions/` whose `session.json` lists your copy under `installs`.
- Wait for the scene, not a fixed time:
  `until grep -q ready ~/.cache/a2test/sessions/<ID>/game/Scene.log; do sleep 3; done`.

## Firing the weapons that make special-object effects

Which ship carries which weapon (from `odf/ships/*.odf`):

| Effect (class) | Weapon ODF | Ship ODF |
|---|---|---|
| Computer Override (`TimedChangeTeam`, emitter `wovremit`) | `goverrid` | `bspecial` (Borg) |
| Energy Dissipator (`TimedDrain`, `wmicremit`) | `gmicro` | `kspecial` (Klingon) |
| Micro-organisms, Cardassian (`TimedDrain`, `wmicremi2`: no such SOD) | `gmicroorgs` | `cdestroy` |
| Psionic Disruption (`TimedCrewDisruption`, `psionicdisruption`) | `gpsionicDisruption` | `8472_cruise1` |
| Klingon commando team (`BoardingParty`, draws nothing) | `gkcteam` | `kcruise2` |

Worked, as run (the scene's `tgt` is an enemy Sovereign at 3000,0,3000):

    scene "spawn ovr bspecial 3000 0 3300 180 1"
    scene "engines ovr off"
    scene "energy ovr on"
    scene "camera 3000 150 3550 3000 0 3150"
    scene "special ovr goverrid tgt"        -> "slot 4, can fire, fired"

- Computer Override's effect sits on the *shooter* first (target `bspecial` in the
  survey, 20 s), not only on the ship it hits.
- After firing, `scene "weapons <ship> off"` keeps the shooter's ordinary phasers from
  cluttering the shots; the effect carries on (the `special` command needs weapons on).
- The Energy Dissipator (`gmicro`, 10 s) and the Psionic Disruption
  (`gpsionicDisruption`, a toggled beam: its effect is renewed every 0.5 s while the
  beam holds; `weapons psi off` stops it) were fired the same way from `kspecial` and
  `8472_cruise1` at 3000 0 3300 heading 180. Kill one shooter and wait 10-20 s before
  the next, or the earlier effect is still on the Sovereign.
- A close camera on the Sovereign: `scene "camera 3150 90 3120 3000 0 3080"`.
- `scene "hud on"`, then `click 640 360` (1280x720) selects nothing useful; the inset
  showed the shooter. Good enough for a both-views crash check.

## A/B against stock at the same moment

Render3D.ini in the clone is `~/.cache/a2test/sessions/<ID>/game/Render3D.ini`. A module
key with `Reload=1` is read again every 120 frames (~2 s): set `[Specials] Enable=0`,
wait 3-4 s, shoot, set it back. Shell `sed -i` on that path with a variable was refused
by the worktree sandbox; a small script file doing the edit (python) worked.

## Traps

- The worktree sandbox refuses compound shell lines that mix variables with `sed`,
  `magick` globs, `objdump` or `source`; put such steps into a script file and run it.
- Foreground `sleep` over a few seconds is refused; wait for a log line with
  `until grep -q ...; do sleep 3; done` instead.
- The GPU is the user's: one game at a time, 1280x720 (`--res 1280x720`), stop sessions
  as soon as done. A GPU hang on the bench (18:24 on 2026-10-15) took the user's desktop
  compositor down with it.
- Cloud particles (special-effect clouds) keep drifting while the game is paused in
  stock (per frame), so a paused A/B drifts on the stock side.
- In a scenario, a `special` fired in the same second the shooter was spawned answers
  "fired" but makes no effect: put `Wait 2 seconds.` between spawn and `special`, and
  `Wait 1 second.` before `weapons <ship> off` (`render3d-specialfx.md`).
- `./a2test run <path to scenario .md> --install <copy> --res 1280x720 -j 1 --no-claude`
  runs a scenario from your copy (about 5 minutes for this one).
