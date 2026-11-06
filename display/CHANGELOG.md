# Changelog — display

`d3d8.dll`, "Display8": the game's Direct3D 8, ours. Versioning rules:
[`CLAUDE.md`](../CLAUDE.md), "Changelogs and versions". Newest first. The reasoning is
in [`README.md`](README.md).

## 1.1.0 — 2026-11-06

### Changed
- The engine resolver (`crates/a2engine`, `csdk/a2engine.h`; root 15.8.0) names the executable in its log lines and notes by the matched build's `exe` (`profiles/*/build.toml`), or on an unknown build by the process's own module file name, not by a literal: `engine: Armada2.exe timestamp ...` on Armada II as before. The generated symbol tables carry each build's `exe`.
- Engine fields and values come from the matched build at run time (`field::X()`, `value::X()`), no longer compile-time consts of `gog-1.1` (PLAN-ARCHITECTURE.md, AR-3).
- The stub's walk of the engine's mode list (`stub/modes.rs`) reads the device's list and each mode's width, height and depth through new `display.toml` fields (`dev_modes`, `mode_node_value`, `mode_width`, `mode_height`, `mode_bpp`, hook `modes`), no longer literal offsets. `b524e42`

Confirmed in game 2026-11-06.

## 1.0.7 — 2026-11-05

### Fixed
- a2rt: the game directory, `%LOCALAPPDATA%` and every file path now go through the wide Win32 calls (`GetModuleFileNameW`, `GetEnvironmentVariableW`, `CreateFileW`, `FindFirstFileW`, ...; `wstr` / `from_wide`; the imports join `kernel32.def`). With a non-ASCII install path (`C:\Users\Jörg\...`) the ANSI call's code-page bytes were not UTF-8, the game directory came out empty, `RefitPlayer.ini` was never consulted (every Refit-screen setting saved and ignored) and was written to a relative path, and the log folder was lost. Installed, not yet seen in game. Bench-verified.

## 1.0.6 — 2026-11-04

### Fixed
- Device release: freeing a device now releases everything it held, not only its back buffer and depth surface: the textures bound with `SetTexture` (8 stages), the streams of `SetStreamSource` and the index buffer of `SetIndices`. Each shell/mission device re-creation leaked them and their GPU mirrors (`stub/obj.rs`).
- Render target: the device now holds its current target and depth surface with references of its own (`SetRenderTarget` AddRefs the new and releases the old, the same surface set again included), so `GetRenderTarget` and `GetDepthStencilSurface` no longer hand out a surface the engine already released; `Reset` puts the back buffer and depth surface back as target and depth, as Direct3D does, so `Clear` and the passthrough (which skip a target that is not the back buffer) work again after a `Reset` with an engine target set (`stub/device.rs`). This changes COM refcounting.

Installed, not yet seen in game.

## 1.0.5 — 2026-11-04

### Fixed
- d12probe 0.1.1: frame 0 no longer records a transition whose before and after states are both `COPY_DEST` (the first uploads of the ramp and the white texel); the probe's `barrier` skips such a barrier as Display8's own does. WARP failed the list's `Close` with `E_INVALIDARG` on it (CI run 41880254572, jobs d3d12-probe and gates-windows); vkd3d-proton let it pass. Display8 itself is unchanged.
- d12probe: a failed `Close` or `Present` lists the debug layer's messages first (with `--debug`), so the failure names what was invalid.

Installed, not yet seen in game.

## 1.0.4 — 2026-11-04

### Fixed
- Vulkan: a failed `vkQueueSubmit` in `display8v1_vk_flush` gives the slot a fresh signalled fence (as `end_frame` does) and begins the command buffer again, so the next `begin` on the slot no longer waits for ever and the frame stays one being recorded (`Vk::flush`).
- Vulkan: `make_swapchain` takes the old images' views, framebuffers and semaphores out of its lists as it destroys them, and frees what one image made before a failure, so a failure part way never leaves a destroyed handle to be used or destroyed twice; `present` makes the swap chain again before acquiring while it is incomplete. A changed surface format destroys the old present render pass.
- Direct3D 12: a fence not reached after six 5 s waits (or with the device removed) marks the device lost; `begin` no longer resets the allocator and list or runs the deferred frees the GPU may still be using, `display8v1_dx12_frame` answers null, and `flush` stops there (`Dx12::wait`, `Dx12::ready`).
- A second `Direct3DCreate8` no longer starts the stub again: after a late bail-out it gets the system's `IDirect3D8` (the stub's vtable stays the forward), and while the stub runs another `IDirect3D8` of the stub's (`bail::again`, `stub::another`).
- After a bail-out (`Backend=none` included) the shell's 800x600 request is no longer raised to the desktop's for the UI's menu host, which refuses every dialog without a device: the stock shell fills the screen again (`stub/modes.rs`).
- Overlay writes (`RefitPlayer.ini`) retry the rename 8 x 50 ms while a reader holds the file (`a2rt::move_file`, root 15.6.15).

