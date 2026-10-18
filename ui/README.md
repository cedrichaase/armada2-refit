# UI — the runtime for a HUD, menus and loading screen of our own

`UI.asi` redraws the game's HUD, front-end menus and loading screen in our own code
(`PLAN.md`): a 2D renderer on the game's Direct3D 9 device, text, widgets, input with
capture, and a hook into each of the three places the UI draws, with the HUD's panels
(`HUD.md`) and the menus (`MENUS.md`) taken over on top of them. **With the shipped
`UI.ini` every part is on**: the HUD's panels, the menus, the loading screen and the
cursor. Each part has its key, and a part at 0 is the engine's again ("A part
switched back to stock", below); `[Demo]` draws a demonstration per context.

It is a `no_std` Rust DLL like render3d (`render3d/README.md`, "Building and
installing"). `ui/install.sh` (run by `./install`) puts `UI.asi` and `UI.ini` in the
game directory and takes out the plugins the UI supersedes ("Installing", below);
`--remove` takes the UI out. `a2mod` switches it as the `ui` layer. The bench gets a
working copy's build with `ui/bench-asi.sh` (below).

**The menus** (`[Menus] Enable=1`): every stock shell dialog runs
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
- **Whether the engine would show it** (`cursor::install_probe`). Another plugin can
  hide the engine's cursor by wrapping `RefreshDisplay`'s one `DrawScaled2D` call
  (0x6246fa) and skipping it: Scene.asi's `Cursor=0` and `cursor off`
  (testbench/scene/README.md), which patches it once the scene is built and chains to
  whatever the call went to. The UI wraps the same call when it loads (the probe,
  chaining to stock or to HUD.asi's wrap), so Scene.asi's wrap comes before it. Each
  present with a cursor sprite notes whether the draw got as far as the probe:
  `engine_shows()` is false while something before it skips it, and then the UI draws
  no cursor either (`cursor: Mission draws none`). It follows a change a present late.
  While the UI draws its own, `hide_this_frame()` has the probe skip the engine's draw
  (`+0x60` is left alone when the probe is in), so the probe still sees the draw
  reached. The probe also makes the square-cursor correction ("The engine's cursor,
  square"). It reads the
  engine's state, not Scene.asi's: any plugin that skips that draw is obeyed.
- The UI's own cursor is drawn in the mission frame, a frame earlier than the engine's
  (which `RefreshDisplay` draws at the position it reads just before presenting), at
  the pointer position `input::poll` read.
- **Our own cursor** (`src/cursor.rs`, `[UI] Cursor=1`, the default). It is drawn last in every frame, in all three contexts: in a mission the
  sprite `shape()` names (and the engine's is hidden with `hide_this_frame()`; where the
  engine's cannot be hidden, `synchronous()` false, ours is not drawn, so there are never
  two), in a screen of ours the arrow `c_arrow`, and on the loading screen the arrow
  only while the engine would draw a cursor there itself. It has none: the bench saw
  no cursor sprite at the loading screen's presents (`cursor: Loading draws none`), so
  no cursor is drawn there, as in stock. Nothing is drawn while the engine shows none
  (a right-drag, or `engine_shows()` false) or the pointer is outside the window.
  - **The art is read at run time from the player's own files, never copied**:
    `Sprites/cursor.spr` is parsed (a line is a sprite name, a texture, a rectangle in a
    reference size, `@anim=cursorNx1` for N frames side by side, `@origin=(x,y)` for the
    hotspot in texels of the 32x32 sprite) and each texture is read from `Textures/RGB/`
    with `gfx`'s TGA decoder. Black is the colour key (`@skip=(0,0,0)`), made
    transparent when the texture is registered. If a texture has a `.a2neb-backup`
    beside it (hud/README.md: `cursor-aspect.py` squashed the art once), that one is read,
    so the stock art is what is drawn whatever state the files are in. Frames are
    stepped once a second through the strip; the engine's own timing (`tex_anim.spr`'s
    `offset N 1.0 step`) was not measured.
  - **Size and hotspot.** 32 texels draw at `[UI] CursorSize` units (default 32: the art
    at its own size on a 1080-line screen), snapped to a whole multiple of the art when
    within 20% of one, so the texels land on the pixel grid; the hotspot is the
    sprite's `@origin` scaled the same way, so the click point is exact. The engine's own
    cursor is stretched by its 800x600 reference (hud/README.md); ours is square at any
    aspect and at the back buffer's resolution.
  - **In a front-end screen of ours Windows' arrow is hidden** (`shell::hide_os_cursor`,
    the window's `WM_SETCURSOR` answer) for as long as the screen runs.
  - `UI.log` says each change of the shape drawn (`cursor: Mission draws c_attack`), which
    the scenario `ui-widgets-cursor` reads. Seen on the bench: `standard_cursor` with
    nothing selected, `c_move` over empty space with a ship selected, `c_attack` over an
    enemy.

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
at desktop resolution it does not, which `[UI] Native` arranges from the first launch
(`src/engine/display.rs`: a stock mode in the profile becomes the desktop's before
`InitializeGraphicsSystem` opens the device). Anything that keeps resources must check
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
Credits. Esc, Enter (unless a widget of ours holds the keyboard: a focused text field takes them) or a right click leaves it. With the widget gallery at the right (Widgets, below). A left click is the demo's widgets', so
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

**A loading screen of ours** (`[Loading] Enable=1`): [`LOADING.md`](LOADING.md). It
covers this frame whole, and presents frames of its own between the engine's from a hook
on `LoadBarNext`'s call of the status update (`0x6019eb`), so it animates; a hook on
`GameDoOne`'s call of the splash (`0x4871ad`) tells it a load has begun. Both are
patched only with `Enable=1`, and stand down on their own (the overlay above still
works) when their sites are not stock.

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

### Installing

`ui/install.sh` copies `UI.asi`, merges `UI.ini` into the player's (`ini-merge.py`:
their values stay, a key they lack is added) and then runs `supersede.sh`, which
takes out of the game directory the plugins whose places the parts switched on take:

| Part on in the installed `UI.ini` | Taken out | How |
|---|---|---|
| `[HUD] Enable=1` with any panel on | `HUD.asi`, and the file-based HUD fixes put back to stock | `hud/install.sh --remove`, `hud/revert-files.sh` (the UI reads the stock rectangles, font and cursor art) |
| `[HUD] Enable=1` and `Bar=1` | `GridLayout.asi` | `grid/install.sh --remove` |
| `[Menus] Enable=1` | `Menus.asi` (and `MenuScale.asi`), with their `.ini` and `.log` | the plates in `Menus/` stay: the UI's menus draw them (`menus/install-plates.sh`, still in `./install`) |

`./install` no longer runs `hud/install.sh`, `grid/install.sh` or `menus/install.sh`.
The three layers stay in the repository: they describe the stock HUD and shell the
UI works on, their findings are what several hooks here rest on, and a player who
switches a part back to stock can put the old plugin back by hand (`supersede.sh`
leaves a plugin alone while the part that supersedes it is off). Each pair also
stands down against the other at run time: the UI's bar while `GridLayout.asi` is
loaded, its menus while `Menus.asi` or `MenuScale.asi` is, and its square cursor
(below) while `HUD.asi` is. `ui/bench-asi.sh` runs `supersede.sh` on a bench clone too,
so a clone of an install from before still tests the UI alone.

**Upgrading an install.** Up to ui 0.3 every part shipped off (`0`), so a player's
`UI.ini` from then holds 0 for each. `[UI] Defaults` numbers the shipped defaults (2:
every part on); when the shipped number is higher than the player's (none counts as
1), `ini-merge.py` moves each key that the newer defaults switched on and that still
holds the old default to the new value, and leaves any other value as the player set
it. So `./install` over an older install gives the UI's parts, and a part a player
switched on or set otherwise stays as they set it.

