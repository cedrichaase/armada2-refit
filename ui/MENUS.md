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

**Menus.asi is superseded, not run alongside.** If `Menus.asi` (or `MenuScale.asi`) is
loaded, the menu part stands down at startup and the shell runs as Menus.asi makes it.
It also stands down if any of the user32 imports it hooks is not user32's own (another
plugin hooked it first). `ui/bench-asi.sh --menus` takes Menus.asi out of a bench clone.

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
  `WS_CHILD`, and it is made with `CreateDialogIndirectParamA` (menus/README.md, "One
  window", is the same rewrite; this one also drops `WS_VISIBLE`). Our procedure sits in
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
  nothing (the shell never pairs them anyway: menus/README.md, "GetDC and ReleaseDC do
  not pair up"). Everything a screen draws, its background, the `ShellButton`s, the
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
  our surface's bitmap selected in a DC of its own, and the menus/README.md "stock bug"
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
   hands it (menus/README.md, "A stock bug the memory DC exposes").
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
there is no 3D window or no Direct3D 9 device behind the game's (DXVK's own d3d8); its
owner is a window the UI does not host; its template cannot be read; or the hidden
dialog could not be made. With the shell's mode raised (below) a stock dialog then draws
800x600 in the corner, as stock does without Menus.asi, but it works.

### The display

`do_smain` runs the shell on Storm3D's shell display (`SetActiveDisplay(3)`), and
`DispatchDialog` (0x6066c0) puts that display up again and calls `FlipToGDISurface`
before every screen. Its mode is a hard-coded 800x600x16 (`SetActiveDisplay_Internal`'s
two `push 16; push 600; push 800`, at 0x62c4d4 and 0x62c633). The host raises both to
the desktop's size at 32 bits, at DllMain, as Menus.asi's `RaiseShellMode` does
(menus/README.md, item 1). The hosted dialogs then draw on the shell display's own
device, at the screen's resolution, without switching displays: when the game runs at
the desktop's resolution the shell and the 3D display have the same mode and the
device is never re-created (`README.md`, "What the shell is, as far as Direct3D goes").
The in-mission menus run on the 3D display that is already up.

### The in-mission menus

`do_escapeMenu` (the in-mission menu, template 0x123 like the main menu), Save Game,
Load Game, the Technology Tree and the Admiral's Log are opened from a mission. The
mission's loop is blocked inside the dialog, so the host's loop is the only thing that
presents. When the first dialog opens within two seconds of a mission frame
(`ingame::recent`), the host reads the front buffer back
(`IDirect3DDevice9::GetFrontBufferData`) and draws the menus over that picture, dimmed.
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
focus (menus/README.md, "Keyboard focus", saw the same after an embedded menu), and
with no focus the active window gets every key as `WM_SYSKEYDOWN`: the in-mission
menu never saw its Esc. The first hosted dialog puts the focus on the 3D window when
it finds none, and an Esc without Alt counts as Esc whichever way it comes.

## The shell's widgets

Most screens are not made of windows. The main menu, Options, Game Setup and nearly
every other screen draw `ShellButton`s into the dialog (menus/README.md, "The Admiral's
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
one click at once. The stock click sound plays once per change, not per pixel.

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

**The design area** is 800x600 fitted into the back buffer as Menus.asi fits it: the
full height, centred, at 4:3. `UI.log` logs it as Menus.asi logs its own
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
   bool`, and add it to `SPECS` in `screens/mod.rs`. In it:
   - find each control you lay out, by label (`s.model.by_label`), by id
     (`s.model.by_id`, window controls) or by a design point inside it
     (`s.model.button_at`, for picture buttons without labels); **return false** when
     the controls you expect are not there, and the generic screen draws the dialog;
   - draw with `crate::widgets` (and `generic::shell_button_look` for a button in the
     shell's place), placed by `Anchor`/`Stack` in units, not in design pixels;
   - act through the stock controls only: `s.click_control(c)` for a shell button or a
     window button, `actions::push(Action::Select { .. })` for a list or combo,
     `Action::Key` for typing into an edit. Never write a setting yourself: the stock
     procedure's handler does, and so the screen keeps doing what it did;
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

- **Options**: the stock buttons in the order they stand on the stock screen (y, then
  x), in one column; the one at the foot (Previous Menu, Return to Game: a button in the
  bottom-left corner, `common::is_footer`) is the window's footer. In a mission there
  are nine entries; from the main menu three. The window's title is the stock label at
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
log's last lines, the line as our field) and the way out. A click on a row is the stock
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
| `do_TechTreeWindow` | 0x131 | technology tree | in a mission | the in-mission menu | an `Edit` in Courier (menus/README.md, "Edit boxes") | none: drawn from the edit's text (spec: `tech_tree.rs`) |
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
  as our table, its rows level with the players'. **The other seven pages** are GDI graphs
  and lists inside the pane that the model cannot read, so the pane is cropped from the
  mirror into the page and `kit::forward_part` gives the stock pane the pointer, in the
  pane's own place, for its scroll bar. The owner-drawn tabs, players and buttons are
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
a Mutara nebula (`render3d/README.md`, "The menu scene"). Code: `src/menus/scene.rs`.

- **Where it is drawn.** At the start of each frame of the dialog host's loop, after
  the clear and before `surface::frame(Context::Menus)`, so it is under everything of
  ours. The view is that of the full-screen dialog at the bottom of what shows: `main`
  for "main menu"; for "single player" the campaign last pointed at (`scene::choose`:
  `tutorials`, `federation`, `klingon`, `borg`), `federation` on each visit until one
  is (the mission list over the campaign selection keeps its view); never over a
  mission (`backdrop::over_mission`). render3d moves the camera from one view to the
  next across the map.
- **How it is asked for.** `Render3D_MenuScene(name, x, y, w, h)`, render3d's one
  export, looked up once with `GetModuleHandleA("Render3D.asi")` and
  `GetProcAddress`. It draws into the whole back buffer and returns 1, or 0 when it
  cannot (no Render3D.asi, render3d stood down, no such view, no Direct3D 9 device);
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
  stock screen still hears the hover (its sound). The emblems are the HUD's race icons,
  white silhouettes with their alpha (`federation_icon`, `klingon_icon`, `borg_icon` in
  `Sprites/gui_global.spr`: 84 px cells of `Textures/RGB/allrace.tga`, read from the
  game's own files, so a texture pack's version shows), tinted as the text. The
  tutorials have no race: their emblem is Starfleet Academy's patch, the triangle the
  stock tutorial card shows, drawn as an outline round the Federation icon's delta.
  Without `allrace.tga` the names stand alone (UI.log: `menus: campaign emblems`).
  Without the scene the four cards draw in a 2x2 grid as before.
- **For the bench**, UI.log says `menus: scene <name> on` and `menus: scene off` at
  each change of screen. `testbench/ui.json` gives the main menu's and the campaign
  selection's targets a `label`, which `Click the <target>` clicks (by OCR) while the
  scene is on, since their stock points are on nothing then.

## Testing

`ui/bench-asi.sh --menus` (a scenario's `Setup:` line) puts `UI.asi` in a bench clone
with `[Menus] Enable=1` and takes Menus.asi out. `testbench/operating/menus-host.md`
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
  the command-line screens. Credits is a screen of the engine's own, not a dialog, and
  runs as before.
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
  between the shell and a mission (as with Menus.asi); not measured.
