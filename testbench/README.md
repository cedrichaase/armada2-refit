# Test bench — the game, headless, end to end

`./a2test` runs Armada II on a private, invisible display at any resolution. It drives
the game with a real (virtual) mouse and keyboard, and checks plain-text scenarios
against it. Each run leaves a report with screenshots, logs and every verdict.

    ./a2test run main-menu hud font          # scenarios in testbench/scenarios/
    ./a2test run main-menu --res 16:9,21:9   # override the resolutions
    ./a2test run admirals-log --vnc          # ... and watch it (view-only VNC)
    ./a2test check hud                       # how each step would run, no launch
    ./a2test list

    ./a2test session start --res 16:10       # a live session to drive by hand
    ./a2test drive shot menu                 #   prints the PNG path
    ./a2test drive click-text "Single Player"
    ./a2test drive step 'Wait for the single player screen'
    ./a2test session stop                    #   quits, gathers logs, writes the report

Reports go to `~/.local/share/a2test/results/<run>/`, with `latest` pointing at the
newest. Each holds `index.html` (look at it), `report.md` (read or paste it),
`results.json`, and per case `log.txt`, `shots/` and `logs/`.

**The user's install is never written.** Every case runs on a reflink clone of the
game directory and the prefix, so a run is free to re-tune, break or `a2mod stock` its
copy, and can run while the game is being played.

## How a case runs

| Stage | What | Why this way |
|---|---|---|
| clone | `cp -a --reflink=always` of the game (4.5 GB) and the prefix (558 MB) into `~/.cache/a2test/sessions/<id>/` | btrfs: 0.7 s and no space. Must be the same filesystem as the game |
| prepare | `ARMADA.PRF` line 5 set to the resolution, then `hud/ui-widescreen.py`, `hud/cursor-aspect.py`, `font/ui-font-condense.py` with `--res` and `A2_GAME=<clone>`. For `Mod: stock`, `a2mod stock` on the clone instead | those three layers are built for one resolution; a test at another aspect with 21:9 layers would test the wrong thing |
| display | `sway` with `WLR_BACKENDS=headless`, one output at exactly the resolution, `xwayland force` | the game gets the same Xwayland path it has under Hyprland, and nothing appears on the desktop |
| input | `input/a2input`: a wlr virtual pointer and virtual keyboard on sway's seat, fed through a FIFO | see "Input" |
| game | `umu_run.py` with Heroic's environment, the clone as `WINEPREFIX`/`STEAM_COMPAT_DATA_PATH`; silent: `winepulse.drv=d;winealsa.drv=d` | see "Traps" |
| capture | `grim -o HEADLESS-1` | what the compositor actually shows, scaling included |
| teardown | Proton's `wineserver -k` on the clone, the launcher's process group, sway; logs gathered; clone deleted | |

The launch step also proves isolation: it scans `/proc/<pid>/maps` and `fd` for the
real install's path, and fails the case if the game has it open.

Logs gathered per case: `wine.log` (umu, Proton, Wine; `A2TEST_WINEDEBUG` sets
channels), `Menus.log`, `MSAA.log`, `BinkProxy.log`, `Armada2_d3d9.log` and
`xalia_dxgi.log` (DXVK), `vkBasalt.log`, `sway.log`, `input.log`, `prepare.log`,
`launch-env.txt` (the exact environment and argv), the clone's `ARMADA.PRF`,
`Menus.ini`, `MSAA.ini` and `dxvk.conf`, `exception.txt` if the game crashed, and
anything new in the game's `Logs/`.

## Traps, each measured here

