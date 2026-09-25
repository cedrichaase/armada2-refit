# Platform — Heroic, Proton and the renderer chain

Everything learned getting the GOG release running well on Arch + Hyprland: the part
of the setup every other layer stands on, and the part `a2mod` never switches — DXVK's
`d3d8.dll`/`d3d9.dll`, the Ultimate ASI Loader (`winmm.dll`) and
`STA2WidescreenPatch.asi`. The d3d8 section matters to anything touching the renderer.

What used to sit beside this in one `SETUP.md` now lives with its layer: the HUD
layout and cursors in `hud/README.md`, map scrolling in `gameplay/README.md`, renderer
settings, anti-aliasing and bloom (the Tiers) in `postfx/README.md`, and the menus in
`menus/README.md`.

## The install

| | |
|---|---|
| Game | `/home/cedric/Games/Heroic/Star Trek Armada II` |
| Prefix | `/home/cedric/Games/Heroic/Prefixes/Star Trek Armada II` |
| Heroic config | `~/.config/heroic/GamesConfig/1174788223.json` |
| Runner | Proton-CachyOS-latest |
| Base | GOG release = Armada II + patch 1.1, plus Patch Project 1.2.5 |
| GPU | RX 5700 XT (Navi 10 / gfx1010), 8 GB — Vulkan fine, **ROCm effectively unsupported** |

## Heroic configuration

**Heroic rewrites `1174788223.json` when it exits.** Only edit it while Heroic is fully
closed, or the change is silently lost. Backup: `1174788223.json.bak-20260920`.

Current environment:

    WINEDLLOVERRIDES = winmm=n,b;d3d8=n,b

`n,b` = native first, then builtin. Wine searches the application directory before the
system directory, so this is what makes Wine load the game-directory `winmm.dll`
(the ASI loader) and `d3d8.dll` (the Patch Project proxy) instead of its own.

## Widescreen

`STA2WidescreenPatch` v1.0 ships two files into the game directory:

    STA2WidescreenPatch.asi      9216      the patch itself
    winmm.dll                    2169856   Ultimate ASI Loader (ThirteenAG)

It only loads because of the `winmm=n,b` override above. Without it Wine uses its
builtin winmm, the loader never runs, and the patch is inert with no error.

**Setting the resolution:** the in-game graphics menu was unusable (see Hyprland,
below), so it was written directly into `ARMADA.PRF`, line 5:

    0.5 0.5 5 5 5 4 3440 1440 32 1 <NUL> 0 1 0.625 <CR>
                    ^^^^ ^^^^ ^^
                    w    h    bpp

    perl -0777 -pi -e 'binmode STDOUT; s/ 1024 768 32 / 3440 1440 32 /' ARMADA.PRF

**The file contains an embedded NUL byte** — use binary-safe tooling, not `sed`.
Backup at `ARMADA.PRF.bak` (151 bytes). The game rewrites the file on exit (now 154
bytes) and the resolution persists.

## Patch Project 1.2.5

**The NSIS installer refuses to run against a GOG install**, with
*"Make sure you have Armada II with Patch 1.1 installed in the target directory."*
This is a known GOG incompatibility, not a broken download
(installer md5 `8216c620fb17331a3d647f550ccfa723`).

**Workaround:** download the ZIP distribution of the same version and copy `install/*`
into the game directory by hand:

    Armada2Hook.dll   1865728
    Armada2Hook.mad    105160     MadExcept crash-reporter data
    d3d8.dll            45056     proxy — see below
    FOmsvc.dll         167936

`Armada2.exe` is **not** modified — 1.2.5 is the "loader-free" release, confirmed by
diffing against the original.

## The d3d8 chain

The fiddliest part of the setup. **Two different things both want to be `d3d8.dll`:**

- **GOG's** `d3d8.dll` (1101824) is a full **d3d8to9 translator** — it implements
  Direct3D 8 on top of Direct3D 9.
