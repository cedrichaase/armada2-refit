# Menus — Armada II's front-end menus

`Menus.asi` fixes the game's GDI menu shell: it raises the front-end display mode,
scales the menus to fill the screen, embeds them in the game window (`Embed=1`),
composites hi-res backdrops behind them and removes the menu-switch flash. It was
`MenuScale.asi` until menus 2.0.0; the two must never be installed together, so
`install.sh` removes the old files and `Menus.asi` stands down if it finds one.

Stock, the menus render in the top-left 800x600 of a 3440x1440 screen. `Menus.asi`
makes them fill the height, centred, at 4:3. Nothing is stretched: the pillarboxes are
black, or show the backdrop plate where one is built.

## Why no config can fix this

Two separate facts, both measured, and the second is the one that matters:

1. **The shell is a fixed-pixel GDI UI.** Every menu — main menu, options, load/save,
   campaign select, multiplayer — is a Win32 dialog drawn with GDI at hard-coded
   coordinates, from 800x600 8-bit BMPs in `bitmaps/`. `Armada2.exe` imports
   `GDI32!BitBlt` and **no `StretchBlt`, no `SetWorldTransform`**: there is no scale
   factor anywhere in the shell to change. It also never goes near Direct3D, so the
   d3d8 proxy, DXVK and `dxcfg.ini` cannot touch it either. `misc/gui_*.cfg` is the
   *in-game HUD* canvas — a different system, already handled by
   `hud/ui-widescreen.py`.

2. **The engine asks for an 800x600 screen while the menus are up.** This is the real
   reason, and it is not what it looks like. `GetSystemMetrics(SM_CXSCREEN)` returns
   **800** at the main menu, and the game's own window is 800x600 at 0,0. The shell is
   not drawing small inside a big screen — it is *filling a small screen*. The mode is
   hard-coded:

       ST3D_GraphicsEngine::SetActiveDisplay_Internal
         push 0x10 ; push 600 ; push 800
         call ST3D_DisplayDevice::FindDisplayMode(int w, int h, int bpp)

   So there was nowhere to scale *into*. Raising that mode is half the fix; on its own
   it changes nothing, because the shell would still draw its 800x600 in the corner.

## What it does

`Menus.asi` is loaded by the Ultimate ASI Loader (`winmm.dll`) — the same loader
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

The game is one OS window from launch to exit: main menu, Options and its nested
screens, and the in-mission Options menu, Save Game and Return to Game included.

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
of the game window. Wine then creates no second X window. Five things follow,
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
  loss, it would hang off the bottom (Return to Game unreachable). Embedded
  full-screen menus follow their parent's client area.
- **Keyboard focus.** The game reads keys only as `WM_KEYDOWN`/`UP` on the 3D
  window (`ProcessKeyboardMessages`), so that window must hold the focus. A popup
  gives it back as a side effect of re-activating its owner when it closes; a child
  never deactivates anything, and Wine leaves the focus `NULL` after it is gone.
  Every keystroke is then dropped: Esc opens the in-mission menu once and never
  again, while HUD clicks, routed by position, still work. So the focus is put back
  where it was when the menu returns. That also sends the `WM_SETFOCUS` on which
  the game runs `ClearKeyboardState`, since the Esc key-up went to the menu and
  not to the game. A menu opened from a menu that held no focus leaves it alone.
  (`probe focus` shows the active and focus windows.)

`CreateDialogParamA` is embedded the same way, without the modal bookkeeping. Its
one call site is the Admiral's Log (below). `Embed=0` restores separate windows; both
hooks then only log.

**Esc closes the in-mission menu (`EscapeReturns=1`).** In stock, Esc opens the
in-mission menu and does nothing inside it: `EscapeMenuDlgProc` (0x5cddb0) has no
`WM_COMMAND` case, so the `IDCANCEL` a dialog makes of Esc is ignored. Embedded, Esc
doesn't even reach the menu, which holds no focus. The modality filter catches a
fresh Esc press (not an auto-repeat) while that menu is the innermost one, and feeds
its procedure a left click on Return to Game. That runs the same sound, result code
and `EndDialog` as the mouse. The procedure and the button's rectangle (x 5, y 568,
172x22 in 800x600) are found by signature in `Armada2.exe` and read from its
operands; if either fails to match, the log says so and Esc stays stock. Esc inside
a nested menu, such as Graphics Settings, does nothing, as before. Needs `Embed=1`.

