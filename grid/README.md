# grid — the button bar as a grid of keys

`GridLayout.asi` turns the button bar into a fixed 5×3 grid and gives every cell one
key, by its position on the keyboard:

```
 Q  W  E  R | T      T  cancel, in every build menu
 A  S  D  F | G      G  back, in every submenu
 Z  X  C  V | B      B  the 13th item of a construction ship's build list
```

Each button shows its key in its top-left corner: yellow if it can be used, grey if not.
The left four columns are the grid proper; the fifth holds the keys that never move.
At 16:9 and wider the grid sits at the bottom of the screen between the minimap and the
selected unit's info panel, which moves right to make room; where there is no such room
(4:3, 16:10) it stays above the info panel, where stock's bar is.
This is QOL-1 in [`qol/README.md`](../qol/README.md), built as a plugin of its own so it
can be switched on and off without touching anything else: with `GridLayout.asi`
installed the bar is the grid, without it the bar and all its keys are stock.

It changes only how this player gives orders, never what the orders are: a key does
exactly what clicking its button does. A player with the grid can play a player
without it.

## Using it

```bash
grid/install.sh            # build and install GridLayout.asi + GridLayout.ini
grid/install.sh --remove   # take it out: the stock bar and keys are back
./a2mod stock / refit      # switches it with everything else (layer `grid`)
```

`GridLayout.ini`:

| Key | Default | |
|---|---|---|
| `[GridLayout] Enabled=` | 1 | 0 leaves the bar stock without uninstalling |
| `Place=` | beside | `beside`: between the minimap and the info panel where there is room; `above`: always above the info panel |
| `Labels=` | 1 | the key in each button's corner |
| `LabelSize=` | 87 | its size in percent of the HUD's text (50–150) |
| `Log=` | 1 | `GridLayout.log`: each menu's layout, each key pressed |
| `[Cells] <name>=<key>` | none | move one button, e.g. `bmining=Q`. Names as the log prints them |

`install.sh` keeps an existing `GridLayout.ini`, so `[Cells]` choices survive a
reinstall.

## Where things go

**Keys are positions.** They are read by scancode, so on a QWERTZ keyboard the
bottom-left key is the one printed Y, and the grid is the same shape everywhere.

**T is cancel and G is back** in every menu that has them. Nothing else is put on T or
G unless a menu would otherwise overflow, which none does. The longest menu in the game
is the construction ship's build list: 13 items, with cancel and back 15, exactly the
grid. Its 13th item goes to B.

**The top level** (what a unit shows when selected):

- the top row holds the menus, and the build, research or evolve menu always comes
  first: Q for every unit that has one;
- the middle row holds commands (stop, recrew, transport, rally, …), in stock's order;
- the bottom row holds special weapons, from the left (Z).

**Submenus** use the plan the game already has. Every command's ODF gives a
`preferredPosition` on a three-column grid, so the AI menu is a natural 3×3: alert
levels on Q W E, movement on A S D, special-weapon use on Z X C. The orders menu puts
attack on Q, the repairs beside it, and patrol, explore, guard and decommission below.

**Build lists** fill the four left columns top to bottom, one column at a time, in the
unit's build order, so the first items are under the strongest fingers: Q A Z, then
W S X, then E D C, then R F V, then B.

## What it switches off

The bar's own keys, while the grid is on: the F-keys of the build items, the special
weapons' keys, the command letters (A attack, S stop, G guard, …), and the menu toggles
(B build, C orders, V trade, N AI, F formations, X palette). Only the bar ever reads
them (checked against every reference to those control names), so nothing outside the
bar loses a key. Everything else — selection, groups, camera, chat, saving — keeps its
keys.

No key does anything while text is being typed (chat), while Ctrl or Alt is held, or
while the game is not the foreground window. A key on a disabled (grey) button does
nothing, as a click does.

## How it works

Everything is in memory; no game file is changed (`Input.map` and the GUI configs
stay stock). The bar is `PopupPaletteImp` (`g_pPopupPalette`, 0x763c00). Three calls
inside its `Update` (0x4fb330) are wrapped, and the first instruction of its `Render` is
detoured. No vtable is changed. That matters for `HUD.asi`, which recognises the bar by
the `PostLoad` in its vtable when it lays the HUD out again after a mode change.

