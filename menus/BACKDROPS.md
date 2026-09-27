# Menu backdrop plates

How the widescreen plates behind the main menu and campaign selection are built.
`Menus.asi` composites them at run time; this is the half that starts from the game's
own art, so only the recipes (`backdrops/<name>.conf`) are committed and every layer
and plate is built into `$A2_DATA/backdrops/<name>/`. The run-time side, including
`N.soften=`, is in `README.md`, "Backdrops". Changes to a plate go in
`CHANGELOG.md`, since the plates are part of the menus layer that `a2mod` switches.

**Which screens.** Only screens with open art are outpainted, and they are listed
in `backdrops/*.conf`:

- **`mainbkgr`:** nebula to the edges. Outpainted, and the joins are step-free
  (column means 39→36→35 and 44→45 across them).
- **`singleplay`** (campaign selection): a star-chart grid over nebula, inside a
  drawn black frame that is hard-edged at x 28 and x 771. Outpainting the whole
  screen continued the frame and then started a different sky beyond it. So
  `field=32,0,768,600` hands the outpaint the open field alone. The plate replaces
  the frame too: Menus.asi shows the plate wherever the shell draws stock
  background, and the frame is stock background. The joins are step-free (column
  means 43→45→48 and 83→86→81).
  Of four expands (about $0.02 each): seed 1 invented a lens-flare sun, and seed 3
  was clean but its grid stopped at the old frame. Seed 2 is the chosen one: its
  grid continues outwards. It also wrote a line of fake glyphs into empty sky at
  the bottom left, which `clone=` covers with the patch of sky beneath it. The
  others are kept in `ai/` as `expanded-*.png`.
  **A field edge must not cut through UI art.** The Tutorials panel's left bar
  starts at x 30, and `field=` starts at 32, so the model was handed a bar cut
  open at its edge. It extended the bar 6.5 design px leftwards into what became
  plate-only area. Idle, that looks like a slightly wider bar; hovered, the glow
  video redraws the bar at 30, and the extension showed beside it as a second
  sliver (seen in game). The field is kept, since changing it would mean paying
  for a new outpaint and losing seed 2. A second `clone=` with a 2px feather
  covers the extension with the sky above-left of it, up to exactly stock's bar
  edge (raw-outpaint x 893). That left the bar itself: its first 27 design px
  were the outpaint's redrawn copy, blended into the upscale's across the
  feather. Two slightly misaligned drawings of one edge gave wobbly, chamfered
  corners when idle, which showed when toggling the hover (seen in game).
  `keep=29,16,60,212` makes the plate take the upscaled stock over that bar, so
  its corners are stock's.

**Building a plate:** `menus/backdrop.sh <name>` uses the same recipe as
`models/loading-panel.sh`, and keeps its paid layers in `$A2_DATA/backdrops/<name>/ai/` (the recipe,
`menus/backdrops/<name>.conf`, is all that is committed).

1. Upscale the stock 4x with `bria/increase-resolution`. pruna hung in "running"
   for 10 minutes on the day this was built.
2. Blend 35% over Lanczos.
3. Outpaint at 2048x1536 → 3680x1536 with `bria/expand`, which caps a canvas at
   5000px. With `field=`, the model sees only that rectangle of the art.
4. Apply any `clone=` patches, then feather the full 4x centre back in over 64px,
   inside the field.
5. Resize to exactly 3450x1440, centre 1920 at x 765. The plate is authored at
   the screen height, so at 1440 it goes on 1:1.

`--reblend` re-derives it offline. `menus/install-plates.sh` (run by `./install`) copies each
built plate to `Menus/`, and `a2mod` moves them with the rest of the menus layer.
