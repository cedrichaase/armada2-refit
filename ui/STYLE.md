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
their bevels and rivets go soft, and their shapes are fixed to the 4:3 layout the retired `HUD.asi`
corrected. The menus are 800x600 GDI bitmaps. Neither can be extended: a new
panel would need new art in the same style at the same resolution.

So every frame, edge, button and bar is drawn by our renderer from a few numbers
(`Theme`): analytic shapes with antialiased edges, sharp at any resolution and
aspect, any size, extensible by code. What the game's art is kept for is what carries
**information** the player has learned to read, and cannot be drawn from tokens:

| Kept from the game | Why |
|---|---|
| Unit portraits and wireframes (`*WIREFRAME0*`, the ship icons) | they identify the unit; the wireframe's damage colouring is the engine's |
| Build and special-weapon icons (the square button art) | they identify a unit or a weapon; 64 px art, drawn inside our buttons, never stretched past 2x. The **command** icons are ours (below, "Icons") |
| Race emblems (the Federation delta, the Klingon trefoil, ...) | identity; drawn as images inside our frames |
| Cursor shapes (`cursor.spr`) | the cursor's shape says which order a click gives (`README.md`, "The cursor"); `src/cursor.rs` draws the engine's art sharp |
| The menu plates (`Menus/mainbkgr.bmp`, built by `ui/backdrops/backdrop.sh`) | the front end's backdrop (below) |

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
| **Federation on Roots** | `Starfleet` | Roots' own Federation HUD (below) | dark brushed gunmetal rims round smoked glass, blue lamps in the side rims of the info panel only, lighting its glass from the ends; light-grey tile buttons |
| **The other races on Roots** | `Machine(...)` | each race's Roots HUD (below, "The other factions on Roots") | Starfleet's frame and glass in the race's own metal, glass and lamps |
| **Species 8472** | `Organic` | violet bone `#574C6F`/`#B19FD0`, gold-tan organic strands | purple-black, leaf-shaped corners (large radius on two opposite corners), soft lilac inner light, an amber "eye" |

The tooltip colours are the stock ones from each `gui_<race>.cfg`
(`tooltipBackgroundColor`): Federation 0.23,0.21,0.59 became LCARS black with a
lavender cap (the stock violet is the LCARS lavender's dark), the others are kept as
they are (Klingon 0.37,0.06,0, Romulan 0.02,0.38,0, Cardassian 0.46,0.23,0, Borg
0.36,0.48,0.54 darkened, 8472 0.50,0.28,0.07).

### Starfleet (the Federation on Fleet Operations: Roots)

Roots does not dress the Federation in LCARS: its HUD is **dark machined metal round
smoked glass, lit by blue lamps**, so on Roots the Federation's look is `starfleet`
(`content/sets/fleetops/ui.ini`); Armada II's set keeps `lcars`. The look is matched to
Roots' own Federation HUD **by measurement**: its interface sheet and bench shots of
its HUD at 1280x960 were read with ImageMagick (read only); none of its art is
copied, traced or shipped, and the look is drawn in code (`theme.rs`, "Starfleet").
What Roots' HUD is, in words:

- **The rim** is neutral gunmetal (no blue in it), about 9 pixels at 960 lines on the
  minimap, lit from above (its top edge the brightest), a fine even grain, a dark
  line round the outside, a dark lip where it meets the glass, and a seam across the
  side rims half way down.
- **The glass** is a smoke, near neutral in the middle, a little lighter at the top
  and darker toward the right; the map shows through it dimly (about 70% cover).
- **The blue is the lamps'**: two small slots in each side rim, either side of the
  seam, bright sky blue at their core, and their light falls across the glass from
  each end: strong within a few pixels, gone to a faint tint a hundred pixels in.
  The left end is the stronger. Nothing else on a panel is blue. **Only the unit
  info panel along the bottom has lamps**: the minimap's frame has none (a blue strip
  of mode buttons at its foot, no inset), the top-right buttons sit in no frame at all,
  the resources are bare icons and figures, and the bar's tiles float without a panel.
  Roots' interface sheets hold no other framed panel.
- **The buttons** are translucent light-grey tiles with a bright rim, lit along the
  foot from inside, holding Roots' full-colour icon art; the unit's name is amber,
  values pale steel blue.

