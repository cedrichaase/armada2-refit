# UI — the style

The visual language of the HUD, menus and loading screen that `UI.asi` draws
(`PLAN.md`, "Art"). It is decided once here; every panel and screen follows it. The
tokens live in `src/widgets/theme.rs` (`Theme`), the drawing primitives in
`src/gfx/fx.rs`, and `[Demo] Style=1` shows all of it on one screen (below).

## The decision: our own theme, the game's information art

**We draw the UI; we do not stretch the game's.** The stock HUD is six sets of
256-pixel panel sheets (`fedui0..5`, `klingUI0..5`, `romUI*`, `cardUI*`, `borgUI*`,
`speciesUI0..6`), cut into nine-slices and laid out on a 1600x1200 canvas
(`gui_<race>.cfg`). At 1080 lines and above they are magnified two to three times, so
their bevels and rivets go soft, and their shapes are fixed to the 4:3 layout `HUD.asi`
has to correct. The menus are 800x600 GDI bitmaps. Neither can be extended: a new
panel would need new art in the same style at the same resolution.

So every frame, edge, button and bar is drawn by our renderer from a few numbers
(`Theme`): analytic shapes with antialiased edges, sharp at any resolution and
aspect, any size, extensible by code. What the game's art is kept for is what carries
**information** the player has learned to read, and cannot be drawn from tokens:

| Kept from the game | Why |
|---|---|
| Unit portraits and wireframes (`*WIREFRAME0*`, the ship icons) | they identify the unit; the wireframe's damage colouring is the engine's |
| Build, order and special-weapon icons (the square button art) | players read them at a glance; 64 px art, drawn inside our buttons, never stretched past 2x |
| Race emblems (the Federation delta, the Klingon trefoil, ...) | identity; drawn as images inside our frames |
| Cursor shapes (`cursor.spr`) | the cursor's shape says which order a click gives (`README.md`, "The cursor"); `src/cursor.rs` draws the engine's art sharp |
| The menu plates (`Menus/mainbkgr.bmp`, built by `menus/backdrop.sh`) | the front end's backdrop (below) |

