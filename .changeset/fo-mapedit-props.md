---
layers: [ui]
bump: minor
games: [fo-roots]
---

### Added
- Fleet Operations: Roots: the map editor's object properties (FleetOpsHook's `TGameObjectDialogForm`, a NextSuite property inspector: headings, text, number, choice, check box and colour items, the view box and Restore Class) and map settings (`TMapOptionsDialogForm`: four tabs of fields) are dialogs of ours; every edit goes through Roots' own controls (a click on the hidden inspector's value, the typing to its in-place editor, OK to apply) (PLAN-FLEETOPS.md, decisions 630-639).
- A window that activates while no form of ours is up and is not one the forms take is logged once per class (`forms: <class> ... activated with no form up`).