Ours, measured on the bench at the same view (1280x960, a Federation skirmish; values
are 0..255, Roots' info panel and minimap beside our resource strip and minimap):

| Where | Roots | Ours |
|---|---|---|
| glass, middle | 24,23,32 | 23,24,32 |
| glass, top / foot of the middle column | 27,25,36 / 21,20,28 | 28,29,38 / 21,21,29 |
| glass at the left lamps' edge | 30,48,108 | 28,48,119 |
| glass about 90 px from the lamps | 25,25,37 | 27,29,40 |
| glass about 20 px from the lamps | 29,37,73 | 29,39,70 |
| glass at the right lamps' edge | 17,37,89 | 23,37,84 |
| lamp core | 73,150,255 | `SF_LAMP` 73,150,255 |
| side rim across its width (minimap) | 62, 62, 60, 59 | 45, 66, 65, 66 |
| top rim, outer to inner | 64, 91, 79, then the lip | 60, 55, 62, 43, then the lip (Roots' top is lit brighter) |
| rim grain (luminance deviation along a rim) | 7 | 5 |

So in code: the glass is `panel_top` over `panel` (`0xb41c1c28`, `0xb4111219`) with a
light darkening to the right, and no scan lines (the user's call, 2026-11-07); on a
panel with lamps each side's lamps
throw two soft pools (`sf_lamp_light`: `SF_POOL`, a deeper blue than the core, within
30 units, and the dark blue `accent3` out to 170 units, each kept to its own quarter of
a short panel so the middle stays smoke); the rim is 10 units of the **metal kind**
(`Draw::metal`, shader kind 5: streaks along x at one unit's width, wider streaks,
slow blotches and a per-pixel speckle, fixed in screen pixels so plates side by side
share one surface and nothing crawls when a panel moves by a pixel; strength 0.1, a
broad sheen from the top left), with a light hairline along its top, a dark outline
and a dark lip 2.5 units wide. **Starfleet's buttons are dark** (the user's call,
2026-11-07): a near-black plate (`button_top`/`button`, 0x14/0x0a grey at 91%) with a
dark grey 1.2-unit edge and a faint top highlight, the label in the lamps' blue
(`accent`), lighter on hover and near white pressed; on hover the label **blooms**, a
soft halo of the same blue round its glyphs (`Draw::text_glow`: the text's distance
field drawn wider and softer, shader kind 2's `z`, at most the atlas's reach), and the
plate's edge and a faint outer glow turn blue; disabled is a dimmer plate with the label
in `text_off`; on keeps the lamp along its foot. (Roots' own art tiles in the bar and the
top-right buttons are drawn as they are.) The other machined looks keep the brushed
tiles: the metal kind in the race's translucent colour with a bright rim and a light
along the foot, lit from inside on hover, a lamp along the foot when on. Window
headers keep the glass and carry no mark. Text, status colours, sizes
and the amber `accent2` are the shell's and Roots'.

**Which panels carry lamps** is the panel's own choice, not the theme's guess from where
it sits: `theme::panel_as(ui, r, Lamps::On)` (`Ui::panel_lit`) draws them, `panel`
(`Lamps::Off`) does not, and the forms other than Starfleet ignore it. Lamps on
**the info panel** (`hud/info.rs`, one unit and a group), where Roots has them, and
**the resource strip** (`hud/resources.rs`), which Roots draws with no frame: the user
wants them there (2026-11-07). No lamps: the minimap, the top-right buttons, the bar's
grid, the pause, trade, replay and leftover boxes, the hover card, the menus and the
Refit screen. (On Roots the info strip is Roots' own while `Info` leaves it to the
game, so a Federation HUD of ours may show no lamp at all; that is Roots' look.)

### The other factions on Roots

On Roots every race's HUD is the same idea as the Federation's (a metal rim round glass,
a minimap frame and an info strip along the foot) in that race's own metal, glass and
light, so every race gets a **machined look** of its own: Starfleet's frame and glass
(`starfleet_panel`, `starfleet_button`), drawn from the race's `Machined` tokens
(`theme.rs`, `Theme::machine`). The looks are named by how they look, and the content
set says which race wears which, so no race is named in code (`ui/lint-content.sh`):
the fleetops set gives the Klingon `redline`, the Romulan `jade`, the Cardassian
`bronze`, the Borg `circuit`, Species 8472 `bone`, the Free Borg `circuit-teal` and the
Dominion `violet`. Armada II's set keeps its looks (blade, wing, arch, cube, organic).
Measured like Starfleet's, off each race's Roots interface sheet composited over dark
space (as it shows in game; read only, never copied), in 0..255:

| Race | Rim (minimap side; top light) | Glass (left, middle, right; top, foot) | Lamps | Frame |
|---|---|---|---|---|
| Klingon | dark red-brown 37,14,12..33,7,5; top light 98,91,86 | near black-red: 35,2,3 lit; 11,4,6; 6,4,7; red lines across it (43,4,6) | **red**, 255,0,0 at the core, in the top corners, lighting the glass from there | thin, a chamfer at the top right; grilles |
| Romulan | dark teal-black 8,17,14 / 4,22,15; top light 63,70,67 | teal green lit from the left: 22,51,39; 17,24,17; 8,16,14 | **green** slots along the top rim (30,100,35 lit) | thin, round corners, claws at the left |
| Cardassian | olive 47,42,28; top light 82,75,55 | opaque olive bronze: 50,41,27 flat; 40,34,22 at the right; 60,52,38 at the top | none | thin, clamps over the top rim, a cut bottom-right corner |
| Borg | dark 20,23,20 with a bright green line 6,81,2 | dark green speckle: 8,20,9 middle, 8,44,8 near the lamps, 4,11,5 right | **green** nodes (82,255,49 core) in the top left corner and the foot's middle | cables and pipes; square |
| Species 8472 | thick lavender bone 77,64,116; top light 194,173,229 | violet: 26,19,46 at the top to 15,7,48 at the foot | none | thick, soft, segmented by notches |
| Free Borg | as the Borg, the line teal 2,81,70 | dark teal speckle: 8,20,20; 7,44,41 near the lamps | **teal** nodes (49,255,205) | as the Borg |
| Dominion | dark violet 14,2,39; top light 110,89,154 | violet lit from the left: 32,19,72; 20,11,43; 12,2,40 (ours: black, by request) | **violet** (130,80,255) at the clamps along the foot | as the Cardassian (clamps, cut corner) |

Checked in game (1280x960, `fo-ui-theme-roots-<race>`; Roots' own info strip is in the
same shot, so both sit over the same space): the middle of the glass, Roots' strip
against our resource strip, in 0..255:

| Race | Roots | Ours |
|---|---|---|
| Klingon | 24,15,18 | 21,12,16 |
| Romulan | 28,36,29 | 23,37,28 |
| Cardassian | 65,54,40 | 62,52,37 |
| Borg (foot, by the lamp) | 20,33,22 (17,58,16) | 18,35,21 (22,63,23) |
| Species 8472 | 31,22,59 | 23,15,53 |
| Free Borg | 20,33,35 | 18,35,34 |
| Dominion | 33,21,59 | dark by request (below) |

**Dark, or tinted?** Measured over the whole of Roots' info strip in game (each pixel
counted; near-black: no channel over 35; tinted: chroma over 25), against our resource
strip in the same shot:

| Race | Roots: near-black / tinted | Ours | So |
|---|---|---|---|
| Klingon | 86% / 10% | 80% / 12% | dark with red accents, as ours |
| Romulan | 42% / 6% | 38% / 19% | dark, a little green in the glass, as ours |
| Cardassian | 3% / 33% | 7% / 38% | olive throughout: kept |
| Borg | 63% / 9% | 65% / 6% | dark with green accents, as ours |
| Species 8472 | 0% / 96% | (lifted toward Roots) | violet throughout: kept, a shade lighter |
| Free Borg | 60% / 9% | 62% / 6% | as the Borg |
| Dominion | 1% / 93% | dark by request | Roots' is violet throughout (mean 33,21,60); ours is black gunmetal and black glass with violet only in the accents, at the user's word (2026-11-07) |

How ours carries each (`Machined`): the rim's colours, width and grain; `glass_light`
for the light lying from the left (Romulan, Dominion); `shade_right`; `glass_grain`, the
metal kind's speckle on the Borg's glass; `lines` for the Klingon's structure;
`rim_line` for the Borg's circuit; `notch` for 8472's segments; `clamps` for the
Cardassian's and the Dominion's brackets; and where the lamps sit (`LampAt`: `Sides`
Starfleet's, `Top` the Romulan's, `Corners` the Klingon's, `Nodes` the Borg's and the
Dominion's, `None` the Cardassian's and 8472's). Lamps light only the panels that ask for
them (the resource strip and the info panel), as Starfleet's. The text, title and
status colours are each race's Armada II look's, so a race reads the same on both games.

**Which theme shows** is `[UI] Theme=` in `UI.ini`: `auto` (the default) uses the
player's race in a mission and, in the menus and the loading screen, the content set's
look (`[Looks] Menus`; Armada II's set names none: the shell's; Roots' names
`starfleet`), with the menus over a running mission following `[Looks] MissionMenus`
(`race`: the HUD's race's look, as Roots' set has it; empty: as the front end's);
a theme's name (`shell`, `federation`, `klingon`, `romulan`, `cardassian`, `borg`,
`species8472`), or a look's id (`starfleet`, `redline`, `jade`,
`bronze`, `circuit`, `bone`, `circuit-teal`, `violet`,
`lcars`, `blade`, `wing`, `arch`, `cube`, `organic`), puts that one everywhere. `Theme::for_context` decides; the player's
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
- **Status badge** (`theme::status_badge`, width `status_badge_w`): a small plate with
  one word on it in `micro_size`, bold, from tokens only (`track`, `button_edge`,
  `text_dim`; active in `accent` with `text_on`; dimmed with `text_off`). It states a
  fact about a row of the Refit screen (Now, Restart needed, Must match, Locked); not
  to be confused with `theme::badge`, the HUD bar's key letter.

## Screens

- **The menu backdrop.** A screen of ours in the front end draws `theme::backdrop`:
  the widescreen menu plate (`ui/backdrops/BACKDROPS.md`), read at run
  time from the game's `Menus/mainbkgr.bmp`, where `ui/backdrops/install-plates.sh` installs
  it, scaled to cover the screen and darkened toward the bottom (the panels sit
  there). It is an asset in `$A2_DATA`, never in the repository; **without it** (no
  plate built) the screen is a dark blue gradient with a fixed field of faint stars,
  so the layer works without assets. Panels over it are translucent glass.
- **The loading screen** (`[Loading] Enable=1`, `LOADING.md`) is a screen of ours over
  the whole frame, in the player's race's theme once the load knows the race (the
  shell's until then). **The backdrop is the map's own sky**: the brightest face of the
  skybox its `.bzn` names, cropped to cover the screen, darkened toward the bottom and
  toward the left where the text sits. The faces are the game's own files (stock or
  refit), so this needs no asset of ours; a map whose sky cannot be read gets the menu
  backdrop above (the plate, or the gradient and stars). Not the menu plate everywhere:
  it is the shell's picture, not the mission's, and a briefing has no art of its own.
  Down the left, as a briefing reads: the kicker (the campaign and mission number,
  Instant Action, or Multiplayer) in accent capitals, the mission's title in display
  type, a short accent rule, the briefing in body type, the objectives under a heading
  with accent dots, the hint dimmed. At the right, for a game set up in the shell: a
  multiplayer map's minimap preview, then the players in a window, a row each (team
  badge, name, race; the local player in title colour), with a thin bar each for a
  network player's load. Across the bottom: a ring of dots turning, what the load is
  doing in plain words, the percentage at the right in title type, and the theme's bar.
  The ring is the screen's one living mark; it turns because frames of ours are
  presented between the engine's 5% steps. The engine's logo scene and bar are covered,
  so the widened `logo.SOD` is not needed for it. With `Enable=0` the engine's screen
  stays, and `[Demo] Style=1` still shows the earlier band over it.
