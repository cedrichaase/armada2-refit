# UI — the menus: the stock dialog runs hidden, the UI draws it and drives it

`[Menus] Enable=1` in `UI.ini` makes `UI.asi` host every front-end screen and every
in-mission menu (`PLAN.md`, "Menus: the stock dialog runs, hidden; we draw it and drive
it"). The stock dialog procedure still is the screen's logic: what it reads and writes,
which `g_shellMode` it names next, what it returns. The UI only takes over the window
it lives in, the picture and the input. Off by default: the shipped `UI.ini` changes
nothing.

Three parts, each in its own file:

| Part | File | |
|---|---|---|
| the dialog host | `src/engine/dialogs.rs` | `DialogBoxParamA` in, a loop of ours, `EndDialog` out |
| the shell's own widgets | `src/engine/shellui.rs` | the `ShellButton`s and `TextLabel`s a screen draws into itself |
| the control model | `src/menus/model.rs` | what a dialog shows, read every frame |
| the generic screen | `src/menus/generic.rs` | any dialog, mirrored or as our widgets |
| screen specs | `src/menus/screens/` | a layout of our own for one screen |
| actions | `src/menus/actions.rs` | what the UI does to a dialog, run after the frame |
| the backdrop | `src/menus/backdrop.rs` | the mission, dimmed, under the in-mission menus |

**Menus.asi is gone.** The menu part stands down at startup if any of the user32
imports it hooks is not user32's own (another plugin hooked it first). `retire/retire.sh`
takes `Menus.asi` out of an old install.

## What the shell is, in the engine

The facts about the stock front end that the host below is built on, measured on the
retired `Menus.asi` (which scaled and embedded the shell itself) and kept here because
the host meets every one of them.

**A fixed-pixel GDI shell on a fixed mode.** Every menu (main menu, Options, load and
save, campaign select, multiplayer) is a Win32 dialog drawn with GDI at hard-coded
coordinates from 800x600 8-bit BMPs in `bitmaps/`. `Armada2.exe` imports `BitBlt` and
no `StretchBlt` or `SetWorldTransform`, and the shell never touches Direct3D, so no
`gui_*.cfg` (the in-game HUD's canvas) or renderer setting can scale it. And the
engine asks for **800x600x16 while the menus are up**: `GetSystemMetrics` answers 800 at
the main menu and the game window is 800x600 at 0,0, so a stock shell *fills a small
screen* and there is nowhere to scale into. The request is
`ST3D_GraphicsEngine::SetActiveDisplay_Internal`'s call of `FindDisplayMode` with 800,
600 and 16; raising it is Display8's (`display/README.md`, "The engine's hard-coded
modes" and "Window and display mode"), and with it the host draws the dialogs at the
screen's size.

**Every shell screen is a modal dialog.** `DialogBoxParamA` has 36 call sites in the exe
against 2 for `CreateDialogParamA`, and a modal dialog is pumped by user32's own loop,
which hands nothing to a message-loop hook: a `GetMessage` or `PeekMessage` hook scaled
the picture and left every click and hover on the stock 800x600 position. Whatever maps
the pointer has to sit in front of each dialog's procedure. Only class `#32770` windows
are dialogs; the 3D window, gameplay, the HUD and the Bink videos are not.

**One OS window needs the dialogs to be children.** Every menu, the in-mission ones
included, is `DialogBoxParamA(shell_hInstance, id, <3D window>, proc, lp)` with a
`WS_POPUP` template (all of them but one); `do_escapeMenu` is the main menu's template
(291). An owned popup is a second top-level window, which under Wine is a second X11
window that Hyprland tiles and focuses like a new application (two visible top-level
windows, both re-tiled when Options opened, injected clicks mostly lost). The Wine
virtual desktop would also give one window, but a display mode cannot be set inside it (DXVK could not; Display8's Vulkan or Direct3D 12 swap chain has no mode to set either),
so it stays off. The remedy is the `WS_POPUP` to `WS_CHILD` rewrite of the template (the
host's `CreateDialogIndirectParamA`); a child brings these, each measured:

- **Input.** `DialogBox` disables its owner before the dialog exists (Wine does it for a
  `WS_CHILD` template too) and hit-testing never enters a disabled window; the top-level
  is re-enabled at `WM_INITDIALOG`, and modality is kept by a `WH_GETMESSAGE` filter that
  turns input aimed outside the innermost menu into `WM_NULL`.
- **Z-order.** Wine walks a modal dialog's owner up to the top level, so a menu opened
  from a menu (Options to Graphics Settings, template 1) is a *sibling* of the first and
  is created below it, clipped away by `WS_CLIPSIBLINGS`: each is raised at
  `WM_INITDIALOG`.
- **Geometry and owner.** The shell positions menus in screen coordinates through
  `GetWindow(hDlg, GW_OWNER)`, which a child lacks: answer it with the owner the game
  asked for and convert to parent-client coordinates just before the real `MoveWindow`
  or `SetWindowPos`. A full-screen menu is pinned to (0,0), since it covers the game
  window; taking the game window's screen position and converting back put the main menu
  at x=21000 on one player's NixOS / GE-Proton10-32 install (not reproduced elsewhere).
  A full-screen menu is sized once, so it must follow its parent's client area when the
  window is resized (Hyprland does it when the game drops fullscreen).
- **Keyboard focus.** The game reads keys only as `WM_KEYDOWN` / `UP` on the 3D window
  (`ProcessKeyboardMessages`), which must hold the focus. A popup gives it back by
  re-activating its owner when it closes; a child deactivates nothing and Wine leaves the
  focus `NULL`, so every keystroke is dropped (Esc opened the in-mission menu once and
  never again, HUD clicks still worked). Put the focus back where it was when the menu
  returns, which also sends the `WM_SETFOCUS` on which the game runs
  `ClearKeyboardState`.
- **The cursor.** The 3D window's `WindowProc` answers every `WM_SETCURSOR` with
  `SetCursor(NULL)` (`0x488881`) so the sprite cursor can show in play; a child's
  `DefWindowProc` asks its parent first, so the arrow vanishes over every menu unless the
  class cursor of the window under the pointer is set.

**Esc in the in-mission menu.** `EscapeMenuDlgProc` (`0x5cddb0`) has no `WM_COMMAND`
case, so the `IDCANCEL` a dialog makes of Esc is ignored: stock Esc opens the menu and
does nothing inside it. Closing it from the keyboard is a left click on Return to Game
(x 5, y 568, 172x22 at 800x600) sent to its procedure, which runs the same sound, result
code and `EndDialog` as the mouse.

**The Admiral's Log builds real windows.** `AdmiralsLogDlgProc` (`0x5e22c0`) is a
full-screen `DialogBoxParamA(0x880)` that in its `WM_INITDIALOG` creates eight tab panes
with `CreateDialogParamA` (template `0x123`, a bare `WS_POPUP`, at `0x5e3c04`) placed in
screen coordinates, and the player list, eight tabs, Save and Done as
`WS_CHILD | WS_VISIBLE | BS_OWNERDRAW` buttons (`CreateWindowExA` at `0x5e394e` and
`0x5e3b7b`; the Ships and Battles panes add more, `0x5f7e4f`, `0x5e9b63`). **The game
never destroys the panes**: it relies on an owned popup dying with its owner, which
breaks once the log is a child (a popup's owner is always a top-level window), so they
outlived the log and sat over every later menu. The buttons are not drawn through the
parent's DC; their pixels come from the parent's `WM_DRAWITEM`, which draws only through
`DRAWITEMSTRUCT.hDC` and `.rcItem`. **A stock bug a memory DC exposes:** the tab and
Save/Done helper (`0x5e93a0`) ends by selecting the memory DC's original 1x1 bitmap into
`dis->hDC` (`0x5e94d7`) instead of back into its memory DC. Into a window DC that fails
harmlessly; into a DC that holds the design surface it swaps the surface out after the
drawing landed and the buttons come out plain grey. The player-list helper (`0x5e9510`)
does not do this.

**Edit boxes draw themselves.** The text fields (the player name on Multiplayer
Connection, template 2162, an `Edit` 399x32 at design 208,171; Save Game's name, 476x28
at 176,551) and the in-mission Technology Tree are real `Edit` windows: the shell draws
only their frame into the dialog. The game places the name field twice (first elsewhere,
then into the frame by `MoveWindow` in design coordinates). The fonts are MS Sans Serif
-11 for the name field, Arial -11 for Save Game and a raster Courier of height 29 for the
Technology Tree, whose ASCII branches line up only in a fixed pitch. A raster face does
not scale (asked for -26 at 3440x1440, MS Sans Serif came back at its largest bitmap),
so a scaled field asks for *Microsoft Sans Serif* (the same metrics, TrueType,
`OUT_TT_ONLY_PRECIS`) and the Technology Tree for *Courier New* with
`FIXED_PITCH | FF_MODERN`, given the raster font's own `tmAveCharWidth` because Courier
New is narrower at the same height (about 6.6 design px a character against 8). Colour
needs nothing: `Screen::ControlColorEdit` (`0x5a3360`) answers `WM_CTLCOLOREDIT` on the
dialog, and the dialog has `WS_CLIPCHILDREN`.

**`GetDC` and `ReleaseDC` do not pair.** `ShellButton::UpdateButton` (`0x5a4a10`) takes
`GetDC(hDlg)`, draws the button and falls into eight NOPs at `0x5a4adb` where its
`ReleaseDC` call used to be, so every button redraw leaks the DC. Nothing may wait for a
release: a stack of DCs presented when the outermost comes back is pinned at the first
leak (Options showed its background and no buttons), and after eight leaks the game is
handed real DCs and draws 1:1 in the corner on every `ShellButton` screen. A host hands
out a surface of its own at `GetDC` and presents on a timer, not on release.

**The shell's anti-flicker underlay.** `SetCurrentBackground` (`0x6083f0`) keeps an
800x600 copy of each screen's background (the stock BMP, or a `BitBlt` snapshot of the
dialog, black if taken before the dialog drew) and draws it onto the **3D window**, the
dialogs' parent, at 0,0 through `DrawTransparentBitmap` (`0x608100`); `SnapShotBackground`
(`0x608530`) draws it there again on every dialog's close path, just before
`CleanCurrentBackground`. At 800x600 the parent already held the next screen; scaled,
that draw is a 1:1 flash in the corner for 100 to 120 ms between two menus (main menu to
single player: 2 frames of the first, 4 of bare background; the way back 7 of black).
Seven sites read the copy to restore the background under a button, so it is still made;
only the two draws onto the parent must go nowhere.

**Wine's window surface.** A menu that is a child of the 3D window draws into that
window's Wine window surface, which is pushed to the screen only when a drawing call ends
more than about 50 ms after the first one still pending or when the thread goes idle, and
under the 3D window the idle push did not come (inferred, not traced into Wine). Whatever
a menu drew last in a shorter burst stayed off screen: Options' version label "1.1" (the
last thing `EscapeMenuDlgProc` draws, by `TextLabel::DrawLabelText`, `0x5a77e0`) and the
Manual IP field, black until clicked. Reading a pixel back through the menu's DC is a
drawing call and forces the push.

**Labels run past their rectangles.** Every shell label is one `DrawTextExA` from
`ShellButton::DrawLabelText` with `DT_SINGLELINE | DT_VCENTER | DT_NOCLIP`, so text wider
than its rectangle is drawn on past it, and nearly every label in the shell does so
harmlessly ("Resources: Normal" is 101 px in 87). On Game Setup (template 2096: Instant
Action and every multiplayer setup) "Shroud Off, Fog Off" and "Random Placement" run over
the minimap frame's left border (x 618 to 621) at 800x600 in stock; the rectangles cannot
say where a label must stop, the art does. The face is Arial Bold (`ShellFonts`, cell
heights 8 to 20), which Proton maps to Liberation Sans with Arial's metrics. The minimap
preview stays blank while Minimap Hidden is set: a game rule, one-way, host-only.

**Command-line switches.** `Armada2.exe` has its own switch table (`-nointro`, `-edit`,
`-window`, `-res`, `-bpp` and others). The leading `-` or `/` is required, or the token
is taken as the mission name and the game starts a match on a mission that does not
exist (a HUD with no map). `-nointro` skips `Intro.bik` and the three logo reels; a game
started on a `.bzn` quits when the mission ends, since there is no shell to return to.

## The dialog host

Every shell screen is `DialogBoxParamA(shell_hInstance, template, owner, proc, lp)`, 36
call sites in `Armada2.exe`, and the Admiral's Log's eight panes are the one
`CreateDialogParamA`. The host takes both through **Armada2.exe's import table**: the
slot of each user32 function the shell calls is pointed at a function of ours, after
checking that every slot still holds user32's own function. A window that is not
hosted goes straight to user32, so the 3D window and anything the host refused behave
as stock.

### What a hosted dialog is

- **Created hidden, as a child of the 3D window.** The template is copied and its
  pop-up styling (`WS_POPUP`, `WS_VISIBLE`, the caption, the modal frame) replaced by
  `WS_CHILD`, and it is made with `CreateDialogIndirectParamA` ("What the shell is, in the engine", "One OS window", is the same rewrite; this one also drops `WS_VISIBLE`). Our procedure sits in
  front of the game's: the first message (`WM_SETFONT` or `WM_INITDIALOG`) names the
  window, so even what the game does inside `WM_INITDIALOG` (an `EndDialog`, a
  `GetDC`) finds it hosted.
- **Never shown.** `ShowWindow` on a hosted dialog does nothing and `SetWindowPos`
  loses `SWP_SHOWWINDOW`. A hidden window is drawn by nobody, so no GDI ever reaches the
  screen over the Direct3D frame.
- **Its geometry is its design rectangle.** The shell places a screen against its owner
  in screen coordinates: `init_screen_pos` (0x607a20) sizes a full-screen screen to the
  3D window's client area, offset by `ClientToScreen(GetWindow(hDlg, GW_OWNER))`; the
  smaller dialogs (Yes/No, Manual IP) are placed at design offsets from their owner the
  same way. The host answers `GW_OWNER` with the owner the shell asked for (a child has
  none), converts the position into the parent's client coordinates, and pins anything
  bigger than 800x600 to 800x600 at 0,0. So every hosted dialog's real client
  rectangle is its rectangle in the 800x600 design space, and every coordinate the
  shell computes (`GetClientRect`, `ClientToScreen`, a control's `MoveWindow`) agrees
  with what is drawn.
- **It draws into a surface of its own.** `GetDC` and `BeginPaint` on a hosted dialog
  return an 800x600 32-bit DIB section's memory DC; `ReleaseDC` and `EndPaint` of it do
  nothing (the shell never pairs them anyway: "What the shell is, in the engine", `GetDC` and `ReleaseDC` do
  not pair). Everything a screen draws, its background, the `ShellButton`s, the
  labels, the Bink animations, lands there. While a dialog is hosted, `GetDC` of the 3D
  window returns a scratch DC: the shell's background underlay (`SetCurrentBackground`
  and `SnapShotBackground` drawing onto the 3D window) is drawn into nothing.
- **It is painted on request, and only while it shows.** A hidden window is never sent
  `WM_PAINT`. The host notes every `InvalidateRect` and `RedrawWindow`
  (`RDW_INVALIDATE`) of a hosted dialog and, at the start of each frame, sends
  `WM_ERASEBKGND` (if an erase was asked for) and `WM_PAINT` itself. A new dialog, or
  one that moved, is painted once anyway. A dialog under a full-screen one is not
  painted until it is uncovered, as Windows sends no `WM_PAINT` to a window with
  nothing visible. That is not a nicety: the shell keeps the current screen's
  background in globals (`SetCurrentBackground`), and the main menu, which invalidates
  itself for its animation, painted itself under Game Setup and put its own background
  back; Game Setup then drew its player rows against the wrong one.
- **Its DC is not trusted to hold its bitmap.** The shell selects bitmaps of its own
  into the DCs it is handed and does not always put ours back (Game Setup's paint left
  our surface's bitmap selected in a DC of its own, and the "stock bug" of "What the shell is, in the engine"
  does the same through `WM_DRAWITEM`). So the surface's bitmap is selected back
  whenever its DC is handed out, and the host reads a surface by its bits (after
  `GdiFlush`), never through its DC: a `BitBlt` from our DC read a stale bitmap while
  the surface itself was current, and Game Setup showed its first picture for ever.

### The loop

`DialogBoxParamA` on a hosted dialog runs `run_modal` instead of user32's modal loop.
Each pass, about 60 a second:

1. **Pump the queue.** Timers and the procedure's own posted messages are dispatched as
   usual. Input is sorted:
   - Mouse messages addressed to a hosted window are dropped: the real pointer is in
     screen space, and the UI sends each dialog the pointer in its own design place.
   - Keys addressed to a hosted window (the focus is in the dialog, because the player
     clicked into an edit) go through `IsDialogMessageA(top, ...)`, as user32's modal
     loop gives them: the edit gets its characters, Tab moves, Enter and Esc become
     `IDOK` and `IDCANCEL`. `IsDialogMessageA` translates the key itself; translating it
     first as well typed every character twice.
   - Keys and mouse messages for the 3D window reach its procedure, where the UI's input
     hooks note them for the UI and keep every press from the engine
     (`input::set_hosting`): a modal dialog disables its owner, and no key or click
     reaches a mission behind its menu. A key whose down the engine saw (the Esc that
     opened the in-mission menu) still gets its up. Enter and Esc then reach the dialog
     as `IDOK`/`IDCANCEL` through `IsDialogMessageA`, unless a text field of ours has
     the keyboard.
   - `WM_QUIT` is posted again and ends the dialog, as user32 does.
2. **Paint** what asked to be painted (above).
3. **Run the queued actions** (`menus::actions`).
4. **Compose** each modal dialog's frame (the ones that show): its surface's bits, then
   its panes' surfaces, then
   each visible control printed with `WM_PRINT` (`PRF_CLIENT | PRF_NONCLIENT |
   PRF_CHILDREN`) at its place, bottom of the z-order first. An edit, a list box, a
   combo box and an owner-drawn button all paint themselves on `WM_PRINTCLIENT` with
   the DC they are given, and the dialog's `WM_CTLCOLOR*` and `WM_DRAWITEM` answers draw
   into it. After each print the surface's bitmap is selected back: a stock helper of
   the Admiral's Log selects the 1x1 default bitmap into whatever DC `WM_DRAWITEM`
   hands it ("What the shell is, in the engine", the Admiral's Log).
5. **Draw a frame** through the game's device, as `engine/shell.rs`'s `run_screen`
   does: the device read again, the engine's `TestCooperativeLevel`, `BeginScene`, a
   full viewport, clear to black, `surface::frame(Context::Menus)`, `EndScene`,
   `Present`. A lost device skips the frame and keeps pumping.
6. **Run the actions** the frame queued, and leave the loop if the dialog ended.

**A nested dialog lets go of the mouse capture.** A dialog opened by a click on another
(Game Setup's SELECT MAP opens Map Select) can start while the outer dialog still holds
the capture its button press took; every press then went to that hidden window, which
the pump drops, and Map Select's spec never saw a click. `run_modal` releases a capture
held by a hosted window when it starts (`UI.log`: "opens with the capture held").

`EndDialog` on a hosted modal dialog only records the result; the loop ends, the window
is destroyed (`WM_DESTROY` reaches the game's procedure, as after a stock
`DialogBox`), and the caller gets the result. A dialog opened from a hosted one runs a
loop of its own inside the outer one's, and the outer one is still drawn under it.

**Actions run between frames, never inside one.** A click a screen sends can make the
procedure open a nested dialog, whose loop draws frames of its own; inside a scene that
would be a second `BeginScene`. So widgets queue what the player did and the loop sends
it after `Present`.

### What is refused, and runs stock

A dialog runs through user32 as before, with a line in `UI.log`, when: its procedure is
not in the shell's code (0x5a0000–0x610000: the map editor's dialogs are not screens);
there is no 3D window or no Vulkan or Direct3D 12 device behind the game's (Display8 bailed out); its
owner is a window the UI does not host; its template cannot be read; or the hidden
dialog could not be made. With the shell's mode raised (below) a stock dialog then draws
800x600 in the corner, as stock does, but it works.

### The display

`do_smain` runs the shell on Storm3D's shell display (`SetActiveDisplay(3)`), and
`DispatchDialog` (0x6066c0) puts that display up again and calls `FlipToGDISurface`
before every screen. Its mode is a hard-coded 800x600x16 (`SetActiveDisplay_Internal`'s
two `push 16; push 600; push 800`, at 0x62c4d4 and 0x62c633). Display8 raises both to
the desktop's size at 32 bits (since display 0.18.0, while the host claims the menus;
`display/README.md`, "Window and display mode"; the UI patched the two pushes itself
before, as the retired Menus.asi's `RaiseShellMode` did; "What the shell is, in the
engine"). The hosted dialogs then draw on the shell display's own
device, at the screen's resolution, without switching displays: when the game runs at
the desktop's resolution the shell and the 3D display have the same mode and the
device is never re-created (`README.md`, "What the shell is, as far as Direct3D goes").
The in-mission menus run on the 3D display that is already up.

### Hooks (the engine profile)

The host's engine findings are in `profiles/gog-1.1/ui.dialogs.toml`; `install` makes
both hooks ready before it patches anything and stands down with the engine's reason.

| Hook | What it needs of `Armada2.exe` |
|---|---|
| `ui.dialogs.shell_mode` | the two `push 16; push 600; push 800` of `SetActiveDisplay_Internal` (bytes checked, then raised to the desktop's size), and the front end's 800x600x16 as values; feeds `Menus.Enable` |
| `ui.dialogs.reads_screens` | the 32 shell dialog procedures by address, which name a hosted screen (`screen_name`) and pick the in-mission menu's Esc handling, and the shell's code range that bounds what is hosted; feeds `Menus.Enable` |

### The in-mission menus

`do_escapeMenu` (the in-mission menu, template 0x123 like the main menu), Save Game,
Load Game, the Technology Tree and the Admiral's Log are opened from a mission. The
mission's loop is blocked inside the dialog, so the host's loop is the only thing that
presents. When the first dialog opens within two seconds of a mission frame
(`ingame::recent`), the host draws the menus over the mission's last frame, dimmed.
That frame is the UI's own: at the end of every mission frame (after the UI's frame,
so with the HUD and the UI on it) the back buffer is copied on the GPU into a target
the UI keeps (`menus::backdrop::keep_mission_frame`, a `StretchRect` of the same
size). The front buffer is read back (`IDirect3DDevice9::GetFrontBufferData`, as
before) only when no copy was made, so nothing is read back from the swapchain in the
usual case, and a display of ours (the display plan) needs no front buffer.
A menu that hands over to another (Options to Load Game returns to the shell's
`DispatchDialog` in between, with no mission frame) keeps the picture if it opens
within 1.5 s.

**Esc closes the in-mission menu** (`[Menus] EscapeReturns=1`, the default): in stock
Esc does nothing inside it (`EscapeMenuDlgProc`, 0x5cddb0, has no `WM_COMMAND` case).
A fresh Esc while it is the innermost dialog is a click on Return to Game (design 91,579),
through the menu's own procedure, as Menus.asi's `EscapeReturns` does.

### The cursor and the focus

The 3D window answers `WM_SETCURSOR` with no cursor (for the sprite cursor in a
mission); while a dialog is hosted, `engine/shell.rs`'s subclass answers it with the
arrow, as for a screen of ours. The keyboard focus stays where the shell leaves it: on
the 3D window, or on a hosted edit the player clicked into. Typing reaches a stock edit
either way (above, and "Driving window controls" below).

**A focus left nowhere.** After a mission's load Wine had no window with the keyboard
focus ("What the shell is, in the engine", keyboard focus, saw the same after an embedded menu), and
with no focus the active window gets every key as `WM_SYSKEYDOWN`: the in-mission
menu never saw its Esc. The first hosted dialog puts the focus on the 3D window when
it finds none, and an Esc without Alt counts as Esc whichever way it comes.

## The shell's hooks (the engine profile)

What the screens of ours, the widget finder, the game-setup getters and the `ComboBox`
reads need of `Armada2.exe` is in `profiles/gog-1.1/ui.shell.toml` (`engine/shell.rs`,
`shellui.rs`, `shellcombo.rs`, `setup.rs`). `install` makes the hooks it uses ready
before it writes anything and stands down with the engine's reason; the reads answer
nothing (None, an empty list, 0) while their hook is not ready.

| Hook | What it needs of `Armada2.exe` |
|---|---|
| `ui.shell.screens` | `DispatchDialog`'s call of `Show_Credits` (re-pointed at the Credits of ours, or at the demo screen), and the functions a screen of ours calls between two shell screens: `SetActiveDisplay` (2 the 3D display, 3 the shell's), `ClearDeviceDisplays` with `ST3D_Colour_Black`, `GetWindowHandle`, `HandleMessageQueue`, `TestCooperativeLevel`, `SetCurrentBackground`, `ShellBitmap`'s constructor (0x30 bytes, from `operator new`) with `ALL_BLACK_8_BY_6_BACKGROUND`, and `g_shellMode` (1: main menu); feeds `Menus.Credits` |
| `ui.shell.reads_device` | Storm3D's display, device index and device table (+0x10c, +0xc0, +0xcc), `ST3D_DeviceDirectX8`'s vtable (to tell the device) and its `FlipToGDISurface` slot, `ExitCode` (a `WM_QUIT`'s code) and the `star_trek` `Program`'s first byte (lets Alt+Tab through); read by the screens, the dialog host and the loading screen |
| `ui.shell.cursor` | the one `call [SetCursor]` in `WindowProc`'s `WM_SETCURSOR` case, bytes checked, replaced by a call of ours; the arrow over a screen of ours or a hosted dialog |
| `ui.shell.widgets` | `Shell::Widget`'s constructor, re-pointed (its bytes checked) at one that stores the same two fields and remembers the object, and `Shell::Widget`'s vtable; feeds `Menus.Enable` |
| `ui.shell.reads_widgets` | the vtables of the shell's button, toggle, cycle, selection box, scroll bar, label, `ComboBox`, `GameSelectionSlot` and `PlayerListButton` classes, the two virtuals (`UpdateButton`, `TextLabel::Draw`) that tell a class by its slot, the `.rdata` range a vtable pointer must lie in, and every field read by offset |
| `ui.shell.reads_combo` | a `ComboBox`'s item count, item height, pull-down rectangle, hidden combo window and direction (the table in `shellcombo.rs`) |
| `ui.shell.reads_progress` | `g_shellSettings`, read only: the campaigns' furthest missions (+0x0d), complete flags (+0x11) and mission counts (+0x15), four bytes each; the campaign selection's locks ("Locked campaigns") |
| `ui.shell.reads_setup` | `g_transport` and the `Transport` / `GameSetup` getters (map, title, slots, race, team, name, vacancy), `RaceRandom` and `Race::FindInstantActionSlot`, with the `Transport` virtuals asked by vtable offset; the loading screen's mission details and the Refit screen's network test |

## The shell's widgets

Most screens are not made of windows. The main menu, Options, Game Setup and nearly
every other screen draw `ShellButton`s into the dialog ("What the shell is, in the engine", "The Admiral's
Log") and hit-test them themselves. A `ShellButton` is a `Shell::Widget`: a C++ object
holding the dialog it draws into, its rectangle, its bitmaps, its label and its state.
There is no list of them anywhere: each screen keeps its own in globals.

**How the UI finds them** (`engine/shellui.rs`). Every widget's constructor calls
`Shell::Widget::Widget(HWND)` (0x5a8060), which stores the dialog at +4 and its vtable.
Its first five bytes are a jump to a function of ours that does the same two stores and
remembers the object. A remembered object is read only after `VirtualQuery` says its
memory is committed and readable, and only while its first field is one of the shell's
vtables (`.rdata`, 0x6b0000–0x6d0000) other than `Shell::Widget`'s own, which is what a
widget's destructor leaves behind. Nothing is called on a widget.

**Not every button is a widget.** The main menu's three big pictures (Single Player,
Instant Action, Multiplayer) are hit areas of `MainMenuDlgProc` itself, with Bink hover
animations; only Options, Exit, Replay Intro and Credits are `ShellButtonSelBox`es.
Both answer a click at a point inside them, so a spec acts on a design point when the
model has no widget there.

**Which class**, by its vtable: a class whose slot 8 is `ShellButton::UpdateButton(HDC)`
(0x5a4af0) is a button (`ShellButton` itself, `CycleButton` 0x6bb064,
`ShellToggleButton` 0x6bb18c, `ShellButtonSelBox` 0x6bb0e4, and the screens' own
`FileSelectButton`, `RoomSelectButton`, `PlayerListButton`, ...); one whose slot 1 is
`TextLabel::Draw` (0x5a7590) is a label (`TextLabel`, `StdTextLabel` 0x6bbaa4).
`ShellScrollBar` (0x6bb164) is a slider (`Kind::Scroll`): the volumes, the speeds,
gamma, the AI handicaps. `ComboBox` (Game Setup's player
rows, Map Select's filter) is a `ShellButton` with an `UpdateButton` of its own, so it
is recognised by its vtable (`Kind::Combo`, below).

**A `ShellScrollBar` is driven by one click.** Its fields, read from its constructor
(0x5a6280), `Draw` (0x5a6520) and `CheckButtons` (0x5a63f0): the whole bar's rectangle
at +0x28, the first pixel of its track (the end of the first arrow button) at +0x10
(+0x14 for a vertical bar), the value at +0x3c, the track's length at +0x40, a
horizontal flag at +0x44, the range at +0x48 and +0x4c. It overrides only
`NotifyMouseLeftUp` (a press and a drag do nothing): a release on the track sets the
value to the range's low end plus the pointer's distance along the track times the
range's span over the length plus one, and a release on an arrow button steps it; the
screen's procedure reads the value with `GetValue`. So a slider of ours draws its own
drag from the model's value and, when the player lets go, sends **one click at the
point of the track that maps to the value** (`generic::click_scroll`); a key step is
one click at once. The stock click sound plays once per change, not per pixel. The top of
the range is the exception: the point that maps to it lies past the track, so a click on
the track reaches at most the top minus one and the top takes a further click on the far
arrow button (`click_scroll`; before 2.2.1 a slider could not be set back to its top).

**What is read**, each offset from the method that writes it:

| Field | Offset | Written by |
|---|---|---|
| dialog | button +0x20, label +0x4 | the constructors |
| label text | button +0x3c, `char[64]`, `"NULL"` for none | `SetLabelText` (0x5a5330) |
| state | button +0xc0: 0, 1 under the pointer, 2 held | `SetButtonMode` |
| rectangle | button +0xc4 (left, top, right, bottom) | constructor, `MoveButton` (0x5a48c0) |
| hit rectangle | `ShellButtonSelBox` +0x178: the part of its shared picture that takes the pointer (+0xc4 is the whole picture, the same for every box of a set) | its constructor (0x5a5ee0); `CheckRollOver` (0x5a5f60) tests it |
| shown, enabled | button +0xe4, +0xe8 | `ShowButton`/`HideButton`, `EnableButton`/`DisableButton` |
| label colour | button +0x28 (`COLORREF`) | `SetButtonTextColor` |
| label alignment | button +0x174 left, +0x175 right | `SetLeftJustifyText`, `SetRightJustifyText` |
| toggle on | `ShellToggleButton` +0x188 | `SetButton` (0x5a69b0), which also copies the on or off label to +0x3c |
| cycle choice | `CycleButton` +0x198 | `GetSelection` (0x5a5dd0) |
| hidden combo box | `ComboBox` +0x198 (an `HWND`) | `ComboBox::Send`, which forwards every message to it |
| pull-down: item count, item height, rectangle, opens downwards | `ComboBox` +0x178, +0x17c, +0x180 (left, top, right, bottom, the button's own row included), +0x1b4 | `Send` (counts `CB_ADDSTRING`/`CB_INSERTSTRING`, sizes the pull-down), `InDropDownArea` |
| label text | `TextLabel` +0x8 (`char[1024]`); `StdTextLabel` the `std::string` at +0x468 | `SetLabelText` (0x5a7650, 0x5e8bd0) |
| label rectangle, flags | `TextLabel` +0x408, +0x418 (`DrawText` flags) | constructor (0x5a73a0) |
| label shown, colour | `TextLabel` +0x45c, +0x464 | `HideLabelText`/`ShowLabelText` (0x5a7690), the constructor |

**How a button is driven.** `ShellButton::NotifyMouseLeftDown` and `NotifyMouseLeftUp`
check `CheckRollOver` (the point in the rectangle, the button shown and enabled) and
then set the held state and call `Pressed`; the screen's procedure calls them from its
`WM_LBUTTONDOWN` and `WM_LBUTTONUP`, and `NotifyMouseMove` from `WM_MOUSEMOVE` (the
hover picture and sound). The shell reads the point from `lParam`: it never calls
`GetCursorPos` (none of its five call sites is in the shell). So **a click is the
dialog's own `WM_MOUSEMOVE`, `WM_LBUTTONDOWN` and `WM_LBUTTONUP` at a point inside the
button**, sent to the dialog: the procedure does exactly what a real click does, its
sound, its handler, the `EndDialog` and the next `g_shellMode` included.

## The control model

`menus::model::read` reads a hosted dialog every frame (`Model`, a list of `Control`s):

- **Shell widgets** of the dialog and of each visible pane, as above: kind, design
  rectangle, label, shown, enabled, hover state, toggle and cycle state, colour,
  alignment.
- **Child windows**: class (`Edit`, `List`, `Combo`, `ScrollBar`, `Button`,
  `OwnerDrawn`, `Static`, `Pane`), control id, style, design rectangle, text
  (`WM_GETTEXT`), enabled and visible from the style, a button's check state
  (`BM_GETCHECK`), a list's or combo's items (`LB_GETTEXT`/`CB_GETLBTEXT`, when its items
  are strings; at most 300), selection and first visible item, a scroll bar's position
  and range.

Labels therefore come from the game, in its own (localised) strings. `UI.log` lists each
dialog's model once, when it first draws (`menus: model of "main menu" ...`): that list
is where a spec's author finds the controls.

**A text field of ours and the stock edit's focus.** A screen often puts the keyboard
focus in its stock edit (Multiplayer Connection's name, at `WM_INITDIALOG`), and keys
for a hosted window then went both to our field (which sends the stock edit what
changed) and, through `IsDialogMessageA`, to the stock edit itself: every character was
typed twice ("hosthost"). Under a nested dialog the focus stays in the outer dialog's
edit, which `IsDialogMessageA(top)` does not dispatch, so nothing was typed at all (the
Manual IP field). While a field of ours has the keyboard (`menus::wants_keys`), the
pump now only translates a key for a hosted window, so its `WM_CHAR` comes back for our
field and no stock control gets it (`engine/dialogs.rs`, `pump`). **Enter in a field of
ours** is the stock edit's Enter (`Action::Enter`): the edit is given the focus and the
key goes through `IsDialogMessageA` of its dialog, which makes the default button's
`IDOK`. Game Setup's procedure sends its chat line on `IDOK` only when `GetFocus()` is
that edit, which is why the focus is moved first.

**Driving window controls** (`menus::actions`): a click is the control's own mouse
messages at its client coordinates (as the pointer would send them); typing is
`WM_CHAR` (and `WM_KEYDOWN` for the arrows, Home, End and Delete) to the edit, which keeps
its own text, caret and limits, and notifies its parent itself; a choice in a list or
combo box is `LB_SETCURSEL`/`CB_SETCURSEL` and the `WM_COMMAND` notification a user's
choice sends the parent (`LBN_SELCHANGE`, and `LBN_DBLCLK` for a double click;
`CBN_SELCHANGE` then `CBN_SELENDOK`). A combo box's drop-down list is never opened: it
is a pop-up window of its own, which would be drawn over the frame.

## The generic screen

Every dialog without a spec, `[Menus] Generic`:

- **`mirror`** (the default): the composed frame (above), uploaded into a texture every
  frame (`gfx::texture_dynamic`/`tex_update`) and drawn into the design area. The
  pointer goes to the hidden dialog as it would to the stock one: each move to the
  window under the pointer in the dialog's design coordinates (the deepest visible,
  enabled child, as user32 hit-tests), a press to that window, and the moves and the
  release that follow to the same window, as if it had captured the mouse; the wheel to
  the window under the pointer; typing to the edit the player clicked into, when the
  keyboard focus is not in the dialog. Everything works this way: art, hover pictures,
  Bink animations, the Admiral's Log, the Technology Tree, map previews.
- **`widgets`**: the model as our widgets, in their design places, over a backdrop of
  ours. A labelled shell button, cycle or toggle is our button; a picture button (no
  label) or an owner-drawn control is cropped from the mirror, with our hover frame; a
  label is our text in the game's colour; an edit is a text field that sends its typing
  to the stock edit; a list box is a list; a combo box is a drop-down. Labels that a
  screen paints into its background art are not controls, so a screen built on them
  (the main menu, the campaign selection) needs a spec. The text field, list and
  drop-down are `crate::widgets`': an edit's text is shown from the stock edit every
  frame, what the player changes is sent to it as its whole new text
  (`Action::SetText`, `WM_SETTEXT`: the edit applies its own limit, which is asked of it
  with `EM_GETLIMITTEXT`, and notifies its parent itself); a list's or combo's choice
  is `Action::Select`; a `ShellScrollBar` is a slider (above).

**The design area** is 800x600 fitted into the back buffer as the retired Menus.asi fitted it: the
full height, centred, at 4:3. `UI.log` logs it as Menus.asi logged its own
(`menus: MoveWindow 800x600 @0,0 -> 960x720 @160,0 ...`), and the bench reads either log
to map a scenario's design coordinates to the screen (`testbench/bench/scenario.py`,
`design_to_screen`), so the existing scenarios' `Click at design X,Y` steps and
`ui.json` targets land on the mirrored screens.

## Writing a screen spec

A spec lays a screen out our way; the stock dialog still runs under it. The main menu
(`src/menus/screens/main_menu.rs`) is the example; the setup and multiplayer screens
below are built from `screens/kit.rs`, which draws each kind of stock control as our
widget and acts on it the stock way (`kit::button`, `cycle`, `combo`, `edit`, `list`,
`picture`, and `form` for a whole small dialog). To write another:

1. **Find the screen's name** in `SCREENS` in `src/engine/dialogs.rs` (the table below
   has them all). A spec is chosen by the dialog procedure, never the template: 0x123 is
   five different screens.
2. **Read its model**: run the screen on the bench with `--menus` and read the
   `menus: model of "<name>"` lines in `UI.log`: every control's kind, id, design
   rectangle and label.
3. **Write `src/menus/screens/<name>.rs`** with `pub fn draw(ui: &mut Ui, s: &Screen) ->
   bool`, and add it to `SPECS` in `screens/mod.rs` as a `Spec { screen: "<name>", draw }`.
   A `Spec` is the plain form of the trait `screens::Screen` (the dialog's key and a
   draw), which `screens::find` looks the screen up by; a screen that needs state of its
   own can implement the trait itself. What it needs of the engine it takes from
   `crate::engine::api` (`api::dialogs`, `api::shellui`, `api::sound`, ...) and nothing
   else (README.md, "The engine boundary"). In it:
   - find each control you lay out, by label (`s.model.by_label`), by id
     (`s.model.by_id`, window controls) or by a design point inside it
     (`s.model.button_at`, for picture buttons without labels); **return false** when
     the controls you expect are not there, and the generic screen draws the dialog;
   - draw with `crate::widgets` (and `generic::shell_button_look` for a button in the
     shell's place), placed by `Anchor`/`Stack` in units, not in design pixels;
   - act through the stock controls only: `s.click_control(c)` for a shell button or a
     window button, `actions::push(Action::Select { .. })` for a list or combo,
     `Action::Key` for typing into an edit. Never write a setting yourself: the stock
     procedure's handler does, and so the screen keeps doing what it did. A setting of
     ours (a Refit choice, not an engine one) goes only to the overlay `RefitPlayer.ini`,
     through `crate::settings::set_row`, never to a plugin's ini or the engine's files;
   - keep the art where it carries information with `s.mirror_part(ui, design_rect,
     screen_rect, tint)`: a map preview, a portrait, the screen's own background
     (the main menu fades it behind its buttons);
   - only `s.top` takes input: draw a lower dialog without acting;
   - a shell button whose label is justified beside a small square (Game Setup's
     "Fixed Placement", the Advanced panel's settings) takes the pointer on the square
     only: `kit::press` clicks the square, at the end the label is justified to.
4. **Hover**, if the stock screen plays a sound or an animation on it, is a
   `WM_MOUSEMOVE` to the stock button's point when your hover changes (the main menu
   does this).
5. A scenario in `testbench/scenarios/ui-menus-<name>.md` that reaches the screen with
   `--menus` and uses it.

`[Menus] Specs=0` draws every screen generically, for comparing a spec with the stock
screen.

## The Refit screen

`menus/refit.rs`: every setting of every plugin that exports a settings table
(`tools/settings/README.md`), as rows on three tabs (Graphics, Interface, Gameplay; a tab with no row is not shown, which is why Movies and Multiplayer are gone: cutscenes 3.0.1, Pay when queued on Gameplay, Online entry on Interface), in **a modal of ours over the hosted Options dialog**. It is not a
shell screen of its own: the in-mission Options runs inside the dialog host's loop, which
is the only thing that presents while a mission's menu is up, and the modal drawn by the
host's frame (`menus::frame`, after the dialog stack, before the frame's claim) works the
same in the shell and in a mission with the stock dialog still running under it.

- **By its key, over a mission.** With `[UI] RefitKey` set (a virtual-key code, pressed
  with Ctrl; 0, none, in the shipped `UI.ini`), the same modal opens over a running
  mission without Options: `crate::input` keeps the key from the engine and
  `refit::mission_frame`, called last in the mission frame, opens it (`refit: opened in a
  mission, by its key`) and draws it; Esc, Done or the key again closes it. The game is
  not paused. Rows count as "over a mission" for the locks and the Next mission badges.
  This is how Fleet Operations: Roots reaches the screen (Ctrl+F12 in its defaults;
  PLAN-FLEETOPS.md, FO-6, decision 86), whose Options the host does not draw.
- **Life.** The Options spec's **Refit settings** button calls `refit::open(host)` (with the
  shell's click sound). The modal closes on Done, on Esc, when its host leaves
  `dialogs::stack()` (`menus::closed`; log `refit: closed (Options ended)`) and when a
  dialog opens above its host (`refit: closed (a dialog opened over Options)`), so that a
  network box takes the input as stock would. While it is open the Options spec is drawn
  inert (`inert_unless`).
- **Keys.** `Ui::modal` claims the keyboard every frame, so `menus::wants_keys()` is
  true and `pump` sends neither Enter nor Esc to the stock dialog (IDOK, IDCANCEL, Return
  to Game). The last branch of `pump` used to call `escape` without asking; an Esc that
  arrives as a `WM_SYSKEYDOWN` at the game window (a focus left nowhere) is now noted as
  the plain key for the modal and goes no further. Esc closes an open dropdown, then an
  open question, then the modal; the first Esc after it has closed is stock's again
  (shell: IDCANCEL; mission: Return to Game). On close `ui.blur()`.
- **Rows** come from `settings::tables()` in `EXPORTS` order, left out when `FLAG_NO_ROW`,
  `TAB_NONE`, refused by the hub or a plugin that is absent; `FLAG_ADVANCED` rows show
  with "Show advanced" (session only). Two columns when one does not hold the page; a
  scroll area only if two do not (not needed at 1280x720 today). A click anywhere on a
  BOOL row toggles it; on an ENUM row's label it opens the dropdown. Every change is
  `settings::set_row`; the screen writes nothing else (`RefitPlayer.ini` only).
- **Badges**: apply class (`Now`, `Next mission`, `Restart`), replaced by `Restart needed`
  or `Next mission` drawn active when the value differs from the one at the hub's start
  (RESTART) or at the start of this mission (MISSION, in a mission), `Must match` on a
  SCOPE_MATCH row, `Locked` with the control disabled. A must-match row is locked over a
  mission when `engine::setup::network_game()` is true or unreadable.
- **Notes**: `<Plugin>_SettingsNote` (optional export) says a part is not running now;
  a plugin-wide note is a line under the tabs, a row note replaces the help (dimmed) and
  the row is drawn dimmed but stays switchable. Asked when the screen opens and once a
  second.
- **Redrawn menus off** (an ADVANCED row) asks first, since it takes this screen with it.
- **Reset tab** asks, then removes the overlay keys of the tab's visible rows
  (`settings::remove_many`, the plugins told once).
- **Log lines** (UI.log; the bench reads these words): `refit: opened in the shell|a
  mission (Graphics 19, ...)`, `refit: closed`, `refit: [Render3D.Modules] sky 1 -> 0
  (now|next mission|restart)`, `... could not be written`, `... locked in a network
  game`, `refit: <Tab> reset (n keys removed)`, `refit: <Plugin>: <note>`, `refit:
  menus off asked; yes|no`.

## The Options family

Options (`options.rs`), Game, Graphics and Sound Settings (`game_settings.rs`,
`graphics_settings.rs`, `sound_settings.rs`, all `form.rs`), the small confirmations
(`notice.rs`: message, yes or no, net waiting) and the text windows (`text_window.rs`:
help, licence). All draw with `crate::widgets` in the shell theme over `theme::backdrop`
(a full-screen dialog), the mission's dimmed picture (in a mission) or a dimming of the
dialog under them (a dialog smaller than the screen), and act only through the stock
controls: a click on a stock button is `Screen::click_control`, a slider is a click on
the bar's track, a combo choice is below. **Every word is the stock control's own**
(`TextLabel` and `ShellButton` text, localised); the specs never write one.

- **Game Settings' "Command Bar"** is the one row of ours on a stock settings screen
  (`form::draw_with`, `form::Own`): the button bar shown or hidden, the toggle that was
  the top-right palette button in a mission (HUD.md, "The top-right buttons"). It is a
  setting of the UI's, not the game's, so it is kept in the player's overlay,
  `RefitPlayer.ini` (`[UI.HUD] BarHidden`, `prefs.rs`), and its words are ours; a click plays the shell's click sound (`cornrclk.wav`).
- **Options**: the stock buttons in the order they stand on the stock screen (y, then
  x), in one column, with one entry of ours, **Refit settings** (the Refit screen,
  below), directly under Game Settings; the one at the foot (Previous Menu, Return to Game: a button in the
  bottom-left corner, `common::is_footer`) is the window's footer. In a mission there
  are nine stock entries (ten with Refit); from the main menu three (four). The entry is
  drawn only by this spec, and not when the settings hub found no row to show. The window's title is the stock label at
  the top ("OPTIONS"); the version is the small label at the bottom.
- **The settings form** (`form.rs`) finds what belongs together by place, not text: the
  stock screens put a caption 16 to 30 pixels above the control it names, so each
  control (button, slider) takes the nearest caption above it that overlaps it sideways
  and is not taken yet. Two labels in the same place are a slider's end captions (Game
  Settings' "Hard" and "Easy", on the line of the caption of the slider below them, the
  right-aligned one the high end). Captions left over are headings. A screen with
  controls on both sides of its middle is two columns. Hover and click go to the stock
  button, so its sound and state are stock's.
- **Display Mode and Display Driver** (`do_graphicsOptions`): the screen keeps two
  combo boxes (ids 2102 and 2103) as hidden windows with meaningless rectangles, draws
  the fields itself and opens a list of its own on a click. Setting the combo's
  selection and sending the notifications changed the combo but **was not kept** when
  the screen was left (the mode read the old one on re-entry, as `Menus.Specs=0` showed
  stock keeping it). A choice is therefore two clicks of the screen's own: on the
  field, which stands under its caption (the caption's centre, 15 pixels below its
  bottom), and on the row, 30 pixels apart, the current choice first and then the combo's
  items in order. The combo boxes are found by id and named by the place of their
  caption (the nth caption left of the middle, counting the heading), since their
  rectangles say nothing.
- **Message, yes or no, net waiting**: the first stock label is the title when there is
  a second, the rest is the text; the buttons in a row in the order they stand. Net
  waiting has no title.
- **Help and licence**: the text is read whole from the stock control that holds it
  (the longest edit, static or label), since the model keeps only 4 KB of a control's
  text; our scroll area shows it, the stock buttons stand below. The help was seen
  from the Internet lobby (2026-10-16). **Not reached on the bench**: the licence shows
  on a first start, net waiting while a game is joined; each returns false (the generic screen draws) when it
  does not find the controls it expects.

## The setup and multiplayer specs

Each finds its controls in the model by kind and place, not by label, so it works in
every language and in each of the screen's states, and returns false (the generic
screen draws it) when they are not there. All act only through the stock controls.

### Game Setup (`game_setup.rs`)

Instant Action's setup and every multiplayer game's (`do_multiplayerSetup`).

- **The player rows are `ComboBox`es, not cycle buttons.** Each row is three (player or
  computer, side, team; the player one doubled, one of the pair hidden), and each shows
  the choice of a hidden Win32 combo box and draws a pull-down list of its own into the
  dialog when clicked (`engine::shellcombo`). The model reads a `ComboBox` like a
  button (its rectangle, shown, enabled) and its items and choice from the hidden combo
  box (`CB_GETLBTEXT`, `CB_GETCURSEL`). Our drop-down shows them; a choice is the two
  clicks a player would give: on the stock button, which opens the pull-down, and on
  the item in it. The pull-down's rows are `h` (+0x17c) tall from the top of its
  rectangle (+0x180); opening downwards, the first row is the button itself, so item
  `i` is row `i + 1`; opening upwards, item `i` is row `i` and the last row is the
  button (`InDropDownArea` tests the same bands). Sending the hidden combo box
  `CB_SETCURSEL` instead would change what is shown without the screen's handler.
- **Colours** are the stock swatches, cropped from the mirror, left of each row and in
  the palette under the table, and clicked where they are: a click on a row's swatch
  and then on a palette swatch recolours that player, as in stock.
- **Width**: on a screen wider than 16:9 the screen is laid out in a centred box of
  16:9 (`setup_kit::wide_box`), as the lobby is: the tables and the map window would
  otherwise stretch across a 21:9 monitor.
- **The map** window: the map's labels (name, players, "Teams are locked."), the
  minimap preview cropped from the mirror (design 627,129, 155x155: the frame's inside,
  clear of the "Shroud On, Fog On" label, which the stock screen draws across the
  frame); a click on it is a click on the stock preview (start locations, with Fixed
  Placement). The settings are buttons labelled as the game labels them, which say
  their state ("Resources: Lots", "Fixed Placement"); a toggle's on flag does not
  always mean on ("Warp Enabled" is a toggle that is off), so no check mark is drawn.
- **Messages and chat**: the multi-line read-only edit under the table is the screen's
  log (it says "Resources: Lots" when the host changes it, in Instant Action too); a
  single-line edit is the chat line, our field, whose typing goes to the stock edit.
- In stock the Instant Action "Minimap Hidden" toggle does nothing when clicked.
- **A multiplayer game's rows** have a fourth combo, the "Armada" column (x 295:
  Personal, ...), between the side (x 203) and the team (x 375), and a ready light at
  x 14 (a 10x10 picture, red, green once the player has pressed LAUNCH; under the
  "Comm" heading). The spec lays four columns out when it finds the Armada combos, and
  crops each row's light from the mirror. The screen has a second multi-line edit (id
  2120, x 600, the observers), so the log is the multi-line edit under the table (2093);
  the chat line (2094) has the chat target toggle ("All", x 315 y 533) beside it, which
  the spec draws as a button beside our field. The map preview stays black in a
  multiplayer Game Setup on the bench, in stock too (Warzone).

