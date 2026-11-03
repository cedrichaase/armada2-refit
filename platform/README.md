# Platform — Heroic, Proton and the d3d8 slot

The part of the setup every other layer stands on, and the part `a2mod` never switches:
the game's `d3d8.dll` slot (Display8's; nothing stands behind it since 4.0.0), the
Ultimate ASI Loader (`winmm.dll`) and `STA2WidescreenPatch.asi`, run through Heroic and
Proton on Arch + Hyprland. The d3d8 section matters to anything touching the renderer.

The HUD layout and cursors are in `ui/HUD.md`; renderer settings and anti-aliasing
are render3d's and Display8's (`render3d/README.md`, `display/README.md`); the menus are
the UI's (`ui/MENUS.md`). The version
resource every plugin we build carries is `versioninfo/` (its `README.md`); it is part
of the build and installs nothing of its own.

## The install

| | |
|---|---|
| Game | `~/Games/Heroic/Star Trek Armada II` (`A2_GAME`, see `a2env.sh`) |
| Prefix | `~/Games/Heroic/Prefixes/Star Trek Armada II` (`A2_PREFIX`) |
| Heroic config | `$A2_HEROIC/GamesConfig/1174788223.json`: `~/.config/heroic` for the native package, `~/.var/app/com.heroicgameslauncher.hgl/config/heroic` for the Flatpak (`a2env.sh` takes whichever exists, native first) |
| Runner | Proton-CachyOS-latest |
| Base | GOG release = Armada II + patch 1.1 |
| GPU | RX 5700 XT (Navi 10 / gfx1010), 8 GB — Vulkan fine, **ROCm effectively unsupported** |

## Heroic configuration

**Heroic rewrites `1174788223.json` when it exits.** Only edit it while Heroic is fully
closed, or the change is silently lost.

The environment the game runs with:

    WINEDLLOVERRIDES = winmm=n,b;d3d8=n,b