**The cursor.** The 3D window's `WindowProc` answers every `WM_SETCURSOR` with
`SetCursor(NULL)` (`0x488881`), so the engine's sprite cursor can show in play. A child's
`DefWindowProc` asks its parent first, so once embedded, the arrow would vanish over
every menu (measured: `GetCursorInfo` handle NULL). The dialog wrapper sets the class
cursor of the window under the pointer itself, as a top-level dialog does.

### The Admiral's Log

The log needs its own handling, because it is built from real windows where every
other screen draws `ShellButton` bitmaps into itself. Embedded naively, the score table
stays on top of every menu after the log closes, and the log's buttons sit 1:1 in the
top-left corner while its backdrop is scaled.

`AdmiralsLogDlgProc` (`0x5e22c0`) is opened by `do_admiralsLog` as an ordinary
`DialogBoxParamA(0x880)`, full-screen, so the log itself was always letterboxed. In
its `WM_INITDIALOG` it then creates:

- **Eight tab panes**: `CreateDialogParamA(shell_hInstance, 0x123, hDlg,
  ScreenInformation::CallDialogProc)` at `0x5e3c04`, then `SetWindowPos` in screen
  coordinates (`ClientToScreen(hDlg)` of `dialogWinRect`). Template `0x123` is a
  plain `WS_POPUP` with no controls, the main menu's. **The game never destroys the
  panes.** It relies on an owned popup dying with its owner. Once the log is a
  `WS_CHILD`, that breaks: a popup's owner is always a top-level window, so the panes
  were owned by the 3D window, outlived the log, and sat over every later menu. Each
  was also its own X11 window, which is the "window inside the window" border
  (Hyprland's frame).
- **The player list and the eight tabs**: `CreateWindowExA("button", ...,
  0x5000200b / 0x5000000b, x, y, w, h, hDlg)` at `0x5e394e` and `0x5e3b7b`, i.e.
  `WS_CHILD | WS_VISIBLE | BS_OWNERDRAW` at design coordinates. **Save** and **Done**
  are the same kind of button. The Ships and Battles panes add more (`0x5f7e4f`,
  `0x5e9b63`). A child window is not drawn through its parent's DC, so the design
  surface never saw them.

The fix, in `menus.c`:

- `CreateDialogParamA` goes through `child_template()` like `DialogBoxParamA`, so the
  panes are children of the log and die with it.
- A dialog whose anchor is a letterboxed dialog is fitted through *that dialog's*
  placement, not the screen's. It is the same answer while the game fills the
  screen, and the right one when Hyprland has re-tiled it.
- **Owner-drawn buttons** of any scaled dialog have their geometry mapped at
  `CreateWindowExA`/`MoveWindow`/`SetWindowPos`. Any created before their dialog was
  scaled are adopted from their current, still-design, position. Their pixels come
  from the parent's `WM_DRAWITEM`, which draws only through `DRAWITEMSTRUCT.hDC` and
  `.rcItem`. The subclass hands it a design-sized memory DC and stretches the result
  onto the button.
- **A stock bug the memory DC exposes.** The tab and Save/Done helper (`0x5e93a0`)
  ends by selecting the bitmap its own memory DC started with, the 1x1 default,
  into **`dis->hDC`** (`0x5e94d7`) instead of back into its memory DC. Into a window
  DC that fails harmlessly. Into ours it swapped the surface out after the drawing
  had landed, and those buttons came out as plain grey button face. `ctl_draw`
  re-selects its bitmap after the call. The player-list helper (`0x5e9510`) does not
  do this, which is how the two were told apart: `GetPixel` on the surface read a
  colour for one and `CLR_INVALID` for the other.

The log is scaled and centred with everything in place, from both Options and the end
of a mission. Tab clicks switch panes, and Done leaves exactly one dialog; after
aborting a skirmish, Done returns to a clean main menu.

## Edit boxes

The text fields, such as the player name on Multiplayer Connection (template 2162; an
`Edit` 399x32 at design 208,171) and Save Game's name (476x28 at 176,551), are real
`Edit` windows. The shell draws the frame around each into the dialog, which the design
surface scales. The edit itself is a child window that draws itself, so it sat 1:1 at
its design coordinates: at 21:9, in the top-left corner of the screen, in a font sized
for 800x600, and the scaled frame stayed empty.

The fix treats an edit like an owner-drawn button for geometry. Its design rectangle is
mapped through the dialog's fit when it is created, moved or adopted. Its pixels cannot
go through a surface, because it draws its own text, caret and selection and takes its
own clicks. So it keeps drawing itself, at the mapped size, with a font scaled by the same
factor:

- **The font.** The game gives the name field MS Sans Serif at -11 and Save Game's field
  Arial at -11 (both read out of Menus.log). `ctl_font` reads the font back
  (`WM_GETFONT`; the system font if there is none) and creates it at `lfHeight × dh /
  600`. The edit is subclassed so that a later `WM_SETFONT` is scaled too, and
  `WM_GETFONT` still returns the game's own font.
- **Not the raster face.** MS Sans Serif is a raster font. Asked for -26 at 3440x1440,
  it came back at its largest bitmap: "Player" measured 21 px tall on screen against the
  41 that 2.4x stock wants, and hardly bigger than at 1600x1200. The scaled font asks for
  **Microsoft Sans Serif**, the TrueType successor drawn to the same metrics, with
  `OUT_TT_ONLY_PRECIS`. "Player" then measures 85x37 on screen, against 84 wide for stock's
  35 px × 2.4.
- **Colour** needs nothing. The shell answers `WM_CTLCOLOREDIT` on the dialog
  (`Screen::ControlColorEdit`, `0x5a3360`: its text colour, a transparent background
  mode, a stock brush), and that reaches the game through the dialog subclass as before.
  The dialog has `WS_CLIPCHILDREN`, so presenting the design surface never paints over
  the edit.

The game places the name field twice. It creates the field somewhere else (the log shows
`adopted edit 149x36 @180,37`), then moves it into the frame with `MoveWindow` in design
coordinates, which `ctl_moved` maps. `testbench/scenarios/multiplayer-name.md` checks
that "Player" lies inside the field at stock's size, and that a name typed after
clicking into the field shows up there.

## Labels that run into the art: `[Labels]`

On Game Setup (Instant Action, and every multiplayer setup: all of them are template
2096), "Shroud Off, Fog Off" and "Random Placement" run over the left border of the
minimap's frame, and under the map once it shows. This is stock. It is there at 800x600
without the plugin, and the face is the game's own: the shell asks for Arial Bold
(`ShellFonts` makes it at cell heights 8 to 20), and the Arial Bold in Proton's prefix is
Liberation Sans under that name, drawn to Arial's metrics.

Every shell label is one `DrawTextExA`. `ShellButton::DrawLabelText` passes
`DT_SINGLELINE | DT_VCENTER | DT_NOCLIP` (`0x124`, or `0x125`/`0x126` centred or
right-aligned), so text wider than its rectangle is drawn on past it. **The rectangles
cannot say where a label must stop.** Logged on this screen, the options' rectangles
are 87 px, and nearly every label in the shell overruns its rectangle harmlessly
("Resources: Normal" is 101 px in 87). "Shroud Off, Fog Off" meanwhile asks for 136 px,
across the frame. Fitting labels to their rectangles would squeeze labels that touch
nothing and miss the one that collides. The limit is in the art, so it is configured
there:

- `[Labels]`, `N=template,x,y,w,h` in design pixels. A left-aligned, single-line label
  whose rectangle starts inside the box, while that template is the innermost menu, is
  kept inside the box's right edge. The template is the one `DialogBoxParamA` was
  last asked for and has not yet returned from (every menu is modal).
- The shipped box is `2096,520,130,95,125`. The frame's border is at x 618–621 (read off
  stock at 800x600), and the options' labels start at 523, so they get 92 px.
