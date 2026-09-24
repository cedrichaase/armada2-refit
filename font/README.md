# Font — the in-game text at widescreen

`font/ui-font-condense.py` condenses the in-game bitmap font so it is not drawn 1.9x too
wide at 21:9. It is its own `a2mod` layer (`font`), separate from the HUD layout in
`hud/`, because it has its own backups (`.a2font-backup`) and must never be reverted by
`a2tex`. It follows from the canvas change described in `hud/README.md`.

## The font does not ride on the canvas either, and that is the canvas fix's bill

Re-declaring the canvas un-stretches every panel, icon and rect. **It does not touch the
text**, which comes out huge and visibly wide — and that is not a leftover of the old
bug, it is a new mismatch the fix created.

The in-game font is a bitmap sprite, not a system font. `Armada2.exe` builds the name

    Font%s%d.spr        %s = "Final4_", %d = a point size

and `Sprites/` ships eight of them — 10, 12, 13, 15, 16, 19, 20, 24. Each is an atlas:
`Textures/RGB/FontFinal4_<size><page>.tga` is **solid white RGB with the glyphs entirely
in alpha** (measured: mean R=G=B=255, mean A=35.5), and the `.spr` carries a per-glyph
`(u,v)` offset table and a per-glyph advance-width table. The `*Color` keys in
`gui_glob16x12.cfg` tint the white.

Those atlases are authored for a **1280x1024** tier, and the font path scales glyph quads
by back-buffer / 1280x1024 — **2.6875x across against 1.40625x down** at 3440x1440,
whatever `screenWidth` says. Measured against the advance tables, off a 3440x1440
screenshot:

| element | font | predicted | measured |
|---|---|---|---|
| `OBJECTIVES:` (header) | 24 | 620.9 x 32.3 px | 619.5 x 32.7 |
| `BRIEFING SUMMARY:` | 24 | 962.2 px wide | 960.2 |
| `The Borg Queen and a ...` | 16 | 1355 px wide | 1368 |
| `4000` (resource bar) | 16 | 137.1 x 21.1 px | 135.9 x 22.4 |

So every glyph is drawn 2.6875/1.40625 = **1.911x too wide**, and against a canvas that
now scales at 1.20 it is also 2.24x too wide for the panel around it.

**The vertical axis is already correct and is not touched.** 1.40625 against the canvas's
1.20 is a ratio of 1.171875, and that ratio is stock: at 4:3 the font scales by H/1024
against a canvas of H/1200, the same 1.171875. The game has always drawn text 17% taller
than its layout coordinates imply.

**Why 1280x1024 and not 1600x1200.** A rival fit — glyphs scaled by back-buffer /
1600x1200, the stock canvas — matches the body text and the line pitch just as well,
because `sz16 x 2.6875` and `sz20 x 2.15` differ by 1.3% and nothing measurable here
separates them. Word-wrap does not separate them either; both reproduce all five
paragraph breaks. **The headers do.** At 2.15x across, `OBJECTIVES:` at 619.5 screen px
needs an atlas 288 texels wide with a 27-texel cap — a ~30pt font. `FontFinal4_30` does
not exist. At 2.6875x it is `sz24` (231 x 23) to 0.2%, the largest atlas shipped, which
is what the largest tier should reach for.

**`font/ui-font-condense.py`** squeezes each glyph's art and its advance width by
`(H/1024)/(W/1280)` = `1.25 * H / W` — 0.5233 here — so the engine's own 1.911x stretch
lands the glyph back at its authored proportions. Cell height, atlas size, page layout,
row assignment, frame counts and the white RGB plane are untouched; only the alpha plane
is rebuilt and only the `u`/width numbers move. 1792 glyphs across 8 sizes.

    font/ui-font-condense.py --preview /tmp/p.png   # stock vs condensed, at game scale
    font/ui-font-condense.py --dry-run
    font/ui-font-condense.py
    font/ui-font-condense.py --check                # do .spr and .tga still agree?
    font/ui-font-condense.py --revert

**The cost is horizontal sampling, and it is unavoidable from data alone.** The
destination quad is `texels x scale`, so the only lever on width is texels: a glyph that
was 28 texels wide is now 15, and `@tmaterial=font #No filtering, ever.` means the engine
point-samples it back up 2.6875x. Text is correctly proportioned and horizontally
chunkier.

