---
layers: [display]
bump: minor
games: [armada2, fo-roots]
---

### Added
- Block-compressed textures for a game that loads DDS through D3DX8: CheckDeviceFormat answers DXT1, DXT3 and DXT5 as 2D textures when the backend samples BC1, BC2 and BC3 (Vulkan textureCompressionBC, enabled when present; Direct3D 12 always), mirrored and sampled natively with every level, small mips included. CheckDeviceMultiSampleType accepts a multisample type (ignored). A later CreateDevice on another window moves the swap chain to it, on Vulkan and on Direct3D 12 (both seen on the bench). Bench-only keys BenchTextures and BenchWindow (which hides its popup when it hands the game's window back, gives the activation back to the game's window when the popup takes it, and logs the Presents made after each handover). GetDesc of a standalone block-compressed surface gives its bytes. The stock engine is unchanged.
