---
layers: [testbench]
bump: minor
status: none
---

### Added
- Scenario `fo-ui-dialogs-log` (Roots' Admiral's Log through its spec: a tab chosen, Done)
- Scenario `fo-ui-dialogs-setup` (Roots' Game Setup through its spec: a side chosen through our drop-down, LAUNCH, the skirmish under way)
- Scenario `fo-ui-dialogs-settings` (Roots' Game Settings through its spec: the difficulty stepped, a slider set). Scenario `fo-ui-dialogs-load-mirror`: Roots' Save and Load from a mission with the adopted dialogs as their pictures (`Specs=0`), the reference for `fo-ui-dialogs-saves`

### Changed
- `fo-ui-dialogs-saves` answers Roots' question Yes and checks the save loads