### Advanced Options (`advanced_options.rs`)

`LaunchAdvancedOptionsWindow`. Two columns as in stock. The sliders are
`ShellScrollBar`s, cropped from the mirror here (written before the model read them as sliders): each slider's three labels
(the name centred, the two ends justified left and right) are read, and the bar is
cropped from the mirror 16 design pixels under them, from the left end's label to the
right end's. The pointer over a bar, and a drag that started on one, goes to the stock
dialog at the bar's design place (`generic::forward_input` with the pointer mapped
into it), so the stock bar moves itself.

### Map Select (`map_select.rs`)

`do_mapSelect`. The stock list is fourteen `FileSelectButton` rows whose labels are the
maps at the scroll position, two arrow buttons beside them, a filter (`ComboBox`), and
Select Map and Cancel. Our list shows the fourteen; the wheel and our arrows click the
stock arrows; a click on a row is the stock click (which chooses it), a double click
that and Select Map. The stock rows do not say which map is chosen: a clicked row is
drawn held (button state 2) and stays so when another is clicked or the list scrolls
under it, so the spec marks the row it last clicked, by label, and the held one before
any click. The preview is Game Setup's (it shows the chosen map once Map Select
closes).

### Multiplayer Connection (`multiplayer.rs`) and the small dialogs (`forms.rs`)

