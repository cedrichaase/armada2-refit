---
layers: [ui, render3d]
bump: minor
games: [fo-roots]
---

### Added
- Roots: screen specs of ours for FleetOpsHook's adopted dialogs (`[Menus] Forms=1`, `Specs=1`): Single Player as Armada II's campaign selection (over the menu scene a column of Roots' four campaigns, named by the dialog's own hover labels, each with a cut of its card's art, the camera moving to its view; the cards without the scene), the mission list as Armada II's list, Load Game and Save Game as Armada II's save list with the name typed into Roots' own edit, the one-button message as Armada II's; every action a player's click on the hidden dialog, the pointer resting first; a dialog without a spec stays its picture
- Roots: the shell's widget reader runs under the forms (the dialogs' buttons, labels and rows read as on Armada II); the profile's `forms.dialog_specs` names the eight dialog procedures, verified on Roots 5.0Z11
- The fleetops content set's four campaigns (`[Campaign.*]`, `FirstCampaign`), and the fleetops menu map's views `dominion`, `species8472` and `freeborg` (Roots' Dominion and Cardassian fleet, Species 8472 bioships with a Borg cube, the Free Borg sphere)

### Changed
- Armada II's campaign selection, mission list and save screens share their layouts (`campaign_column`, `campaign_cards`, `mission_list`, `saves_screen`, `notice_window`) with Roots' specs; Armada II's screens are drawn as before
