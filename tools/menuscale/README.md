# MenuScale — making Armada II's menus fill the screen

The menus render in the top-left 800x600 of a 3440x1440 screen. This makes them fill
the height, centred, at 4:3, with black pillarboxes. Nothing is stretched.

**Confirmed in game**, on the main menu, at 3440x1440.

## Why no config can fix this

Two separate facts, both measured, and the second is the one that matters:

1. **The shell is a fixed-pixel GDI UI.** Every menu — main menu, options, load/save,
   campaign select, multiplayer — is a Win32 dialog drawn with GDI at hard-coded
   coordinates, from 800x600 8-bit BMPs in `bitmaps/`. `Armada2.exe` imports
   `GDI32!BitBlt` and **no `StretchBlt`, no `SetWorldTransform`**: there is no scale
   factor anywhere in the shell to change. It also never goes near Direct3D, so the
   d3d8 proxy, DXVK and `dxcfg.ini` cannot touch it either. `misc/gui_*.cfg` is the
   *in-game HUD* canvas — a different system, already handled by
   `tools/ui-widescreen.py`.

2. **The engine asks for an 800x600 screen while the menus are up.** This is the real
   reason, and it is not what it looks like. `GetSystemMetrics(SM_CXSCREEN)` returns
   **800** at the main menu, and the game's own window is 800x600 at 0,0. The shell is
   not drawing small inside a big screen — it is *filling a small screen*, which Wine
   then parks in the corner of the virtual desktop because a tiling compositor will not
   let the desktop window shrink to match. The mode is hard-coded:

       ST3D_GraphicsEngine::SetActiveDisplay_Internal
         push 0x10 ; push 600 ; push 800
         call ST3D_DisplayDevice::FindDisplayMode(int w, int h, int bpp)

   So there was nowhere to scale *into*. Raising that mode is half the fix; on its own
   it changes nothing, because the shell would still draw its 800x600 in the corner.

## What it does

`MenuScale.asi` is loaded by the Ultimate ASI Loader (`winmm.dll`) — the same loader
`STA2WidescreenPatch.asi` uses, so it needs no setup beyond the `winmm=n,b` override
that is already in `WINEDLLOVERRIDES`.

1. **Raises the front-end display mode** to the desktop size, by patching the two
   `SetActiveDisplay_Internal` sites *in memory*. `Armada2.exe` on disk is never
   modified.
2. **Scales the shell** into the room that creates. Dialogs come in two kinds and need
   opposite treatment:
   - **full-screen dialogs** (`init_screen_pos` sizes them to the whole client area) —
     geometry left alone, content drawn into an 800x600 offscreen DC and `StretchBlt`
     into the window, fitted and centred, surround painted black. *This is where the
     main menu actually lives.*
   - **design-sized dialogs** (fixed sizes at fixed offsets, e.g. `353x293 @140,70`) —
     position and size scaled, content stretched to fill.
3. **Maps the mouse back** by subclassing each dialog's window procedure, so
   `WM_MOUSE*` lParam and `GetClientRect` agree with what was drawn.

   It has to be a subclass, not a `GetMessageA`/`PeekMessageA` hook. Every shell
   screen is a **modal** dialog — `DialogBoxParamA` has 36 call sites in
   `Armada2.exe` (`do_mainMenu` among them) against 2 for `CreateDialogParamA` — and
   a modal dialog is pumped by user32's own internal loop, which dispatches straight
   to the dialog procedure and never hands the message to the application. The
   message-loop hook scaled the picture perfectly and left every click *and hover*
   registering on the stock 800x600 position.

Only class `#32770` (Win32 dialog) windows are touched. The 3D window is not a dialog,
so gameplay, the HUD and the Bink videos are untouched by construction.

## One window: `Embed=1`

**Confirmed in game by the user (2026-09-23): "much improved".** Also checked in test launches: the game is one OS window from
launch to exit, main menu, Options and its nested screens, and the in-mission
Options menu, Save Game and Return to Game included.

