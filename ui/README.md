# UI — the runtime for a HUD, menus and loading screen of our own

`UI.asi` is the runtime that the game's HUD, front-end menus and loading screen will
be redrawn with, in our own code (`PLAN.md`). This first step is the runtime alone:
a 2D renderer on the game's Direct3D 9 device, text, widgets, input with capture, and
a hook into each of the three places the UI draws. **With the shipped `UI.ini` it
draws nothing**; `[Demo]` turns on a demonstration panel per context.

It is a `no_std` Rust DLL like render3d (`render3d/README.md`, "Building and
installing"), and is **not in `./install` or `a2mod` yet**: nothing uses it until a
panel or screen of ours replaces a stock one. The bench gets it with
`ui/bench-asi.sh` (below).

**The menus** (`[Menus] Enable=1`, off by default): every stock shell dialog runs
hidden and is drawn and driven by the UI, generically or by a screen spec of ours.
[`MENUS.md`](MENUS.md) has how: the dialog host, the shell's own widgets, the control
model, the generic screen, writing a spec, and the table of every screen.

## Where the UI draws

`surface::frame(context)` is a frame of UI: it reads input, runs the UI for that
context (`app.rs`) and draws it into the bound back buffer. Three hooks call it, one
per place the game shows a picture, each in `src/engine/` with its addresses, each
checked against stock before it patches, each standing down on its own (`UI.log` says
which): a mission's frame, a screen of ours in the front end, and the loading screen.
Input is read and captured at the game window's messages, in every context.

## The mission frame

`engine/ingame.rs`. A mission's frame is drawn by `Armada_RenderFrame` in one scene:
the 3D view (`Armada_RenderAllOurStuff`, render3d's seam), the HUD
(`DisplayInterface::RenderAll`), the user's team data, the frame rate, the text-entry
line (`TextInput_Update`) and the chat lines (`ChatDisplay::Render`). Then it asks the
game type to render (`GameType::Render`, which in every game type plays computer voice
lines and draws nothing), ends the scene, and presents (`RefreshDisplay`, through the
device's vtable). The UI hooks **the call of `ChatDisplay::Render` (0x598a16)**: the
hook calls the stock function and then `surface::frame(Context::Mission)`. That makes
the UI the last thing drawn in the scene, over the HUD and the chat, with the scene
still open and the back buffer bound (render3d has resolved its own target into it by
then).

Why that site:

- **One call per presented frame.** `Armada_RenderFrame` presents exactly once, and is
  called only from the mission loop (`Program::DisplayInputProcess`). The HUD's action
  camera draws the 3D scene a second time per frame, but inside `RenderAll`, before
  the hook; the UI sees one frame. On the bench the hook ran at the game's frame rate,
  60 a second (UI.log: "mission: 301 UI frames in 5.0 s (60.0/s)").
- **Only in a mission.** The chat display is made by `Program::GameOpen` for every
  mission and freed when it ends, and the call is skipped while there is none. The front
  end and the loading screen draw through other paths and never reach it.
- **No other plugin is there.** render3d patches the two `Armada_RenderAllOurStuff`
  calls and the steps inside that body; HUD.asi wraps `RefreshDisplay`'s cursor draw
  (0x6246fa) and `SimulateAll` in `DisplayInputProcess`; GridLayout.asi and QOL.asi
  patch inside the HUD's panels. The hook checks the call still goes to
  `ChatDisplay::Render` and stands down otherwise.
- The other candidates were worse: the `EndScene` and `RefreshDisplay` calls are
  indirect (through the device's vtable), so a hook there needs the vtable and fires in
  every context; `TextInput_Update` comes before the chat, which would then draw over
  the UI.

**The engine batches its 2D quads.** HUD sprites and text go into the device's trifan
buffer (`ST3D_DeviceDirectX8+0xe4`), which `EndScene` flushes. Left alone, whatever was
still batched would land on top of the UI, so the hook flushes it first
(`ST3D_TrifanBuffer::Flush`, 0x621a90), as `EndScene` would; the flush at `EndScene`
then finds it empty.

What the hook sees in the other states of a mission:

- **Paused**: the loop still renders, so the UI keeps drawing and taking input.
- **The in-mission menu** (Esc) is a modal dialog with a loop of its own: no mission
  frame runs, so no UI frame either. Capture lapses on its own (below).
- **Cinematics**: `Armada_RenderFrame` keeps running, so the hook does too. When the
  engine hides its HUD (`DisplayInterface::SetInterfaceState`, for a cinematic or the
  player's hide-HUD key), it sets the ship panel's hidden flag (`shipDisplay+0x16`);
  `ingame::hud_hidden()` reads it, for the UI to hide with the HUD.
- **The end of a mission**: frames stop when the mission loop does; the next mission's
  first frame starts again (UI.log notes each start after a gap of more than a second).
  Leaving a mission for the main menu and opening a screen of ours there logged no
  further mission frames.

## Input and capture

`input.rs`. **In a mission the engine reads the keyboard and mouse from its window's
messages**, not DirectInput and not `GetAsyncKeyState`. Its `WindowProc` sends
keyboard messages to `ProcessKeyboardMessages`, which stores each key down for the
game (`StoreGameKey`) and updates the key state (`UpdateKeyboardState`), and mouse
messages to `ProcessMouseMessages`, which forwards each to the input thread
(`CreateInputThread`, made when a mission starts) with `PostThreadMessageA`; that
thread's `MouseDriver::ReadChannels` turns them into button bits and the wheel, and
reads the position itself with `GetCursorPos`. Both functions then **drain** the
queue's other messages of their kind with `PeekMessageA` before returning, so a
hook on the window procedure (a subclass) would see only the first message of each
burst: a click arriving with a move would slip past it.

So the UI sits at four points of that path, all on the game's thread, and each
message passes exactly one of them:

| Site | What it was | Now |
|---|---|---|
| 0x488869, in `WindowProc` | `call ProcessKeyboardMessages` | records the message; drops it if captured, else calls the stock function |
| 0x4dd709, in `ProcessKeyboardMessages` | `call [PeekMessageA]`, the drain | `call` to a `PeekMessageA` that records each message and, for one capture keeps, translates it (so its character still comes) and peeks the next in its place |
| 0x488952, in `WindowProc` | `call ProcessMouseMessages` | as for the keyboard |
| 0x4de260, in `ProcessMouseMessages` | `mov ebx, [PeekMessageA]`, the drain (`call ebx`) | `mov ebx, <ours>`: records each message and peeks past the ones capture keeps |

Hooking where the messages enter, not where the mouse path posts them to the input
thread, also covers the front end: there is no input thread there, but a screen of ours
(`engine::shell`) dispatches the 3D window's messages to the same `WindowProc`.
A first version hooked the `PostThreadMessageA` of the mouse path; it worked in a
mission and saw no clicks at all in the front end.

Each site's bytes are checked against stock first; any mismatch and input stands down
whole (the UI still reads the pointer, but cannot keep input from the engine and sees
no keys).

**What capture keeps from the engine.** `set_capture(mouse, keyboard)`, set each frame
by the widgets after layout, applies to the messages that arrive after it.

- Mouse: button downs (and double-clicks) and the wheel. Moves always pass, so the
  engine keeps its pointer, hover and edge scrolling. A button's up is dropped exactly
  when its down was: a press the engine saw always gets its release, and one it did not
  see never gets a stray release. The engine's own `SetCapture` follows its button
  bits, so a press it did not see does not capture the mouse for it.
- Keyboard: key downs and characters. The system keys (anything with Alt, F10) always
  pass, so Alt+F4 and Alt+Tab work whatever has focus. Ups follow downs as for the
  mouse: `UpdateKeyboardState` latches a "released" bit on an up, so a stray up is not
  harmless.
- **Capture lapses after 250 ms without a UI frame**: the in-mission menu's modal loop,
  a mission's end and the loading screen stop UI frames, and a click there must not be
  eaten by capture left over from the last frame.

**What the UI reads** (`input::poll`, once per frame): the pointer from `GetCursorPos`
(as the engine reads it), in the client area of the engine's window
(`ST3D_GraphicsEngine::GetWindowHandle`) and scaled to the back buffer, which need not
be the client size; buttons with pressed/released/double-click edges from the
messages, so a click shorter than a frame is not lost; the wheel in notches; the keys
that went down (virtual-key codes, auto-repeat included) and the characters typed
(`WM_CHAR`, the ANSI code page read as Latin-1), for text fields; and Shift, Ctrl and
Alt. A button released outside the window after a press the engine did not see sends no
up (nothing captured the mouse), so `poll` also asks `GetAsyncKeyState` and releases a
button the hardware says is up.

## The cursor

`engine/cursor.rs`. hud/README.md has how the engine draws its cursor; what the UI
uses of it:

- **Which cursor the engine wants.** Each frame the HUD components pick a cursor
  sprite (`cOverViewImp::SetCursorAiAction` picks the move, attack, ... cursor for the
  order the pointer would give, from a table indexed by the `AiAction`), and
  `CursorInterface::Render`, inside `DisplayInterface::RenderAll`, hands it to the
  device's `SetCursor` (none while the cursor is suppressed: the engine does that
  during a right-drag), which keeps it at `ST3D_DeviceDirectX8+0x60`. A cursor sprite
  is a Storm3D database element, so its name, the one `cursor.spr` gives it, is at
  `+0x8`. `cursor::shape()` reads it; by the mission hook this frame's choice has
  reached the device.
- **Hiding it.** Under DXVK the cursor is the synchronous one (`+0xe0` set):
  `RefreshDisplay` draws the sprite at `+0x60` in a scene of its own just before it
  presents, and draws none when `+0x60` is empty. `cursor::hide_this_frame()` makes the
  next `RefreshDisplay` run with `+0x60` emptied and put back after, through a wrap of
  its vtable slot (`ST3D_DeviceDirectX8`'s vtable, slot +0x68, checked to be stock), so
  nothing the engine chose changes and nothing is rebuilt. It lapses after one present,
  so the UI calls it every frame it draws a cursor of its own. Calling the device's
  `SetCursor(NULL)` instead would also hide it, but `SetCursor` frees and rebuilds the
  cursor's texture whenever the sprite changes, every frame. With a hardware cursor
  (`+0xe0` clear, not seen under DXVK) the wrap has no effect; `cursor::synchronous()`
  says which.
- The UI's own cursor is drawn in the mission frame, a frame earlier than the engine's
  (which `RefreshDisplay` draws at the position it reads just before presenting), at
  the pointer position `input::poll` read.

## The front end: screens of our own

### What the shell is, as far as Direct3D goes

The shell's menus are GDI dialogs (`menus/README.md`), but the engine's Direct3D device
lives through all of them. `do_smain` runs the shell. It puts Storm3D on its *shell
display* (`ST3D_GraphicsEngine::SetActiveDisplay(3)`), calls the device's
`FlipToGDISurface` so GDI can draw on the window, and then loops on
`DispatchDialog(g_shellMode)` (`0x6066c0`) until `g_shellMode` (`0x7a8980`) is 0. Each
`do_*` screen it dispatches names the next one in `g_shellMode` before it returns
(1 is the main menu).

`SetActiveDisplay` takes a display type: 2 is the 3D display at the game's own
resolution, 3 the shell's. The shell asks for 800x600x16, which Menus.asi raises to the
desktop size. When the mode the new display wants differs from the current one,
`ST3D_Device::ChangeDisplayMode` destroys the Direct3D device and creates a new one.
When the two modes are the same, it does nothing at all. **So the `IDirect3DDevice8` can
change whenever the shell and the 3D display switch.** In stock (an 800x600 shell
under a bigger game resolution) it changes on every switch. With Menus.asi and the game
at desktop resolution it does not. Anything that keeps resources must check
`engine::d3d8_device()` every frame and drop what it made on an old device.

While a GDI menu is up, the game thread is inside that dialog's modal loop. Nothing
presents through the device, and the menus' Bink animations are drawn by the shell
through GDI. Only two things in the shell use the device: `PlayIntroMovie` (the
cutscene proxy) and the Credits.

### The Credits are the model

`Show_Credits` (`0x5913a0`, the `g_shellMode` 30 case of `DispatchDialog`) is the one
shell screen that is not a dialog. It is a 3D scene with a message loop of its own. It
switches to the 3D display, then each frame drains the queue and draws through the
device. At the end it sets a black shell background (`SetCurrentBackground` with
`ALL_BLACK_8_BY_6_BACKGROUND`), clears every display to black
(`ClearDeviceDisplays`), puts the shell display back, calls `FlipToGDISurface`, drains
the queue through `HandleMessageQueue`, and sets `g_shellMode` to 1. `run_screen` does
exactly that around a closure, so a screen of ours leaves the shell as the engine's own
3D screen does. The screen is not a dialog, so Menus.asi's `DialogBoxParamA` hook,
subclasses and modality filter never see it.

Each frame, `run_screen`:

- drains the queue the way `HandleMessageQueue` does, Alt+Tab/Alt+Esc rule included,
  and notes fresh key presses and button downs for the screen. A `WM_QUIT` stores its
  code in `ExitCode` (`0x737bb0`) as the engine's pump does, is posted again, and ends
  the screen with `QUIT`;
- reads the device again (see above), calls the engine's own
  `ST3D_DeviceDirectX8::TestCooperativeLevel` (`0x623c70`, which resets a device that
  can be reset, as it does at every scene in a mission), and skips the frame if the
  device is still lost. The closure is still asked, so the screen can be left;
- otherwise calls `BeginScene`, sets a full back-buffer viewport, clears to black, calls
  `surface::frame(Context::Shell)`, asks the closure, then `EndScene` and `Present`.
  These are plain `IDirect3DDevice8` calls (`engine/dev8.rs`, the same slots the
  engine's `RefreshDisplay` uses);
- holds itself to about 60 frames a second.

The 3D window's `WindowProc` answers `WM_SETCURSOR` with no cursor, so that the sprite
cursor shows in a mission (and the Credits show none). While a screen of ours runs,
the window is subclassed to answer it with the arrow, as the GDI menus have. The
subclass is installed once and only passes messages on while no screen runs, so it never
unhooks a subclass someone else added on top of it. A screen that starts with no
keyboard focus (Wine leaves it `NULL` after an embedded menu, `menus/README.md`) puts
it on the 3D window.

The hook is one call: `DispatchDialog`'s `call Show_Credits` at `0x606bd4`, checked to
call stock. It is re-pointed only with `[Demo] Shell=1`. Without that, the shell is
stock and the sites are only checked. Every engine function it calls is checked by its
first bytes, and on any mismatch the shell part stands down.

### The demonstration

`[Demo] Shell=1` puts the demo screen where the Credits were: Main menu, Options panel,
Credits. Esc, Enter or a right click leaves it. A left click is the demo's widgets', so
it does not. It returns to the main menu over black, as the Credits do.

### Facts measured on the bench

On the bench (2026-10-16, 1280x720, `./install` with Menus.asi `Embed=1`, UI.log):

- At the main menu, the shell display is up (Storm3D's display type 3) and the device
  exists: one `ST3D_DeviceDirectX8` on the 3D window. Its back buffer is 1280x720
  X8R8G8B8, it is **not windowed** (fullscreen, flip swap effect, default present
  interval), and the window's client area is 1280x720. Menus.asi raises the shell mode
  to the desktop size, and the game's resolution is the same, so switching to the 3D
  display changed nothing: same device, same back buffer, same mode.
  `engine::d3d8_device()` is valid in the shell.
- The demo screen replaced the Credits, presented 970 frames with none lost, and Esc
  returned to a working main menu on the same device.

## The loading screen

A mission's load (`Program::GameDoOne`) puts the splash up with
`RenderLoop_Show_Splash_Screen`, which loads the background (`mbg02`) and `logo.sod`
instances and a light. It sets `Armada_AssetLoadStatusVisible` (`0x773be4`). It then
runs `Program::GameOpen`, whose load is bracketed by `LoadBarStart` and `LoadBarEnd`,
and finally takes the splash down with `RenderLoop_Kill_Splash_Screen`, which clears the
flag. The loading screen is never drawn on a clock. Each loading step calls
`LoadBarNext`, which lowers `s_fractionRemaining` (`0x7a8828`, 1 at `LoadBarStart`) by
that step's weight and calls `RenderLoopAssetLoadStatusUpdate` (`0x598ba0`).

`RenderLoopAssetLoadStatusUpdate` first drains the message queue. It draws a frame
**only when the fraction has risen by another 5%** since the last one. Its threshold
starts just below 0 when the splash goes up, so the first step draws. A load therefore
presents about twenty frames, each at a step boundary, with nothing in between, and a
long step leaves the screen still. A frame selects the 3D display and device 0, clears
depth, opens the scene, and draws the skybox and the logo model through the camera
(`models/README.md`). It then draws the progress bar as a rectangle whose width is the
fraction, calls the transport's `LoadPercentageUpdate` (multiplayer), and `Flush`es the
sorted geometry. Then come `DeleteRegisteredLights`, `EndScene` and `RefreshDisplay`,
the present.

**The hook** replaces that `DeleteRegisteredLights` call (`0x598f53`, a `call rel32`,
thiscall on Storm3D) with one that makes the engine's call and then
`surface::frame(Context::Loading)`. By then the engine has drawn all of its frame. The
scene is still open, and render target 0 is the back buffer (render3d's seam is in
`Armada_RenderAllOurStuff`, which the loading screen does not use). The UI therefore
draws over the logo, the bar and the skybox, on every frame the loading screen presents,
and on no other. Before patching, the site is checked to call stock and the `EndScene`
after it (`call [eax+0x34]`) is checked to be where it should be. A mismatch makes the
loading part stand down. The widened `logo.sod` changes only geometry and art, not this
code, and HUD.asi does not touch the function.

**Progress** (`loading::progress()`) is `1 - s_fractionRemaining`, the value
`LoadBarComputeFraction` (`0x601ce0`, checked by its bytes) returns and the bar is
drawn from. It is reported only while `Armada_AssetLoadStatusVisible` is set, and is
None otherwise. The UI sees the value the frame being drawn was made for.

**Not done: smoother loading frames.** Because the engine presents only at 5% steps,
anything animated in the loading UI moves in jumps. A later hook on `LoadBarNext`'s call
of the status update (`0x6019eb`) could present frames of its own between the engine's,
but it would have to redraw the logo scene, or keep a copy of it, to have something to
draw over.

### Facts measured on the bench

On the bench (2026-10-16, Instant Action at 1280x720): the hook drew 20 loading frames
in one load, the first at 3% and the last at 94%. The demo panel and its progress bar
drew over the logo scene (it read 99% on the last frame shown), and the mission started
normally afterwards.

## Building and installing

    ui/build.sh                                   # -> ui/build/UI.asi
    ui/bench-asi.sh <state.json> --demo mission,shell,loading [UI.Scale=1.5]

`build.sh` compiles `shaders/*.hlsl` with render3d's `hlsl-rs.sh` (the generated
`src/shaders/*.rs` is committed and stamped, so a machine without `vkd3d-compiler`
builds) and the font atlas from the vendored TTFs (`src/text/`), then the DLL.
`bench-asi.sh` copies `UI.asi` and `UI.ini` into a bench session's clone and sets the
keys it is given; it is what a scenario's `Setup:` line runs.

## `UI.ini`

| Key | Default | |
|---|---|---|
| `[UI] Enable` | 1 | 0: the plugin patches nothing |
| `[UI] Log` | 1 | `UI.log` in the game directory: which hooks are in, and why one stood down |
| `[UI] Scale` | 1.0 | the UI's size, as a multiple of a 1080-pixel-tall screen |
| `[Demo] Mission`, `Shell`, `Loading` | 0 | the demonstration panel in that context |
| `[Menus] Enable` | 0 | the UI hosts the shell's dialogs (`MENUS.md`); stands down while Menus.asi is loaded |
| `[Menus] Generic` | mirror | a screen without a spec: `mirror` (as the game paints it) or `widgets` (its controls as ours) |
| `[Menus] Specs` | 1 | 0: no screen spec, every screen generic |
| `[Menus] EscapeReturns` | 1 | Esc in the in-mission menu is Return to Game |
| `[Menus] Trace` | 0 | UI.log: each hosted dialog's messages of note, its actions, and its surfaces as `UI-dlg<depth>-*.bmp` every 5 s |
| `[Menus] Host` | 1 | 0 (with `Trace=1`): the dialogs run stock and are only traced, for comparing |

## The renderer (`src/gfx/`)

Everything a frame of UI draws goes into one vertex list and is drawn with **one
shader pair** (`shaders/ui.hlsl`), so a frame costs a handful of draw calls whatever is
on screen. Each vertex carries its *kind*: solid colour, textured, signed-distance
text, or an analytic rounded rectangle (its distance to the edge computed per pixel,
antialiased over one pixel, so corners are smooth at any radius without a texture).
The pixel shader weights the kinds with `step()` rather than branching on them, as
vkd3d needs (`render3d/README.md`). A new draw call starts only when the texture or the
clip rectangle changes; solid quads take whatever texture the run has, since they do
not sample it.

**State.** d3d8to9 keeps its own record of what the game bound (`platform/D3D9.md`), so
the frame is drawn between a `D3DSBT_ALL` state block's capture and its apply: once a
frame, not per draw, which is where render3d found `SBT_ALL` expensive. Positions are
in back-buffer pixels and move half a pixel on the way to clip space, so a rectangle
with whole-number edges covers whole pixels exactly.

**No `Reset` to handle.** Textures are in the managed pool and the vertices go through
`DrawIndexedPrimitiveUP`, so nothing of ours is in the default pool. Every texture is
registered once with its pixels (the font atlases from the DLL, images from the game's
TGAs through `a2fmt`) and made on each device the first time it draws there.

**Without a Direct3D 9 device** (DXVK's own d3d8, GOG's d3d8to9) `surface::frame` returns
at once and the game draws as stock.

## Text (`src/text/`)

**The font is Noto Sans Condensed** (OFL-1.1), Medium as the regular weight and
SemiBold as the bold, vendored unmodified in `platform/vendor/noto-sans-condensed-2.015/`.
A condensed face because the game's own UI font is one, and the HUD's panels are laid
out for its widths; Noto because it covers every Latin-1 letter the game's localised
strings use, with real kerning.

**Baked at build time, never committed.** `fontbake` (a host crate `build.rs` runs)
reads each TTF with `skrifa` and writes a single-channel signed-distance atlas into
`OUT_DIR`: 1024x512, 48 texels to the em, distances out to 6 texels (128 on the outline,
higher inside), for ASCII, Latin-1 and the typographic punctuation (‘ ’ “ ” – — … • € ×).
Distances are exact: curves flattened into short segments, the nearest segment found
for every texel, inside by nonzero winding. Kerning is the font's GPOS `kern` feature.
The blob (format at the top of `src/text/mod.rs`) is read in place, so a `Font` is a
static and lookups are binary searches. The bake is brute force and runs with
`opt-level = 3` even in a debug build, or it would take minutes.

**One atlas for every size.** The pixel shader turns distance into coverage over one
screen pixel: the distance scale is `2 * spread * size / em_texels` screen pixels per unit
of the atlas, so 14 and 60 pixel text are equally sharp. Baselines snap to whole pixels.
`layout_wrapped` breaks at spaces and newlines; a character the font lacks draws as `?`.

`fontbake`'s preview test renders sample lines through the runtime's own layout code into
PGM files (outside the repository); the command is at the top of `fontbake/tests/preview.rs`.

## Widgets (`src/widgets/`)

Immediate mode: a widget is a call made every frame, which lays itself out, draws, and
answers for its input at once (`if ui.button(r, "Launch") { ... }`). What has to
outlive a frame is kept by widget id, a hash of the label and the scopes it is in:
which widget a press started on, so a click is a release over the same widget.

Sizes are **units**, pixels of a 1080-pixel-tall screen times `[UI] Scale`
(`Ui::u`), so a layout is the same at every resolution and aspect; positions come
from `Anchor` (a rectangle against a screen edge or corner) and `Stack` (rows down a
rectangle). Panels and every interactive widget **claim** the pointer: while it is
over one, `Ui::end` asks `input` to keep the mouse from the engine, from the next frame.

The widgets so far are what the first screens need: `panel`, `window` (a panel with a
title), `label`, `note`, `button`, `toggle`, `slider` and `progress`, over the drawing
calls (`rect`, gradients, `rect_round`, `frame`, `line`, `image`, `nine`, `text`).

## Testing

`./a2test run testbench/scenarios/ui-runtime.md --install .` (`--res 1920x1080`: the
Assembler's place is in screen pixels): the demo screen in the Credits' place and a
click on it, the loading-screen panel, the panel over the HUD, and a click on it that
the engine does not see (the selection stays).

`ui-menus-options.md` and `ui-menus-match.md` (`--menus`, 1280x720): the hosted menus,
`MENUS.md`, "Testing".

## Not done

- **A cursor of our own.** The engine's can be hidden a frame at a time and its shape
  read (above); nothing draws one yet.
- **Text fields.** Input has the keys and characters, and capture keeps them from the
  engine once a widget calls `claim_keyboard`; there is no text-field widget yet.
- **A device left behind.** Objects made on a device keep it alive (`platform/D3D9.md`,
  "Which device"), and the renderer lets go of them when it first draws on a new
  device. In stock, where the shell and the game's display re-create the device at
  every switch, the last mission's device therefore lives on behind the GDI menus until
  the UI next draws. With Menus.asi at the game's resolution the device is never
  re-created. Not seen to matter; not measured.
- **Smoother loading frames**: the engine presents only at 5% steps (above).
- Not verified: device loss in a screen of ours, the wheel, characters outside ASCII,
  a stock 800x600 shell, `Embed=0`.