`a2mod stock` puts `UI.asi` and `UI.ini` aside with the rest (the `ui` layer), and
the plates under `menus`: the stock game has neither `HUD.asi` nor `Menus.asi`, so
nothing else changes. An older install that still has the superseded plugins is
snapshot as it is.

### A part switched back to stock

Every part's key can be set to 0 in the player's `UI.ini`. What they then get, now
that the old plugins are not installed:

- **A `[HUD]` panel at 0** is the stock panel, on the stock 1600 x 1200 canvas that
  the engine scales to the screen per axis: on a wide screen it is stretched (the
  minimap, the info panel and the bar wider than tall, their text wide), as without
  `HUD.asi`. The panels of ours lay themselves out around what they read of it (the
  bar is placed against the stock minimap's rectangle when ours is off). For the
  stretch to be corrected, `hud/install.sh` puts `HUD.asi` back; with `[HUD] Enable=0`
  the whole HUD is stock and `HUD.asi` corrects it as before. Not tested on the bench
  with a mix of panels of ours and `HUD.asi`'s canvas.
- **`[HUD] Bar=0`** is the stock bar with stock's keys; `grid/install.sh` gives the
  grid back through `GridLayout.asi`, which places it against the stock info panel and
  minimap: beside ours (drawn elsewhere) it lands in the wrong place, so it belongs
  with `[HUD] Enable=0` and `HUD.asi` (as `grid-layout.md` sets it up).
- **`[HUD] UnitView=0`** leaves the unit view where the stock canvas puts it (the
  bottom right, 350 x 384 canvas units: wide on a wide screen), with its stock frame.
- **`[Menus] Enable=0`** gives the stock GDI menus: the shell's 800 x 600 display mode,
  every menu its own window under Wine. `menus/install.sh` gives the scaled, embedded
  shell back through `Menus.asi`.
- **`[Loading] Enable=0`** is the engine's logo scene and bar (the widened `logo.SOD`
  if `models/` installed it).
