# Intros scout
Film: intros-scout
Final: 3840x2160
Pointer: 1920x1080
Music: 1
Variant refit: --install . --install testbench/scene

Scratch board, 2026-10-17: one long take per campaign intro (the in-engine cinematic that
opens each first mission) plus the stock main menu with its music, to log timestamped
moments for the promo. Nothing here is a cut; nothing is approved.

## Scout

### s00 · Main menu and its music
Kind: mission
Scene: menu.ini
Args: -nointro
Variants: stock
Layout: full
Audio: game
Stems: music, voice
In: 0
Length: 75
Status: draft

```setup
step Wait up to 300 seconds for the main menu
```
```action
wait 75
```

### s01 · Federation, a2_fed01
Kind: mission
Scene: mission.ini
Args: -nointro a2_fed01.bzn
Variants: refit, stock
Layout: dissolve 2 1.2
Audio: game
Stems: music, voice
In: 0
Length: 150
Status: draft

```action
wait 150
```

### s02 · Klingon, a2_kling01
Kind: mission
Scene: mission.ini
Args: -nointro a2_kling01.bzn
Variants: refit
Layout: full
Audio: game
Stems: music, voice
In: 0
Length: 150
Status: draft

```action
wait 150
```

### s03 · Borg, a2_borg01
Kind: mission
Scene: mission.ini
Args: -nointro a2_borg01.bzn
Variants: refit
Layout: full
Audio: game
Stems: music, voice
In: 0
Length: 150
Status: draft

```action
wait 150
```

