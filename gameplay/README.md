# Map scrolling

The stock scroll feel is slow because **two of the three knobs ship at or near their
minimum**, and the one that matters most is a per-user setting the game already exposes
in its own options screen. Derived from `Armada2.exe` + `armada2.map`, not guessed:
`cOverViewImp::mCheckCameraPan` (`0x5244f0`) and `ParabolicCamera::Pan` (`0x4dfa50`).

The whole chain, per frame:

    pan = edgeOrKeyFactor
        * UserProfile.<mouse|keyboard>_scroll_speed     // ARMADA.PRF, set by the slider
        * SCROLL_ACCELERATION                           // RTS_CFG.h, 3
        * ramp                                          // INITIAL_SCROLL_SPEED -> MAX_SCROLL_SPEED
        * dt
        * SCROLL_COEFFICIENT / (OVERVIEW_INIT_HEIGHT + |OVERVIEW_INIT_HEIGHT - camHeight|)

`ramp` starts at `INITIAL_SCROLL_SPEED`, grows by `dt` each frame, clamps at
`MAX_SCROLL_SPEED` and **resets the moment you stop**.

**The ramp gains exactly +1.0 per second, and that rate is hardcoded** — `g_scrollSpeed
+= dt`, with no config name over it. It is the constraint that shapes every other choice
here, because it fixes how long the ramp takes to traverse whatever range you give it.
Stock's 1 → 2 therefore takes one second to double and stops there.

**Do not set `INITIAL_SCROLL_SPEED = MAX_SCROLL_SPEED`.** It looks like a clean way to
kill the wind-up on short nudges, and it is — the clamp fires on frame one. But it also
flattens the ramp to a constant, which **removes acceleration entirely**, and that is
worse: crossing the map is exactly the case that wants to speed up as you hold. Tried
here, and it read in game as "very linear — reasonable around the base, cumbersome
between areas, holding at the edge doesn't do anything". Both halves of the ramp matter,
and they want *separating*, not collapsing.

The shape that works is a **low floor and a high ceiling**, with the base speed made up
by `SCROLL_COEFFICIENT` instead:

- `INITIAL_SCROLL_SPEED` sets what a quick corrective nudge gets. Keep it at 1.
- `MAX_SCROLL_SPEED` sets what a held scroll builds to. 8 means a 8x spread, reached
  after 7 seconds of holding — but 2x at one second and 3x at two, which is where the
  feel actually lives. Much above 8 and the top of the range is unreachable in practice.
- `SCROLL_COEFFICIENT` scales both ends together, so it is the knob for "everything is
  too slow", and the one to raise when lowering the floor would otherwise cost you.

## Every navigation path funnels through Pan

Worth knowing before tuning anything, because it is not obvious from the names:
`ParabolicCamera::Scroll` (`0x4dfbb0`) is nine instructions that forward straight to
`Pan` via vtable slot `+0x38`. So edge-scroll (slot `+0x38` directly), the arrow keys and
**right-mouse-drag** (`cOverViewImp::mMouseRightDrag`, slot `+0x3c`) all end up in the
same `Pan`, and **`SCROLL_COEFFICIENT` is the one multiplier that scales all three**. It
is the knob to reach for when the per-device sliders are not enough — and the only one
that also touches right-drag, which bypasses both profile speeds entirely.

`FASTSCROLL_COEFFICIENT` (0.005) is right-drag's own scale factor, applied before `Pan`.
Left alone here: `SCROLL_COEFFICIENT` already lifts that path, and right-drag is direct
manipulation where a 1:1 feel against the cursor is the point.

## The two speeds live in ARMADA.PRF, not in any config file

`GameConfiguration::LoadProfile` (`0x53dc00`) reads **line 2 of `ARMADA.PRF`** as eleven
whitespace-separated values. Fields 3 and 4 are the scroll speeds:

    2 3 1 2 0 0 1 0.28 1 1 0
        ^ ^
        | keyboard_scroll_speed
        mouse_scroll_speed

(Confirmed three ways: fields 1/2/7/9/10 match the constructor's defaults at `0x53da80`,
field 4 matches `KEYBOARD_SCROLL_RATE = 2.0` from `RTS_CFG.h`, and field 8 matches
`cfgMOUSE_HOLD_LEVEL = 0.28` from the same file.)

Both are also **sliders in the game's own Options → Game Settings screen**, and the
slider is the supported way to tune them. The mappings, from `GameSettings.obj`:

| Setting | Slider range | Stored value | Stock | Headroom |
|---|---|---|---|---|
| `mouse_scroll_speed` | 1–50 | `slider / 10` | 1.0 (slider 10) | up to **5.0**, a 5x lift |
| `keyboard_scroll_speed` | 1–20 | `slider + 1` | 2.0 (**slider 1 — the minimum**) | up to **21.0**, a 10.5x lift |

So no binary patching and no config surgery is needed for speed: the keyboard slider
ships pinned to its lowest setting and the mouse slider to a fifth of its range.

`LoadProfile` clamps only the difficulty field (0–2); the two scroll floats are read
unclamped, so `ARMADA.PRF` can hold values above the slider maxima — but opening the
options screen rewrites them back into range, so don't rely on it.

