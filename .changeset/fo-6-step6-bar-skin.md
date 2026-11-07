---
layers: [ui]
bump: minor
---

### Added
- Fleet Operations: Roots: with `[HUD] Bar=1` the button bar is a skin over the stock buttons (`hud/barskin.rs`): each shown button is drawn in the theme at the palette's origin plus its own rectangle, hover and pressed from the button's bytes, icons read from the install at run time; the stock bar keeps the pointer, the tooltip and Roots' range rings (`engine::hud::set_bar_skin`). Off in `ui/defaults/fo-roots.ini`; scenario `fo-ui-bar`. Stock's bar is unchanged.
