# Gameplay — map scrolling and draw distance

Engine settings that change how the game plays rather than how it looks, which is
why `a2mod` leaves them alone in both states: the map-scroll knobs in `ARMADA.PRF` and
`RTS_CFG.h` (`gameplay/scrollspeed.py`, applied), and the cutscene draw distance, which
is understood and deliberately not changed.

## Map scrolling

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

### Every navigation path funnels through Pan

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

### The two speeds live in ARMADA.PRF, not in any config file

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

### SCROLL_BORDER_WIDTH is the other half of "unwieldy"

`RTS_CFG.h` ships `SCROLL_BORDER_WIDTH = 2` — the mouse must be within **2 pixels** of a
screen edge for edge-scrolling to engage at all, and the factor ramps linearly across
that band. The EXE's own compiled-in default is **20**, so stock's 2 is the config
file overriding the engine down to a hair's width. At 3440x1440 that is the difference
between a usable edge and one you have to hunt for.

Raised to 20 here.

### What is set

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

### Telling whether a change actually landed

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

## Draw distance: why ships pop in during cutscenes

**Investigated and deliberately not changed** (2026-09-23). The cheap version of the fix
is to edit one float in one mission script, and the recipe for that is below. The general
version, an ASI hook, was judged not worth building for a cosmetic effect.

### What pops them in: object culling, not the far plane

The far clip plane is 20000 (`FAR_CLIPPING_PLANE`). Objects vanish long before that,
because `GameObjectInstance::DistanceCull` (`0x4d5bb0`) hides anything further than a
**culling distance D** from the camera:

- Distance is measured with the **height axis weighted by 0.25**. That is why a
  near-top-down RTS camera almost never trips it and a low cutscene camera trips it
  constantly.
- `D = cfgOBJECT_CULLING_DISTANCE` (`0x6fcac0`), multiplied by **0.7** in one camera
  mode. The flag is at `[[0x76b5ac]+0x80]+0x40`; which mode it is has not been
  identified.
- For an object whose radius exceeds 100 (`[obj+0x40]`), D becomes
  `D × radius / 100`. So small ships pop first and big stations last.

All constants were read out of `Armada2.exe`, not guessed. `armada2.map` names every
function above.

### Why ART_CFG.h does not reach the cutscenes

`ART_CFG.h` sets `cfgOBJECT_CULLING_DISTANCE = 2800.0`; the exe's built-in default is
2000. The loader (`0x491b7a`) stores the value and **copies it to
`cfgDEFAULT_OBJECT_CULLING_DISTANCE`** (`0x6fcac4`). That affects normal play only.

The mission scripts (`missions/*S.dsl`) are compiled Win32 DLLs. They drive the engine
through the `ScriptInterfaceImp` vtable (`0x6af380`), and three of its slots matter here:

| slot | offset | method | effect |
|---|---|---|---|
| 305 | `+0x4c4` | `SetClippingDistance(float)` | writes `CINERACTIVE_FAR_CLIPPING_PLANE` (`0x6fcab4`), which the cutscene camera copies into the far plane when it is built (`0x64b874`) and restores afterwards (`0x64b986`) |
| 306 | `+0x4c8` | `SetObjectCullingDistance(float)` | overwrites D |
| 307 | `+0x4cc` | `RestoreObjectCullingDistance()` | copies the `ART_CFG.h` value back into D |

**Every campaign mission and both tutorials set their own D for their cutscenes**, as
a float immediate. The values run from 1000 to 6000. They override `ART_CFG.h` for the
length of the cutscene, and `Restore` hands control back afterwards. Two side notes:

- `CINERACTIVE_FAR_CLIPPING_PLANE` in `ART_CFG.h` is **dead**: that string is not in the
  exe, and only scripts set the value.
- The `.drl` files make none of these calls.

### Every call site, with the byte to change

`File offset` is the offset in the `.dsl` **file** of the 4-byte little-endian float
operand of the `push imm32` (opcode `68`) that feeds the call. Every row was checked:
the byte before each offset is `0x68`, and the four bytes at it decode to the listed
value.

