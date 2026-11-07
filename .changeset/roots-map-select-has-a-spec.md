---
layers: [ui]
bump: minor
---

### Added
- Roots' Map Select has a spec (`menus/roots/mapselect.rs`): the list of maps is read from the NextSuite grid's cells (`forms_grid_*` profile fields, read only), with the filter, the preview and Roots' Cancel and Close; `Action::GridKey`; the failed grid print is gone. Scenario `fo-ui-forms-mapselect`, and `ui-menus-connection` for the Armada II connection screen