- **Multiplayer Connection**: the name (the stock edit, as our field), and the
  connection buttons in their stock order, labelled as the game labels them. With
  `Online.asi`, *Internet - Online* is the stock IPX button relabelled
  (online/README.md, "The menu entry"), so it is in the model like the others and a
  click on it is the stock click that `Online.asi` turns into the online flow.
- **Manual IP, the game's name, create room, password, map transfer, room select and
  the GameSpy account screens** are `kit::form`: one window with the dialog's labels
  (the first, when it stands above everything, as the title), edits as fields, lists
  as lists, and the buttons along the bottom. GameSpy is gone, so the account screens
  and the room list are not reached; they are forms so that they work if they are.

### The Internet lobby (`lobby.rs`)

`do_internetGame` (`InternetGameDlgProc`, 0x5b15f0): the screen every network
connection type comes to before Game Setup, *Internet - Manual IP*, the LAN types and
*Internet - Online* alike (online/README.md). Its template (0x871) holds three windows:
the chat log (id 2098, a read-only multi-line edit), the chat line (2097) and the hidden
combo box behind the game-type filter (2114); the rest is shell widgets the procedure
makes in `WM_INITDIALOG`, each labelled from `label.map`'s `internet_game` section: the
title (`internet_title`, `lan_title`, `gamespy_title` by connection), the captions
"current games", "players in room" and "chat", Create Game and Join Game under the game
list (design y 258), Help, Previous Menu and Main Menu along the bottom (y 562), the
game rows (`GameSelectionSlot`s in a `RadioBox`, 537x16 from design 24,72, one per
game the search has found: `SetDescriptor` writes `name  (players/max)  ping  "map"
(type)` as the button's label), the players in the room (`PlayerListButton`s at
x 598; outside GameSpy they hold the stock sentence "Player list is available in
GameSpy sessions only.", one line per disabled row), the filter (a shell `ComboBox`),
and three of the shell's `ScrollBar`s, whose arrow buttons are 13x10 picture buttons.

**The rows are not `ShellButton::UpdateButton`'s.** Both `GameSelectionSlot` (vtable
0x6badcc) and `PlayerListButton` (0x6bae74) draw themselves with an `UpdateButton(HDC)`
of their own, so the model, which knows a button by that slot, left them out: the lobby
showed no game though the search had found one (`Online.log`: `ENUM_HOSTS_RESPONSE`).
`engine/shellui.rs` names the two vtables as buttons; their label is the text the row
shows. Unused rows are hidden, a row of a game that can be joined is enabled.