- **A run must be silent, and `PULSE_SINK` does not make it so.** The first version
  pointed `PULSE_SINK` at a private null sink and never checked it. winepulse ignores
  it: it connects every stream to the device it chooses itself (the server's default).
  The user heard the menu music and the Borg cutscene through their speakers during
  runs they could not see. The bench now disables Wine's audio drivers for the game
  (`winepulse.drv=d;winealsa.drv=d`, unless `--audio`), and after every launch it
  watches `pactl list sink-inputs` for a stream from the session's processes. If one
  appears, it stops the game at once and fails the case.

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
  line contains Armada2.exe" latched onto a short-lived one, reported the game dead
  2–8 s in, and left the real game running unwatched past its teardown. Only argv[0]
  identifies it. The Wine errors in those runs' `wine.log` (`failed to update … wine.inf`)
  were a red herring, not the cause.
- **The game ignores WM_CLOSE.** It is still up 20 s after one. `Quit the game`
  therefore goes through its menus: in a mission Esc, "Exit to Windows", "Yes"; on the
  main menu the Exit panel, then "Yes".
- **Menu buttons are the pictures.** The labels ("SINGLE PLAYER") are painted into the
  background and take no clicks; the emblem above them does. Named targets in
  `ui.json` are the pictures, in 800x600 design space. They are mapped to the screen
  with the transform Menus.asi logs (`MoveWindow 800x600 @0,0 -> WxH @X,Y`).
- **OCR needs three passes.** Bright text on space reads best as-is. Grey-on-grey shell
  buttons (the in-mission Options) read only after an adaptive threshold (as-is:
  nothing). Mixed screens read best with the background subtracted. Every pass
  contributes; a phrase is matched within one pass's reading. Frames up to 1200 px
  tall are upscaled 2x first; 1440-tall ones are not, because at 1.5x "Graphics
  Settings" was lost that 1.0x reads at 91%. OCR still misses short grey words
  ("Save"), so a check should not hang on one.
- **OCR boxes cannot measure shape.** The same word's box came back 28 px tall in one
  pass and 36 in another: a phantom 30% stretch. Text is only *located* by OCR in the
  reference shot. Its shape is measured by the template match, like any patch.
- **Dark art is not a pillarbox.** The main menu's plate fades to near-black space at
  21:9, and a "mostly dark column" test called that 267 px of black bar. A bar is a
  column with *no* pixel above 4/255.
- **A campaign mission takes ~100 s to reach its briefing**, most of it an opening
  cinematic that Esc does not skip. For anything that only needs *a map*, launch
  straight into one (`Launch: -nointro a2_borg01`): a bare argument is a mission name
  (see `menus/run-wine.sh`), and the HUD is up in ~30 s.
- **A direct map launch deals a random faction.** One run drew the Federation,
  Cardassian and Romulan HUDs across its cases, and their panels have different shapes
  (the Federation's minimap panel is about half as wide as the Romulan's). A comparison
  across resolutions then measures different sprites. The user caught it from the
  screenshots. Anything compared across resolutions goes in through the Borg campaign
  (`Include "_enter-borg-mission"`), which is always Borg. The Admiral's Log and the
  in-mission menu are shell UI and faction-independent, so they keep the fast launch.
- **Fog of war is flat grey.** A map opens scrolled to its top-left corner, so its
  unexplored area fills part of the 3D view with flat grey, and so does the minimap.
  The flat-area check will flag it. Don't use that check on a map.
- **The briefing's OK button did not take a click** in one session, though Esc did.
  Not investigated.

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

## Judged and agent steps

Steps that name no measurable check go to `claude -p` (`bench/judge.py`). A judged
assertion reads the screenshot and the same-named reference shot, and returns pass,
fail or inconclusive, with its reasoning and cost in the report. An agent step drives
the live session through `a2test drive` until it reports done or failed; every command
it runs is logged like any other. `--no-claude` turns judged steps into REVIEW, left
for a person, and makes agent steps fail. `A2TEST_MODEL` picks the model. The judge
prompt carries a short brief of what is *supposed* to look odd (pillarbox areas, fog of
war, the 21:9-tuned font). Add to it when it flags something known.

## Watching

`--vnc` (per case) or `session start --vnc` starts a view-only `wayvnc` on the headless
output: `vncviewer localhost:5910` (the port is printed; it counts up if busy). `--record`
writes each case to `run.mp4` with `wf-recorder`. Both are optional and need `wayvnc`,
`wf-recorder` and a viewer (`tigervnc`), none of which is installed yet. Without them
the bench warns and carries on.

## Requirements

`sway`, `grim`, `tesseract` (+ `eng`), `python-opencv` (numpy with it), `wayland-scanner`
and `cc` (for `a2input`), `pactl`, Heroic's Proton-CachyOS and umu, and `claude` for
judged and agent steps. Optional: `wayvnc`, `wf-recorder`, `tigervnc`. The texture
pipeline's "no numpy" rule is about what that pipeline may assume. The bench is
separate and needs opencv.