- **The Refit screen** (`menus/refit.rs`, MENUS.md) is a modal over the Options dialog in
  the menus' theme (`Theme::for_context(Context::Menus)`, so a live `UI.Theme` restyles
  it): a tab strip, a page of rows in one or two columns (label over its badges at the
  left, the control at the right, a dot at the left edge when the overlay holds the key),
  a help strip two lines tall, a banner in `warn` when a restart is pending, and a foot
  (Reset tab, Show advanced, Done).
- **The HUD** takes the player's race's theme. Its panels are anchored to the screen
  edges (`Anchor`) at any aspect, translucent, the 3D view showing through.

## Icons

**The commands' and the HUD buttons' icons are ours** (`[HUD] Icons=1`), drawn in
code: the game's are bevelled metal tiles with the symbol in the middle, painted for
64 px and a 4:3 HUD, and looked out of place in flat panels. Ours keep **the game's
metaphor for each** (the captain's chair for orders, the steel I-beam for building, the
flask for research, the brain for the AI menu, the hand for trade, binoculars for
explore, the wrench for repair, darts in each formation's shape) and draw it flat:

- one **ink** colour, the button's content colour (light on the dark buttons, dark on
  LCARS's lit pills), with at most one part in a **role** colour where the colour is
  the information: the status colours for alert levels and autonomy, stop and cancel;
  the resource strip's colours for dilithium, latinum, metal and crew; the accent for
  a secondary part (the flask's liquid, the transporter's sparkle); a dimmer ink for
  depth (the beam's length). On a lit pill the role colours are taken most of the way
  to the ink, so they still stand out from the fill;
- **levels** are one glyph and a meter of three steps along the foot, lit one, two or
  three in green, yellow or red (alerts, movement and special-weapon autonomy);
- a 64-unit grid, strokes about 5 units, round caps, the symbol inside 6..58.

**On Fleet Operations: Roots the game's art goes first** (the user's wish; PLAN-FLEETOPS.md,
decision 340): an icon of ours stands in for a Roots sprite only where the sprite has no art
(`icons::ours_for_sprite`). Roots' art is read from the player's install, never shipped.

**A sprite that carries its own tile is the button** (decision 344): Roots' button sheets draw
each icon on a rounded translucent plate, and its top-right buttons are square tiles, so a plate
of ours behind them reads as a tile inside a tile. Whether a sprite does is measured on its art,
per sprite (`sprite::own_tile`: the band a sixth in from each edge covered along nine tenths of
every side). Such a sprite is drawn filling the button at the button's size, as Roots draws it,
with no plate, frame or edge of ours (`icons::tile_button`); on top go only the state, in the
theme's colours (the hover light and an accent edge, the active edge, a darker tile while pressed,
a grey tile while disabled) and the key badge. Empty cells, our own flat icons and art without a
tile keep the theme's plate. Armada II keeps its flat icons, so it never reaches this.