Every menu, the in-game ones included, is
`DialogBoxParamA(shell_hInstance, id, <3D window>, proc, lp)` with a `WS_POPUP`
template (every template in `.rsrc` but one). `do_escapeMenu` is the same call
with the same template as the main menu, 291. An owned popup is a separate
top-level window. On Windows that is invisible; under Wine it is a second X11
window that Hyprland tiles and focuses like a new application. Measured without
Embed: Wine lists two visible top-level windows, `hyprctl clients` two
`steam_proton` clients, and opening Options re-tiled **both** to 3410x1378
@15,47, the screen less gaps. Injected clicks usually did not reach the popup
at all.

`Embed=1` hooks `DialogBoxParamA`, rewrites the template `WS_POPUP` ->
`WS_CHILD`, and creates it with `DialogBoxIndirectParamA` so it becomes a child
of the game window. Wine then creates no second X window. Four things follow,
each handled, each measured:

- **Input.** `DialogBox` disables its owner *before* creating the dialog (Wine
  does this even for a `WS_CHILD` template), and hit-testing never descends into
  a disabled window, so the child would get no input. The top-level is
  re-enabled at `WM_INITDIALOG`. Modality is kept by a `WH_GETMESSAGE` filter
  that turns mouse and key input aimed outside the innermost menu into `WM_NULL`.
- **Z-order.** For a modal dialog Wine walks the owner up to its top-level, so
  a menu opened *from* a menu (Options -> Graphics Settings, template 1) is a
  **sibling** of the one that opened it, not its child. It was created **below**
  it and clipped away by `WS_CLIPSIBLINGS`. Each embedded menu is raised to the
  top at `WM_INITDIALOG`, which is where an owned popup always is.
- **Geometry and owner.** The game positions menus in screen coordinates via
  `GetWindow(hDlg, GW_OWNER)`. A child has no owner, so `GW_OWNER` is answered
  with the owner the game asked for, and positions are converted to
  parent-client coordinates just before the real `MoveWindow`/`SetWindowPos`.
- **Resizing.** A full-screen menu is sized once. When the game window is
  resized under it, as Hyprland does when the game drops fullscreen on focus
  loss, it used to hang off the bottom (Return to Game unreachable). Embedded
  full-screen menus now follow their parent's client area.

`CreateDialogParamA` (two sites, both the admiral's log, one already a
`WS_CHILD` template) is left alone. `Embed=0` restores separate windows; the
`DialogBoxParamA` hook then only logs.

**The cursor.** The 3D window's `WindowProc` answers every `WM_SETCURSOR` with
`SetCursor(NULL)` (`0x488881`), so the engine's sprite cursor can show in play. A child's
`DefWindowProc` asks its parent first, so once embedded, the arrow vanished over every
menu (measured: `GetCursorInfo` handle NULL). The dialog wrapper now sets the class cursor
of the window under the pointer itself, as a top-level dialog did. Checked on the main
menu and Options. The in-mission menu, and the sprite cursor returning after Return to
Game, are left to the user's play-through.

### Open: the Admiral's Log still shows a window inside the window