Not kept: the panel sheets, the GDI button and frame bitmaps, the bitmap font (the
UI's text is Noto Sans Condensed, `README.md`, "Text").

## Faction identity

**The stock HUD changes per race, and so do we**, but by *form* as much as colour:
each race keeps the silhouette its stock panels have, measured off the art (the
dominant colours below were read from the stock sheets with ImageMagick), drawn
clean. The front end is not any race's, so it has a theme of its own.

| Theme | Form | Read off the stock art | Ours |
|---|---|---|---|
| **Shell** (the menus) | `Shell`: gunmetal, chamfered corners | the stock dialogs' frames: grey bevelled metal with chamfered corners (Options, Game Setup), amber-gold labels (`SINGLE PLAYER`, `Comm`/`Side` headers) | blue-black glass panels, steel edges, 12-unit chamfers; amber titles and accent |
| **Federation** | `Lcars` | lavender `#4F4C94`, lilac-grey `#A19FB2`, sky `#6291B1`, pale `#C5CEF1`, gold bars, pink crew bar; pill ends and elbows: LCARS | black panels framed by elbows (lavender above, pale below) and a gold end bar; pill buttons with black capitals at the right, as on a console |
| **Klingon** | `Blade` | rusted red-brown plate `#482D29`, bright red blades `#9F2726`, yellow-black hazard stripes, elongated black hexagon insets | dark metal, big chamfers on opposite corners with red blades along them, hazard stripes in the header |
| **Romulan** | `Wing` | strong green `#015701`/`#42B044`, chamfered plates, diagonal hatching, steel-teal `#8BACBB` | green-black, chamfers all round, diagonal hatch in the header |
| **Cardassian** | `Arch` | ochre/amber `#823E10`..`#D0A056`, trapezoids and pointed arches | amber on brown, chamfered tops over square feet (the trapezoid), arch marks |
| **Borg** | `Cube` | grey machinery `#586564`, green circuit lights, dense greebles | near-black, square, double edge with green corner brackets, faint scan lines, blinking node lights |
| **Species 8472** | `Organic` | violet bone `#574C6F`/`#B19FD0`, gold-tan organic strands | purple-black, leaf-shaped corners (large radius on two opposite corners), soft lilac inner light, an amber "eye" |

The tooltip colours are the stock ones from each `gui_<race>.cfg`
(`tooltipBackgroundColor`): Federation 0.23,0.21,0.59 became LCARS black with a
lavender cap (the stock violet is the LCARS lavender's dark), the others are kept as
they are (Klingon 0.37,0.06,0, Romulan 0.02,0.38,0, Cardassian 0.46,0.23,0, Borg
0.36,0.48,0.54 darkened, 8472 0.50,0.28,0.07).

**Which theme shows** is `[UI] Theme=` in `UI.ini`: `auto` (the default) uses the
player's race in a mission and the shell's theme in the menus and the loading screen;
a theme's name (`shell`, `federation`, `klingon`, `romulan`, `cardassian`, `borg`,
`species8472`) puts that one everywhere. `Theme::for_context` decides; the player's
race comes from `theme::set_player_race`, which the mission frame calls with the race
the stock HUD is loaded in (`engine/race.rs`, HUD.md "The player's race").

## Tokens

All sizes are **units**: pixels of a 1080-line screen times `[UI] Scale` (`Ui::u`).

### Colour roles

| Role | What uses it |
|---|---|
| `panel`, `panel_top` | a panel's body, bottom and top of its gradient (translucent, 88-92%) |
| `panel_edge`, `panel_hi` | its edge, and the light along its top bevel |
| `text`, `text_dim`, `text_off`, `text_on`, `title` | body text, secondary, disabled, on a bright fill, titles |
| `accent`, `accent2`, `accent3` | the race's colour (hover, selection, fills), and two companions (LCARS uses all three) |
| `button`, `button_top`, `button_hot`, `button_down`, `button_on`, `button_off`, `button_edge` | a button's states |
| `track`, `fill` | the empty and filled part of sliders, bars, fields |
| `focus` | the keyboard focus ring: a lighter accent, never the accent itself |
| `tip`, `tip_text`, `tip_edge` | tooltips |
| `row_hot`, `row_sel`, `row_alt` | list rows |
| `good`, `warn`, `bad` | status (hull, shields, warnings): shared by every theme so they always read |
| `backdrop_top/bottom`, `scrim`, `shadow` | a screen with no plate, a modal's dimming, panel shadows |

### Type

One family, Noto Sans Condensed, Medium (`Regular`) and SemiBold (`Bold`).

| Token | Units | Weight | For |
|---|---|---|---|
| `display_size` | 44 | Bold | a screen's name |
| `title_size` | 26 | Bold, capitals | window titles |
| `heading_size` | 22 | Bold, capitals | a group inside a panel |
| `text_size` | 20 | Regular; Bold on buttons | body, labels, buttons, rows |
| `small_size` | 17 | Regular | notes, tooltips, secondary values |
| `micro_size` | 14 | Regular | HUD counters, badges: the smallest that reads at 720p (10 px) |

Titles are set in capitals (`caps_titles`); LCARS also sets buttons in capitals
(`caps_buttons`). Labels are the game's own localised strings otherwise, case kept.

### Space

A 4-unit grid: `gap` 8 between widgets, `pad` 12 inside panels, `row_h` 32,
`button_h` 40 (34 in dense panels), list rows 26.

### Shape

`corners` (panels), `control` (buttons, fields, tabs) and `small` (check boxes,
thumbs, bars) are `Corners`: four values, top-left clockwise, positive a radius,
negative a 45-degree chamfer, in units. `edge` 1.5 is the edge line; `glow` 14 the
hover glow's reach; `texture` stripes a panel's body (the Borg scan lines);
`bar_stripes` segments a bar's fill (LCARS blocks, Borg cells, Klingon and Romulan
slashes).

