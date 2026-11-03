# d12probe — does Direct3D 12 work for Display8 here?

`vkprobe`'s sister (`../vkprobe/README.md`): a small 32-bit Windows program
(`d12probe.exe`, no installer, no dependencies beyond the system's `d3d12.dll` and
`dxgi.dll`) that does what Display8's Direct3D 12 backend does, on its own, and writes
down how it went in `d12probe.txt` beside itself. The first line answers the question:

    VERDICT: Direct3D 12 works for Display8
    VERDICT: no: <the first step that failed, and what it returned>

It exits 0 when it works and 1 when it does not, shows no message box, and needs no
input, so CI and the gates kit (`tools/gates/`) run it unattended.

## What it does, and why it is not a copy of Display8

1. A device at feature level 11_0 on the adapter asked for (`--warp`: Windows' software
   adapter, which a CI runner has; otherwise Direct3D 12's default), a queue and a fence.
2. A flip-model (discard) swap chain of two B8G8R8A8 buffers on a 640x480 window.
3. The two shader-visible heaps (views and samplers), an **8 MB upload ring, mapped
   once and kept mapped**, shared by two frames in flight (half each), a read-back
   buffer, and the targets (B8G8R8A8 colour, D16 depth, a 256x1 ramp, a white texel).
4. **Display8's root signatures and pipeline states, built by Display8's own code.**
   `../src/dx12/build.rs` holds them (the present pass's root signature and pipeline
   description, the passthrough's root signature and `pass_pipeline`), taking only a
   device; Display8 calls it from `dx12/mod.rs` and `dx12/pass.rs`, and the probe
   includes the very file by `#[path]` (with `raw.rs`, `passthrough/state.rs` and the
   committed DXIL). Microsoft's runtime, and with `--debug` its debug layer, then judge
   exactly what Display8 makes, which vkd3d-proton (more forgiving) does not. Seven
   passthrough keys are built: `vs_ff` opaque, blended, without depth, as a line list,
   at render3d's scene pass (half-float, D32, four samples), and `vs_bump` twice.
5. 300 frames: filler, ramp and vertices into the ring; the left and right halves of
   the offscreen target cleared to two colours; a passthrough draw (constant blocks are
   zeros, so its triangle is clipped: the root signature, tables, vertex streams and
   pipeline are what is judged); the present pass into the swap chain. The last frame's
   back buffer is read back and **seven pixels** checked (±3).
6. With `--debug`, the debug layer's messages are listed and an error among them fails
   the run.

The passthrough's own pixels are not checked: that is the bench's job (`display-passthrough*`).

## Options

    d12probe.exe [--warp] [--debug] [--agility <sdk version> <folder>]

- `--warp` WARP, the software adapter.
- `--debug` the D3D12 debug layer (fails if there is none), its messages counted.
- `--agility 616 D3D12` use the Agility SDK's `D3D12Core.dll` (and its
  `D3D12SDKLayers.dll`, the debug layer) from the folder (relative to the exe) as SDK
  version 616 (`ID3D12SDKConfiguration::SetSDKVersion`). Without it the system's
  Direct3D 12 is used, and `--debug` needs Windows' "Graphics Tools".

CI (`.github/workflows/ci.yml`, job `d3d12-probe`) downloads the Agility SDK from NuGet
at run time and never keeps or ships it (hard rule 8): the gates kit has the probe, not
the SDK, and runs it without `--debug`.

## Building and running

    display/d12probe/build.sh          # -> display/d12probe/build/d12probe.exe
    display/d12probe/run-wine.sh       # one short run here, under Wine on vkd3d-proton

Built like `vkprobe` (rustup's `i686-pc-windows-msvc`, `lld-link`, no C runtime, import
libraries from `llvm-dlltool`, a version resource). `run-wine.sh` runs it under the
project's Proton Wine on **vkd3d-proton, which is the machine's GPU**: one run of a few
seconds, never beside a game, never beside another Direct3D 12 program. vkd3d-proton
has no debug layer to list messages from (it only answers `D3D12GetDebugInterface`),
and no WARP: those two are CI's.

## Results

- 2026-11-03, Proton-CachyOS 11.0 (wine-11.0), vkd3d-proton, Radeon RX 5700 XT:
  `VERDICT: Direct3D 12 works for Display8`, all 7 pixels right. `--warp --debug` fails
  at once on the debug layer, as it should (`0x887a002d`, no layer).
- CI on windows-2025 (WARP and the debug layer): see the workflow run of the commit.