- **`[UI] Cursor=0`** is the engine's cursor, kept square by the UI (below).

### What the old plugins did, and where it is now

| Plugin, part | Now |
|---|---|
| `HUD.asi` canvas (the panels' rectangles at the screen's aspect) | every panel of ours is laid out by the UI; the unit view is placed by us (`HUD.md`, "The unit view"). Still on the stock canvas, stretched: the multiplayer comm panel, the dropped-player box and the load-percentage box (`HUD.md`, "Not done here") |
| `HUD.asi` font (the HUD font condensed to the aspect) | every text of ours is ours; the stock font is still drawn only by the stock panels above, wide on a wide screen |
| `HUD.asi` seams (sprite edges on pixel boundaries) | only the stock panels above still draw tiled sprites |
| `HUD.asi` re-layout after a mode change | nothing to do: our panels lay out every frame, the stock canvas is the same at every mode |
| `HUD.asi` cursor (square at any aspect) | ours with `Cursor=1`; with `Cursor=0` (or where ours cannot hide the engine's), `engine::cursor::install_square` makes HUD.asi's two changes (below) |
| `GridLayout.asi` (the bar as a grid of position keys) | `[HUD] Bar`, `BarKeys` (`HUD.md`, "The button bar"). `GridLayout.ini`'s `[Cells]` has no counterpart yet |
| `Menus.asi` display mode raised | the menu host raises it (`MENUS.md`, "The display") |
| `Menus.asi` scaling, mouse mapping | the menus are drawn by us, and the pointer sent to each dialog in its design place |
| `Menus.asi` `Embed=1` (one window) | every hosted dialog is a hidden child of the 3D window; the game is one window throughout |
| `Menus.asi` `EscapeReturns` | `[Menus] EscapeReturns` |
| `Menus.asi` `Underlay=0` (the 1:1 flash between menus) | while a dialog is hosted, the shell's paint onto the 3D window goes into a scratch DC |
| `Menus.asi` backdrops (the plates) | drawn by the UI's menus behind the main menu; installed by `menus/install-plates.sh` |
| `Menus.asi` `[Labels]` (labels kept off the art) | the spec screens draw labels of their own; a mirrored screen shows the stock overrun |

### The engine's cursor, square

`engine/cursor.rs`, `install_square`, made when the mission hook goes in unless
`HUD.asi` is loaded or `hud/cursor-aspect.py`'s squashed art is installed (its
`Curs_Move.tga.a2neb-backup`). The engine draws its cursor under the 2D scale it sets
to the back buffer over 800 x 600 on each axis (hud/README.md, "HUD.asi", the
cursors), so on a wide screen it is wide. The synchronous path's one draw (the
`call ST3D_Sprite::DrawScaled2D` at 0x6246fa in `RefreshDisplay`) goes through the
probe ("The cursor"), which, for that draw only, sets the x scale to the y scale and moves the
position the engine computed so the hotspot (`ST3D_Sprite+0x30`) stays on the
pointer, and puts the scale back; the hardware path's `SetCursor` multiplies the
cursor's width by the y scale (`fmuls 0x18(%esi)` at 0x625dd9 reads `+0x1c`). The
probe must have found the call going to stock `DrawScaled2D`, and `SetCursor`'s site
is checked to be stock. `UI.log`: "cursor: the engine's cursor is kept square".

## `UI.ini`