**`MOUSE_SCROLL_RATE` is not settable from `RTS_CFG.h`.** The name is absent from the
EXE's lookup table (`KEYBOARD_SCROLL_RATE` is present), so the global keeps its compiled
1.0 and is only ever used as the seed for a *fresh* profile. Once `ARMADA.PRF` exists,
the profile wins for both. Editing `KEYBOARD_SCROLL_RATE` in `RTS_CFG.h` likewise does
nothing to an existing profile.

## SCROLL_BORDER_WIDTH is the other half of "unwieldy"

`RTS_CFG.h` ships `SCROLL_BORDER_WIDTH = 2` — the mouse must be within **2 pixels** of a
screen edge for edge-scrolling to engage at all, and the factor ramps linearly across
that band. The EXE's own compiled-in default is **20**, so stock's 2 is the config
file overriding the engine down to a hair's width. At 3440x1440 that is the difference
between a usable edge and one you have to hunt for.

Raised to 20 here.

## What is set

| Where | Key | Stock | Now | Effect |
|---|---|---|---|---|
| `ARMADA.PRF` field 3 | `mouse_scroll_speed` | 1 | **5** (slider 50/50) | 5x, edge-scroll |
| `ARMADA.PRF` field 4 | `keyboard_scroll_speed` | 2 | **10** (slider 9/20) | 5x, arrow keys |
| `RTS_CFG.h:40` | `SCROLL_BORDER_WIDTH` | 2 | **20** | usable edge band |
| `RTS_CFG.h:41` | `SCROLL_COEFFICIENT` | 90000 | **300000** | 3.3x, *every* path |
| `RTS_CFG.h:42` | `FASTSCROLL_COEFFICIENT` | 0.005 | **0.015** | 3x, right-drag only |
| `RTS_CFG.h:49` | `MAX_SCROLL_SPEED` | 2.0 | **8.0** | held scroll ramps 8x, not 2x |
| `RTS_CFG.h:50` | `INITIAL_SCROLL_SPEED` | 1 | **1.0** | unchanged — keeps ramp headroom |

Compounding, against stock's own equivalent at each end: an edge-scroll nudge is about
17x stock's nudge, and a fully ramped held scroll about 67x stock's ramped scroll. More
to the point, the *spread between them* goes from 2x to 8x — holding now accelerates
instead of crawling at one speed. Right-drag is 10x and stays linear: it calls `Pan`
directly and never touches the ramp, which is correct for direct manipulation.

`gameplay/scrollspeed.py` reads and writes all of these; run it with no arguments to print
the current state, and `--revert` to restore both backups. Use it rather than editing by
hand — it keeps `ARMADA.PRF`'s CRLF and field count intact and tells you the slider
position each value corresponds to.

Both files have a `.a2neb-backup` beside them; `cp X.a2neb-backup X` reverts either.
**Edit `ARMADA.PRF` only while the game is closed** — it is rewritten on exit, so a live
game will clobber the change. Tuning further is best done from the in-game slider, which
writes the same fields.

One caveat on `RTS_CFG.h`: the EXE hashes it across clients — *"EXE / RTS_CFG.h files do
not match node %d"* — so any change there has to be mirrored on every machine in a
multiplayer game. `ARMADA.PRF` is per-user and carries no such constraint.

## Telling whether a change actually landed

Both files are read **at launch**, so nothing applies to a running game. Beyond that the
two behave differently, and confusing them wastes a round trip:

- **`RTS_CFG.h`** is parsed fresh every launch (`0x490d73` opens it by name; a name the
  EXE's table does not know is silently ignored and keeps the compiled default). Edit it
  any time.
- **`ARMADA.PRF` can be rewritten by the game**, from the values held in memory, which
  would clobber an edit made while it runs. In practice it has **not** been observed to
  rewrite on exit under Heroic/Proton across several sessions here — its mtime stayed at
  the hand-edit through a number of launches, and the one time it did change was the
  first run, rewriting the GPU name and resolution. Treat a rewrite as possible but not
  the norm, and edit with the game closed anyway.

The cheapest confirmation is the options screen: **Options → Game Settings**, and read
the slider positions against the table above. Neither `RTS_CFG.h` value shows in any UI —
those you judge by feel. When the menus themselves are not usable, `gameplay/scrollspeed.py`
with no arguments prints the same information from the files.

**Compare the game's start time against the file mtimes** — that is the one check that
settles "did this session have the change", and it needs no UI:

    ps -o lstart= -p $(pgrep -f '[A]rmada2\.exe$')
    stat -c '%y  %n' RTS_CFG.h ARMADA.PRF

**`pgrep -f Armada2` matches the shell that is running the pgrep**, so it reports the
game as running when it is not — a self-match that produced a confident wrong conclusion
here twice in one session, in both directions. Bracket a character (`'[A]rmada2'`) or
match on the process's own command line. Under Heroic the real process is the last of a
six-deep stack — `umu_run.py`, `srt-bwrap`, `pv-adverb`, `proton`, `umu.exe`, and finally
`X:\Games\...\Armada2.exe` — so "did it actually quit" is a question worth checking
rather than assuming; see also the note that Wine calls the process `Main`.