| Site | What is there | What the plugin does |
|---|---|---|
| call at 0x4fb3ec, and at 0x4fd74f in `Show` | `mUpdateButtonsAndPosition` (0x4fc670) | sets the bar to 5×3 before it, gives every shown button its cell after it |
| call at 0x4fb56a | `mProcessKeyboardInput` (0x4fb5b0) | runs it with the stock hotkey tables empty, then presses the button under a grid key |
| 0x4fbce0, `cmp [0x7643cc], ecx` | the start of `PopupPaletteImp::Render` | jumps to a trampoline that runs `Render`, then draws the labels |
| call at 0x4f0097, in `ShipDisplay::PostLoad` | `DisplayInterface::LoadRectangle` (0x51b430) for `infoPanelArea_0`/`_1`/`_2` | moves the info panel right when the grid goes beside it |

### The layout

The bar keeps 21 `ControlButton`s at +0x28. Stock fills them by a 3×7 slot plan: a
command goes to slot `row * [+0x7c] + col` from its `preferredPosition` (the command's
`CommandInfoClass` holds it at +0x1b0), and the build list follows in `buildItemN`
order (`mSetupCommandInfoClassButtons`, 0x4fcfc0). Then it packs the used slots to the
front, caps them at columns × rows of the current mode, and gives the k-th one the cell
`(k % columns, k / columns)`. In mode A, the bar along the bottom, rows count up from
the bottom, which is why stock's construction menu puts ten buildings on the bottom row
and the other three, with cancel and back, above them.

The columns and rows of each mode are fields of the bar: +0xa0/+0xa4 for mode A
(`staticPaletteWidthA`/`HeightA`, 10×3), +0xa8/+0xac for B, +0x7c/+0x80 floating
(`paletteWidth`/`Height`, 3×7), copied into +0xb0/+0xb4 for the mode in use
(`SetPaletteMode`, 0x4fbdd0). The plugin sets A, B and the active size to 5×3 before
the call, so stock caps a menu at 15. **It leaves +0x7c alone**: that width is also the
slot plan's, and with 5 in it the first build of this plugin put recrew and transport
into slots that the fixed menu buttons then overwrote, and they vanished from the bar.