| Key | Default | |
|---|---|---|
| `[UI] Enable` | 1 | 0: the plugin patches nothing |
| `[UI] Log` | 1 | `UI.log` in the game directory: which hooks are in, and why one stood down |
| `[UI] Scale` | 1.0 | the UI's size, as a multiple of a 1080-pixel-tall screen |
| `[UI] Cursor` | 1 | draw the mouse cursor ourselves, from the game's own cursor art ("The cursor"); 0: the engine's, kept square |
| `[UI] Defaults` | 2 | which shipped defaults the file was brought up to ("Installing"); not read by `UI.asi` |
| `[UI] CursorSize` | 32 | its size in units (32 texels of art at 1080 lines) |
| `[UI] Theme` | auto | the look (`STYLE.md`): `auto` is the player's race in a mission and the shell's theme elsewhere; or `shell`, `federation`, `klingon`, `romulan`, `cardassian`, `borg`, `species8472` everywhere |
| `[Demo] Mission`, `Shell`, `Loading` | 0 | the demonstration panel in that context (`Shell`: with the widget gallery) |
| `[Demo] Style` | 0 | the style gallery (`STYLE.md`) in place of the demos: every theme and widget state in the front end (the Credits' place) and over a mission, the loading look on the loading screen |
| `[HUD] Enable`, `Bar`, `BarKeys`, `BarLabels` | 1, 1, 1, 1 | the HUD's panels of ours, one key per panel ([`HUD.md`](HUD.md)), every one on |
| `[HUD] UnitView`, `UnitViewSize` | 1, 260 | the unit view placed by us, a square of that many units at the bottom right, in our frame (`HUD.md`, "The unit view") |
| `[Menus] Enable` | 1 | the UI hosts the shell's dialogs (`MENUS.md`); stands down while Menus.asi is loaded |
| `[Menus] Generic` | mirror | a screen without a spec: `mirror` (as the game paints it) or `widgets` (its controls as ours) |
| `[Menus] Specs` | 1 | 0: no screen spec, every screen generic |
| `[Menus] EscapeReturns` | 1 | Esc in the in-mission menu is Return to Game |
| `[Menus] Scene` | 1 | the front end's screens over render3d's 3D scene (`MENUS.md`, "The menu scene"); without Render3D.asi, as before |
| `[Menus] Credits` | 1 | the Credits are ours, over the menu scene (`MENUS.md`, "The Credits"); 0: the game's |

What the player sets on a screen of ours is not in `UI.ini` but in `UIPlayer.ini` beside
it (`prefs.rs`), a file no install ships or writes, so a reinstall keeps it: `[HUD]
BarHidden` (Game Settings' "Command Bar", `HUD.md`, "The top-right buttons").
| `[Menus] Trace` | 0 | UI.log: each hosted dialog's messages of note, its actions, and its surfaces as `UI-dlg<depth>-*.bmp` every 5 s |
| `[Menus] Host` | 1 | 0 (with `Trace=1`): the dialogs run stock and are only traced, for comparing |
| `[Loading] Enable` | 1 | the loading screen of ours (`LOADING.md`) in place of the engine's logo scene and bar |
| `[Loading] Hold` | 0 | the test bench's: seconds each load waits half-way, the screen still drawn, to be looked at. Never in play |

## The renderer (`src/gfx/`)

Everything a frame of UI draws goes into one vertex list and is drawn with **one
shader pair** (`shaders/ui.hlsl`), so a frame costs a handful of draw calls whatever is
on screen. Each vertex carries its *kind*: solid colour, textured, signed-distance
text, an analytic rounded rectangle (its distance to the edge computed per pixel,
antialiased over one pixel, so corners are smooth at any radius without a texture), or
the style's *shape* (`src/gfx/fx.rs`, `STYLE.md`): each corner rounded or chamfered,
filled or stroked, soft-edged for glows and shadows, an LCARS elbow's cut-out, stripes.
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

**How every widget looks is `STYLE.md`**: the tokens of one theme per race and one
for the shell are `widgets/theme.rs` (`Theme`, chosen per context by `[UI] Theme`), and
each widget draws itself by calling a function there with its `State` (hover eased by
`widgets/anim.rs`), so a theme restyles them all.

The first widgets: `panel`, `window` (a panel with a title), `label`, `note`, `button`,
`toggle`, `slider` and `progress` (button, toggle and slider take the keyboard focus),
over the drawing calls (`rect`, gradients,
`rect_round`, `frame`, `line`, `image`, `nine`, `text`). The rest, one file each in
`src/widgets/`:

| Widget | Call | Notes |
|---|---|---|
| text field | `text_field(r, label, &mut String, max_len) -> FieldEvent` | caret, selection by Shift+arrows, mouse drag and double-click (a word), Ctrl+Left/Right by word, Home/End, Backspace/Delete, Ctrl+A/C/V/X through `clipboard.rs` (user32's CF_TEXT), a length limit, horizontal scroll. Enter is `Submit`, Esc `Cancel` (and drops the focus). Edited as `char`s, Latin-1 |
| scrollbar, scroll area | `scrollbar(..)`, `scroll_area(r, label, content_h, \|ui, content\| ..)` | drag the thumb, click the track to page, wheel over the view. The body gets the content rectangle already moved by the scroll; what is scrolled out neither draws nor takes the pointer |
| list | `list(r, label, count, row_h, &mut Option<usize>, row_fn) -> ListEvent`, `list_text(..)` | single selection, Up/Down/PageUp/PageDown/Home/End when focused, double-click or Enter is `Activated`; rows are drawn by a closure |
| dropdown | `dropdown(r, label, &[&str], &mut usize) -> bool` | the list opens below (above when there is no room), scrolls beyond eight rows, Up/Down/Enter/Esc |
| tabs | `tabs(r, label, &[&str], &mut usize) -> bool` | the caller draws the page; Left/Right when focused |
| check box, radio | `checkbox`, `radio`, `radio_group` | |
| image button | `image_button(r, label, tex, uv)` | any `gfx::Tex` (game art by `load_tga`, or `cursor::sprite`) |
| number spinner | `spinner(r, label, &mut f32, lo, hi, step, decimals)` | held buttons repeat, the wheel steps (Shift: x10), Up/Down/PageUp/PageDown/Home/End |
| tooltip | `tooltip(r, text)` after the widget | half a second of rest, follows the pointer, kept on the screen, wraps |
| modal | `modal(title, w, h, \|ui, inner\| ..) -> bool` | dims the screen and takes all input; true when Esc was pressed. **Call it last in the frame** (it draws where it is called) |

Behind them, three pieces in `widgets/state.rs` and `focus.rs`:

- **Per-widget state by id**: `Ui::slot(id)` is four `f32`s kept between frames (a caret,
  a scroll offset, whether a dropdown is open).
- **The overlay layer**: `Ui::overlay(closure)` is drawn last, over everything (a
  dropdown's list, a tooltip). Input cannot be deferred, so the widget that opens
  something answers its input in its own call and calls `Ui::occlude(rect)`; from the next
  frame `mouse_live` keeps the widgets below from seeing the pointer there (one frame of
  lag, invisible at 60 Hz). A modal occludes the screen and the keyboard. `scroll_area`
  and `list` also keep widgets in their content from seeing the pointer outside the view
  (an input clip).
- **Keyboard focus**: a focusable widget calls `focus_register(id)` each frame; Tab and
  Shift+Tab walk them in call order, Enter/Space activate, Esc backs out. A focused
  widget claims the keyboard from the engine, so focus is dropped by a click on nothing
  focusable and when the focused widget is not called. In a mission Tab only starts
  navigation when something is focused; a screen with no game keys calls
  `keyboard_nav()`. A click focuses only a widget that takes typing or arrow keys
  (text field, list, dropdown, spinner); buttons, toggles, sliders, check boxes and
  tabs take the focus from Tab alone, so a click on one never keeps keys from the engine.

`gallery.rs` is every widget on one screen: `[Demo] Shell=1` shows it beside the old
demo panel, in four tabs (Text, Lists, Choice, Misc). The scenario `ui-widgets` drives it.

## The HUD

The in-mission HUD is taken over a panel at a time: each stock `DisplayInterface`
component keeps simulating, and one switch stops it drawing and taking the mouse while a
panel of ours reads its state and presses its buttons. The button bar is the first;
the minimap, the selection overlay and the unit view's frame followed.
[`HUD.md`](HUD.md) has the inventory of the HUD's components, the switch, the bar and
the engine reads each panel uses.

## Testing

`./a2test run ui-widgets ui-widgets-cursor` (1280x720): the gallery driven by click,
keys, the wheel and the clipboard, and our cursor in a match (Scene.asi's `ui-cursor`
scene); `ui/bench-widgets.sh` is the second one's setup.

`./a2test run testbench/scenarios/ui-runtime.md --install .` (`--res 1920x1080`: the
Assembler's place is in screen pixels): the demo screen in the Credits' place and a
click on it, the loading-screen panel, the panel over the HUD, and a click on it that
the engine does not see (the selection stays).

`ui-whole-game.md` (`--install .`, at `--res 1280x720`, `21:9` and `1024x768`): the game as `./install` gives it, from the intro through a campaign mission, a match won and the Admiral's Log to Options and Exit (`testbench/operating/ui-switchon.md`).

`ui-menus-options.md` and `ui-menus-match.md` (`--menus`, 1280x720): the hosted menus,
`MENUS.md`, "Testing".

## Not done

- **The cursor**: the animation rate is a guess.
- **Widgets**: `button`, `toggle` and `slider` take no keyboard focus; the text field
  has no undo and no IME; a dropdown's press outside the list closes it but the press
  also reaches whatever is below it.
- Not verified: device loss in a screen of ours, the wheel, characters outside ASCII,
  a stock 800x600 shell, `Embed=0`.