| mission | call | value | file offset |
|---|---|---|---|
| a2_borg01S | cull | 1500 | `0x13a76` |
| a2_borg02S | cull | 3000 | `0x7b87` |
| a2_borg03S | **clip** | 4000 | `0x10032` |
| a2_borg03S | cull | 6000 | `0x10053` |
| a2_borg03S | cull | 4000 | `0x1142c` |
| a2_borg04S | cull | 3000 | `0x6f86` |
| a2_borg04S | cull | 3000 | `0xd2d0` |
| a2_borg05S | cull | 3000 | `0xcb97` |
| a2_borg06S | cull | **1000** | `0x9b19` |
| a2_borg07S | cull | 6000 | `0xa35b` |
| a2_borg07S | cull | 3000 | `0xfbfe` |
| a2_borg08S | cull | 3000 | `0x8af1` |
| a2_borg08S | cull | 1200 | `0x974a` |
| a2_borg09S | cull | 4000 | `0x9766` |
| a2_borg10S | cull | 6000 | `0x119ab` |
| a2_fed01S | cull | 2000 | `0xafb9` |
| a2_fed01S | cull | 1800 | `0xc768` |
| a2_fed02S | cull | 2500 | `0xc524` |
| a2_fed03S | **clip** | 6000 | `0xe4b2` |
| a2_fed03S | cull | 6000 | `0xe4d3` |
| a2_fed04S | cull | 3000 | `0x1152b` |
| a2_fed05S | cull | 1500 | `0xcddc` |
| a2_fed06S | cull | 4000 | `0xb536` |
| a2_fed06S | cull | 3000 | `0x13f5c` |
| a2_fed07S | cull | 3000 | `0x7b97` |
| a2_fed08S | cull | 3000 | `0x6791` |
| a2_fed09S | **clip** | 4000 | `0x12222` |
| a2_fed09S | cull | 4000 | `0x12243` |
| a2_fed09S | cull | 4000 | `0x13ce6` |
| a2_fed10S | cull | 3500 | `0xe996` |
| a2_fed10S | cull | 3000 | `0xf537` |
| a2_fed10S | cull | 4500 | `0xf6e5` |
| a2_fed10S | cull | 3000 | `0xf939` |
| a2_kling01S | cull | 1500 | `0xf215` |
| a2_kling02S | cull | 3000 | `0x8e88` |
| a2_kling03S | **clip** | 4000 | `0x9ab2` |
| a2_kling03S | cull | 4000 | `0x9ad3` |
| a2_kling04S | **clip** | 4000 | `0xb322` |
| a2_kling04S | cull | 5000 | `0xb343` |
| a2_kling05S | cull | 2000 | `0xc57b` |
| a2_kling06S | cull | 5000 | `0xd059` |
| a2_kling07S | cull | 3000 | `0x80be` |
| a2_kling07S | cull | 3000 | `0xa2f8` |
| a2_kling08S | cull | 3000 | `0x80ca` |
| a2_kling08S | cull | 4000 | `0x89da` |
| a2_kling08S | cull | 3000 | `0x8b97` |
| a2_kling09S | cull | 4000 | `0x9d06` |
| a2_kling10S | cull | 2000 | `0x11422` |
| a2_kling10S | cull | **1000** | `0x11d3c` |
| a2_kling10S | cull | 3000 | `0x12647` |
| a2_tutorial3s | cull | 3000 | `0x64f6` |
| a2_tutorial4s | cull | 3000 | `0x5e66` |

### Changing one

```sh
. ./a2env.sh; cd "$A2_GAME/missions"
f=a2_borg06S.dsl; off=0x9b19; new=6000
[ -e "$f.a2neb-backup" ] || cp -p "$f" "$f.a2neb-backup"
python3 - "$f" "$off" "$new" <<'EOF'
import struct, sys
f, off, new = sys.argv[1], int(sys.argv[2], 0), float(sys.argv[3])
d = bytearray(open(f, 'rb').read())
assert d[off - 1] == 0x68, 'not a push imm32 -- wrong offset or wrong file'
print('old', struct.unpack_from('<f', d, off)[0], '-> new', new)
struct.pack_into('<f', d, off, new)
open(f, 'wb').write(d)
EOF
```

Common values: 6000 = `0x45bb8000`, 10000 = `0x461c4000`, 20000 = `0x469c4000`.
**`./a2tex revert all` does not know about these files**; restore one by copying its
`.a2neb-backup` back.

Caveats, all unverified until someone tries one in game:

- **Where a mission also calls `clip`, raise that too.** A culling distance beyond
  the far plane buys nothing. The cutscene far plane is otherwise the 20000 default.
- **Which call belongs to which cutscene is not known.** File order is not necessarily
  play order. In a mission with several calls, change one, watch, and note the result
  here.
- **The very low values may be deliberate.** 1000 and 1200 in particular may be hiding
  ships staged off-camera until their entrance, and raising them would reveal those
  early. That is the first thing to watch for.
- **Whether the engine checksums mission DLLs is not known.** Nothing suggests it,
  but it has not been tested. If a patched mission fails to load, that is why.
- `cfgOBJECT_CULLING_DISTANCE` in `ART_CFG.h` is the knob for **normal play**, and
  what every cutscene restores to. That the root `ART_CFG.h` is the file actually read
  is inferred (2800 in the file against 2000 in the exe), not shown in game.

### The general fix, not built

The general fix is an ASI hook with the same structure as `menus/`. It would
patch the two setters at `0x4578a0` and `0x457890` to store
`max(script value, floor)`. That covers every mission at once, with no per-file edits.
It was not built: the effect is cosmetic, and per-mission edits cover the one or two
cutscenes where the pop-in is actually noticed.
