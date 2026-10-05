# Platform — Heroic, Proton and the renderer chain

The part of the setup every other layer stands on, and the part `a2mod` never switches:
DXVK's `d3d8.dll`/`d3d9.dll`, the Ultimate ASI Loader (`winmm.dll`) and
`STA2WidescreenPatch.asi`, run through Heroic and Proton on Arch + Hyprland. The d3d8
section matters to anything touching the renderer.

The HUD layout and cursors are in `hud/README.md`; renderer settings, anti-aliasing and
bloom (the Tiers) in `postfx/README.md`; the menus in `menus/README.md`.

## The install

| | |
|---|---|
| Game | `~/Games/Heroic/Star Trek Armada II` (`A2_GAME`, see `a2env.sh`) |
| Prefix | `~/Games/Heroic/Prefixes/Star Trek Armada II` (`A2_PREFIX`) |
| Heroic config | `~/.config/heroic/GamesConfig/1174788223.json` |
| Runner | Proton-CachyOS-latest |
| Base | GOG release = Armada II + patch 1.1 |
| GPU | RX 5700 XT (Navi 10 / gfx1010), 8 GB — Vulkan fine, **ROCm effectively unsupported** |

## Heroic configuration

**Heroic rewrites `1174788223.json` when it exits.** Only edit it while Heroic is fully
closed, or the change is silently lost.

The environment the game runs with:

    WINEDLLOVERRIDES = winmm=n,b;d3d8=n,b;d3d9=n,b

with `autoInstallDxvk` false. `n,b` = native first, then builtin. Wine searches the
application directory before the system directory, so this is what makes Wine load the
game-directory `winmm.dll` (the ASI loader), `d3d8.dll` and `d3d9.dll` (DXVK) instead of
its own. `platform/d3d8-chain.py` owns the `d3d9` entry and `autoInstallDxvk` (below).

## Widescreen

`STA2WidescreenPatch` v1.0 ships two files into the game directory:

    STA2WidescreenPatch.asi      9216      the patch itself
    winmm.dll                    2169856   Ultimate ASI Loader (ThirteenAG)

The same two files, unmodified, are vendored in `platform/vendor/STA2WidescreenPatch-1.0/`
with their MIT licences; the release zip installs them from there (`publish/README.md`).

It only loads because of the `winmm=n,b` override above. Without it Wine uses its
builtin winmm, the loader never runs, and the patch is inert with no error.

**Setting the resolution:** the in-game graphics menu sets it. It can also be written
directly into `ARMADA.PRF`, line 5:

    0.5 0.5 5 5 5 4 3440 1440 32 1 <NUL> 0 1 0.625 <CR>
                    ^^^^ ^^^^ ^^
                    w    h    bpp

    perl -0777 -pi -e 'binmode STDOUT; s/ 1024 768 32 / 3440 1440 32 /' ARMADA.PRF

**The file contains an embedded NUL byte** — use binary-safe tooling, not `sed`. The
game rewrites the file on exit and the resolution persists.

## The d3d8 chain

The game renders through DXVK and Vulkan. Confirmed in game at 3440x1440, with every
key in `dxvk.conf` applied and no errors in DXVK's log:

    Armada2.exe
      -> <game dir>/d3d8.dll          DXVK d3d8
      -> <game dir>/d3d9.dll          DXVK d3d9
      -> Vulkan

Four parts, all required together:

| part | value | why |
|---|---|---|
| `GAME/d3d8.dll` | DXVK d3d8 | the prefix is not durable — Proton restores it from symlinks |
| `GAME/d3d9.dll` | DXVK d3d9 | DXVK's d3d8 imports `d3d9.dll` by name |
| `WINEDLLOVERRIDES` | `winmm=n,b;d3d8=n,b;d3d9=n,b` | without the d3d9 entry Wine resolves it to builtin WineD3D |
| Wine virtual desktop | **off** | inside it DXVK's `ChangeDisplaySettingsEx` fails and the game falls back to 640x480 |