- **How a label is fitted.** It is drawn in a narrower cut of its own font (`lfWidth`
  below `tmAveCharWidth`), then with `ExtTextOutA` and per-character advances scaled so
  the last few pixels come out of the spacing. One step of `lfWidth` is ~15% at these
  sizes (7 → 6 for Arial Bold at 12). Taking the cut that fits outright put "Random
  Placement" at 81 px in 92 of room, visibly narrower than "SELECT MAP" above it. So the
  widest cut within 8% of the room is taken, and the spacing does the rest: 95 → 92 and
  96 → 92. The vertical position is `DrawText`'s own for `DT_VCENTER`.
- A label that fits, a label outside every box, anything with `&` (a mnemonic prefix),
  and every other flag combination go to the real call. `LabelFit=0` removes the hook.

**The minimap preview is blank while "Minimap Hidden" is set.** That is a game rule
(the players do not see the map), and the preview follows it, observed on the bench:
choosing a map changes the name and player count but not the frame. Clicking the option
reveals the map, and the option then disappears. It is one-way in stock too, and only
the host can do it. Nothing here changes that.

## GetDC and ReleaseDC do not pair up in this game

`ShellButton::UpdateButton` (`0x5a4a10`) takes `GetDC(hDlg)`, draws the button,
and falls into **eight NOPs at `0x5a4adb`** where its `push; push; call
ReleaseDC` used to be. It was patched out, so every button redraw leaks the DC.