Lanczos held the stems best of the three filters tried (`--filter`; Triangle is softer,
Box loses sub-texel stem placement).

**A crisper alternative was built, measured, and rejected in game — `--method runs`.**
Keep it and keep this note: every static metric favoured it and it still looked worse on
screen, so the next person who finds the text soft can learn that the experiment has
already been run.

The argument was that horizontal antialiasing buys nothing here. The axes magnify very
differently — 2.6875x across against 1.40625x down — so downward a part-covered texel
spans about a screen pixel and reads as a real soft edge, while across it is painted as a
flat 2.7px block that softens nothing and merely puts a grey slab where a stroke edge
should be. Stock's stems are *one texel* wide, so a 0.5233 resize asks for half a texel
and gets a slab every time: on `FontFinal4_24a` the fully-opaque texel count goes
6000 → 1572, with hundreds more picking up an alpha 1–4 ringing halo. Thresholding is no
answer either — at 45% it erases `!` `"` `I` `i` `l` and at 50% it erases 31 glyphs, all
of them the ones already one texel wide. So `runs` works a scanline at a time: map the
stock line's ink runs by the factor, give every run **at least one texel**, keep stock's
gaps, and fill each run flat at that line's own peak alpha — crisp across, stock's shading
kept down, no dropouts, and an invariant stroke count per line. Widths are floored rather
than rounded because rounding gives `1` a foot twice its stem width and `1187` renders as
`[187`.

| method | ink/expected | erased | thinned | fattened |
|---|---|---|---|---|
| **Lanczos resample** — ships | **1.028** | **0** | **0** | **0** |
| Box + threshold 35% | 1.224 | 0 | 2 | 227 |
| Box + threshold 45% | 1.074 | 6 | 29 | 64 |
| Box + threshold 50% | 0.904 | 31 | 120 | 4 |
| runs, width rounded | 1.007 | 0 | 1 | 25 |
| runs, width floored — rejected | 0.900 | 0 | 9 | 16 |

`runs` also takes `24a` from 1572 opaque texels back to 3948 against stock's 6000, and the
alpha plane back to 14 discrete levels from 256.

**And none of that settled it.** Judged in the actual game the crisp variant looked worse
than the soft one. The likely reason is that the reasoning models the engine as a bare
point-sampled blit, while the real text is tinted, drawn over lit panel art and read at a
normal viewing distance — conditions under which a grey slab reads as a soft edge after
all and hard 2.7px blocks read as jagged. The offline renders reproduced the sampling but
not the context.

**The lesson is the part worth keeping:** ink ratio, opaque-texel count, dropout count and
alpha-level count all favoured the variant that lost. They measure weight and structure,
not legibility. Do not change the font's appearance on the strength of that table — put it
in the game and look at it.

Per-glyph integer rounding costs a little accuracy: measured over the whole charset the
realised factor is 0.514–0.558 against the 0.5233 target, biased slightly narrow. The two
sizes actually drawn at this resolution land at 0.5235 (24) and 0.5281 (16). `sz10` is the
worst at 0.5584 because its glyphs are 3–8 texels wide, and it belongs to the 640x480
tier, so it is not drawn here.

**Confirmed in game.** `OBJECTIVES:` measures 323.5 screen px wide against 325.2
predicted — 0.5% — with the cap height unchanged at 32.7 px, exactly as intended, and the
glyph aspect back to 9.89 against the atlas's authored 10.04. The briefing paragraph
reflowed from five lines to three, which is the same prediction seen from the other side.

One thing the model does not capture, and it is **stock behaviour, not a condensing
artefact**: measured line widths run a few percent over the sum of the advance widths,
because **the engine rounds each glyph's advance up to a whole screen pixel**. The drift
is 0.36 screen px per glyph condensed against 0.28 stock — the same effect at the same
per-glyph rate, just accumulated over the longer lines a condensed font fits. Do not
"correct" it in the `.spr`; the widths are right.

**The backups are `.a2font-backup`, not `.a2neb-backup`, deliberately.** `a2tex revert
all` restores every `Textures/RGB/*.a2neb-backup`; if the atlases went back to stock while
the condensed `.spr` files stayed, every `u` and width would point into the wrong place in
a wider glyph — garbled text, out of the command that is supposed to be the safe way out.
A distinct suffix keeps a2tex out of it, the same division ui-widescreen.py already has
with `misc/`. `--check` is what catches a mismatch if one ever happens.
