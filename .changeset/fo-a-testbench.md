---
layers: [testbench]
bump: patch
---

### Fixed
- The Vulkan validation layer is set in the game's environment again: `vulkan_env` (and the recorder's check) no longer wait for a retired `Mode=stub` in `Display8.ini`, which the shipped ini has not carried since display 1.0.0, so every bench game on Vulkan ran without the layer. `display/bench-mode.sh` finds `Display8.ini` beside the executable (Roots' `Data/`). `fo-display8` expects Display8 on Vulkan through a skirmish instead of the bail-out.
