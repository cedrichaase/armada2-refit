---
layers: [display]
bump: minor
games: [armada2, fo-roots]
---

### Added
- Block-compressed textures for a game that loads DDS through D3DX8: CheckDeviceFormat answers DXT1, DXT3 and DXT5 as 2D textures when the backend samples BC1, BC2 and BC3 (Vulkan textureCompressionBC, enabled when present; Direct3D 12 always), mirrored and sampled natively with every level, small mips included. CheckDeviceMultiSampleType accepts a multisample type (ignored). A later CreateDevice on another window moves the Vulkan swap chain to it (Direct3D 12 move written, not verified on the bench). Bench-only keys BenchTextures and BenchWindow. GetDesc of a standalone block-compressed surface gives its bytes. The stock engine is unchanged.
