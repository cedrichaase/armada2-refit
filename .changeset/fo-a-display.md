---
root: minor
display: minor
games: [fo-roots]
status: Untested: seen on the bench only (Fleet Operations: Roots on Vulkan, shell and a skirmish); stock 1.1 unchanged
---

### Added
- Display8 runs Fleet Operations: Roots 5.0Z11 on its own device: `display.modes` is verified beside Roots' changes (`[[changed]] verified` on FindDisplayMode, SetActiveDisplay_Internal, InitializeGraphicsSystem and PlayIntroMovie in `profiles/fo-roots-5.0Z11/build.toml`), so the stub starts in place of the bail-out; `profiles/fo-roots-5.0Z11/display.toml` gives Roots' shell request (1200x900x32) and intro request (1200x900) as the build's values. `display.native` and `display.drop` still stand down under Roots.