- **Patch Project's** `d3d8.dll` (45056) is a **proxy** that exports only
  `Direct3DCreate8`, and loads the real implementation from the **system directory**
  via `GetSystemDirectoryA`.

Because the proxy looks in the system directory, the two can be stacked rather than
chosen between:

    Armada2.exe
      -> <game dir>/d3d8.dll          Patch Project proxy      45056
      -> syswow64/d3d8.dll            GOG d3d8to9 translator   1101824
      -> syswow64/d3d9.dll            DXVK                     7798798
      -> Vulkan

GOG's original was moved aside in the game directory as `d3d8.dll.gog-backup`, and
DXVK's own d3d8 was backed up as `syswow64/d3d8.dll.dxvk-backup`.

### PCGamingWiki's advice is wrong for this build

It suggests renaming the patch's `d3d8.dll` to `dinput.dll` to dodge the conflict.
Verified against this executable:

    objdump -p Armada2.exe | grep -i dinput     # no matches

The exe imports **no** dinput or dinput8 at all, so a `dinput.dll` would never be
loaded and the patch would be silently inert.

### ⚠ CORRECTION: 320548 is Wine's builtin d3d8, not DXVK's

**The section below is wrong about what is in the prefix, and the error inverted the
whole diagnosis.** It records `syswow64/d3d8.dll` at 320548 bytes as "DXVK's exact
size". It is not. Measured with `sha256`, not with size folklore:

| file | bytes | what it actually is |
|---|---:|---|
| `syswow64/d3d8.dll` | 320548 | **byte-identical to Wine's builtin d3d8** |
| `syswow64/d3d8.dll.dxvk-backup` | 320548 | the builtin as well — the backup never held DXVK |
| DXVK d3d8 (Proton's) | 1658894 | ~1.66 MB, and has never been in this prefix |
| DXVK d3d8 (Heroic's 3.1.1) | 1687566 | likewise |
| `syswow64/d3d9.dll` | 7798798 | genuinely DXVK (Proton's), but **bypassed** |

So the chain that has actually been running is:

    Armada2.exe
      -> <game dir>/d3d8.dll      Patch Project proxy   45056
      -> syswow64/d3d8.dll        WINE BUILTIN d3d8     320548
      -> wined3d
      -> OpenGL

**DXVK is not in this game's render chain and never has been.** Wine's d3d8 talks to
`wined3d` directly; it never loads `d3d9.dll`, so the DXVK d3d9 sitting in the prefix is
never reached. That is why no `d3d9.*` key in `dxvk.conf` changed anything, why the DXVK
HUD never appeared, and why `--diagnose` produced a `xalia_dxgi.log` (a different
process, which does use DXVK) but no `Armada2_d3d9.log`.

It also means the entire texture project — 2048 skyboxes, a 4096 atlas, 1024 hulls — has
been rendering through wined3d/OpenGL, not Vulkan. Worth knowing before any of it is
attributed to DXVK.

**And the prefix is not a durable place to fix it.** Proton's `default_pfx` holds
`syswow64/d3d8.dll` and `d3d9.dll` as **symlinks** to its own Wine builtins and restores
them on prefix sync. A DXVK `d3d8` written into `syswow64` was verified by hash, then
was Wine's builtin again after a single launch — with `autoInstallDxvk` already false,
so Heroic was not the cause that time. Proton was.

The durable slot is the **game directory**, which nothing manages and which Wine
searches *before* the system directory — the same mechanism that already makes the
game-directory `winmm.dll` and `d3d8.dll` load at all. So `--use dxvk` puts DXVK's
`d3d8.dll` **and** `d3d9.dll` beside `Armada2.exe`, replacing the Patch Project proxy
(kept as `d3d8.dll.proxy-backup`) and making the prefix irrelevant to the outcome.

`autoInstallDxvk` also turns out to be the whole of the "regression" recorded below:
set to true it redeploys over the slot on every launch, which is it working as designed
rather than misbehaving.

`platform/d3d8-chain.py` exists so this cannot recur: it identifies every link by hashing
it against the candidates actually present on the machine and **names** what it found,
reporting `UNKNOWN` rather than guessing. Never identify one of these by size again.

    platform/d3d8-chain.py --status      identify the live chain
    platform/d3d8-chain.py --use dxvk    DXVK d3d8 -> DXVK d3d9 -> Vulkan
    platform/d3d8-chain.py --use gog     GOG d3d8to9 -> DXVK d3d9 -> Vulkan
    platform/d3d8-chain.py --revert      back to Wine's builtin, Heroic managing it again

`--use` also sets `autoInstallDxvk`, so it needs Heroic closed, and it refuses before
touching anything rather than half-applying.

### The original note, kept for the record

### ⚠ This fix does not currently survive a launch

`autoInstallDxvk` is `true`, and **Heroic redeploys DXVK's DLLs into the prefix on every
launch**, overwriting the GOG translator. Confirmed: `syswow64/d3d8.dll` is back to
320548 bytes (DXVK's exact size) with the same mtime as `d3d9.dll`, `d3d11.dll` and
`dxgi.dll` — a bulk redeploy.

Consequences:

1. The chain above is **not in effect right now**.
2. **The cutscene-crash fix was therefore never actually tested** — it was reverted
   before the next play session.

Re-checked 2026-09-21 and still true. `syswow64/d3d8.dll` is 320548 bytes with the same
mtime as `d3d9.dll`, both rewritten at the last launch; `autoInstallDxvk` is still
`true`; `WINEDLLOVERRIDES` is still `winmm=n,b;d3d8=n,b`.

So the chain actually in effect is **the proxy into DXVK's own d3d8**, not into the GOG
translator:

    Armada2.exe
      -> <game dir>/d3d8.dll          Patch Project proxy      45056
      -> syswow64/d3d8.dll            DXVK d3d8                320548
      -> Vulkan

Worth knowing rather than only regretting: DXVK's native d3d8 is what the whole texture
project has actually been rendering through, and it has handled 2048 skyboxes, a 4096
atlas and 1024 hull textures without complaint.

Remedies, in order of preference:

- Set `autoInstallDxvk` to `false` (Heroic closed), then restore the translator:
  `cp "<game dir>/d3d8.dll.gog-backup" "<prefix>/pfx/drive_c/windows/syswow64/d3d8.dll"`
  DXVK's `d3d9.dll` stays in place; only the d3d8 slot needs to stop being managed.
- Or re-copy the translator after every launch, which is fragile.

### RESOLVED: DXVK works, and the Wine virtual desktop was the blocker

**Confirmed in game at 3440x1440, with all of `dxvk.conf` applied — stages 1, 2 and 3.**
The log for the working launch: config found, **no errors at all**,
`last mode set: 3440x1440`, and all four keys under "Effective configuration":

    d3d9.samplerAnisotropy = 16
    d3d9.samplerLodBias = -0.5
    d3d9.clampNegativeLodBias = False
    d3d9.seamlessCubes = True

The working configuration, all four parts required together:

| part | value | why |
|---|---|---|
| `GAME/d3d8.dll` | DXVK d3d8 | the prefix is not durable — Proton restores it from symlinks |
| `GAME/d3d9.dll` | DXVK d3d9 | DXVK's d3d8 imports `d3d9.dll` by name |
| `WINEDLLOVERRIDES` | `winmm=n,b;d3d8=n,b;d3d9=n,b` | without the d3d9 entry Wine resolves it to builtin WineD3D |
| Wine virtual desktop | **off** | inside it DXVK's `ChangeDisplaySettingsEx` fails and the game falls back to 640x480 |

Set it up with `platform/d3d8-chain.py --use dxvk` (reverse with `--revert`). The
virtual desktop stays off in both states: it is superseded by `Menus.asi`'s
`Embed=1` (see "Hyprland / window management", which also has the check).

**The `d3d9=n,b` override is load-bearing, not diagnostic.** It arrived as part of
`dxvk-logging.py --diagnose`, so `--off` used to strip it — which would have silently
broken the chain the moment diagnostics were switched off. Ownership now sits with
`d3d8-chain.py` (`--use` adds it, `--revert` removes it) and the logging tool only ever
adds, never removes.

**The two-window focus bug the virtual desktop was added for** is fixed by
`Menus.asi`'s `Embed=1` — see the Hyprland section.

### How it was found — kept because the method is the lesson

Settled by measurement, after three wrong explanations for "I can't see a difference".

**The settings were never the problem, and neither was subtlety.** With DXVK's `d3d8.dll`
and `d3d9.dll` in the game directory the HUD appeared and `Armada2_d3d9.log` reported:

    info:  Found config file: dxvk.conf
    info:  Effective configuration:
    info:    d3d9.samplerAnisotropy = 16
    info:    d3d9.samplerLodBias = -0.5
    info:    d3d9.clampNegativeLodBias = False

So `dxvk.conf` is found and every key applies, once DXVK is actually reached.

**But the game then collapses to 640x480.** The same log:

    err:   D3D9: EnterFullscreenMode: Failed to change display mode   (x4)
    err:   D3D9: Failed to set initial fullscreen state               (x4)

It alternates 3440x1440 and 640x480 across 25 mode sets and ends on 640x480;
`MenuScale.log`'s last line agrees, reporting `screen 640x480`. So the engine asks for
exclusive fullscreen, DXVK cannot change the display mode, and the fallback wins.

**The prime suspect is the Wine virtual desktop** (`Software\\Wine\\Explorer`,
`Desktop=Default`, `Default=3440x1440`). wined3d never needed a real mode change inside
it; DXVK calls `ChangeDisplaySettingsEx` and it fails. Turning it off was the next
experiment, and it was the fix (above).

(While that was open, the chain was reverted to stock so the game stayed playable.
That is history, not the current state.)

**Note that the renderer question is now separable from the texture question.** Nothing
about the texture work depends on any of this: it has always rendered through
wined3d/OpenGL and continues to.


### No longer outstanding

`d3d9=n,b` is in `WINEDLLOVERRIDES` — `winmm=n,b;d3d8=n,b;d3d9=n,b`, confirmed in
Heroic's config. It is part of the working DXVK chain above.

## Hyprland / window management

**Symptom:** the game opened a second window; the settings menu drew in one while input
stayed grabbed by the other, so the menu was visible but could not be clicked. Focus
kept snapping back to a black fullscreen frame.

**Cause:** every menu, the in-game Options included, is a `WS_POPUP` dialog, which Wine
turns into a second X11 window that Hyprland tiles and focuses like a new application.

**Fix: `Menus.asi` with `Embed=1`** (the default it ships with) re-creates each
menu as a child of the game window, so the game is one OS window from launch to exit.
See `menus/README.md`, "One window: `Embed=1`".

**The Wine virtual desktop is superseded and must stay off.** It was the first fix for
this bug: `"Desktop"="Default"` under `[Software\\Wine\\Explorer]` in
`<prefix>/pfx/user.reg`, which put everything in one window. It is incompatible with
DXVK: inside it `ChangeDisplaySettingsEx` fails and the game falls back to 640x480 (see
above). Do not turn it back on to fix a window problem; fix the window in
`menus.c`.

To check it is off (a fresh prefix never has it):

    grep -F -A3 '[Software\\Wine\\Explorer]' "<prefix>/pfx/user.reg"

No output, or no `"Desktop"=` line under that key, means off. A plain
`grep '"Desktop"='` is wrong for this: the Shell Folders keys carry unrelated `"Desktop"`
values. The `[Software\\Wine\\Explorer\\Desktops]` key that remains in this prefix only
defines a size and does nothing without that value. To remove the value, delete that one
line with Heroic and the game closed — while `wineserver` runs, Wine rewrites `user.reg`
from memory and discards the edit. (`platform/virtual-desktop.py` did this until
platform 2.0.0.)

## Known issues

- **Cutscene crash.** Finishing Federation mission 1 threw a DirectX-related error and
  crashed when the completion cutscene tried to play. Startup videos (`Intro.bik`) play
  fine, so Bink itself works; the hypothesis is a D3D8 device reset on the transition
  from live 3D to fullscreen video. No diagnostics were produced at all — `Logs/` empty,
  no coredump, no MadExcept report, no Heroic session log. **Unresolved**, and per the
  section above the intended fix is not currently installed.

## Campaign progress format

`save/shell.set`, 78 bytes. Backup at `save/shell.set.bak`.

- Stock: all zeros except offset 11 = `0x01`.
- Setting **all** bytes to `0x01` unlocked the first **two** missions of every campaign.
- So the bytes are **progress counters** (value = missions completed), not booleans.
  `0x09`/`0x0A` should open all ten.
- Offset 11 is *not* the Federation counter — it held `1` while only mission 1 was
  selectable.

`mshell.set` in the game directory is the mission list: 40 entries, six tutorial plus
ten each for Federation, Klingon and Borg.

## Gotchas

- **Neither obvious way of finding the game process works**, and the advice that used to
  stand here — "use `pgrep -x Armada2.exe`" — is **wrong**:
  - `pgrep -f "Armada2.exe"` matches its own command line and reports a false positive.
    Likewise `pkill -f startrekarmada2` killed its own shell (exit 144).
  - `pgrep -x Armada2.exe` matches **nothing, even while the game is running**, because
    **Wine reports the process `comm` as `Main`.** This is not cosmetic. It made a test
    harness report "game DOWN" for a game that was plainly up — producing several
    confident, wrong conclusions about the game exiting on its own — and it made a
    `pkill -x Armada2.exe` cleanup a silent no-op, so **seven orphaned instances
    accumulated over half an hour**, none with a window, each still holding a PipeWire
    stream and audibly playing the menu music.

  Collect PIDs with `ps` and kill them individually:

      ps -eo pid=,args= | awk '/Armada2\.exe/ { print $1 }'

  **`menus/stop-game.sh` does this properly** — it also kills the Wine helpers
  for this prefix only (matched by `WINEPREFIX` out of `/proc/<pid>/environ`, so an
  unrelated Wine app cannot be caught in it), and drops stale PipeWire nodes, which
  outlive the process, stay in state `running`, and keep playing. Killing the processes
  is *not* enough on its own.
- **`md5sum` wedges on Wine-backed paths** — it blocked in `unix_stream_read_generic`
  partway through a manifest. For before/after comparison use
  `find -printf '%s\t%TY-%Tm-%Td\t%p\n'` instead; size+mtime is enough and is instant.
- **Heroic overwrites its per-game JSON on exit.** Close it before editing.
- **Web sources:** moddb.com and pcgamingwiki.com return HTTP 403 to automated fetches;
  armadafiles.com has a broken TLS certificate (altnames are `*.kasserver.com`) — reach
  it over plain `http` with `curl`.

## Verification recipes

    # what is actually in the d3d8 slot? 320548 = DXVK, 1101824 = GOG d3d8to9
    stat -c '%s %y' "<prefix>/pfx/drive_c/windows/syswow64/d3d8.dll"

    # did DXVK redeploy? these four sharing an mtime means yes
    cd "<prefix>/pfx/drive_c/windows/syswow64" && stat -c '%n %y' d3d8.dll d3d9.dll d3d11.dll dxgi.dll

    # what does the exe actually import?
    objdump -p Armada2.exe | grep -i 'DLL Name'

    # current overrides, without opening Heroic
    python3 -c "import json;print(json.load(open('$HOME/.config/heroic/GamesConfig/1174788223.json'))['1174788223']['enviromentOptions'])"

    # is the game really running?  NOT `pgrep -x Armada2.exe` -- Wine calls it "Main"
    ps -eo pid=,args= | awk '/Armada2\.exe/ { print $1 }'

    # stop it, its Wine session, and any stale audio node it left behind
    menus/stop-game.sh

    # did Menus.asi load, and what did it patch?
    cat "<game dir>/Menus.log"