## Components

Each is one function in `theme.rs`, given its rectangle and a `State` (hover eased
0..1, down, on, off, focus), so a widget only lays itself out and handles input.

- **Panel** (`theme::panel`): a soft shadow, the body's vertical gradient, the edge,
  a one-pixel bevel light along the top fading down, and the form's mark (Klingon
  blades on the two big chamfers; Borg inner edge and green brackets; Cardassian
  lintel; 8472 inner glow). LCARS: no shadow, no edge: a black body and the elbow
  bracket.
- **Window** (`theme::window`): a panel with a header as tall as the title plus
  padding, tinted from `panel_hi`, the title in capitals in `title`, an accent
  underline fading to the right, and the form's mark at the header's right (a short
  amber dash, hazard stripes, hatching, three arch marks, three node lights, an
  amber eye). LCARS: the title sits in the top bar's gap, right of the elbow, with a
  sky-blue cap at the end.
- **Button**: normal / hover / pressed / active / disabled / focus.
  Hover fades in over ~100 ms (`HOVER_RATE`): the fill lightens toward `button_hot`,
  the edge turns `accent`, and a faint accent glow appears around it. Pressed is
  `button_down`, a darker accent, with white text nudged down a pixel. Active (a
  chosen option, the open tab) keeps the accent edge and adds an accent bar inside the
  left edge. Disabled halves the fill and edge and greys the text. LCARS: flat pills
  whose colour carries the state (lavender, light lavender, near-white, gold, dark),
  black capitals at the right.
- **Focus ring** (`theme::focus_ring`): a 2-unit line in `focus` 3 units outside the
  widget, following its shape, and a faint glow. Shown only for keyboard focus.
- **Check box** (`theme::check`): the `small` shape, `track` inside, an accent square
  with a glow when on. LCARS: a pill switch, gold with a black knob at the right when
  on. Radio buttons are round in every theme.
- **Slider** (`theme::slider`): a 6-unit track, the filled part a gradient into
  `fill`, the thumb in the form's shape (a pill, a diamond, a chamfered square, an
  arch, a square, a circle), glowing on hover.
- **Bar** (`theme::bar`, progress, hull, shields): `track` with a hairline edge, the
  fill inset, striped where the form stripes, its leading edge glowing.
- **List row** (`theme::row`, `row_text`): alternate rows faintly lit, hover
  `row_hot`, selected `row_sel` with an accent bar at the left (a gold half-pill in
  LCARS).
- **Scroll bar** (`theme::scrollbar`): a 6-unit pill track, the thumb `button_edge`,
  accent on hover.
- **Text field** (`theme::field`, `caret`): `track`, an inner shadow at the top, the
  edge accent when focused with a glow; the caret is a 2-unit accent bar that stays
  solid for half a second after it moves, then blinks.
- **Tab** (`theme::tab`): the open tab is a panel-coloured tab with an accent line
  along its foot and the title colour; others are dim text whose line grows on hover.
  LCARS: half-pills, gold for the open one.
- **Tooltip** (`theme::tooltip`): small text on `tip` with a hairline `tip_edge` and a
  shadow; LCARS black with a lavender cap.
- **Separator**, **heading**, **dropdown arrow**: as their names say.

## Screens

- **The menu backdrop.** A screen of ours in the front end draws `theme::backdrop`:
  the widescreen menu plate the menus layer builds (`menus/BACKDROPS.md`), read at run
  time from the game's `Menus/mainbkgr.bmp`, where `menus/install-plates.sh` installs
  it, scaled to cover the screen and darkened toward the bottom (the panels sit
  there). It is an asset in `$A2_DATA`, never in the repository; **without it** (no
  plate built) the screen is a dark blue gradient with a fixed field of faint stars,
  so the layer works without assets. Panels over it are translucent glass.