So nothing may wait for a `ReleaseDC`. Keeping the real DC on a stack and presenting
only when the outermost one comes back fails at the first leak: the stack is pinned,
**nothing drawn after it reaches the screen** (Options shows its background and no
buttons), and after eight leaks the game is handed real DCs and draws 1:1 in the
corner — on every screen built from `ShellButton`s, which is most of them past the
main menu.

So no real DC is held. `GetDC` hands out the design surface and marks it dirty,
`ReleaseDC` presents at once through a DC of the plugin's own, and a 30 ms
thread timer presents whatever is still dirty. The timer is what catches the
leaks. `Trace=1` logs the first paint events per dialog, which is how this was
found.

## The 1:1 flash between menus: `Underlay=0`

With `Underlay=1` (stock), switching between the main and single-player menus shows
an 800x600 picture 1:1 in the top-left corner for 100–120 ms. Measured off a 60 fps
recording:
- Main → single player showed the main menu for 2 frames, then single player's bare
  background for 4.
- The way back showed plain black for 7.

None of it passes through a dialog, so nothing above could scale it. The source is the
shell's own anti-flicker code:
- **`SetCurrentBackground`** (`0x6083f0`) keeps an 800x600 copy of each screen's
  background. That copy is either the stock BMP or a `BitBlt` snapshot of the dialog. The
  snapshot is black when it is taken before the dialog has drawn. The function then draws
  the copy onto the **3D window**, the dialogs' parent, at 0,0 through
  `DrawTransparentBitmap` (`0x608100`).
- **`SnapShotBackground`** (`0x608530`) draws it there again. Every dialog's close path
  calls it just before `CleanCurrentBackground`.

At 800x600, the parent already held the next screen when a dialog went away. Scaled, that
draw is the flash. Without it, the parent keeps the last scaled frame, and one menu cuts
straight to the next.

Only the two `call`s onto the parent are NOP'd, in memory and matched by signature. Both
sites must match and both must call the same function, or nothing is patched. The copy
itself is still made, because seven sites in `ShellButton` and its siblings read it to
restore the background under a button. The caller pops the arguments (`add esp,0x18`), so
the stack is unchanged. `Underlay=1` restores stock.

## Backdrops: hi-res art, and the pillarboxes filled

The main menu's background is upscaled 4x and outpainted to 2.4:1. The pillarboxes
show the outpainted sides, and the centre shows the upscale wherever the shell is
still drawing the stock background. The campaign selection screen has a plate too
(below).

**Why the art cannot simply replace the BMP.** The shell draws each screen at 1:1
into the 800x600 design surface: first the stock background, then buttons, hover
states, text and Bink animations on top. A bigger `mainbkgr.bmp` would still be drawn
800x600 of it. So Menus.asi composites the picture itself, per frame, from the
frame the shell drew:

- **Which screen:** a 40x30 grid of the frame is sampled against each stock BMP
  listed under `[Backdrops]`. The listed screen that matches exactly (within 2/255)
  at 50% or more of the points is the one on show.
- **Sides:** these come from the plate `Menus/<same name>.bmp`, scaled so its
  centre 4:3 lands exactly on the stretched design area.
- **Centre:** `out = stretch(frame) + w · (plate − stretch(stock))` per pixel, with
  `w` falling linearly from 1 to 0 as the frame's pixel departs from stock by up to
  `DetailTolerance` (48). On untouched background the stretches cancel and the
  result *is* the plate. Under an opaque overlay, such as a hover highlight or text
  100+ off stock, it is the stretched frame exactly as before.

**It is a weight and not a mask because of the flare.** `MainBk_flare.bik` is not
drawn over the logo. It *is* the logo band: 800x145 at y 210, with the background
baked in, through a lossy codec. It sits a mean 8/255 off stock, and 94% of its pixels
are within 24. An exact-match mask would have left the logo, the thing anyone looks
at, as the one low-res band on the screen. With the weight, the band measures
mean |Laplacian| 56.4, against 60.6 for the plate and 35.7 for a plain stretch.

**The weight carries codec offsets through, hence `NoiseFloor`.** The hover Binks
(`singleplayer.bik` 320x168 at (14,31), `InstantAction.bik` 320x200 at (480,0)) also
bake the background in, and theirs decodes darker than `mainbkgr.bmp`: -1.7/-1.5/-2.0
per channel, uniform out to the rectangle's edge. `stretch(frame) + w·(plate − stock)`
reproduces that as a plate 2/255 darker, so the whole rectangle dimmed on hover,
visibly when toggling. A frame pixel within `NoiseFloor` (6) of stock is taken
as stock, ramping back to the frame by 12: the offset falls to -0.2, the background
there is the plate exactly, and the art, 16+ off, is untouched. `bd_soften` continues
the snapped edge rather than the raw one.

**Cost, measured.** Doing the whole centre per present (a GDI `HALFTONE`
`StretchBlt` of the frame plus the composite over 2.76M pixels) took **59 ms**, and
the flare runs at 30 fps. Two changes brought it to **7.8 ms**:

- **The stretch is done in C, bilinear, with fixed taps.** Every screen pixel is
  then a fixed function of the design pixels under it. A GDI stretch of a
  sub-rectangle filters its edges differently from a stretch of the whole.
- **Only what changed is redone.** The frame is diffed against the last one
  composited, and only the screen rectangle the changed design pixels reach is
  recomputed and blitted. That is about 88k screen pixels per present on average.

The stretched stock is built with the same function, which is what makes
untouched background come out *exactly* as the plate. Measured on a dumped frame:
the sides have RMSE 0 against the plate, and so does the lower centre, labels
included.

**Which screens, and how their plates are built.** Only screens with open art get a
plate: `mainbkgr` (the main menu) and `singleplay` (campaign selection). The plates
are the game's own art, upscaled and outpainted, so only their recipes are here
(`backdrops/<name>.conf`); `backdrop.sh` builds each into `$A2_DATA/backdrops/<name>/`.
[`BACKDROPS.md`](BACKDROPS.md) has the keys (`field=`, `clone=`, `keep=`, `fade=`,
`dehaze=`) and the paid seeds. `install-plates.sh` (run by `./install`) copies each
built plate to `Menus/`, which `install.sh` leaves alone and `--remove` clears; `a2mod`
moves them with the rest of the menus layer.

**Glows cut off by their own rectangle (`N.soften=`).** Hovering Tutorials plays
`single/TutorialGlow.bik`, 320x200 at (28,20). It has the background baked in, and
its blue halo runs right up to the rectangle. Measured against stock along its
edges it is off by 56, 12, 59 and 26 (top, bottom, left, right). The stock game
hides the left cut on the black frame, which the plate paints over, so without help
it shows as a hard box. The other three campaign glows fade out inside their
rectangles (1–3 at the edges), and so do the main menu's hover bitmaps (RMSE 0 at the
edges).
Each `N.soften=x,y,w,h` rectangle gets a ring 12 design px wide around it. There,
wherever the frame still equals stock, the difference at the nearest edge is
carried on and fades linearly to nothing. It is averaged over 7px *along* the
outermost row or column, which smooths codec noise; averaging a 7x7 box reaching 3px
*into* the rectangle instead copies the panel bar that starts 2px in outwards as a
ghost bar. The ring keeps detail weight 1, so the plate shows through. With nothing
drawn in the rectangle, the difference is zero and nothing changes.
Fading inwards instead would eat the panel's left bar, which starts 4px inside.
A generic "continue every long edge" rule cannot tell a halo from the grey button
bars, hence the explicit list.