They are built from a few primitives with exact distances (circles, rounded boxes,
capsules, polygons, arcs; union, cut and intersection) in `ui/fontbake/src/icons.rs`,
baked at build time into a signed-distance atlas like the font (1 MB, 64 texels per
icon), and drawn by the text's shader kind (`src/icons.rs`), so they are sharp at any
size. Each is named by the game sprite it stands in for (`b_stop`, `comm_button`):
the bar and the top-right buttons look the sprite's name up and fall back to the
game's art. `cargo run --example icons --target x86_64-unknown-linux-gnu -- out.ppm`
in `ui/fontbake` renders every icon at the bar's size and twice that, for looking at
a change. Not ours: special weapons and build items (unit and weapon art). The menus
draw from the same atlas where they need a symbol: the campaign selection's padlock
(`padlock`, in the name's colour, MENUS.md, "Locked campaigns").

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
(the shader compilers unroll and dislike branches, `render3d/README.md`). Three float4s per vertex carry the parameters, so the
vertex is 84 bytes; a whole frame of UI is still a few draw calls.
Kind 5, *metal*, is the same shape with its colour worked into machined metal by a grain
fixed in screen pixels (`Draw::metal`, `Metal`: grain size, strength, stretch along the
brushing, its angle, a broad sheen); Starfleet's rims and buttons are drawn with it.

## The gallery

`[Demo] Style=1` in `UI.ini` (`ui/bench-asi.sh <state> Demo.Style=1`): in the front
end the Credits show the style gallery over the menu plate (Esc leaves it); in a
mission it is drawn over the game; on the loading screen the loading look. Eight
cards: each theme with buttons in every state, a live toggle and slider, a bar, a
list with hover, selection and scroll bar, a focused field with caret and drop-down
arrow, tabs, a tooltip and the colour swatches; and the type scale and spacing.