- **The loading screen** keeps the engine's logo scene (`models/README.md`, the
  widened `logo.SOD`) and puts a band across the bottom: the mission's name in title
  type, a line of status in small type, the percentage at the right, and the shell's
  bar. Nothing animates there, because the engine presents only at 5% steps
  (`README.md`, "The loading screen").
- **The HUD** takes the player's race's theme. Its panels are anchored to the screen
  edges (`Anchor`) at any aspect, translucent, the 3D view showing through.

## The cursor

The engine's cursor art, drawn by us at the screen's resolution (`src/cursor.rs`,
`[UI] Cursor=1`), in every theme: its shapes are information (move, attack, board,
...), and players know them. The menus show the same arrow. No theme recolours it.

## Motion

Subtle and cheap: nothing moves that is not reacting to the player or opening.

- **Hover**: an eased value per widget id (`widgets/anim.rs`, `State::of`), ~100 ms in
  and out. No motion on press: pressed is immediate, so a click always feels instant.
- **Opening**: a window's title underline sweeps in over 220 ms and its edge flashes
  in the accent and fades (`anim::appear`); LCARS fades its title in and grows the gold
  end bar. Closing is immediate (an immediate-mode panel that is not drawn is gone).
- **Living marks**: the Borg node light blinks (1.6 s), the caret blinks. Nothing
  else loops.

`anim.rs` keeps 96 values by widget id; a value not asked for in 250 ms starts afresh.

## How a new widget gets the style

1. Lay out and answer input in the widget (`widgets/`), then **draw by calling
   `theme::*`** with a `State` (`State::of(ui, id, hot, held)` for the eased hover,
   then set `on`, `off`, `focus`). A button-like thing calls `theme::button`, a list
   `theme::row`/`row_text`/`scrollbar`, an input `theme::field`/`caret`.
2. Something new: compose `Draw`'s shape primitives (`fill`, `fill_v`, `stroke`,
   `stroke_v`, `glow`, `inner_glow`, `shadow`, `stripes`, `pill`, `elbow`, `shape`)
   with the theme's tokens, with corners from `theme.control`/`small` scaled by
   `ui.scale` (`Corners::scaled`), and branch on `theme.form` only for what a form
   draws differently (LCARS most often). Put it in `theme.rs` as a function so the
   next widget can reuse it.
3. Never hard-code a colour except pure black and white; never draw a rounded
   rectangle where the theme's corners say otherwise; sizes in units.
4. Add it to the gallery (`src/style.rs`) in its states, and look at it on the bench
   in every theme.

## The primitives (`src/gfx/fx.rs`)

One more shader kind (`shaders/ui.hlsl`, kind 4, the *shape*): a box whose four
corners are each rounded or chamfered, filled or stroked, with a soft or sharp outer
and inner edge, an optional rounded cut-out at one corner (the LCARS elbow's inside,
mirrored to any corner), and optional stripes at any angle. Everything is computed
from the pixel's offset to the box's centre, so one quad draws it at any size,
antialiased over one pixel; glows, inner glows and shadows are the same shape with a
wide soft edge. The corners and parts are chosen with `step`/`lerp`, no branches
(vkd3d, `render3d/README.md`). Three float4s per vertex carry the parameters, so the
vertex is 84 bytes; a whole frame of UI is still a few draw calls.

## The gallery

`[Demo] Style=1` in `UI.ini` (`ui/bench-asi.sh <state> Demo.Style=1`): in the front
end the Credits show the style gallery over the menu plate (Esc leaves it); in a
mission it is drawn over the game; on the loading screen the loading look. Eight
cards: each theme with buttons in every state, a live toggle and slider, a bar, a
list with hover, selection and scroll bar, a focused field with caret and drop-down
arrow, tabs, a tooltip and the colour swatches; and the type scale and spacing.