Reported in game: a border around the centre content. `AdmiralsLogDlgProc` builds its
panes itself. It calls `CreateDialogParamA(shell_hInstance, 0x123, ..., 
ScreenInformation::CallDialogProc)` at `0x5e3c04`, positioned via `ClientToScreen`
(it's modeless, so the `DialogBoxParamA` hook never sees it), plus two `CreateWindowExA`
(`0x5e394e`, and `0x5e3b7b` with style `0x5000000b`). Next step: hook
`CreateDialogParamA` the same way as `DialogBoxParamA` (WS_POPUP -> WS_CHILD, no modal
bookkeeping), then read the two `CreateWindowExA` styles.

## GetDC and ReleaseDC do not pair up in this game

`ShellButton::UpdateButton` (`0x5a4a10`) takes `GetDC(hDlg)`, draws the button,
and falls into **eight NOPs at `0x5a4adb`** where its `push; push; call
ReleaseDC` used to be. It was patched out, so every button redraw leaks the DC.

The first version of this plugin kept the real DC on a stack and presented only
when the outermost one came back. One leak pinned the stack, so **nothing drawn
after it ever reached the screen**: the Options screen showed its background
and no buttons, **with or without Embed**. After eight leaks the game was handed
real DCs and drew 1:1 in the corner. This predates Embed and affected every
screen built from `ShellButton`s, which is most of them past the main menu.

Now no real DC is held. `GetDC` hands out the design surface and marks it dirty,
`ReleaseDC` presents at once through a DC of the plugin's own, and a 30 ms
thread timer presents whatever is still dirty. The timer is what catches the
leaks. `Trace=1` logs the first paint events per dialog, which is how this was
found.

## Use

    tools/menuscale/build.sh              # clang + lld-link, 32-bit PE, no CRT
    tools/menuscale/install.sh            # copy .asi + .ini into the game directory
    tools/menuscale/install.sh --mode 0   # install, but only observe and log
    tools/menuscale/install.sh --remove   # take it out again

`install.sh --remove` restores stock behaviour exactly: the plugin only ever *adds*
`MenuScale.asi`, `MenuScale.ini` and `MenuScale.log` to the game directory, so there is
no backup to keep and nothing to revert.

`MenuScale.ini` keys are documented in the file. The ones worth knowing:

| key | meaning |
|---|---|
| `Mode` | `0` observe and log only, `1` centre at stock size, `2` centre and scale |
| `IntegerScale` | `1` snaps to a whole factor (2x = 1600x1200) — crisper, leaves a band |
| `Smooth` | `1` HALFTONE, `0` nearest neighbour |
| `RaiseShellMode` | `0` leaves the engine's 800x600 front-end mode alone |

**If anything misbehaves, `Mode=0` is a safe diagnostic and `--remove` is a full
uninstall.** `MenuScale.log`, beside `Armada2.exe`, records what it patched and every
dialog it moved.

## Command-line switches

`Armada2.exe` carries its own switch table, found in the binary:

    off shelltest synchost host connect create join name email pass game spass gid
    netshell noai allowai nods win window deepspace full fullscreen wire wireframe
    multi multimon res resolution bpp dev device pri primary sec secondary aux
    auxilliary demo vmcheck resave nointro audioasserts edit type gamebalance
    loadsavetest loadsavetext trekphysics nodl settingsFile

**`-nointro` skips `Intro.bik` (35 MB) and the three logo reels.**

**The leading `-` or `/` is not optional.** The parser checks the first character of
each token and only then matches the switch table; anything else is stored as the
**mission name**. A bare `nointro` therefore starts a match on a mission that does not
exist — which presents as a game with a HUD and no map, not as a rejected argument.

## Test harness

`probe.exe` (`run-probe.sh`) runs inside the game's own Wine session and is how
every Embed claim above was checked without a human at the mouse:

    run-probe.sh tree          top-level windows and every descendant
    run-probe.sh click X Y     SetCursorPos + SendInput, routed by wineserver
    run-probe.sh key VK        with the scancode: in a mission the game reads
                               the keyboard by scancode, and a VK-only Esc never
                               reaches the menu binding (Space still skipped the
                               cutscene, which is what made this confusing)
    run-probe.sh post X Y      post a click straight to the window under X,Y

`capture.sh` takes `NOSHOT=1` to launch without photographing, and
`A2_ARGS="-nointro a2_fed01.bzn"` starts a mission directly. **The `.bzn` is
required**: without it the name does not resolve and you get the HUD with no
map. Missions open with an in-engine cutscene, and Space skips it.

`run-wine.sh` launches the game with Proton's bundled Wine directly against the existing
prefix, skipping Heroic and the proton wrapper. It is the only launch path that prints
Wine's diagnostics to a terminal — which is how `Unable to load MenuScale.asi. Error:
317` turned out to be

    wine: Call from ... to unimplemented function KERNEL32.dll.GetModuleFileNameA@12

i.e. `llvm-dlltool` needs `--kill-at`, because the `.def` files carry stdcall-decorated
names but kernel32/user32/gdi32 export them undecorated.

`capture.sh` runs the game on its own Hyprland workspace and photographs it; `shot.sh`
grabs one frame and puts the desktop back where it was; `stop-game.sh` shuts the game
and its Wine session down properly.

Two traps that cost real time here, both recorded so they are not rediscovered:

- **Wine reports the process `comm` as `Main`, not `Armada2.exe`.** So
  `pgrep -x Armada2.exe` matches nothing even while the game is plainly running and
  writing to `MenuScale.log`. This is not cosmetic: it made the harness report "game
  DOWN" for a game that was up, and it made every run's `pkill -x Armada2.exe` cleanup
  a no-op, so instances accumulated — seven of them over half an hour, each with no
  window, each still holding a PipeWire stream and audibly playing the menu music.
  `pkill -f Armada2.exe` does match, but also matches its own shell (see `SETUP.md`),
  so **`stop-game.sh`** collects PIDs with `ps` and kills them individually. It also
  drops stale PipeWire nodes, which survive the process and stay in state `running`.
  Wine helpers are matched by `WINEPREFIX` out of `/proc/<pid>/environ`, so it cannot
  take down an unrelated Wine application.
- **Hyprland 0.56 routes `hyprctl dispatch` through Lua**, so
  `hyprctl dispatch movetoworkspacesilent 5,class:...` is a Lua *syntax error* — and a
  silent one if stderr is redirected. Use
  `hyprctl repl 'return hl.dispatch(hl.dsp.window.move({ workspace = "5", follow = false, window = "class:^(steam_proton)$" }))'`.
  `hyprctl keyword monitor ...` is likewise rejected ("can't work with non-legacy
  parsers"); use `hl.monitor({ ... })` through `hyprctl repl`.

## Building

No MSVC and no mingw is needed. The plugin declares the few Win32 prototypes it uses
itself and links against import libraries generated from the `.def` files with
`llvm-dlltool`, with **no CRT at all** (`/nodefaultlib`). That is why the source uses
32-bit integer arithmetic throughout — there is no `_alldiv` or `_ftol` to call — and
why it defines its own `strlen`/`memcpy`/`memset`: clang lowers a byte loop and a struct
assignment to those even under `-ffreestanding -fno-builtin`.

## Known issue: menu animations stall while the cursor is moving

**Reported in game, not yet diagnosed.** The animated elements on the menu screens
(`bitmaps/main/MainBk_flare.bik`, `singleplayer.bik`, `Multiplayer.bik`) appear to stop
while the mouse is being moved, and resume when it stops.

The leading hypothesis is cost per mouse-move, and it is a hypothesis — **nothing here
has been measured.** `present()` runs on every outermost `ReleaseDC`, and a mouse-move
makes the shell redraw the widget under the cursor, so each `WM_MOUSEMOVE` costs a
**full-frame 800x600 -> 1920x1440 `StretchBlt` in HALFTONE**, not a redraw of the part
that changed. A stream of mouse-moves could then starve whatever drives the animation.

Three cheap experiments, in the order that actually discriminates:

1. **`install.sh --remove`, then watch stock.** Establishes whether this is a regression
   at all. A 2001 GDI shell flooding its own message queue on mouse-move is entirely
   plausible stock behaviour, and if it stalls without the plugin there is nothing here
   to fix.
2. **`Mode=1`** (centre, do not scale). No DC redirection happens at all in that mode,
   so `present()` never runs. If the stall survives `Mode=1`, the blit is not the cause.
3. **`Smooth=0`** (COLORONCOLOR instead of HALFTONE). Much cheaper resampling. If that
   alone fixes it, the cost hypothesis is right and the fix is to make `present()`
   cheaper rather than rarer.

If it is the cost, the real fix is to stop presenting a whole frame per `ReleaseDC` —
either coalesce (mark dirty, present at most once per timer tick) or scale only the
rectangle that changed. Both are more bookkeeping than the current code has, which is
why neither was done up front.

## Not done

- **Confirmed rendered:** the main menu, the single-player/campaign screen, Options,
  Graphics Settings, and in a mission the Options menu and Save Game. Multiplayer
  has not been seen scaled.
- **Hover and click are confirmed to land correctly**; nothing has been measured about
  how *fast* they are. See the animation stall above.
- **Real child controls** (edit boxes, list boxes — the multiplayer screens use them)
  are separate HWNDs that Windows draws itself. They are not covered by the offscreen
  redirect and will sit unscaled. The main screens are custom-drawn `ShellButton`
  bitmaps and are fine. **Seen:** Save Game's name field (`Edit`, 476x28 at design
  176,551) draws at 1:1 at its unscaled position. The user confirms this in game for
all text boxes, and they are still usable. Embed does not change this. The fix is
  to map child-control geometry through the same fit, plus a scaled font.
- **Menus drawn inside the renderer** (the Direct3D route) was considered and
  deferred. Embed gets one OS window without it, and the GDI child draws correctly over
  the DXVK surface because the game loop is blocked while any menu is open.
