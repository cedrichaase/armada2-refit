# The promo — plan for review

The board is `storyboard.md` beside this file: 8 acts, 42 shots, `./a2film check` clean,
nothing filmed yet. This file is what has to be decided and built before filming. It
goes away (or shrinks to notes) once the board is approved.

## The shape

| Act | What it argues | Shots | Length | Bed |
|---|---|---|---|---|
| Cold open | stock as it shipped, then refit, then the title | o01–o05 | 0:15 | Borg |
| Hulls and light | the same models, lit like a modern engine | h01–h05 | 0:38 | Federation |
| Battle | effects drawn as light and gas, not flipbooks | b01–b07 | 0:44 | Klingon |
| Space | planets, moons, rocks, nebulae, sky from recipes | s01–s06 | 0:44 | Federation (MP) |
| Widescreen | menus and HUD laid out for your screen | u01–u02 | 0:13 | Cardassian |
| Gameplay | plays better, still plays with stock players | g00–g06 | 0:57 | Cardassian |
| Installing | six steps through the GUI installer, ending in the game | i00–i08 | 0:55 | Federation, quiet |
| End | the address | z01 | 0:06 | Federation (MP) |

About 4:30 by shot lengths. Visuals carry the first three minutes; gameplay comes
after; the install guide closes, as asked.

## Decided (2026-10-14)

- **D1/D2, superseded 2026-10-17:** the user wants the recording done with the HD
  textures (`Variant refit: --install . --install testbench/scene`, `$A2_DATA`'s assets),
  and the descriptions to say they aren't redistributed, for copyright. Before that:
  **What "refit" is:** the game as a release will install it once the new effects
  are in the installer: every code layer, no remastered textures, no menu backdrop
  plates (`Variant refit: --assets none ...`). **Published once the installer installs
  the new effects.** The menu comparison is out (black sides without plates); the guide
  still ends on the refit main menu, black sides and all, unless that reads as broken.
- **D3. Windows and macOS:** a card now; the Tk window (`A2_INSTALLER_TK=1`) as a desktop
  shot once Tk is installed on this machine (`pacman -S tk`).
- **D4. Online play:** filmed as two games side by side; g06 is a card until T4 exists.
- **D5. Length:** cut to about 3:30 after the first rough cut (the candidates are b03,
  b07, s02 or s03, h04, merging i07 and i08, a tighter gameplay act).
- **D6. Music:** the game's own score; the October video drew no claims.
- **D7. No 800x600 shots.** The cold open starts on stock's Enterprise at full frame
  (h01's stock take), then the same frame in refit.
- **D8. Thumbnail: concept A**, the Enterprise-E split "2001 / NOW", *with* the
  remastered textures, the NCC-1701-E registry legible in the refit half, a class M
  planet and/or the Mutara nebula behind (`thumbnail.ini`, `thumbnail.sh`; output in
  `$A2_DATA/films/promo/thumbnail/final/`).
- **The nebula shot** (the user's idea): a fleet inside a dense purple Mutara nebula,
  the camera following it towards the Key light, at an angle where the hulls' shadows
  in the gas read clearly (`nebula-fleet.ini`); it replaces or joins s05.

## Open after probe round 1 (2026-10-15)

- **The HUD skin differs between takes** (purple, green, gold): the player's race is not
  pinned on the bench, so u02's stock/refit dissolve compares two skins. Scene.asi's
  `Race=` key (or `SCENE_RACE=<race>` at install) now pins it (testbench 3.32.0); the
  promo's scene files and `films/film/shoot.py` do not set it yet, and which race the
  takes use is the user's call.
- **g06 shows the machine's LAN address** in the lobby chat ("Local IP Address: ...").
  Blur it in the cut, or film with the address hidden, before publishing.
- **Bugs found while filming, for their layers** (not hidden by framing):
  - lighting: a faded hull is drawn behind a planet that is behind it (stock too); in
    refit a hull built with the camera close can stay faded until the camera goes far;
    refit's fully cloaked glass is nearly invisible over a planet (b06).
  - surfaces: vertical streaks on the class M surface at 4K.
  - nebulae: the hulls' shadows in the gas are faint, and vanish when a hull is near the
    camera (s05b); refit's Mutara reads dimmer than stock's billboards (s05).
  - gui-installer: the version line truncates ("pre-release, not yet seen i…", i04).
  - textures: the Enterprise's registry reads "NCC-1701E", no hyphen.

## Tooling

| | What | State |
|---|---|---|
| T1 | `Kind: desktop`: the GUI installer on a bare bench desktop, installing into a stock clone (`films/README.md`, "Desktop shots") | done |
| T2 | pointer coordinates in the film's `Pointer` space (1920x1080), scaled to each take | done |
| T3 | a per-shot resolution | dropped (D7) |
| T4 | two games in one shot (`Players:`, `Prepare:`, `typelog`) | done |
| T5 | refit as a release installs it | done: `a2test session start --assets none` |
| T6 | a still from a take, for the thumbnail | not needed: the thumbnail is shot as stills |

## Filming

1. Decisions made and T1, T2 and T5 built (2026-10-14).
2. **Probe round 1:** every game shot at 1280x720, four games at once. That is about 50
   takes, roughly 25 minutes (the whats-new board ran 36 takes in 18.5). The desktop
   shots follow one at a time, in a few minutes. Then the contact sheet, the animatic
   and the shots I'm unsure of, for you.
3. **We review together.** Your notes go into the board as edits. Only the changed shots
   are re-probed.
4. **Finals:** the shots you approve, at 3840x2160, one at a time: about 50 takes at ~3
   minutes, so 2–3 hours unattended. Then `animatic --tier final` and the manifest for
   Filmcraft.
5. **Thumbnail:** concept A from 4K stills of its own scene (`thumbnail.ini`, `thumbnail.sh`).

## Reused, not re-invented

- Hull, colony and QOL camera moves from the October showcase's shot lists
  (`$A2_DATA/promo/2026-10-showcase/tools/`): h01–h03, h05, g01–g04.
- The Battle and Space shots from `films/whats-new`, trimmed to one beat per effect.
  The two boards film separately. A shared take would save a few minutes of probing but
  tie the boards together.
