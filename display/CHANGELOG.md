# Changelog — display

`d3d8.dll`, "Display8": the game's Direct3D 8, ours. Versioning rules:
[`CLAUDE.md`](../CLAUDE.md), "Changelogs and versions". Newest first. The reasoning is
in [`README.md`](README.md).

## 0.10.0 — 2026-10-29

### Added
- Passthrough (`Passthrough=` in `Display8.ini`, 1 as shipped, 0 in the code): in stub
  mode on Vulkan the engine's own Direct3D 8 draws are drawn, through a fixed-function
  emulation (`shaders/ff.hlsl`: pre-transformed and transformed vertices, per-vertex
  lighting, two texture stages, alpha test, specular add), into the pass render3d
  publishes (its scene) or the engine's targets. What render3d or the UI does not draw
  (a module off, a SOD handed back) shows as stock instead of vanishing. ROADMAP
  phase 4, first slice; Direct3D 12 and claims are still to come (README.md,
  "Passthrough"). `24be351`
- The engine's one vertex shader, the Borg hulls' bump pass, through the passthrough:
  vertex shaders' tokens, constants and streams 1 to 3 are kept, the shader is known by
  a hash of its tokens and drawn by a port of it (`vs_bump` in `shaders/ff.hlsl`); any
  other shader is refused and logged (ROADMAP 4d). `7771201`
- `display8_vk_pass` export: a plugin's `Cmd` begins, ends, or ends with its pass
  published (`gpu/src/display.rs`, `VkPass8`).
- Every engine texture a passed-through draw reads is mirrored; a mirror upload over
  256 KB goes through a staging buffer of its own.

### Changed
- The stub device starts with Direct3D's default texture-stage and render states
  (stage 0 modulate, `TEXTUREFACTOR`, `COLORWRITEENABLE`, ...), as its `Get*` now
  return. `[Drop]` is read again every 60 presents in stub mode too.

Installed, not yet seen in game.

## 0.9.0 — 2026-10-25

### Added
- `VSync=` in `Display8.ini` (1 as shipped): at 0 the device is made with
  `D3DPRESENT_INTERVAL_IMMEDIATE`, so the frame rate is no longer held to the
  display's refresh rate (Vulkan mailbox or immediate, Direct3D 12 sync interval 0,
  Direct3D 9 and the forward chain immediate). An install adds the key to an existing
  `Display8.ini`. `4f1d1de`

Confirmed in game 2026-10-25.

## 0.8.0 — 2026-10-23

### Changed
- The shipped default is the stub on `Backend=auto` (`Mode=stub`, `Backend=auto`; were
  `forward`, `d3d9`), the user's decision once Vulkan was confirmed in game.
- `Defaults=2` in `Display8.ini`: `display/install` (`ui/ini-merge.py`) moves an
  installed `Mode=forward` and `Backend=d3d9` to the new defaults, and keeps any other
  value a player set.

Installed, not yet seen in game.

## 0.7.0 — 2026-10-23

### Added
- `Backend=auto`: the first backend that starts at the engine's first `CreateDevice`,
  in the user's order: Vulkan, Direct3D 12, Direct3D 9, on every system. `Backend=` also takes a list (`d3d12,vulkan`).
  Available, not the default: the shipped values stay `Mode=forward`, `Backend=d3d9`.