The options, load/save and multiplayer screens are metal frames on black, so they
are left as they are.

**Testing without taking over the desktop.** Set `DumpBackdrop=1` and launch with
`NOSHOT=1 capture.sh`. Menus.asi then writes the composed screen to
`Menus-backdrop<n>.bmp` ~60 presents after each backdrop first shows. `shot.sh`
has to switch the visible workspace to grab a frame, and this does not.

## Use

    menus/build.sh              # clang + lld-link, 32-bit PE, no CRT
    menus/install.sh            # copy .asi + .ini into the game directory
    menus/install.sh --mode 0   # install, but only observe and log
    menus/install.sh --remove   # take it out again

`install.sh --remove` restores stock behaviour exactly: the plugin only ever *adds*
`Menus.asi`, `Menus.ini`, `Menus.log` and the backdrop plates in
`Menus/` to the game directory, so there is no backup to keep and nothing to
revert.

`Menus.ini` keys are documented in the file. The ones worth knowing:

| key | meaning |
|---|---|
| `Mode` | `0` observe and log only, `1` centre at stock size, `2` centre and scale |
| `IntegerScale` | `1` snaps to a whole factor (2x = 1600x1200) — crisper, leaves a band |
| `Smooth` | `1` HALFTONE, `0` nearest neighbour |
| `RaiseShellMode` | `0` leaves the engine's 800x600 front-end mode alone |
| `Backdrops` | `0` black pillarboxes and a plain stretch, no plates |

**If anything misbehaves, `Mode=0` is a safe diagnostic and `--remove` is a full
uninstall.** `Menus.log`, beside `Armada2.exe`, records what it patched and every
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
    run-probe.sh focus         the game thread's active, focus and capture windows:
                               where a keypress will go

`capture.sh` takes `NOSHOT=1` to launch without photographing, and
`A2_ARGS="-nointro a2_fed01.bzn"` starts a mission directly. **The `.bzn` is
required**: without it the name does not resolve and you get the HUD with no
map. Missions open with an in-engine cutscene, and Space skips it.

**For an in-game menu, launch a skirmish map instead** (`A2_ARGS="-nointro
mp02eye.bzn"`, with `WAITLOG=0`). It has no script, so there is no cutscene and no
briefing, and the game takes input about 3 s after its window appears. `WAITLOG=0`
skips `capture.sh`'s wait for a menu to lay out, which otherwise sits out its whole
minute because no menu opens. Then press Esc once a second until `Menus.log` shows a
`DialogBoxParamA` line: that is Options, open. A game started on a map **quits
when the mission ends** (abort included), because there is no shell to return to.
To test a return to the menus, start from the main menu: Instant Action, then
LAUNCH.

`run-wine.sh` launches the game with Proton's bundled Wine directly against the existing
prefix, skipping Heroic and the proton wrapper. It is the only launch path that prints
Wine's diagnostics to a terminal — which is how an ASI loader's `Unable to load
<name>.asi. Error: 317` turned out to be

    wine: Call from ... to unimplemented function KERNEL32.dll.GetModuleFileNameA@12

i.e. `llvm-dlltool` needs `--kill-at`, because the `.def` files carry stdcall-decorated
names but kernel32/user32/gdi32 export them undecorated.

`capture.sh` runs the game on its own Hyprland workspace and photographs it; `shot.sh`
grabs one frame and puts the desktop back where it was; `stop-game.sh` shuts the game
and its Wine session down properly.

Two traps that cost real time here, both recorded so they are not rediscovered:

- **Wine reports the process `comm` as `Main`, not `Armada2.exe`.** So
  `pgrep -x Armada2.exe` matches nothing even while the game is plainly running and
  writing to `Menus.log`. This is not cosmetic: it made the harness report "game
  DOWN" for a game that was up, and it made every run's `pkill -x Armada2.exe` cleanup
  a no-op, so instances accumulated — seven of them over half an hour, each with no
  window, each still holding a PipeWire stream and audibly playing the menu music.
  `pkill -f Armada2.exe` does match, but also matches its own shell (see `platform/README.md`, Gotchas),
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

**Reported in game, not diagnosed.** The animated elements on the menu screens
(`bitmaps/main/MainBk_flare.bik`, `singleplayer.bik`, `Multiplayer.bik`) appear to stop
while the mouse is being moved, and resume when it stops.

The leading hypothesis is cost per mouse-move, and it is a hypothesis — **nothing here
has been measured.** `present()` runs on every `ReleaseDC`, and a mouse-move
makes the shell redraw the widget under the cursor, so each `WM_MOUSEMOVE` costs a
**full-frame 800x600 -> 1920x1440 `StretchBlt` in HALFTONE**, not a redraw of the part
that changed. A stream of mouse-moves could then starve whatever drives the animation.

On a screen with a backdrop this does not apply in that form. There, `present()`
redoes only the changed rectangle, measured at 7.8 ms per present (see
"Backdrops"). A fix for the other screens could follow the same pattern.

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
either coalesce (mark dirty and leave it to the 30 ms timer) or scale only the
rectangle that changed, as the backdrop path does.

## Not done

- **Confirmed rendered:** the main menu, the single-player/campaign screen, Options,
  Graphics Settings, and in a mission the Options menu and Save Game.
- **Seen on the bench only** (2026-10-02, every screen against stock at 800x600, three
  findings below): Sound and Game Settings, Credits, Replay Intro, Instant Action's Game
  Setup (its drop-downs, the Advanced panel, SELECT MAP's list box), Load Game, the four
  mission lists, the in-mission menu (Admiral's Log Score and Military tabs, the
  confirmations), and Multiplayer as far as the LAN and Internet lobbies (Current Games,
  Show all, chat, Players In Room, Create Game, Help). The lobbies' list boxes, combo
  boxes and scroll bars scale with their dialog. Not reached: the multiplayer game setup
  as host or guest (the bench has no network, so every host or join ends in a connection
  error, in stock too), a skirmish's LAUNCH, Save/Load Settings, and the other six
  Admiral's Log tabs.
- **Known wrong, not yet fixed** (bench, 2026-10-02):
  - *The Manual IP field is not painted until it is clicked.* "Internet - Manual IP"
    opens with a bare black panel where stock draws a bordered field. One click and it
    draws, takes typing and keeps its scaled font; the edit is adopted (Menus.log). Why
    this edit misses its first paint, and Enter Name and Save Game's do not, is not known.
  - *The Technology Tree is in a proportional font*, so its ASCII branches no longer line
    up. Its edit uses raster "Courier" (`edit font "Courier" height 29`), and `ctl_font`
    holds every scaled face to TrueType but maps only MS Sans Serif to its successor
    ("Not the raster face", above). Courier comes back as some proportional TrueType face.
    Mapping it to Courier New, or keeping `FIXED_PITCH`, should fix it.
  - *Options' version label "1.1"* (bottom of the panel, main menu and in a mission) is
    not drawn anywhere on screen. Probably a control kind `ctl_kind()` does not handle;
    not checked.
- **Hover and click are confirmed to land correctly**; nothing has been measured about
  how *fast* they are. See the animation stall above.
- **Other real child controls.** Owner-drawn buttons and edit boxes are scaled (see
  "The Admiral's Log" and "Edit boxes"). Anything else is a separate HWND that Windows
  draws itself, and would sit 1:1 at its design position. None has been seen doing so:
  the lobbies' list and combo boxes come out scaled with their dialog (how is not
  checked).
  Adding a class is one line in `ctl_kind()`, if its own font is all it needs.
- **Menus drawn inside the renderer** (the Direct3D route) was considered and
  deferred. Embed gets one OS window without it, and the GDI child draws correctly over
  the DXVK surface because the game loop is blocked while any menu is open.