The spec finds them by place: the game rows are the labelled buttons left of x 560
between the caption and the actions, the players those right of it, the actions the
labelled buttons at y 250..280, the way out those below y 540, each list's arrows the
picture buttons of its column (the games' at x 568 above y 260, the players' at x 774;
the chat log's at x 570 below are the log's own and not drawn). Ours: the title, a
"current games" window (the filter as our drop-down, the rows as our rows, Create Game
and Join Game under them), the players' window, a chat window across the bottom (the
log's last lines, the line as our field) and the way out, in a centred box no wider
than 16:9 (`setup_kit::wide_box`, as Game Setup). A click on a row is the stock
row's click, which selects it (its held state shows as our selection); the arrows click
the stock arrows. `Online.asi`'s messages ("No game with the join code ...", the "Local
IP Address" line the game writes itself) are lines of the chat log, so they show there.

**`Online.asi` opens no dialog of its own.** *Internet - Online* runs the Manual IP
flow: the Manual IP dialog (`do_manual_ip`) with its prompt relabelled ("Join code or
host address"), then this lobby, then Game Setup, and the server's messages and the
host's join code go into the lobby's and Game Setup's chat boxes (online/README.md,
"Showing the join code"). So every screen of the online flow is a stock dialog with a
spec here (`forms::manual_ip`, `lobby`, `game_setup`); nothing of `Online.asi`'s needs
to look like ours.

