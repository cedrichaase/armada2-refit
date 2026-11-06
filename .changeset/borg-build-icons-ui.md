---
layers: [ui]
bump: patch
---

### Fixed
- The button bar draws every icon of the Borg Assembler's build list (issue #85): a 32-bit TGA with alpha 0 in every texel (gbbsensor, gbbturret, gbbturret2, gbbtechass) is drawn opaque, as the stock bar draws it, not fully transparent (`gfx::load_tga`; UI.log: "tga: <file> has no alpha; drawn opaque").