`platform/d3d8-chain.py` sets and identifies it:

    platform/d3d8-chain.py --status      identify the live chain, every link by hash
    platform/d3d8-chain.py --use dxvk    DXVK d3d8 -> DXVK d3d9 -> Vulkan
    platform/d3d8-chain.py --use d3d8to9 crosire's d3d8to9 (vendored) -> DXVK d3d9 -> Vulkan
    platform/d3d8-chain.py --use gog     GOG d3d8to9 -> DXVK d3d9 -> Vulkan
    platform/d3d8-chain.py --use wine    Wine's builtin d3d8 -> wined3d -> OpenGL
    platform/d3d8-chain.py --revert      the GOG release as shipped, Heroic managing DXVK again
    platform/d3d8-chain.py --upgrade     what ./install runs: DXVK's chain -> the d3d8to9 one

`--use` keeps GOG's `d3d8.dll` as `d3d8.dll.gog-backup` the first time it replaces it
(it knows GOG's file by hash), sets the `d3d9=n,b` override for DXVK and GOG and
`autoInstallDxvk` false, so it needs Heroic closed; it refuses before touching anything
rather than half-applying, and refuses outright to replace a `d3d8.dll` it cannot
identify. `--revert` puts GOG's `d3d8.dll` back, removes the game-directory `d3d9.dll`
and the override, and turns `autoInstallDxvk` back on.

### What can sit in the d3d8 slot

Two different things want to be the game directory's `d3d8.dll`, and with neither
there the prefix's is used:

- **GOG's** `d3d8.dll` (1101824) is a full **d3d8to9 translator** — it implements
  Direct3D 8 on top of Direct3D 9.
- **DXVK's** `d3d8.dll` (~1.66 MB) implements Direct3D 8 on Vulkan through its own d3d9.
- **Wine's builtin** `d3d8.dll` (320548), in the prefix, talks to `wined3d` directly and
  never loads `d3d9.dll`, so a DXVK d3d9 in the prefix is not reached from it.

A plugin can reach Direct3D 9, and with it shaders, only through a translator that
answers `QueryInterface(IDirect3DDevice9)`. Measured on the bench, crosire's current
d3d8to9 release does; neither DXVK's d3d8 nor GOG's build does. The spike and the plan
built on it are in [`D3D9.md`](D3D9.md). That release is vendored in
`vendor/d3d8to9-1.16.0/` and `--use d3d8to9` puts it in the slot; `d3d9/d3d9dev.h` is
what a plugin includes to draw through the device behind it, and `d3d9/hlsl.sh` builds
its shaders.

**`./install` puts the translator in place** (`--upgrade`, since platform 3.2.0) when the
game directory holds exactly the DXVK chain: DXVK's d3d8 on DXVK's d3d9. The two chains
differ only in the game directory's `d3d8.dll`; both need the `d3d9` override and
`autoInstallDxvk` off, which the DXVK chain already has. So the step swaps that one file
and never touches Heroic's config: it runs with Heroic open, and in a test-bench clone.
Any other chain (GOG's, Wine's, a `d3d8.dll` of the player's own) is left as it is and
named. `--use dxvk` goes back. The release zip's installer does not do this yet.

The GOG release as shipped, which `--revert` returns to, is GOG's translator on the
prefix's d3d9 — Heroic's DXVK while `autoInstallDxvk` is on, else Wine's:

    Armada2.exe
      -> <game dir>/d3d8.dll      GOG d3d8to9           1101824
      -> syswow64/d3d9.dll        Heroic's DXVK d3d9, or Wine builtin -> wined3d

### Traps

- **Never identify a DLL in this chain by size.** `syswow64/d3d8.dll` at 320548 bytes
  was once recorded as "DXVK's exact size"; it is byte-identical to *Wine's builtin*,
  and on the strength of that the game ran through wined3d/OpenGL while every
  `dxvk.conf` key was silently ignored and the DXVK HUD never appeared. DXVK's d3d8 is
  ~1.66 MB (Proton's 1658894, Heroic's 3.1.1 1687566). `d3d8-chain.py` hashes each link
  against the candidates actually present on the machine and names what it found,
  reporting `UNKNOWN` rather than guessing.
- **The prefix is not a durable place for DXVK.** Proton's `default_pfx` holds
  `syswow64/d3d8.dll` and `d3d9.dll` as **symlinks** to its own Wine builtins and
  restores them on prefix sync — a DXVK `d3d8` written there was Wine's builtin again
  after one launch, with `autoInstallDxvk` already false. The game directory is durable:
  nothing manages it, and Wine searches it first.
- **`autoInstallDxvk` true redeploys Heroic's DXVK into the prefix on every launch** —
  `d3d8.dll`, `d3d9.dll`, `d3d11.dll` and `dxgi.dll` sharing one mtime is the sign. That
  is it working as designed, not a regression.
- **The `d3d9=n,b` override is load-bearing, not diagnostic.** `dxvk-logging.py
  --diagnose` also adds it, but the logging tool only ever adds; `d3d8-chain.py` owns
  removing it, so switching diagnostics off cannot break the chain.

### When a renderer setting seems to do nothing

Separate "the setting was ignored" from "the setting is subtle" before judging it.
`platform/dxvk-logging.py --diagnose` answers it in one launch: no DXVK HUD means DXVK is
not in the chain; no `Armada2_d3d9.log` means its d3d9 never loaded; a key missing from
`--check`'s "Effective configuration" was never read from `dxvk.conf`. If the log shows

    err:   D3D9: EnterFullscreenMode: Failed to change display mode
    err:   D3D9: Failed to set initial fullscreen state

and the game sits at 640x480, the Wine virtual desktop is on (below).

## Hyprland / window management

**Symptom:** the game opened a second window; the settings menu drew in one while input
stayed grabbed by the other, so the menu was visible but could not be clicked. Focus
kept snapping back to a black fullscreen frame.

**Cause:** every menu, the in-game Options included, is a `WS_POPUP` dialog, which Wine
turns into a second X11 window that Hyprland tiles and focuses like a new application.

**Fix: `Menus.asi` with `Embed=1`** (the default it ships with) re-creates each
menu as a child of the game window, so the game is one OS window from launch to exit.
See `menus/README.md`, "One window: `Embed=1`".

**The Wine virtual desktop must stay off.** It also puts everything in one window
(`"Desktop"="Default"` under `[Software\\Wine\\Explorer]` in `<prefix>/pfx/user.reg`),
but it is incompatible with DXVK: inside it `ChangeDisplaySettingsEx` fails and the game
falls back to 640x480. Do not turn it on to fix a window problem; fix the window in
`menus.c`.

To check it is off (a fresh prefix never has it):

    grep -F -A3 '[Software\\Wine\\Explorer]' "<prefix>/pfx/user.reg"

No output, or no `"Desktop"=` line under that key, means off. A plain
`grep '"Desktop"='` is wrong for this: the Shell Folders keys carry unrelated `"Desktop"`
values. A `[Software\\Wine\\Explorer\\Desktops]` key only defines a size and does nothing
without that value. To remove the value, delete that one line with Heroic and the game
closed — while `wineserver` runs, Wine rewrites `user.reg` from memory and discards the
edit.

## Known issues

- **End-of-mission crash** — a `Wine C++ Runtime Library` R6025 box on finishing a
  mission, then madExcept. Captured in `$A2_DATA/archive/error-mission-finish/`. **It
  occurs without any modding**, so it is not this project's, and nothing here has been
  shown to affect it either way. Recorded so it is not mistaken for a texture problem.
- **Cutscene crash.** Finishing Federation mission 1 once threw a DirectX-related error
  and crashed when the completion cutscene tried to play, under the old wined3d chain.
  Startup videos played fine, so Bink itself worked; the hypothesis was a D3D8 device
  reset on the transition from live 3D to fullscreen video. It produced no diagnostics
  at all — `Logs/` empty, no coredump, no MadExcept report, no Heroic session log — and
  it has not been retested under DXVK or `binkproxy`.

## Campaign progress format

`save/shell.set`, 78 bytes.

- Stock: all zeros except offset 11 = `0x01`.
- Setting **all** bytes to `0x01` unlocked the first **two** missions of every campaign.
- So the bytes are **progress counters** (value = missions completed), not booleans.
  `0x09`/`0x0A` should open all ten.
- Offset 11 is *not* the Federation counter — it held `1` while only mission 1 was
  selectable.

`mshell.set` in the game directory is the mission list: 40 entries, six tutorial plus
ten each for Federation, Klingon and Borg.

## Gotchas

- **Neither obvious way of finding the game process works:**
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

    # what implements d3d8 right now? by hash, never by size
    platform/d3d8-chain.py --status

    # did Heroic redeploy DXVK into the prefix? these four sharing an mtime means yes
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