## The screens

Every shell dialog, by the `do_*` function that opens it. Template is the resource id
(`DialogBoxParamA`'s second argument), name is `screen_name`'s. **Mirror** says what the
model cannot read and a spec would have to crop from the mirror. "Reach" is how a bench
scenario gets there (`testbench/ui.json` has the named targets).

| `do_*` | Template | Name | What it is | Reach | Controls of note | Mirror |
|---|---|---|---|---|---|---|
| `do_mainMenu` | 0x123 | main menu | the title screen | launch | 7 picture `ShellButton`s, labels in the art; the flare Bink | the art (spec: faded behind) |
| `do_single` | 0x73 | single player | campaign selection | Single Player | 5 picture buttons (tutorial, 3 campaigns, previous), Load Game; hover Binks | the art; labels are in it (spec: `single_player.rs`, a cut of the art per card) |
| `do_missionSelect` | 0x124 | mission select | a campaign's mission list | a campaign | `ShellButtonSelBox` rows, OK, Cancel | none (spec: `mission_select.rs`) |
| `do_loadGame` | 0x123 | load game | saved games | Load Game | `FileSelectButton` rows | none; no details in the stock list (spec: `saves.rs`) |
| `do_saveGame` | 0x129 | save game | a mission's Save Game | Esc, Save Game | the name `Edit` (476x28 @161,504), rows | none (spec: `saves.rs`) |
| `do_escapeMenu` | 0x123 | options | Options, from the main menu and in a mission (more entries in a mission) | Options; Esc in a mission | buttons; version label | none |
| `do_gameSettings` | 0x123 | game settings | Options -> Game Settings | Options | cycle/toggle buttons, scroll bars | scroll bars |
| `do_graphicsOptions` | 0x1 | graphics settings | Options -> Graphics Settings | Options | display-mode list (`display mode` target), toggles | the mode list |
| `do_soundSettings` | 0x123 | sound settings | Options -> Sound Settings | Options | `ShellScrollBar` volumes, toggles | scroll bars |
| `do_multiplayerSetup` | 0x830 | game setup | Instant Action's and every multiplayer Game Setup | Instant Action | cycle buttons (race, AI, resources), SELECT MAP, LAUNCH, chat `Edit` | the minimap preview |
| `LaunchAdvancedOptionsWindow` | 0x881 | advanced options | Game Setup's Advanced panel | Game Setup -> Advanced | toggles, cycles | none |
| `do_mapSelect` | 0x87f | map select | Game Setup's map list | SELECT MAP | a list box | the preview |
| `do_mapTransfer` | 0x87e | map transfer | sending a map (multiplayer) | not on the bench | progress | — |
| `do_multiplayerConnection` | 0x872 | multiplayer connection | name and connection type | Multiplayer | the name `Edit` (399x32 @208,171), connection buttons | none |
| `do_internetGame` | 0x871 | internet game | the lobby | Multiplayer -> a connection | game rows, players, chat `Edit`s, filter `ComboBox` | none (spec: `lobby.rs`) |
| `do_manual_ip` | 0x887 | manual ip | the IP field | Multiplayer -> Manual IP | an `Edit` | none |
| `do_roomSelect` | 0x87f | room select | GameSpy rooms | not on the bench (no GameSpy) | list | — |
| `do_createRoom`, `do_gameName` | 0x873 | create room, game name | a name | Internet lobby -> Create Game | an `Edit` | none |
| `do_password` | 0x874 | password | a game's password | joining a passworded game | an `Edit` | none |
| `do_changePassword`, `do_createNewAccount`, `do_gamespyLogin` | 0x884, 0x883, 0x882 | | GameSpy accounts | not on the bench | `Edit`s | — |
| `do_netWaiting` | 0x73 | net waiting | waiting for the network | hosting or joining | a message, Cancel | none |
| `do_helpWindow` | 0x885 | help | the lobby's help | Internet lobby -> Help | text | the text window |
| `do_licenseAgreement` | 0x885 | licence agreement | first start only | not reached | text | the text window |
| `do_oneButton`, `do_yesNo` | 0x73 | message, yes or no | confirmations (Exit, abort, errors) | main menu -> Exit | text, buttons | none |
| `do_TechTreeWindow` | 0x131 | technology tree | in a mission | the in-mission menu | an `Edit` in Courier ("What the shell is, in the engine", edit boxes) | none: drawn from the edit's text (spec: `tech_tree.rs`) |
| `do_admiralsLog` | 0x880 | admiral's log | score after a mission, or from Options | end of a mission | 8 owner-drawn tab buttons, player list, 8 panes (`CreateDialogParamA` 0x123, `ScreenInformation`) | the seven graph/list panes; Score is drawn from labels (spec: `admirals_log.rs`) |
| `Shell::Screen::Show` | (varies) | shell screen | voice options, command-line host/join | not on the bench | `Shell::Screen` widgets | — |

The `g_shellMode` cases of `DispatchDialog` that open these: 1 main menu, 2 single
player, 11 and 25 the in-mission menu, 13 multiplayer connection, 18 and 24 load game,
22 internet game; Options and the rest are opened from those screens.

## The single-player specs

Six specs, in `src/menus/screens/`, sharing `kit.rs` (the backdrop, a centred window, a
stock control drawn as our button, `forward_part`). All draw with `crate::widgets` in the
shell theme and act only by clicking stock controls.

- **`single_player.rs` (campaign selection).** Four cards (Tutorials, Federation,
  Klingon, Borg), each a cut of the stock art (`mirror_part`: the faction's picture, which
  a theme cannot draw) under a title, and Previous Menu / Load Game at the foot. The words
  of the stock screen are not on its controls: the two top panels are hit areas of the
  procedure itself and the lower two picture `ShellButton`s, with no label; the four names
  are hidden `TextLabel`s the screen shows in the middle on hover, which the cards use.
  A click is a click on a point inside the stock panel, and the card's hover is a
  `WM_MOUSEMOVE` to it, so the stock screen's hover animation (a Bink drawn into the art
  the card shows) and sound play. **The stock screen opens the mission list on the button
  *down*, not the up**; a click sent as down then up (`actions::ClickDesign`) therefore
  never reaches the up before the nested loop starts, and the hidden dialog keeps the
  mouse capture its down took: the real pointer's messages then went to a hosted window
  and were dropped, so the mission list took no click. `run_modal` now releases a capture
  held by a hosted window when a dialog opens (the generic screen never met it: it sends
  the up as a separate action, frames later).
- **`mission_select.rs`.** The rows (`ShellButtonSelBox`, one per mission, text from the
  game) as our list in a window over the (dimmed) campaign screen, O.K. and Cancel. The
  two buttons lowest on the dialog are O.K. and Cancel, the rest are rows, top to bottom.
  A click on a row is a click on the stock row; **the stock rows do not show which one is
  chosen** (their mode stays 0, hover 1, held 2), so the highlight is ours, the row last
  clicked. A double click is O.K.