`n,b` = native first, then builtin. Wine searches the application directory before the
system directory, so this is what makes Wine load the game-directory `winmm.dll` (the ASI
loader) and `d3d8.dll` (Display8) instead of its own. Until 15.6.0 it also held `d3d9=n,b`
and `autoInstallDxvk` was false, for the chain DXVK's `d3d9.dll` was in; `platform/d3d8-chain.py
--retire` takes the `d3d9` entry out (below), and `autoInstallDxvk` is left as the player has it.

## Launch time

Measured 2026-10-10 on the bench (umu as Heroic runs it, warm caches) and in Heroic's
own log (`~/.local/state/Heroic/logs/heroic.log`). Seconds from the launcher's start:

| s | What |
|---|---|
| 0 → ~1 | `umu_run.py` starts; asks `repo.steampowered.com` about the Steam runtime |
| ~1 → ~3 | pressure-vessel builds the container (`srt-bwrap`, the capsule libraries) |
| ~3 → ~4.7 | Wine boots the prefix: `wineboot`, `services`, `explorer`, then `Armada2.exe` |
| ~4.7 → ~4.8 | every ASI plugin loads (all of them in 60 ms) |
| ~4.8 → ~5.9 | the game's own start until its first frame; the D3D device at ~6.9 |
| ~8.3 | the loading screen (a direct map launch); with the intro, the Activision reel ~2.2 s after the exe |

So the game and the plugins are about two seconds of it; the rest is umu, the Steam
runtime and Wine. **Heroic adds more before any of that**, and that is most of the wait
for the first splash: before it starts the game it runs umu twice more, each a whole
container and Wine boot to itself:

- `createprefix` (`verifyWinePrefix`), on every launch: 4–9 s measured on the bench.
- `cmd /c winepath -u C:\ProgramData\GOG.com\Galaxy\redists\GalaxyCommunication.exe`,
  to place GOG's Comet (Galaxy achievements and presence): ~4 s. Only while
  `experimentalFeatures.cometSupport` is not false, a **global** Heroic setting
  (Settings → Advanced → experimental features), so it is the player's choice and no
  script here touches it. Armada II has no Galaxy achievements.

Heroic's log put the two at 7–20 s per launch (2026-10-10: 8, 17 and 20 s from "Launching"
to the game's own start).

**Every umu run asks Steam's repo about the runtime, even with `UMU_RUNTIME_UPDATE=0`**
(umu 1.4.4 fetches `latest-public-beta.txt` first and checks that variable only to
decide whether to act on the answer). With umu's defaults, a 5 s timeout and 3 retries,
a slow server costs up to ~15 s per run, and Heroic makes three runs. Measured: the request alone took
10.4 s on a day the repo was answering 503; one `createprefix` with the server silent
(a black-hole proxy) took 19.0 s, and 9.9 s with `UMU_HTTP_TIMEOUT=3 UMU_HTTP_RETRIES=0`.
A failed request is caught (`log.exception`) and the launch goes on with the installed
runtime; one that answers inside 3 s still updates it. 1 s was too tight: the request
failed on a normal day. Heroic passes the game's `enviromentOptions` to all three runs
(`runWineCommand` → `setupEnvVars`), so **`platform/heroic-umu.py --on`** sets the two
there. It is a tool for this machine, run by hand: neither `./install` (which never edits
Heroic's config, and which the bench runs into clones) nor the release installers set
them, since a player's umu, network and runner are not ours to tune.

Proton-CachyOS's protonfixes also fetches an FSR4 upscaler manifest from github.io on
every run (`upscalers.py`, enabled unconditionally): 0.3 s normally, 3 s when the
network misbehaves. It has no switch; GE-Proton does not do it.

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

## The d3d8 slot

**What `./install` leaves** (platform 4.0.0; before it, a d3d8to9 chain on DXVK):

    Armada2.exe
      -> <game dir>/d3d8.dll          Display8 (display/): draws on Vulkan or Direct3D 12 itself
      -> (bail-out) the system's d3d8.dll, found with GetSystemDirectory: Wine's builtin

Nothing stands behind Display8: no `d3d8_real.dll`, no `d3d9.dll`, no DXVK. Its Vulkan and
Direct3D 12 devices are its own, and when neither can start the game draws through the
system's own `d3d8.dll` without the plugins (`display/README.md`, "The bail-out"). The
`d3d9=n,b` override is not needed either; the override that remains is
`WINEDLLOVERRIDES = winmm=n,b;d3d8=n,b`, so that Wine loads the game directory's ASI loader
and Display8 first (Wine searches the application directory before the system directory).

**What was here before, and why it went.** Until 15.6.0 the slot held crosire's d3d8to9
(vendored in `vendor/d3d8to9-1.16.0/`), behind Display8 as `d3d8_real.dll`, on DXVK 3.1.1's
`d3d9.dll` (`vendor/dxvk-3.1.1/`), because Render3D and the UI drew through the Direct3D 9
device behind it (`D3D9.md`, `d3d9/d3d9dev.h`, `d3d9/hlsl.sh`) and `dxvk.conf` set the
sampler filtering. Display 1.0.0 and gpu removed Direct3D 9; the sampler settings are
`Display8.ini [Samplers]`; the vendored binaries, `D3D9.md`, `d3d9/` and `dxvk-logging.py`
(DXVK's own logs and HUD, which nothing in the chain could now show) are deleted. Their
hashes stay in `retire/known.txt` so that an old install is recognised.

`platform/d3d8-chain.py` identifies and retires what is left:

    platform/d3d8-chain.py --status     identify the live slot, every link by hash
    platform/d3d8-chain.py --retire     what ./install runs: take the retired chain out of the game directory
    platform/d3d8-chain.py --retire --heroic   and `d3d9=n,b` out of Heroic's overrides (Heroic closed)
    platform/d3d8-chain.py --revert     the GOG release as shipped, Heroic managing DXVK again

`--retire` removes from the game directory, each only when its **hash** says it is ours: the
vendored d3d8to9 as `d3d8.dll` (GOG's `d3d8.dll`, which an install kept as
`d3d8.dll.gog-backup`, returns in its place; with none kept the slot is empty and the
system's is the game's), DXVK 3.1.1's `d3d9.dll`, a `d3d8_real.dll` that is the vendored
d3d8to9, and `d3d8.dll.a2display-backup` when it holds the retired chain (it becomes GOG's
file, else it goes: that is what `a2mod stock` puts back). With `--heroic` it also takes
`d3d9=n,b` out of Heroic's `WINEDLLOVERRIDES` **only when the rest of the entry is ours**
(`winmm=n,b` and `d3d8=n,b`); a player's own overrides are theirs, and the entry is left
whole with a note. `./install` leaves Heroic alone (it runs in bench clones too, and the
config is the player's), so the entry is a one-time `--heroic` run with Heroic closed (a
leftover `d3d9=n,b` is harmless: the game directory has no `d3d9.dll` for it to find); the
graphical installer does it for a player. With Heroic open or no config there is a note and
nothing changes. `retire/retire.sh --chain` removes the same
files for the other installers and also a marked `dxvk.conf`, after carrying its sampler
values into `Display8.ini [Samplers]`. A file that is none of these is the player's and
stays. `--use` and `--upgrade` are gone; asking for them says so.

`--revert` returns to the GOG release as shipped: GOG's translator in `d3d8.dll`, no
game-directory `d3d9.dll`, the `d3d9` override out, `autoInstallDxvk` back on:

    Armada2.exe
      -> <game dir>/d3d8.dll      GOG d3d8to9           1101824
      -> syswow64/d3d9.dll        Heroic's DXVK d3d9, or Wine builtin -> wined3d

### What stock means for `d3d8.dll`

`a2mod stock` puts back whatever `display/install` found in the slot when it first put
Display8 there (`d3d8.dll.a2display-backup`): GOG's translator on a GOG install that never
had the chain, and **no file at all** when the slot was empty, in which case Display8's
`d3d8.dll` is set aside with nothing in its place and the game runs on the system's, as it
does after the bail-out. `a2mod` files Display8's `d3d8.dll` under the `display` layer by
content (the message every build carries), so a clone that holds GOG's file alone is never
mistaken for ours. The snapshot layout is unchanged.

### Traps

- **Never identify a DLL in this slot by size.** `syswow64/d3d8.dll` at 320548 bytes
  was once recorded as "DXVK's exact size"; it is byte-identical to *Wine's builtin*,
  and on the strength of that the game ran through wined3d/OpenGL while every
  `dxvk.conf` key was silently ignored and the DXVK HUD never appeared. DXVK's d3d8 is
  ~1.66 MB (Proton's 1658894, Heroic's 3.1.1 1687566). `d3d8-chain.py` hashes each link
  against the candidates actually present on the machine and names what it found,
  reporting `UNKNOWN` rather than guessing.
- **The prefix is not a durable place for a DLL.** Proton's `default_pfx` holds
  `syswow64/d3d8.dll` and `d3d9.dll` as **symlinks** to its own Wine builtins and
  restores them on prefix sync. The game directory is durable: nothing manages it, and
  Wine searches it first.
- **`autoInstallDxvk` true redeploys Heroic's DXVK into the prefix on every launch** —
  `d3d8.dll`, `d3d9.dll`, `d3d11.dll` and `dxgi.dll` sharing one mtime is the sign. That
  is it working as designed. It matters only to the bail-out's look (the system's
  Direct3D 8 on the prefix's Direct3D 9), never to Display8's own devices.

## Hyprland / window management

**Symptom:** the game opened a second window; the settings menu drew in one while input
stayed grabbed by the other, so the menu was visible but could not be clicked. Focus
kept snapping back to a black fullscreen frame.

**Cause:** every menu, the in-game Options included, is a `WS_POPUP` dialog, which Wine
turns into a second X11 window that Hyprland tiles and focuses like a new application.

**Fix: `Menus.asi` with `Embed=1`** (the default it ships with) re-creates each
menu as a child of the game window, so the game is one OS window from launch to exit.
See `ui/MENUS.md`, "What the shell is, in the engine" (the UI's host does what `Embed=1` did).

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

  **`platform/stop-game.sh` does this properly** — it also kills the Wine helpers
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

    # what is in the d3d8 slot right now? by hash, never by size. Heroic's DXVK builds
    # (tools/dxvk/dxvk-*/) are recognised from the native and the Flatpak Heroic
    platform/d3d8-chain.py --status

    # did Heroic redeploy DXVK into the prefix? these four sharing an mtime means yes
    cd "<prefix>/pfx/drive_c/windows/syswow64" && stat -c '%n %y' d3d8.dll d3d9.dll d3d11.dll dxgi.dll

    # what does the exe actually import?
    objdump -p Armada2.exe | grep -i 'DLL Name'

    # is the game really running?  NOT `pgrep -x Armada2.exe` -- Wine calls it "Main"
    ps -eo pid=,args= | awk '/Armada2\.exe/ { print $1 }'

    # stop it, its Wine session, and any stale audio node it left behind
    platform/stop-game.sh

    # did Menus.asi load, and what did it patch?
    cat "<game dir>/Menus.log"