Installed, not yet seen in game.

## 1.0.3 — 2026-11-03

### Fixed
- Vulkan: a spent descriptor pool no longer freezes the picture (PR #23 by bmh129, ported from 0.15.3 and 0.15.4 onto the 1.0 code). A frame that could not be presented was left open, so its pool was never reset and every later frame failed the same way, ending in `vulkan: no swap chain image (TIMEOUT)`. The present pass takes its two sets from a pool of its own per slot (`Slot::ppool`), everything that can fail comes before the image is acquired, and a frame that still cannot be presented is ended unshown (`Vk::abandon`, logged as `vulkan: frame N not presented (...)`).
- An abandoned frame returns true (the frame is dropped, the device is fine): it was `D3DERR_DEVICELOST`, and the stub's `Reset` rebuilt every plugin over one skipped frame.
- Vulkan: `Vk::end_frame` ends a frame for `present` and `abandon` alike. A failed `vkEndCommandBuffer` leaves the fence alone and submits nothing; a failed `vkQueueSubmit` gives the slot a fresh signalled fence; `abandon` saves the pipeline cache every `cache::EVERY` frames as `present` does.
- Vulkan: the frame pool's `COMBINED_IMAGE_SAMPLER` count is 65536, so its 16384 sets cover 4096 of gpu's 16-sampler texture sets (it was 2048).
- Direct3D 12: the present pass's upload ring and descriptors come before the barriers, and a frame that cannot get them is ended unshown (`Dx12::abandon`, logged as `d3d12: frame N not presented (...)`), where it stayed open and froze every later frame.
- Built on `gpu` 2.0.1.

Installed, not yet seen in game.

## 1.0.2 — 2026-11-03

### Fixed
- Below the top Graphics Detail, every texture the engine drew through the passthrough was white (the nebulae left to the engine as hard-edged flat squares; the fog's texture without a mirror for the overlays): the stub's `CheckDeviceFormat` said yes to R5G6B5, X1R5G5B5, A1R5G5B5 and A4R4G4B4 (and A8, L8, A8L8, DXT), which the engine asks for first below the top detail, and no mirror holds them. It now takes A8R8G8B8 and X8R8G8B8 alone as textures, so the engine creates them 32-bit at every detail (`stub/d3d.rs`, `texture_format_ok`).

### Changed
- A texture the passthrough draws white is logged once per format in `Display8.log`, tracer or not (`passthrough: a texture of format N ... drawn with a white one`).

Installed, not yet seen in game.

## 1.0.1 — 2026-11-03

### Removed
- The d3dbc form of the shaders (`src/shaders/ff.rs`, `present.rs`), unused since 1.0.0;
  the stamps of the SPIR-V and DXIL moved (their code did not). `build.sh` no longer
  passes `--fix`. The contract's `BACKEND_D3D9` is retired in `gpu/src/display.rs`
  (Display8 never answers it).

Installed, not yet seen in game.

## 1.0.0 — 2026-11-03

### Added
- The bail-out (phase 11 R1): when Display8 cannot draw the game it hands the engine the system's own `d3d8.dll` (`GetSystemDirectoryA`), never a `d3d8_real.dll`, and `display8v1_backend` answers none, so the plugins stand down (`bail.rs`). Early at `Direct3DCreate8` (`Backend=none`, an unknown build, a stood-down `display.modes`, a legacy plugin loaded, or both probes failing: `vk::probe`, `dx12::probe`); late at the engine's first `CreateDevice` when every backend fails there (the stub's `IDirect3D8` becomes a 16-method forward of the system's). Logged `bail-out: <why>; the game draws through <path>; the plugins stand down`.
- `Backend=none`: the bail-out on purpose (the stock renderer); `none` ends a list.

### Changed
- `[Samplers]` is read from `Display8.ini` alone, shipped `Anisotropy=16`, `LodBias=-0.5`, `ClampNegativeLodBias=0` (`auto` is the same); `dxvk.conf` is no longer read. `Defaults=3` (`tools/ini-merge.py`) carries a marked `dxvk.conf`'s differing values into `[Samplers]`.
- `display/install` moves no chain behind Display8 any more (no `d3d8_real.dll`); `--remove` puts back the backup, else an old `d3d8_real.dll`.
- The tracer, `[Drop]` and the detail capture run on the stub's device; `display/bench-mode.sh` takes `stub` only (`forward` runs as the stub, with a warning) and can write the retired `Mode=`/`Real=` for `display-retired-keys`.
- `display/bench-abi.sh old-display` puts the old Display8's chain (d3d8to9 as `d3d8_real.dll`, DXVK's `d3d9.dll`) into the clone itself, from the old commit's `platform/vendor`.

### Removed
- Forward mode: `Mode=` and `Real=` (read past, logged `settings: Mode=forward retired`), the forward wrappers (`d3d8/device.rs`, `direct3d.rs`, `resource.rs`, `com.rs`'s registry, `pass!`/`unknown!`/`draw!`).
- Direct3D 9: `Backend=d3d9` (the token is skipped, logged `settings: Backend token d3d9 retired (Direct3D 9 removed)`; a list left empty is `auto`), the Direct3D 9 device, its mirrors and the QI for `IDirect3DDevice9`/`IDirect3DTexture9`, the gamma pass (`stub/gamma.rs`); `display8v1_backend` never answers `BACKEND_D3D9`.

Installed, not yet seen in game.

## 0.21.0 — 2026-11-03

### Added
- `Display8.ini [Samplers]` (phase 10 X2): `Anisotropy`, `LodBias` and `ClampNegativeLodBias`, rows in `settings.toml` (no screen row, advanced). A value that is not `auto` is what the plugins' samplers on Vulkan and Direct3D 12 and the passthrough's (`vk/pass.rs`, `dx12/pass.rs` through `passthrough/state.rs`) take; the shipped `auto` reads that key from the game's `dxvk.conf`, as before. `Display8.log` says which: `samplers: Anisotropy=16 (Display8.ini)` or `(dxvk.conf)`. `Defaults=` is not bumped; the merge adds the section and keeps the player's values.

Installed, not yet seen in game.

## 0.20.0 — 2026-11-03

### Added
- Passthrough on Direct3D 12 (phase 10 X1; `src/dx12/pass.rs`): the engine's own draws are recorded onto the frame's list as `vk/pass.rs` records them on Vulkan, into the pass render3d published last in the frame (`exports::published`, by its own `rtv`/`dsv`, formats and sample count) or else the engine's targets. One root signature (the vertex block as root CBVs at b0 spaces 0 and 1, the pixel block at b1, tables of t0-t1 and s0-s1), pipelines by the target's formats and samples, topology, vertex layout, blend, depth and cull, a defaults record as instance data in slot 1, a white texture, mirrors for the engine's textures (`mirror::passthrough`), per-stage samplers as DXVK makes them. Nothing is assumed set that a plugin's `Cmd` may have changed and nothing is recorded while one is alive: `pass::forget` at each plugin `Cmd` event and flush.
- `display/hosttest`: host tests of `passthrough/state.rs` (`cargo test --release --target x86_64-unknown-linux-gnu --manifest-path display/hosttest/Cargo.toml`).

### Changed
- `src/passthrough.rs` is `src/passthrough/{mod.rs,state.rs}`: `state.rs` holds the API-neutral translation that sat in `vk/pass.rs` (layouts, constant blocks, blend/depth/cull/topology keys, sampler parameters, attributes, viewport clamp, notes on state not drawn as Direct3D 8 would); the Vulkan frame is unchanged (RMSE 0 against the base).
- The once-log "passed through on Vulkan only" is gone: a stub on Direct3D 9 still drops the engine's draws and says so. `Passthrough=` logs "Vulkan and Direct3D 12".

Installed, not yet seen in game

## 0.19.0 — 2026-11-03

### Added
- Contract 1.4 (`gpu/src/display.rs`, "Pass publishing on Direct3D 12"; gpu 1.3.0): the export `display8v1_dx12_pass(event, pass)` and `Dx12Pass8`, the Direct3D 12 twin of `display8v1_vk_pass`. Display8 keeps the pass published last for the frame (`dx12/exports.rs`, `published()`, forgotten at `Present`) and counts the `Cmd`s alive (`cmd_alive()`), for the passthrough on Direct3D 12 (phase 10 X1); nothing draws into it yet. `Display8.log` logs `dx12: pass published WxH xN` once for each kind of pass (on the bench render3d's 4x scene pass and a 1x pass, each frame). The rebind rule: Display8 sets no descriptor heaps but its own, and gpu binds everything again after any call into Display8.

Installed, not yet seen in game

## 0.18.0 — 2026-11-03

### Added
- `[Window]` in `Display8.ini` and the first `display/settings.toml` (`Display8_Settings`, three rows, all `NO_ROW`): `Native`, `KeepClip` and `RaiseIntro`, the first two moved from `UI.ini`'s `[UI]` (`moved_from`, which the UI hub and `./install` carry over). Display8 is now the only plugin that changes a display-mode request, the game's window or the pointer's clip (`display/README.md`, "Window and display mode").
- The shell's 800x600x16 request is raised to the desktop's mode while a plugin claims the new `CLAIM_MENUS`, and the intro's 640x480 under `RaiseIntro=1`, both inside the engine's mode lookups (`stub/modes.rs`, no byte patch), in forward mode too. `Native` is Display8's hook at the call of `InitializeGraphicsSystem` (hook `display.native`). `KeepClip` runs at `Present` (hook `display.keepclip`).
- Contract 1.3 (`gpu/src/display.rs`): `CLAIM_MENUS` and the export `display8v1_mode`, the mode the device was opened at. The profile gets the entries the moved code reads.

Installed, not yet seen in game

## 0.17.0 — 2026-11-03

### Changed
- On the symbol profile (`profiles/gog-1.1/display.toml`, generated `src/engine_syms.rs`): hooks `display.modes` (FindDisplayMode, its six calls, the front end's 800x600x16 as `value` entries) and `display.drop` (the `[Drop]` and claimed-scene code ranges). No address is left in the sources. Display8.log logs the build line and `engine: symbols: 15/15 resolved; 0 hooks blocked` after the identity line.
- `display.modes` is now all or nothing: if FindDisplayMode or any of its six calls is not stock, none is written.

### Added
- On a build it does not know, or with `display.modes` standing down, Display8 forwards to the old chain instead of starting the stub (`engine: unknown build: Display8 forwards to the old chain`): measured, the stub cannot open a display without its mode hook. `display.drop` standing down drops nothing and claims no scene of the engine's.

Installed, not yet seen in game

## 0.16.1 — 2026-11-03

### Added
- Display8.log logs `engine: Armada2.exe timestamp 0x…, image size 0x…, sections 0x…` (the executable's identity: `crates/a2engine`, `csdk/a2engine.h`) right after its start-up line. Changes no behaviour. `trace::exe_range` reads the module through the same PE reader.

Installed, not yet seen in game

## 0.16.0 — 2026-11-02

### Added
- `Display8.ini`'s keys may now come from the player's overlay, `RefitPlayer.ini`
  (`[Display8.<Section>]`, read first; the a2rt overlay lookup,
  `tools/settings/README.md`), with a `settings: RefitPlayer.ini [...]` line in
  Display8.log for each value it supplies. Without the file nothing changes.

Confirmed in game 2026-11-03.

## 0.15.2 — 2026-10-30

### Fixed
- Passthrough: the Borg hulls' bump passes no longer vanish (all but their glow) on a
  map with lights. The engine leaves `LIGHTING` on for them; its lights were written
  into the vertex block over the bump shader's constants (both from register 16),
  which threw its transform away. Lights are left out under a vertex shader, as
  Direct3D ignores them there, and the case is logged once ("the Borg bump shader
  drawn with LIGHTING on").

Confirmed in game 2026-10-30. (b08d6a8)

## 0.15.1 — 2026-10-30

### Fixed
- Stub mode starts with no `UI.asi` (or its `[Menus]` at 0): the engine's own mode
  requests (the shell's 800x600x16, the profile's, the intro's 640x480; six calls of
  `FindDisplayMode`, `src/stub/modes.rs`) get the stub's one mode when theirs is not
  listed, instead of a stack overflow before the first `CreateDevice`. Display8 alone
  runs the stock game through passthrough.

Confirmed in game 2026-10-30. (c6d6189)

## 0.15.0 — 2026-10-30

### Added
- Direct3D 12 pipeline library (contract 1.2): `Dx12Device8::pipeline_library`, an
  `ID3D12PipelineLibrary` persisted in `%LOCALAPPDATA%\armada2-refit\Display8.d3d12pipelines`
  (`src/dx12/cache.rs`; a header with file format, Display8 version and adapter, discarded
  on a mismatch or a driver refusal, written atomically when it has grown, looked at every
  120 frames). The present pass goes through it too. Uninstalling on Windows takes the
  file out.

Confirmed in game 2026-10-30. (d860f47)

## 0.14.0 — 2026-10-30

### Changed
- `Display8.log` moves out of the game directory: `%LOCALAPPDATA%\armada2-refit\logs\Display8.log`, beside `Display8.pipelines`'
  folder, made if missing; the game directory when `LOCALAPPDATA` is unset or not
  writable. The first line says where (`log: <path>`). An uninstall leaves the folder.

Confirmed in game 2026-10-30. (9a5e70a)

## 0.13.0 — 2026-10-30

### Added
- Claims (contract 1.1): `display8v1_register(&Plugin8)` and `display8v1_claims(except)`;
  every registered plugin's claims asked at each of the engine's `BeginScene`s in stub
  mode and logged when they change (`claims: <plugin> claims ...`, `src/claims.rs`).

### Changed
- Passthrough routes by the claims (README.md, "Claims"): the loading loop's scenes are
  dropped while `CLAIM_LOADING` is claimed (was: while `UI.asi` is loaded), the stock
  Credits' (`CLAIM_CREDITS`) and `RefreshDisplay`'s cursor scene (`CLAIM_CURSOR`)
  likewise; no 3D, HUD or selection draw is dropped by a claim.

Confirmed in game 2026-10-30. (cf3dec5)

## 0.12.0 — 2026-10-30

### Added
- `display8_abi(caller_major, caller_minor, caller, &major, &minor)`: the contract's
  handshake (contract 1.0); `Display8.log` names each plugin that asks and its version,
  and, in stub mode 300 frames into the first mission, a loaded `Render3D.asi` or
  `UI.asi` that never asked (built before the handshake).

### Changed
- The contract's exports are named with its major: `display8_backend`, `display8_vk*`,
  `display8_dx12*`, `display8_engine_*` are now `display8v1_*`, so plugins built before
  them find nothing rather than misreading. Every struct of the contract starts with
  `size`; Display8 fills a plugin's struct no further than its `size` says.
- The code's defaults are the shipped `Display8.ini`'s: `Mode=stub` (was forward),
  `Backend=auto` (was d3d9), `Passthrough=1` (was 0).

Confirmed in game 2026-10-30. (c470071)

## 0.11.3 — 2026-10-30

### Changed
- `src/rt` moved into `crates/a2rt`, shared with render3d and ui (with `move_file` and
  `env`, which only Display8 had); vkprobe includes its `win.rs` and `mem.rs` from there.
  Pure refactor: same pixels, same log lines. Confirmed in game 2026-10-30 (3030e0c).

## 0.11.2 — 2026-10-30

### Changed
- `install` calls the merge at `tools/ini-merge.py` (was `ui/ini-merge.py`); no change to
  what it does. Confirmed in game 2026-10-30 (1973ecb).

## 0.11.1 — 2026-10-30

### Changed
- `build.sh` and `vkprobe/build.sh` compile shaders with `tools/shaderc/hlsl-rs.sh`
  (moved from `render3d/`); the generated shader files differ in their header line only.
  `64aee16`. Confirmed in game 2026-10-30

## 0.11.0 — 2026-10-29

### Added
- On Vulkan, the mirrors of engine textures can be copied from (`TRANSFER_SRC`), so a
  recorded `gpu` frame reads back the mirrors it samples (`gpu/README.md`, "Record and
  replay"). `239f29c`

Confirmed in game 2026-10-30.

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

Confirmed in game 2026-10-30.

## 0.9.1 — 2026-10-29

### Changed
- Vulkan picks its physical device by type: discrete, integrated, virtual, other, CPU
  last (it took the first when none was discrete, so a software device could win), and
  logs every candidate's rank and the device it `chose`. `bench-mode.sh` leaves
  `Device=` empty unless `A2TEST_VULKAN_DEVICE=lavapipe` (the bench's Vulkan is on the
  GPU now, testbench 3.70.0).

Confirmed in game 2026-10-30.

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

Confirmed in game 2026-10-30.

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

Confirmed in game 2026-10-30.

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

Confirmed in game 2026-10-30.

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
