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

## Decisions for you

Each one has my recommendation first.

**D1. What the film shows against what the installer installs.** The film ends in "install
it with this", so a viewer will expect what they saw. Today a release (and so the GUI
installer) carries lighting, sky, models, HUD, menus, QOL, grid, online, MSAA, bloom and
the renderer. It does **not** carry:

| Not in a release | Where the film shows it |
|---|---|
| explosions, shields, venting, repair, specials (not yet) | all of Battle except the cloak and impacts |
| surfaces, nebulae (not yet) | most of Space |
| fog (not yet) | not in this board |
| textures (never: built from the player's files with `./a2tex`) | the hulls in h01–h05 |
| the menu backdrop plates (`$A2_DATA/backdrops`, not packaged) | u01 and i06: a release's refit menus have black sides |

- **(a) Recommended:** film and cut now, and publish once those layers are in a release.
  The final 4K takes wait for that release, so the footage is what ships.
- **(b)** Publish sooner, with a small "in development" tag on the shots of unshipped
  layers.
- **(c)** Show only what ships today. That loses most of Battle and Space, which is
  most of the film's case.

**D2. Textures and menu plates in the "refit" takes.** Even after (a), textures and
plates are not something the installer gives.

- **Recommended:** film "refit" as a release installs it, with no remastered textures
  and no plates (a `release` variant, T5). Lighting carries the hull shots on its own,
  and the guide's promise holds exactly.
- **Alternative:** keep today's full refit and say on i08 that textures are separate.
  The draft does this now. It is weaker, because the hull detail in h01 is the first
  thing a viewer notices.
- **Or:** package the menu plates in releases, which is a change for `publish/`, not
  the film.

**D3. Windows and macOS in the guide.**
- **Recommended:** Linux in full, then one step showing the Tk window (the same screen,
  filmed on Linux with `A2_INSTALLER_TK=1`) in a corner, with the SmartScreen and
  Gatekeeper notes.
- **Alternative:** cards only, as now.

**D4. Online play (g06).**
- **Recommended:** film it, as two games side by side: the host's join code, the joiner
  typing it, both in the lobby. "Online works under Wine and Proton" is the one gameplay
  claim no one else can make. This needs T4.
- **Alternative:** a card.

**D5. Length.**
- **Recommended:** about 3:30. Cut after the first animatic, not before. Likely cuts:
  - b03 impacts (it reads as part of the shields beat): −5 s
  - b07 glass: −6 s
  - s03 moon or s02 six classes: −6 s
  - h04 torpedoes, since h03 already makes the weapons-light point: −6 s
  - i07 and i08 merged into one card: −6 s
  - the gameplay act tightened (g02 to 7 s, g03 to 8 s): −4 s
  - the install steps 5 s each, i05 excepted: −5 s

  That is about −40 s, landing near 3:40–3:50.
- **Alternative:** keep everything. A promo that long only works if the guide is
  chaptered on YouTube.

**D6. Music.** The beds are the game's own score, as in October. The video is yours to
publish. If the October video drew a Content ID claim, say so, and the beds should
change before the cut.

**D7. The opening shot (o01).**
- **Recommended:** the stock main menu at 800x600, small in the middle of the black 16:9
  frame (needs T3), then the hard cut to the refit Enterprise.
- **Alternative:** stock at the film's resolution. That is weaker: the stretch reads as a
  bug, not as "2001".

**D8. The thumbnail.** Four mockups from existing footage are in
`$A2_DATA/films/promo/thumbnail/` (`review.png` shows them full size and at 168x94;
`notes.md` has the source frames and the camera for each final).
- **Recommended: A**, one Enterprise-E split diagonally into "2001" and "NOW". It is the
  film's argument in one picture, and it still reads small. Its final frame should be a
  new 4K still, stock and refit (T6), from a camera where stock looks clearly older than
  it does in the mockup.
- **D**, the refit Borg cube with "25 YEARS LATER", also reads small, without its small
  captions.
- **B**, the hero Enterprise, loses its word at small size.
- **C**, the explosion: refit's flat yellow shockwave band looks like stock's old disc.
  That is worth a look as an explosions issue, not only a thumbnail one.

## Tooling before filming

Each item is a change to `films/` (format, `shoot.py`, `edit.py`) with its README and
changelog entry, tested on `films/selftest`. In the order the film needs them:

| | What | For | Size |
|---|---|---|---|
| T2 | Pointer coordinates in a fixed 1920x1080 reference space, scaled to the take. Today they are in the take's own pixels, so a 720p probe and a 4K final can't share an action block | g01–g05 | small |
| T1 | `Kind: desktop`: a bare headless sway (the bench's own `start_display`, without the game) with output scale 2 at 4K, so the window sits at a desktop's size, and a wallpaper still from a take. A scratch `HOME` with a Heroic config pointing at a **bench clone of the stock game**, so the window finds it, really installs into it and Launch game has something to launch. The installer from the checkout, or a release's AppImage. Same action commands, same recording | i03–i06, i07 pip | medium |
| T3 | A per-shot `Res:`, pillarboxed or letterboxed in the cut | o01 (800x600); optionally a 21:9 HUD shot | small |
| T4 | `Players: 2`: two sessions and a local online server for one shot, `split` in the cut, steps prefixed `host:` and `joiner:` as the bench's scenarios do | g06 | medium |
| T5 | A `release` variant: the clone gets a packaged release (`publish/package.sh`, then its `install.sh`), not the checkout. Only if D2 goes that way | every "refit" take | small |
| T6 | `./a2film still <shot> <variant> <t>`: a 4K frame from a final take, for the thumbnail | thumbnail | small |

The rest is in place: mission shots for the menus, scenes for every visual, the October
QOL scene (`qol.ini`, copied here), cards, footage teasers and music beds.

## Filming

1. **When you've answered D1–D7:** I build T2, then T1, T3, T4 and T5 as decided, and
   edit the board.
2. **Probe round 1:** every game shot at 1280x720, four games at once. That is about 50
   takes, roughly 25 minutes (the whats-new board ran 36 takes in 18.5). The desktop
   shots follow one at a time, in a few minutes. Then the contact sheet, the animatic
   and the shots I'm unsure of, for you.
3. **We review together.** Your notes go into the board as edits. Only the changed shots
   are re-probed.
4. **Finals:** the shots you approve, at 3840x2160, one at a time: about 50 takes at ~3
   minutes, so 2–3 hours unattended. Then `animatic --tier final` and the manifest for
   Filmcraft.
5. **Thumbnail:** the concept you pick, made from a final 4K frame (T6).

## Reused, not re-invented

- Hull, colony and QOL camera moves from the October showcase's shot lists
  (`$A2_DATA/promo/2026-10-showcase/tools/`): h01–h03, h05, g01–g04.
- The Battle and Space shots from `films/whats-new`, trimmed to one beat per effect.
  The two boards film separately. A shared take would save a few minutes of probing but
  tie the boards together.