- **`saves.rs` (Load Game and Save Game).** The rows (`FileSelectButton`s, "name Mon day
  time", split by our code into name and date) as our list; Previous Menu, the action and
  Delete Game at the foot (Load and Delete are drawn off until a row is picked: the stock
  buttons' flag stays set); the stock scroll buttons (two 24x16 picture buttons) as our
  arrows, also the wheel over the list. The stock list is one page whose rows are
  re-labelled by those buttons, so the choice resets when it pages. Save adds our text
  field for the stock name `Edit`: it keeps its own text and, when it changes, rewrites the
  stock edit with End, one Backspace per character it held and the new characters, as
  typed (`Action::Key`), and takes the stock text again when the stock edit changes under it
  (a row was picked: stock copies its name in). Enter in it is the Save button. The model
  has no "details" of a save (stock's list shows none), so there are none to draw.
- **`admirals_log.rs`.** A window with our tab strip, the players down the left (the
  owner-drawn buttons' text, "name", line break, "team"), the page, and Done (stock's Save to text is not offered).
  A click on a tab, player or button is a click on the stock owner-drawn window. Which tab
  shows is the visible pane among the eight (the panes are listed in the tabs' order
  backwards), the last clicked when it cannot be told. **Score** is drawn from the pane's
  labels (the model reads them: column headers and the value rows, in the game's colours)
  as our table, its rows level with the players'. **The other seven pages** (Military,
  Economy, Timeline, Battles, Ships, Build, Tally) are frame art (`bitmaps/AdmiralsLog/
  Screens/FRAME*.bmp`) with the pane's `StdTextLabel`s drawn over it
  (`IndividualScreenInfo::DrawLabels`, clipped to the pane), and the model reads every
  one: its design rectangle, colour, alignment and all its lines (Military's and
  Economy's cells are three and four lines; the Ships page keeps labels of rows
  scrolled out of view, below the pane). `typeset` sets them again: each label in its
  place in the pane mapped into the page, rows (labels whose tops are within 4 pixels)
  striped, a row of several one-line labels without a digit set as a header (Tally's
  stands under its summary; Build has none), each cell's lines fitted to the room its
  row has; labels below design y 566 are scrolled out and left out. What is a picture is
  cropped from the mirror: Timeline's graph (design 177,125 612x442) and its legend's
  two icons, and each Ships row's portrait (design x 183, 216 wide, from the row's top to
  the next). The lists' stock scroll buttons (two unlabelled `Shell(Button)`s at x 760)
  are our arrows at the page's corners, and the wheel over the page clicks them; the
  stock list scrolls by pixels and hides a row that is partly out of view. The owner-drawn tabs, players and buttons are
  blank in the mirror (their `WM_DRAWITEM` painting does not reach the composed frame:
  not investigated, as the spec does not need it).
- **`tech_tree.rs`.** The stock window is one disabled multi-line Courier `Edit` with the
  tree as ASCII art and four scroll buttons. The model reads the edit's text whole (the
  limit of its reader is now 128 KB; a full tech set's tree can pass 4 KB), and the tree is
  drawn from it on its character grid: `|`, `-` and `\` as thin lines, `*` as a dot, a name
  set in the theme's font at its column (`(*Name)`, a requirement shown again, dimmed), the
  roots in amber, in a clipped scroll area of the widgets. The size is chosen so the widest
  line fits, so there is no sideways scroll. The edit is never driven.
- **A text field of the widgets has the keyboard**: `menus::wants_keys` now also asks
  `input::keys_captured`, so Enter and Esc go to it and not to the dialog as `IDOK` and
  `IDCANCEL` while it is focused (`local::focused` covered only the stand-in).

**There is no end-of-mission screen to host.** `do_success_mission` (0x6079d0) shows
nothing: `Program::GameClose` calls it when the mission was won (`RunState`'s state 1,
which `AiMission::DoEndOfMission` sets from the script's `SucceedMission`), and all it
does is mark the campaign's next mission as reached (`ShellSettings::AdvanceOneMission`)
when the one won was the furthest yet. (An earlier reading here took the functions that
follow it in `SMain.obj`, `init_screen_pos`, `draw_cursor` and `Read_Shell_Bitmap`, for
its body.) `do_fail_mission` (0x6077c0) is never called. What follows a mission is
`Program::SetupNextScreen`'s choice: after a won campaign mission with another after
it, that mission's name, and the game loads it at once, so the next thing on screen is
its cinematic and then the loading screen (`LOADING.md`, ours with `[Loading]
Enable=1`); after the last of the last campaign, the Credits; when the campaign is
complete, the single-player screen; after a lost one, back to the shell. **On the
bench the Admiral's Log came up after the first Federation mission was won** (2026-10-16,
`Scene.asi`'s `succeed`), and its Done led to the **single-player screen**, not to the
next mission (2026-10-16, on the way through the whole game; after an aborted mission
the Log's Done led to the single-player screen too (once, in a session where the save
had been loaded before, to the campaign's mission list), after an Instant Action match to the
main menu), which
the reading above did not predict; so a won mission returns to the shell as a lost one
does, and the campaign goes on from the mission list. It is a dialog, hosted with its spec, so `[Menus] Enable=1` still needs nothing
of its own for a mission's end, and its outcome is the stock one by construction. `testbench/scenarios/
ui-menus-success.md` wins and loses the first Federation mission with `Scene.asi`'s
`succeed` and `fail` and checks what comes next.

## The menu scene

`[Menus] Scene=1` (the default): the main menu and the campaign selection stand over
render3d's map, a 3D scene with a place for each of them: a Federation fleet in orbit
over a planet, a starbase, Klingon ships before a Metreon nebula, a Borg fusion cube in
a Mutara nebula with the queen's Diamond (`render3d/README.md`, "The menu scene"). Code: `src/menus/scene.rs`.

- **Where it is drawn.** At the start of each frame of the dialog host's loop, after
  the clear and before `surface::frame(Context::Menus)`, so it is under everything of
  ours. The view is that of the full-screen dialog at the bottom of what shows: `main`
  for "main menu"; for "single player" the campaign last pointed at (`scene::choose`:
  `tutorials`, `federation`, `klingon`, `borg`), `federation` on each visit until one
  is (the mission list over the campaign selection keeps its view); for every other
  full-screen screen of the front end (Options and its settings, Instant Action's and
  multiplayer's Game Setup, Multiplayer and its lobby, Load Game) the view that showed
  last, so the camera stays where the screen was opened from; never over a mission
  (`backdrop::over_mission`). Those screens' backdrops (`theme::backdrop`, the setup
  and single-player kits') then dim the scene instead of drawing the old main menu's
  plate (`Menus/mainbkgr.bmp`), which is what showed behind them before. render3d moves the camera from one view to the
  next across the map. A dialog opened within 1.5 s of the last one closing keeps the
  mission's picture (Options handing over to Load Game), and aborting a mission opens
  Options, the admiral's log and the campaign selection in turn. So the main menu and
  the campaign selection, which no mission opens, drop it (`backdrop::begin`); else the
  scene stayed off after an abort (`ui-menus-scene-abort`).
- **How it is asked for.** `Render3D_MenuScene(name, x, y, w, h)`, a render3d
  export, looked up once with `GetModuleHandleA("Render3D.asi")` and
  `GetProcAddress`. It draws into the whole back buffer and returns 1, or 0 when it
  cannot (no Render3D.asi, render3d stood down, no such view, no device);
  the frame then goes on as without it.
- **The specs over it** ask `scene::shown()`. The main menu (`over_scene`) draws none
  of the stock screen's art and **passes no input to it**: off the column a click does
  nothing, where over the faded art it reached the stock screen's hidden targets
  (Instant Action, Exit) from empty space. The column keeps its places entry for entry
  (the bench clicks them by position); its entries are text that lights up with an
  accent bar and glow, the title rises above it, and the left of the screen is shaded
  for the text. Without the scene it draws as before.
- **The campaign selection over it** (`single_player.rs`, `over_scene`) is a column of
  the same kind under SINGLE PLAYER: each campaign its faction's emblem and its name,
  then Load Game and Previous Menu, quieter. Pointing at a campaign lights it and
  chooses its view; the row whose view shows keeps its accent bar at half strength. A
  click on a row is a click on the stock panel's point, as the cards' was, and the
  stock screen still hears the hover (its sound).
- **Hover sounds.** The stock main menu plays a hover sound only for its four
  `ShellButtonSelBox`es (Options, Exit, Replay Intro, Credits): Single Player, Instant
  Action and Multiplayer are hit areas of `MainMenuDlgProc` with Bink hover animations
  and no sound, and on the campaign selection the Tutorial and Federation panels are
  the same. `ShellButton::SetButtonMode` plays a button's hover sound as an
  `AudioSound2D` of the file `SetSoundFile` gave it (`+0xf0`, when `+0xec` is set),
  else `cornron.wav`, at priority 8; `engine::sound::shell_hover` plays the same for an
  entry of ours whose stock target is no button (the screen's own buttons' file), and
  for the campaign column's Load Game and Previous Menu, whose stock buttons the
  pointer is never sent to. The emblems are the HUD's race icons,
  white silhouettes with their alpha (`federation_icon`, `klingon_icon`, `borg_icon` in
  `Sprites/gui_global.spr`: 84 px cells of `Textures/RGB/allrace.tga`, read from the
  game's own files, so a texture pack's version shows), tinted as the text. The
  tutorials have no race: their emblem is Starfleet Academy's patch, the triangle the
  stock tutorial card shows, drawn as an outline round the Federation icon's delta.
  Without `allrace.tga` the names stand alone (UI.log: `menus: campaign emblems`).
  Without the scene the four cards draw in a 2x2 grid as before.
- **Locked campaigns** (`src/menus/locks.rs`, `src/engine/progress.rs`, issue #13): the
  stock screen draws a campaign the player cannot play yet exactly as an open one, and a
  click on it does nothing (no mission list, no sound of its own). Ours greys it: its
  name and emblem in the theme's off colour, a padlock after the name (the icon atlas's
  `padlock`, STYLE.md, "Icons"), a grey bar in place of the accent, and on hover a line
  under the name, "Complete the <campaign> to unlock" (in the 2x2 cards without the
  scene: the picture dimmed, the line across its foot). The view still follows the
  pointer, and render3d shows it locked (`Render3D_MenuLocked`, render3d/README.md, "The
  menu scene": the Klingons held cloaked, shimmering in the cloak's glass; the cube,
  turned a corner towards the camera, and the Diamond sit down in the Mutara gas). A
  click plays `[Menus] LockedSound` (`mnu_back.wav`, the game's own LCARS "back" chirp:
  short, soft, falling at its end, and used by nothing else in the shell; the stock
  panel plays nothing on a locked campaign, and its click sound, `cornrclk.wav`, would
  say the click worked; empty for none; UI.log: `menus: locked campaign <key> clicked:
  sound <file>`) and still goes to the stock panel, which decides; `CampaignLocks=0`
  turns the look off.

  **`GIVE_ALL_MISSIONS` opens everything.** `ART_CFG.h` carries `int GIVE_ALL_MISSIONS
  = 1` in the GOG install this was measured on (every copy on the bench, back to
  October, has it). While it is 1, `SingleDlgProc` calls
  `ShellSettings::GiveAllMissions` each time the single player screen opens: every
  counter goes to its last mission and every campaign is marked complete, in memory
  (UI.log: `reached [5, 9, 9, 9], complete [1, 1, 1, 1]`), which the game then saves
  as any progress. So with it nothing is locked, in the game or in our look; with 0
  the progress is the player's own. (The same procedure gives all missions for a typed
  code, the other caller.) `ui/bench-progress.sh` sets it to 0 in a bench clone; a
  player who wants the campaigns to open in order sets it to 0 in their own
  `ART_CFG.h`.

  **A bench clone read the player's own `save\shell.set`.** The single player screen
  opens `save\shell.set` under the install path of the prefix's GOG key
  (`HKLM\Software\Wow6432Node\GOG.com\Games\1174788223`, `path`), not the working
  directory, and a clone's prefix kept the real install's path (seen with inotify: the
  real file opened, the clone's not). `ui/bench-progress.sh` points the key at the clone.

  **What the game decides by, measured on the bench** (2026-11-05, by crafted
  `save\shell.set` files, a won mission, and the running game's memory read from outside;
  platform/README.md, "Campaign progress format"): the file's first eight bytes are, per
  campaign in the order Tutorials, Federation, Klingon, Borg, the furthest mission
  reached (0-based: a counter of `n` lists `n + 1` missions; winning the first Federation
  mission took byte 1 from 0 to 1), then whether the campaign is complete (set when its
  last mission is won; the counter stays at 9). **The Klingon campaign opens when byte 5
  (the Federation complete) is set, the Borg when byte 6 (the Klingons complete) is**:
  with only byte 5 set, the Klingon panel listed its first mission and the Borg's did
  nothing; byte 6 alone, the reverse; the counters open nothing (byte 1 at 10, the
  Klingons stayed shut). Byte 4 (the Tutorials complete) gates nothing: the Federation
  campaign is always open. A new profile is all 0 but byte 11 (1), which none of this
  reads.

  **The UI reads it in memory, the file only when it must** (`ui.shell.reads_progress`):
  the game writes `shell.set` when it exits and when a won mission moves a counter on,
  so after the last Federation mission was won the Klingon panel opened at once while
  the file still said otherwise until the game was left. `g_shellSettings` holds the
  same eight bytes at +0x0d (counters) and +0x11 (complete), followed by each campaign's
  mission count at +0x15 (6, 10, 10, 10); found by reading the object of that name in
  a game started with a recognisable file. The UI takes it only when it looks like
  progress (each counter below its count, each flag 0 or 1), and reads the file's bytes
  0..7 otherwise. Read four times a second; UI.log: `menus: campaign progress from
  memory: reached [...], complete [...]; locked: ...` each time it changes.
- **A mission loaded over it departs from it**: the loading screen draws over the
  view that showed last, whose ships go to warp into render3d's tunnel
  (`LOADING.md`, "Over the menu scene"; `Render3D_MenuWarp`).
- **For the bench**, UI.log says `menus: scene <name> on` and `menus: scene off` at
  each change of screen. `testbench/ui.json` gives the main menu's and the campaign
  selection's targets a `label`, which `Click the <target>` clicks (by OCR) while the
  scene is on, since their stock points are on nothing then.

## Roots' forms

Fleet Operations: Roots only (PLAN-FLEETOPS.md, FO-6 step 7, option C), with
`[Menus] Forms=1` (0 in `UI.ini` and in Roots' defaults until each screen is seen in game;
the Refit screen shows an advanced "Redrawn front-end forms" row, Interface tab, only on a build
that has forms; it applies at the next start and does not need `Enable`).
On Roots the main menu, Options (the in-mission menu too), Yes/No, Manual IP, Multiplayer
Connection, Map Select, Graphics and Advanced Options never run the executable's dialogs:
`FleetOpsHook.dll` shows a VCL form of its own (a Delphi library's window class), modal,
over the engine's device window, and frees it when the screen closes. Its buttons are
graphic controls with no window. The dialog host above cannot reach them, so the forms have
a host of their own, `engine/forms.rs` (drawn by `menus/roots.rs`):

- **Found by its window.** A thread CBT hook on the game's thread (the one `DllMain` runs
  on, which is the thread the library made its windows on) sees each window activate. A
  window whose class is one of the forms the UI takes is a form; its object is the data of
  the window property the VCL sets on every window it makes, named `ControlOfs` and the
  library's module handle and thread id in hex. The object counts only when its class lies
  in FleetOpsHook's image, is named as the window's class and derives from the VCL's form
  class (read through the class's own name and parent slots, which the language runtime
  lays out).
- **Read, never written.** Each tick the form's owned components are walked: each control's
  name, class, rectangle, visibility, enabled state and text, at the VCL's field offsets
  (`profiles/gog-1.1/ui.forms.toml`, hook `ui.forms.vcl`: the VCL's fields, read only on a
  build whose `forms_present` is 1). Every pointer is checked (aligned, in user memory,
  readable), only on the game thread, only while the form is the modal one and not being
  destroyed, and the frame sees only copies. Text fields and combo boxes have windows: their
  text and entries come through those (`WM_GETTEXT`, `CB_GETLBTEXT`).
- **Hidden by moving it off screen.** An empty window region clips the form's painting but
  on Wine under XWayland its pop-up still covers the screen in black, and a layered window
  at alpha 1 does the same (the probe of step 0): the form is moved far off screen, its size
  kept, and moved again whenever it comes back. It keeps the activation, the keyboard focus
  and its modal loop.
- **Drawn from a timer.** While a form is modal the engine draws nothing. A thread timer of
  the UI's (16 ms) is dispatched by the form's own loop; each tick runs the actions the last
  frame queued, reads the form, prints it into a memory DC every 0.1 s (`WM_PRINT`, the
  mirror's picture) and presents a whole frame of ours on the engine's device. The design
  space is the full-screen forms' client size, Roots' 1200x900 shell
  (`forms_design_width`/`_height`), fitted to the back buffer; UI.log says `menus: forms
  1200x900 @0,0 -> WxH @X,Y`. The pointer's shape is set to the arrow each tick: with the
  form away nothing else sets it.
- **Acting as a player would.** A press of ours is a posted mouse move, left down and left
  up at the control's centre, to the form's window: the VCL finds the graphic control there
  and Roots' own button code runs the action Roots binds at that moment (Back is bound at
  run time). Combo boxes are set by `CB_SETCURSEL` and the parent's `CBN_SELCHANGE`, which
  the VCL turns into the control's change event. A click on a text field gives the real
  field the keyboard; the typing goes to it, and ours shows its text. Areas the UI does not
  draw as its own (check boxes, sliders, grids) are crops of the form's picture with the
  mouse forwarded to the window under the point. Nothing calls a handler of Roots' and
  nothing writes a setting: Roots' handlers write what they write (Graphics saves the
  profile on Back; Manual IP saves its settings file on Join).
- **A grid's rows are read, not printed.** NextSuite's `TNextGrid` (Map Select's list, the
  Mods list) paints through its own window's device context, which is clipped to nothing
  while the form is off screen: neither a print of the form nor of the grid's window has
  the rows. The UI reads the cells (read only, `grid_of`): the grid's cells object holds a
  list of cells for each column, a string cell a wide string and an integer cell the
  integer, told by the class's name; the selected row and the first row shown are fields of
  the grid, its row and header sizes published properties. Offsets are the profile's
  (`forms_grid_*`, found in the grid's `GetCells`, `GetRowCount` and `GetSelectedRow` and
  the string cell's `GetAsString`, measured on Roots 5.0Z11's FleetOpsHook). A row is
  chosen by a posted click at the row in the grid's own window, or the arrow keys when the
  grid shows another part of its list.
- **A poke is not seen.** `Action::Poke` moves the form under the real pointer for one
  message and back (decision 242). On the bench (sway, window events and a 20 Hz tree
  sample during seven pokes) the form never became a window: no `new`, `focus` or
  `urgent` event and the game window kept the focus, because Wine has the off-screen form
  unmapped and the two moves of one tick do not map it (decision 261). Hyprland is not
  measured.
- **Keyed to FleetOpsHook alone.** `FAMILY` lists the measured builds by the CRC-32 of their
  file's `.text` (`a2engine::companion_crc`; Roots 4.0.93, Roots 5.0Z11, Tactical Maneuvers
  5.2.3); any other stands the forms down, logged once, and Roots draws its own menus.
  `tools/engine/fo-forms.py` reads a new FleetOpsHook (and its map) once, prints its line
  and checks the forms and controls the UI uses are there.
- **What stays Roots'.** Tunngle's wait. The nine screens
  FleetOpsHook opens through its own `DialogBoxParamA` (Single Player, Game Setup, Game
  Settings, Load, Save, the licence, the Technology Tree, the Admiral's Log, Internet) are
  Roots' dialogs drawn by the UI: "The nine dialogs, adopted" below.
- **The fallback: the nine framed** (PLAN-FLEETOPS.md, decision 200), when the
  executable's paint imports cannot be ours (decision 230). The shell sizes a
  full-screen screen to the device window's client area, which on Display8 is the
  desktop's size, while Roots paints its 1200x900 art at the top left: the rest of the
  dialog stayed white (Forms=0 shows the same; it is not the forms' doing). With the forms
  on, every `#32770` owned by the device window is put where the frame wants it as it
  activates: a full-screen one cut to the design size and centred, a smaller one left
  where the shell put it unless that is off the client area (its owner a form of ours,
  moved off screen), then centred. It is checked every tick and put back if anything moves
  it. The pump goes on under it (the scene or the backdrop, the forms under it inert, a
  panel of ours round the dialog), so the area around it is ours. It stays a Win32 window
  at 1:1, and the window manager sees it as a window of its own. UI.log: `forms: #32770
  ... framed at X,Y WxH on the device window`.
- **The menu scene** (`[Menus] Scene=1`, decision 201): the forms' frame draws render3d's
  menu scene first, as the dialog host's does; the main menu's form stands over the
  `main` view and every other form and dialog over the view last shown. The `fleetops`
  set names only `Main` in `[Scenes]` (Roots' campaigns are not the stock four, so no
  campaign view is chosen). The screens' backdrop is then the scene, dimmed
  (`theme::backdrop`), as on stock, but under the main menu, which shades only its column;
without render3d, or when it draws nothing, it is the gradient as before. render3d draws the
`fleetops` set's own map there (DS9 and the Defiant: render3d/README.md, "The menu scene").

The screens: the main menu is Armada II's main menu over the scene (decisions 205 and 206:
`screens::main_menu::scene_column`, the same function Armada II's spec draws with, titled
FLEET OPERATIONS over ROOTS, the entries as text in capitals down the left, no window, the
scene showing through); Options is a window with a column of our buttons. Both take the
form's visible buttons in their on-screen order, each labelled with the caption Roots set on
it and pressed at its own rectangle, so
a button is what the player sees whatever its field is called (on Roots' main menu the
field named for Options reads "Map Editor" and runs the map editor, and the one named for
the map editor reads "Options"; decision 204). Options' Back, which Roots binds when the
form opens, stands at the window's foot, and the main menu's version line (Roots' `labelVersion`) at
the bottom right, small and dim, as Armada II's version. Yes/No draws the form's picture (Roots draws the
question into it) under our two buttons, where the form sits over the screen that asked;
Graphics, Sound, Mods, Advanced Options, Multiplayer Connection and Manual IP have specs of
their own (below); every other form is generic (`Generic=widgets` in Roots' defaults). The in-mission Options
stands over the mission's last frame, as the stock host's in-mission menus do.

**The specs** (`src/menus/roots/`, decisions 240 to 244; one file per form, `kit.rs` for the
controls they share, `specs.rs` choosing by the form's class) lay each screen out with
Armada II's functions (`screens::form::frame` and `connection_layout`) and draw Roots' controls
as ours: a combo as a dropdown (its choice is the window's selection and the parent's
notification), a check box with the state the control holds, a slider at its value, a text
field of Roots' own with the keyboard. A check box or slider is read through its published
properties (`engine/forms_props.rs`) and acted on through `forms::Action::Poke`, since
Roots' check boxes test a click against the real pointer's place. Graphics: driver,
resolution, anti-aliasing, detail, gamma and the switches; Sound: four volumes, the
channel count, unit speech, hardware acceleration, device and speaker set-up; Mods: the list
(a crop of the form's picture with the mouse forwarded), the mod's screenshot, title and
description, three buttons; Advanced Options ("Game Settings"): four dropdowns, the
switches, six sliders with end captions, OK, Cancel, Default; Multiplayer Connection and
Manual IP as Armada II's; Map Select: the filter, the list of maps, the preview and the two
buttons, with the rows read from the grid's memory (decision 260); the map editor's list
(the main menu's Map Editor: the same grid list with six columns, the preview and name, Create
new map, Delete Map and Edit Map) and its new-map name form (as Manual IP, Create dimmed while the
field is empty; decision 280). Edit Map ends, on Roots as for a player, in the editor's licence
dialog. Advanced Settings has a spec that nothing on Roots 5.0Z11 opens (decision 281), so it is
unbenched. Tunngle's wait is the one form left to Roots.

| Hook | What it reads |
|---|---|
| `ui.forms.vcl` | the VCL's fields of FleetOpsHook's forms and controls (owner list, name, rectangle, visibility, enabled, text, window, form and component state), and whether the build has forms; feeds `Menus.Forms` |

`[Menus] Forms=probe` is the bench's probe of step 0: the same host, plus a fixed walk of
posted presses (main menu, Options, Graphics, Back, Back, Instant Action), a combo set on
Graphics, and a log line of the pointer and focus every two seconds
(`testbench/scenarios/fo-ui-forms-probe.md`).

### The nine dialogs, adopted

PLAN-FLEETOPS.md, "Phase two, step one: the dialogs adopted", decisions 230–238;
`engine/fodialogs.rs`, drawn by `menus/roots.rs`. With the forms on, each `#32770` over
the device window (the nine, and what they open: the mission list, the one-button
message, the Admiral's Log's pop-up panes) is **adopted** as it activates. It is the
dialog host's model for a dialog the UI did not create:

- **Off screen, so the window manager never sees it.** It is cut and placed as the
  framing above would put it (its place in the design space), then moved off screen with
  its size kept, as the forms are. A window wholly off every output is not mapped, so the
  compositor manages only the game's window (sway's tree on the bench; the bench step
  `Expect the window manager to see one window`). The dialog keeps the activation, the
  keyboard focus and FleetOpsHook's modal loop; the UI's timer goes on inside that loop.
  A dialog with no control to take the focus is given it (decision 236).
- **Painted into a surface of ours.** The stock procedures draw through the executable's
  imports of `GetDC`, `ReleaseDC`, `BeginPaint`, `EndPaint`, `InvalidateRect` and
  `RedrawWindow`, which are pointed at functions of ours (each slot checked to hold
  user32's own function first; never while the dialog host owns them, which it does on
  Armada II). An adopted dialog gets a memory DC of its client size; any other window
  goes to user32. An off-screen window is never sent `WM_PAINT`, so the invalidations
  are noted and the pump sends the paint; the top dialog is painted at least four times
  a second besides, since an invalidation from FleetOpsHook's or user32's own code never
  passes the imports (decision 233). The surface's bitmap is selected back whenever its DC
  is handed out, as the host's is.
- **Composed into a picture** each time it drew, and every 0.1 s: the surface, then each
  visible child window printed with `WM_PRINT` (bottom of the z-order first). An
  owner-drawn button is printed into a metafile and played back at its place: the
  Admiral's Log's drawing helper selects a bitmap of its own into the DC it is handed,
  which on a memory DC replaced the picture and lost every tab (decision 237).
- **Drawn in the forms' frame**, in the design space with the forms, scaled to fit by
  any factor (decision 232: 1.6 at 3440x1440), in a panel of ours. Forms and dialogs are
  drawn in the order they opened, and a full-screen one hides what is under it. A pane (a
  pop-up owned by an adopted dialog) stands at its place against its owner, just above
  it, without a panel, only while shown (decision 235).
- **Driven as a player would.** The pointer over the picture is posted to the window
  under it in the dialog's client coordinates: every move (the stock buttons light up,
  and take a click, only under the pointer), the press, the moves while held and the
  release; to the uppermost live dialog it is over (a pane before its owner). The real
  pointer's messages can only reach an off-screen dialog through a capture it took on a
  press of ours, and their points lie outside its client area: a message hook drops them
  (decision 234). Keys reach the dialog's focus as they would: a click into an edit
  gives it the focus, and typing goes there (Save Game's name).
- **A model for a spec.** Its child windows (class, id, text, rectangle, style) and
  FleetOpsHook's record of the stock procedure (the record in the window's user data, read
  once; for a dialog the executable opens through its own import, which has no record,
  the dialog's own procedure, `DWL_DLGPROC`) are what the frame sees (`forms::Dialog`),
  and the shell's own widgets are read as under the host ("Roots' dialogs, laid out"
  below). A dialog without a spec is the mirror (`[Menus] Generic` does not apply to
  them). UI.log: `forms: #32770 ... adopted: client WxH, style ..., stock procedure
  0x...`, `... adopted at X,Y WxH in the design space`, and once, the child windows;
  `Trace=1` also writes the picture as a BMP into the game directory.

### Roots' dialogs, laid out

PLAN-FLEETOPS.md, decisions 250 onwards; `menus/roots_dialogs/`. An adopted dialog with a
spec is drawn by it, from the same two things a hosted dialog's spec reads:

- **Its model** (`model::read_adopted`), the host's walk over the adopted window: the
  shell's widgets that draw into it and its child windows, in Roots' design space
  (1200x900, the dialog at its place in it). The widget reader (`engine/shellui.rs`, the
  `Shell::Widget` constructor re-pointed) is installed by the forms on Roots, at DllMain,
  as the host installs it on Armada II: Roots' shell classes are stock's (the same
  vtables, the same fields), and it reads them as on Armada II once one difference is
  allowed for: FleetOpsHook re-protects pages of the executable's `.rdata` as it patches
  them, which splits `.rdata` into regions, and the buttons' vtable (`ShellButton`'s
  family) straddles two; the reader's check that a vtable can be read now spans regions
  (on Armada II the check gives what it gave). UI.log has each dialog's model, as the
  host logs it (`menus: model of "single player" ...`), and the widgets made for it that
  the reader does not know.
- **Its picture**, cut for what a theme cannot draw (a campaign's art).

The spec is chosen by the stock procedure (`forms::dialog_screen`): the profile's
`ui.forms.dialog_specs` lists the eight procedures FleetOpsHook opens or that open from
them (Single Player, the mission list, Load, Save, Game Settings, Game Setup, the
Admiral's Log, the one-button message), told apart by address only; Roots' build marks
each verified beside its edit, since FleetOpsHook's record was measured to hold stock's
address for every one. A spec acts as a player: `Screen::click` moves the pointer to the
stock control's point, and once it has rested there (0.12 s; Roots' screens take a press
only on a control already under the pointer, decision 190) presses, and releases 0.06 s
later, each in a pass of its own through the dialog's messages; one click at a time, in
order, and the clicks still waiting when something opens over the dialog (Roots' Yes/No)
are dropped, as a player's would land on it. Load and Save never click a stock row that is
already picked: two clicks on one are its own double click, which loaded nothing after
Roots' question answered Yes (decision 255). Typing into an edit is `End`, a Backspace for each character
it held, and the new text, posted to the edit as the keyboard would send them. Nothing
writes a setting. A spec that does not recognise its dialog returns false and the frame
draws the picture.

The specs reuse Armada II's layouts, factored out of `screens/` with Armada II's
behaviour unchanged: `single_player::campaign_column` and `campaign_cards`,
`mission_select::mission_list`, `saves::saves_screen`, `notice::notice_window`. Return in a text field of ours on an adopted dialog never reaches it: user32's modal loop turns it into the dialog's OK command first (decision 253).

| Dialog | Spec | What it shows |
|---|---|---|
| Single Player (`SingleDlgProc`) | `roots_dialogs/single_player.rs` | Armada II's campaign selection: over render3d's map the column (the four campaigns, each named by the dialog's hidden hover label and marked by a cut of its own card's art, then Load Game and Previous Menu), pointing at one moving the camera to its view (`federation`, `dominion`, `species8472`, `freeborg` in the fleetops map); without the scene, the cards (2x2). The top two cards are hit areas of the procedure, the bottom two picture `ShellButton`s, as on Armada II |
| The mission list (`MissionSelectDlgProc`) | `roots_dialogs/mission_select.rs` | Armada II's list in a window of ours over the campaign selection: the stock rows by their labels, OK and Cancel |
| Load Game, Save Game (`LoadGameDlgProc`, `SaveGameDlgProc`) | `roots_dialogs/saves.rs` | Armada II's save list: name and date, the stock list's two scroll buttons and the wheel, Save's name field writing the stock `Edit` (id 2100), the foot |
| Game Settings (`GameSettingsDlgProc`) | `roots_dialogs/game_settings.rs` | Armada II's settings form (a caption over each control): the six stock sliders (set by a click on the track, `generic::click_scroll_with`) and the two cycle buttons; Roots' labels hold no text, so the captions are words of ours, given in the order Roots stacks the controls (decision 256) |
| The one-button message (`OneButtonDlgProc`) | `roots_dialogs/notice.rs` | Armada II's message window (`notice::notice_window`): the labels as title and text, the stock OK |

**The campaign tiles are Roots' art**: the column draws each card's own picture, cut from
the dialog's picture at run time (never stored; decision 251), because the α, γ, V and
B on them are the only marks Roots gives its campaigns and no theme of ours can draw them;
the names, the layout and everything else are ours.

## Testing

`ui/bench-asi.sh --menus` (a scenario's `Setup:` line) puts `UI.asi` in a bench clone
with `[Menus] Enable=1`. `testbench/operating/menus-host.md`
has where things are and how to look inside a hosted dialog (`Menus.Trace=1` dumps the
hidden surfaces).

- `ui-menus-scene.md` (run with `--install .`): the main menu and the campaign
  selection over render3d's map, each campaign's view by hovering its row (the
  Klingons decloaking, the cube's shadow in the gas), back again.
- `ui-menus-options.md`: the main menu's spec; Options; Graphics Settings' Bump Mapping
  switched off, the display-mode list opened and a mode chosen, the screen left and
  opened again, and still off; Sound Settings' Music Volume slider and Unit Speech, the
  screen opened again; Game Settings' Game Difficulty stepped; back to the main menu;
  quit.
- `ui-menus-slider-max.md`: Graphics Detail set to its least and back to its most (the
  stock bar's top lies past its track: `click_scroll`), read back from UI.log.
- `ui-menus-refit.md`: the Refit screen (`testbench/operating/refit-screen.md`): the
  tabs, a row of each kind written and logged, Esc closing only the modal, in the shell
  and in a mission.
- `ui-menus-dialogs.md`: the in-mission Options (nine entries), Abort Mission's and
  Exit to Windows' questions answered No, Sound Settings from the mission, Save Game's
  message and its O.K., Return to Game.
- `ui-menus-widgets.md`: `[Menus] Generic=widgets` on the Multiplayer connection screen:
  the name edit as our text field, typed into and read back from the stock edit.
- `ui-menus-single.md`: Single Player's card for the Federation campaign, the mission list,
  the first mission, in it Esc, Save Game with a typed name, the Admiral's Log (Score and
  Timeline), the Technology Tree, Load Game of the save (abort, Yes); quit.
- `ui-menus-match.md`: Instant Action's Game Setup with the local and computer players
  as stock shows them; player 2 set to Computer AI:Easy and player 1's side to Borg;
  LAUNCH; the in-mission menu over the mission, closed by Esc; Save Game with a typed
  name; Load Game of that save (abort, Yes); quit.
- `ui-menus-setup.md`: Instant Action's Game Setup through its spec: two computer
  players set through the drop-downs, a side, Fixed Placement, Map Select (scrolled,
  Four Sisters chosen), the Advanced panel (Warp off, a slider moved), LAUNCH.
- `ui-menus-multiplayer.md` (two games, `--dplay`): the connection screen, the host's
  name typed into our field, Manual IP, the lobby, Create Game and the game's name; the
  joiner's address typed, the host's game found as a lobby row and joined; a chat line
  each way from our field; the joiner's side chosen; LAUNCH three times; the match.

## Not done

- **Specs**: every screen has one except the `Shell::Screen::Show`
  screens (voice options, command-line host and join) and the end-of-mission flow's
  generic messages; those draw with the generic screen. The main menu's and Single
  Player's labels are English (the stock art carries the words, so the spec names its
  buttons itself).
- **Not reached on the bench**: the Tutorials, Klingon and Borg campaigns' lists;
  Multiplayer and its lobbies; help, licence and net waiting; room select, GameSpy and
  the command-line screens. Credits is a screen of the engine's own, not a dialog: it
  is ours with `[Menus] Credits=1` ("The Credits", below).
- **The multiplayer screens** were verified with two games (2026-10-16,
  `ui-menus-multiplayer.md`, and by hand with `--online`: *Internet - Online*'s prompt,
  the join code in Game Setup's chat): the connection screen, Manual IP, the lobby
  (Help, the game row, Join and Create), the game's name, Game Setup with two humans
  (chat both ways, a side chosen, the three LAUNCHes). Not reached: the password dialog
  of a passworded game, map transfer, the room select and GameSpy screens; the lobby's
  filter and its arrows with more games than rows. Game Setup shows "Download Maps: Ask"
  and "Download this Map" both, where stock shows one (both are in the model as shown).
- **The generic `widgets` screen is verified on the Multiplayer connection screen only**
  (the edit and the buttons, `ui-menus-widgets.md`); its list and drop-down branches
  have no screen on the bench.
- **Map Select's list is the stock window of fourteen rows**, scrolled by the stock
  arrows: the whole list of maps is not read (it is `KnownMaps`'), and the stock scroll
  bar's thumb is not drawn. Advanced Options' sliders are the stock art, cropped.
- **Two helper modules**: `screens/kit.rs` (single player) and `screens/setup_kit.rs`
  (setup, multiplayer) were written in parallel and overlap with `common.rs`; to be
  folded into one.
- **Keyboard navigation** of a spec (arrows, Enter) and of the generic screen beyond
  what the stock dialog does with Enter, Esc and Tab.
- **The cursor** is the system arrow over the 3D window, as with the GDI menus; a cursor
  of the UI's own is not drawn.
- **A display mode other than the desktop's** for the game: the shell is raised to the
  desktop size, so a game at another resolution re-creates the device at every switch
  between the shell and a mission (as it did with Menus.asi); not measured.

## The Credits

`[Menus] Credits=1` (the default): `DispatchDialog`'s call of `Show_Credits` (the
shell mode 30 case, from the main menu and after the last campaign mission) runs
`credits::show` instead, a screen of ours through `engine::shell::run_screen_under`,
which draws render3d's map (`scene::draw_view`) after the clear and before the UI's
frame, as the dialog host draws the menu scene. Leaving it does what the demo screen in
the Credits' place does: the black shell background and `g_shellMode` = main menu.

- **The words are the game's**: `Credits.txt` in the game directory, whose pages end at
  `@cls`, each held `@hold=` seconds, the first in `@font=2` (large). A line in capitals
  is set as a heading in the accent colour, the rest as names; each page fades in and
  out, rising a little. A page taller than the screen is set smaller.
- **The logos** are where the stock `Credits3D` puts them, in the four corners for the
  whole run: Mad Doc top left, Activision top right, Bink bottom left, GameSpy bottom
  right, the four 128 x 128 cells of `Textures/RGB/logos.tga` that
  `Sprites/gui_credits.spr` names, drawn square (the stock screen draws them at the
  canvas's scale, so wide on a wide screen). Their sprites are additive, so black is
  keyed out: a texel's brightness is its opacity, its colour divided by it.
- **The scene** tours the map's views, 16 s each: `main`, `federation`, `klingon` (its
  ships decloak, with their sound), `tutorials`, `borg`. Without Render3D.asi the
  theme's backdrop.
- **Keys**: Esc or a right click returns to the main menu (the stock Credits took no
  key); Space, Enter, the right arrow or a left click turn the page, the left arrow goes
  back; the last page's end returns too. The foot of the screen says so.

Seen on the bench (2026-10-17, 1280x720): the title page over the fleet with the four
logos, page 5 of 30, Esc back to the main menu (`credits: left (0)`).