After the call, each shown button (the ones `IsValid`, vtable +0x1c, as `Render` draws
them) is named — `CommandInfoClass` +0x88 with its `buttonName` inline at +0x20; or a
`ModeInfo` +0x84 whose kind (+0x04) is 1 a build item (ODF name at `[+0x0c]+0x7c`), 2 a
special weapon (`[+0x10]+0x208`, named by `cPrjID::GetName`, 0x65dce0) or 3 a fixed
button (+0x14 indexes back, order, build, research, evolve, trade, form, aiMenu,
close) — and given its cell. Its rect (+0x08, relative to the bar's top-left) becomes
the bar's single-button rect (+0x88) moved by whole cells of pitch +0x98/+0x9c plus the
gap +0xb8. That rect is what the button is drawn at and what the mouse hits. The bar's
bounds (+0x04) become five cells wide and three tall; in mode A the bottom row stays at
`popupPaletteYA` (+0x118) and the grid grows upwards, so `HUD.asi`'s placement of the
bar (`popupPaletteXA`, flush with the info panel) is kept as it is.

### Where the grid goes

At 16:9 the bottom of the screen has, in `HUD.asi`'s 2133-wide canvas, the minimap
(350 wide) on the left, the unit view (350) on the right and the info panel (880)
centred between them, leaving about 276 on either side of the panel. The grid is five
buttons of 80 and their gaps, about 410: it fits beside the panel only if the panel
moves over. So the panel goes right, up to a small margin from the unit view, and the
grid is centred in the room that opens between it and the minimap, its bottom row a
margin above the bottom of the screen. At 16:10 (1920) the three panels and the grid
need about 2000 and do not fit, and at 4:3 even less; there, and with stock's
1600-wide canvas (no `HUD.asi`), the grid stays above the info panel.

The info panel is `ShipDisplay`. Its `PostLoad` reads its three heights' rects —
`infoPanelArea_0` low, `_1` middle, `_2` tall, all at the same x; everything inside the
panel is relative to them — through one call to `DisplayInterface::LoadRectangle`
(0x4f0097, in a loop). That call is wrapped. The wrapper reads, through the same
function, the minimap (`minimapPanelArea`), the unit view (`cinematicPanelArea`) and one
button (`paletteSingleButtonArea`), decides, and moves the panel's rect. All of it is
in the 1600 × 1200 space `LoadRectangle` returns (`{left, top, right, bottom}`,
inclusive, through `SetRect`), which is the space `HUD.asi`'s canvas is converted
into. So the decision holds with `HUD.asi` or without it, and is made again when
`HUD.asi` re-runs the panels' `PostLoad` after a display mode change. The bar is then
anchored there each frame, through its mode-A anchor fields (+0x114 left, +0x118 the
top of the bottom row), which `PostLoad` fills from `popupPaletteXA`/`YA`. That
overrides `HUD.asi`'s `popupPaletteXA` while the grid is beside the panel, and leaves it
alone otherwise. `HUD.asi`'s own edit of the GUI config is not touched: the two plugins
hook different places.

### The keys

`mProcessKeyboardInput` walks three tables of `{control state, what it triggers}` that
`PostLoad` builds from `Input.map`: commands (+0xc4, count +0xcc), build items, every
`bc_` control (+0xd0, +0xd8), and special weapons, every `wc_` control (+0xdc, +0xe4).
It checks each against what is selected, not against the menu on screen. That is why
stock's F5 starts a Processing Node from the Assembler's top level, with no build menu
open (seen on the bench). The six menu toggles are control pointers that `Init`
(0x4fa8d0) keeps at 0x763d3c–0x763d50 (popup, build, orders, trade, AI, formations).
Trade is read by `Update` itself; the rest by `mProcessKeyboardInput`.

The wrapper empties the three counts for the stock call and restores them, and points
the six toggles at a zero, so the bar's stock keys do nothing and the tables are
untouched for anything that reads them later. Then, if a grid key went down since the
last frame, it does what releasing a click does in `StandardButton::Simulate`
(0x50b620): nothing if the button's state (+0x34) is 0 (disabled), else the click sound
(`s_buttonClickSound`, as an `AudioSound2D` like stock's) and the button's press
function (vtable +0x20, `ControlButton::mButtonPressFunction`, 0x4e69e0). That hands
the button's command or mode to the bar exactly as a click does.

Keys are read with `GetAsyncKeyState` on the virtual key of each scancode, and only
when the game owns the foreground window. Chat is excluded the way the game excludes
it: `InputNext` skips every key binding while `TextInput_IsActive()` (0x65f120) is set,
and the plugin asks the same function.

### The labels

`DisplayInterface::DrawTextOutside` (0x51afa0) shows how the HUD writes a word: the
default font (`g_pDefaultFont`, 0x736f30, a `MetaFont` whose first field is the
`ST3D_Font`), its colour as three floats at +0x0c, then
`ST3D_Font::PrintString(font, &point, text)` (0x628890) at a point in the same space as
the button rects. The labels are drawn that way after the bar's `Render`, at each
button's corner, with the font's scales (+0x28 x, +0x2c y; `HUD.asi`'s condensing is
in x) multiplied by `LabelSize` for those glyphs and put back after. `Render`'s first instruction compares with an absolute address, so it
runs unchanged in the trampoline.

## Bench

`./a2test run grid-layout` goes into an Instant Action match as the Borg against one
easy AI (`testbench/scenarios/_enter-instant-action.md`). It opens the Assembler's
build menu with Q and starts a Processing Node with S. It checks that the stock F5 and
B do nothing, and that typing in chat presses nothing. `GridLayout.log` says what each
menu put where.

Seen there (2026-10-04, 1920×1080, Borg; `grid beside the info panel: grid x 311, info
panel moved right by 197 (of 1600)`):

```
menu=0 | A=stop W=order S=transport Q=build E=form R=aiMenu Z=gtracbm      (Assembler)
menu=0 | A=recrew W=order S=transport Q=build D=rally E=aiMenu              (Nexus)
menu=2 | Q=bbase A=byard Z=bsensor W=bturret S=bmining X=bresear E=bresear2 D=byard2
         C=bturret2 R=bsuperbl F=btechass V=bupgrade B=brecycle T=cancel G=back
menu=1 | W=repair E=superRepair A=patrol S=scout Z=guard X=decom G=back     (orders)
```

## Not done

- The palette's mode switch (`X` in stock, `toggle_popup`) has no key while the grid is
  on. The HUD's palette button still switches it.
- Only the Borg have been seen on the bench. The other races' lists are the same shape
  (13 construction items, at most 8 at a yard), but their top levels have not been
  looked at.