- A fallback at that `CreateDevice`: whatever `Backend=` names, Direct3D 9 comes last,
  so `Backend=vulkan` or `d3d12` that cannot start falls back to it instead of failing
  the engine's `CreateDevice`. Each attempt and its reason are in `Display8.log`
  ("stub: trying ...", "... could not start: ...; falling back to ...", "stub: the
  backend is ...").
- `FailBackend=` (the test bench only, ignored outside a bench clone and not in the
  shipped `Display8.ini`): the named backends fail on purpose once made.
  `display/bench-mode.sh` adds it to a clone's `Display8.ini`, and applies the bench's
  lavapipe rule to `Backend=auto` and to lists that name Vulkan.

### Changed
- `display8_backend` answers only once the stub's device is made (none before), so a
  plugin is never told a backend that a fallback then leaves; Vulkan and Direct3D 12
  were said from `Direct3DCreate8` on.
- A Vulkan or Direct3D 12 start that fails takes down everything it made (instance,
  surface, device, swap chain, frame), so that the next backend can have the window.

Installed, not yet seen in game.

## 0.6.2 — 2026-10-23

### Fixed
- A stub that cannot start logs the backend it tried ("the stub could not start on
  ...") instead of "no Direct3D 9", which was wrong on Vulkan and Direct3D 12.
- `Display8.ini`'s `Mode=` comment names the device `Backend=` chooses, not Direct3D 9
  alone. The shipped values are unchanged (`Mode=forward`, `Backend=d3d9`).
- `stub-scenario.sh` on a scenario whose `Setup:` is already `display/bench-mode.sh`
  (`display-stub`, `display-drop`) writes one call with the mode replaced and the keys
  added, instead of nesting a second `bench-mode.sh` inside the first.

Confirmed in game 2026-10-23.

## 0.6.1 — 2026-10-23

### Fixed
- Stub mode: the movies were black, their sound playing, on every backend. The back
  buffer the engine writes by lock goes into the frame when it is unlocked
  (`stub::back_buffer_written`), not at `Present`, where it covered the frame the UI had
  just drawn from the cutscene proxy's (`src/stub/res.rs`, `src/stub/mod.rs`).

Confirmed in game 2026-10-23.

## 0.6.0 — 2026-10-23

### Added
- `Backend=d3d12`: render3d and the UI draw the whole frame on Display8's Direct3D 12
  device through `gpu`'s Direct3D 12 backend.
- `display8_dx12_flush` (submit the frame so far, wait, go on in the same list: a
  plugin's read-back) and `Dx12Device8::features`, `sampler_anisotropy`,
  `sampler_lod_bias` (`PixelCentres=d3d9`, the samplers as `dxvk.conf` has DXVK make
  them), appended to the contract (`gpu/src/display.rs`).

### Changed
- `display8_backend` answers Direct3D 12 from the engine's `Direct3DCreate8` on, as it
  does Vulkan.

Installed, not yet seen in game.

## 0.5.0 — 2026-10-23

### Added
- `Backend=vulkan` (stub mode): Display8's own Vulkan 1.1 device on the game's window
  (`src/vk/`): swap chain, the engine's colour and depth images, two frames in flight
  with an upload ring and descriptor pools, mirrors, the gamma pass; the exports of
  `gpu/src/display.rs` for the plugins' `gpu` (`display8_vk`, `display8_vk_frame`,
  `display8_vk_alloc`, `display8_vk_defer`, `display8_vk_target`, `display8_vk_mirror`,
  `display8_vk_flush`, `display8_engine_viewport`, `display8_engine_transform`).
- `Device=` (only a Vulkan device whose name has it), `PixelCentres=` (`d3d9`: Direct3D
  9's pixel centres, for comparing with `Backend=d3d9`), and on Vulkan `Validate=1` the
  Khronos validation layer, its messages in `Display8.log`.
- The pipeline cache on disk, `%LOCALAPPDATA%\armada2-refit\Display8.pipelines`
  (`src/vk/cache.rs`).
- The plugins' samplers on Vulkan as the game's `dxvk.conf` has DXVK make them on
  Direct3D 9 (`d3d9.samplerAnisotropy`, `d3d9.samplerLodBias`,
  `d3d9.clampNegativeLodBias`), handed over in `VkDevice8::sampler_anisotropy` and
  `sampler_lod_bias`.
- `bench-mode.sh`: with `Backend=vulkan`, `Device=llvmpipe` and `Validate=1` unless
  `A2TEST_VULKAN_GPU=1`.

Measured on the bench: the port scenes within RMSE 0.4 to 0.9 of Direct3D 9 on the GPU
(`gpu/README.md`, "Measured"), and `display-stub`, the whole game, through on Vulkan,
with no validation message. Confirmed in game 2026-10-23.

## 0.4.0 — 2026-10-23

### Added
- `Backend=d3d12` (stub mode): Display8's own Direct3D 12 device (feature level 11_0),
  a flip-model DXGI swap chain on the game's window, the engine's colour and depth
  targets, two frames in flight with fences and an upload ring, two shader-visible
  descriptor heaps split per frame, the clear pass, mirrors, and the gamma pass
  (`shaders/present.hlsl` as DXIL) (`src/dx12/`). Exports for the plugins' `gpu`:
  `display8_dx12`, `display8_dx12_frame`, `display8_dx12_alloc`,
  `display8_dx12_descriptors`, `display8_dx12_defer`, `display8_dx12_target`,
  `display8_dx12_mirror`; `display8_backend` answers `BACKEND_D3D12`.
- `Backend=` (`d3d9`, `vulkan`, `d3d12`) and `Validate=` in the shipped
  `Display8.ini`; `Validate=1` turns on Direct3D 12's debug layer on `d3d12`. An
  unknown backend is logged and taken as `d3d9`.
- The present shader's DXIL (`src/shaders/present_dxil.rs`).

Measured on the bench: the intro movies and a mission run on `Backend=d3d12`
(vkd3d-proton, 32-bit), the plugins standing down. Installed, not yet seen in game.

## 0.3.0 — 2026-10-23

### Added
- `Mode=stub` (off by default): the real chain is never loaded; the engine gets an
  `IDirect3D8` and devices of ours that accept everything and draw nothing, with
  textures, surfaces and buffers in memory, and one display mode, the desktop's.
  Display8 makes its own Direct3D 9 device on the game's window, which render3d and the
  UI draw the whole frame on; what they read of the engine's state (transforms,
  viewport, `Clear`, `BeginScene`/`EndScene`) reaches it. A texture a plugin asks for as
  Direct3D 9 is mirrored there; the back buffer's movie frames are copied to the swap
  chain; the engine's gamma ramp is drawn in the frame's last pass
  (`shaders/present.hlsl`). Stands down to forward mode beside `HUD.asi`,
  `GridLayout.asi` or `Menus.asi`.
- `display/bench-mode.sh`: Display8's mode (and keys) in a bench clone from a scenario's
  `Setup:`.

Measured on the bench: the showcase frame's scene, bloom and presented frame are the
same pixels as on the real chain, the screen within 1/255 (the gamma ramp); the whole
game (`display-stub`) runs through, intro to Exit. What the stub drops that the engine
still drew (the fade from black, the order ring, the menu backdrop read from the front
buffer) is phase 1's to take over. Confirmed in game 2026-10-23.

## 0.2.0 — 2026-10-23

### Added
- Installed by `./install` and switched by `a2mod` (layer `display`): `display/install`
  puts the game's `d3d8.dll` behind ours as `d3d8_real.dll`, keeps it as
  `d3d8.dll.a2display-backup` for `a2mod stock`, and merges `Display8.ini`;
  `--remove` puts it back.
- `Mode=forward` (the only mode yet); `Real=auto` (default): `d3d8_real.dll` beside the
  exe, else the system's own `d3d8.dll`; `Trace=auto` (default): on in an `a2test`
  clone only.
- Drop switches by category: `[Drop] Drop2D=`, `Drop3D=`, `DropLoading=` (the engine's
  2D, its meshes, its loading loop), beside `Callers=`.
- Exports for the plugins: `display8_version`, `display8_device_epoch`,
  `display8_device`, `display8_frame`.
- The 400 calls after each `CreateDevice` and `Reset` are logged one by one while
  tracing.

### Removed
- The bench-only refusal of `display/install`.

Confirmed in game 2026-10-23.

## 0.1.0 — 2026-10-23

### Added
- Display8 in forward mode, on the bench only (`display/install` refuses anything but
  an `a2test` clone): a `d3d8.dll` of ours in front of the real chain, renamed
  `d3d8_real.dll`, with an object of ours for every Direct3D 8 interface and every
  method passed through. With `Trace=1` (`Display8.ini`) it counts every call by
  context (`Display8.mark`), method and caller into `Display8.inventory.tsv`, logs each
  distinct answer the engine branches on, every lock, read-back and gamma ramp, and the
  `user32`/`gdi32` calls that read the mode or move the mouse.
- `[Drop] Callers=`: draws whose call chain holds one of these return addresses are not
  passed on (read again while the game runs); `Display8.go` logs the engine's draws of
  the next `[Trace] Frames=` frames with their state and vertices.

Confirmed in game 2026-10-23.
