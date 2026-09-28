# binkproxy — a `binkw32.dll` that plays modern video

Armada II plays every movie through RAD's Bink 1, via ten functions in
`binkw32.dll`. This is a drop-in replacement for that DLL. It does two things:

1. **Replacement movies.** When the game opens `animations/<Name>.bik` and
   `<Name>.mp4` sits beside it, the replacement plays that instead: AV1 video
   decoded by Media Foundation (Proton backs it with GStreamer and `dav1d`),
   and the audio from `<Name>.wav`. Every other movie is forwarded untouched to
   the real DLL, renamed `binkw32_orig.dll`.
2. **Full-screen launch reels.** `Program::PlayIntroMovie` switches to a
   hard-coded **640x480** display mode for the four launch reels (Activision,
   Paramount, Mad Doc, Intro), and `NextBinkFrame` copies each frame **1:1 into
   the locked back buffer at (0,0)**. The replacement raises that mode to the
   desktop size at load, and scales every reel to fit, centred, bars black. That
   applies to the three logo reels too: they stay real Bink, decoded into a
   scratch buffer and scaled from there.

Why a code change and not a re-encode: a larger `.bik` in that path would
have been copied into a 640x480 back buffer at its own height, past the end of
the buffer. Nothing in the game's config reaches either number.

## Use

    cutscenes/binkproxy/ai-movie.sh Intro upscale [--dry-run]   # PAID -> $A2_DATA/movies/Intro/ai/upscale/
    cutscenes/binkproxy/build-movie.sh Intro                    # free -> src/, out/
    cutscenes/binkproxy/install.sh                              # DLL + every built movie
    cutscenes/binkproxy/install.sh --remove                     # full uninstall

`./a2mod stock` / `refit` switches the whole thing as the `cutscenes`
layer: the DLL through its `.a2neb-backup`, plus `binkw32_orig.dll`,
`BinkProxy.ini` and every `animations/*.mp4`/`.wav` that stands beside a
`.bik`.

## Movies: `cutscenes/movies/<Name>/movie.conf` + `$A2_DATA/movies/<Name>/`

Laid out like a texture target, and for the same reason: keep the paid layer, so
everything downstream of it can be changed for free. The recipe is committed; every
layer is in `A2_DATA` (`a2env.sh`, default `~/.local/share/armada2-refit`), never
in the checkout.

| | | |
|---|---|---|
| `movie.conf` | committed | pieces, joins, the interpolation method and its settings, crf, cuts |
| `ai/upscale/p<N>.mp4` | **paid** | raw upscaler output, one per piece |
| `ai/interp/p<N>.mp4` | **paid** | raw AI interpolation; only when `interp=apollo` |
| `src/upscaled.mkv` | derived | the pieces joined, lossless, source frame rate |
| `src/interp.mp4` | derived | at the target frame rate |
| `out/<Name>.mp4`, `.wav` | derived | what `install.sh` copies |

Nothing writes to `ai/` except `ai-movie.sh`, and that never overwrites: `--force`
renames the old piece aside first. `src/` and `out/` can be deleted at will.
**The stock `.bik` is not in the repo.** It is read from the game directory and never
written; the replacement goes in beside it.

**Switching to AI interpolation** is a config change and one paid run:

    # movie.conf: interp=apollo
    cutscenes/binkproxy/build-movie.sh Intro          # stage 1 only needs ai/upscale
    cutscenes/binkproxy/ai-movie.sh Intro interp      # PAID -> ai/interp/, same pieces
    cutscenes/binkproxy/build-movie.sh Intro          # joins ai/interp, repairs the cuts
    cutscenes/binkproxy/install.sh

Apollo blends across hard cuts, making one ghosted frame per cut, as the Picard
cut showed in the comparison. So the build drops the invented frame at each of
`cuts` and repeats the last real one in its place. `minterpolate` needs no such
repair, because `scd=fdiff` finds the cuts itself. **The Apollo branch is
untested end to end**, since no paid `ai/interp/` exists yet. Its join and cut
arithmetic is the same as stage 1's. The comparison clips that led to
`minterpolate` are in `$A2_DATA/archive/intro-upscaler-comparison/`.

**The intro in use:** the stock `Intro.bik`, upscaled 2x by
`bria/video-increase-resolution` in six pieces (Bria takes at most 30 s, not
the 60 s it advertises), joined, then taken from 15 to 30 fps by
`minterpolate`. That comes to 1280x960 at 30 fps and **34.9 MB of AV1, the same
size as the 640x480, 15 fps stock `.bik`**, plus an 11.9 MB WAV.
`build-movie.sh Intro` rebuilds it from `ai/upscale/`: the joined master's
decoded frames match the original MD5 for MD5.

`BinkProxy.ini` switches each part off separately (`ReplaceMovies`,
`RaiseIntroMode`, `FitIntro`) and can pin the intro mode (`IntroWidth` and
`IntroHeight`). `BinkProxy.log` records every open and close, with decode
timing and a count of late frames.

## Why AV1 and a separate WAV: measured, not chosen

Under Proton-CachyOS, through `run-test.sh`:

| MP4 carrying | Result |
|---|---|
| H.264 (High or Baseline) | no video stream (`MF_E_INVALIDSTREAMNUMBER`) |
| VP9 | same |
| **AV1** | **decodes**; RGB32 accepted |
| AAC audio | `PCM refused` |

GStreamer autoplugs a decoder, it "failed to initialise", and the retry asks
for Proton's `protonvideoconvert`, which skips registration outside Steam.
Wine's own MP4 source (`mfmp4srcsnk`) fails first, with "Unsupported demuxer,
status `0xc000007a`". AV1 goes through `dav1d` and avoids all of it. The audio
is 16-bit PCM in a WAV file that the DLL parses itself, with no decoder
involved. If a later Proton decodes AAC, the DLL uses the MP4's own track
first.

`make-movie.sh` tags the video as BT.601 limited range, which is what the
source YUV was (it came from Bink). Left untagged, GStreamer assumes BT.709
for anything 720 lines or taller.

## What was established from `Armada2.exe`

- **Imports:** the ten decorated exports `_BinkOpen@8`, `_BinkDoFrame@4`,
  `_BinkNextFrame@4`, `_BinkWait@4`, `_BinkClose@4`, `_BinkGoto@12`,
  `_BinkSetVolume@8`, `_BinkCopyToBuffer@28`, `_BinkSetSoundSystem@8` and
  `_BinkOpenDirectSound@4`. There are 33 call sites across `BinkThread`
  (in-game cinematics), `Program::PlayMovie` (launch reels), `AnimButton` and
  the main and single-player menus.
- **BINK fields read:** `+0` Width, `+4` Height, `+8` Frames, `+0xc`
  FrameNum (1-based). `NextBinkFrame` ends a movie when `FrameNum == Frames-1`
  after a copy, so at end of stream the DLL pulls `Frames` in to
  `FrameNum+1`.
- **Pacing:** `PlayMovie` spins on `BinkWait` and handles window messages
  between frames; Escape closes the movie. The DLL paces against the
  performance counter from the first `DoFrame`, which is also when the audio
  starts.
- **Back buffer size:** `NextBinkFrame` (`0x4637f0`) reads the back buffer's
  `D3DSURFACE_DESC` at `[ebp-0x28]` and calls `BinkCopyToBuffer` from
  `0x4638af`. The DLL reads Width (`-0x10`) and Height (`-0x0c`) from that
  frame only when its return address is exactly `0x4638b5` and the values are
  sane. That's why `build.sh` keeps frame pointers.
- **Pixel formats:** A8R8G8B8, X8R8G8B8, R5G6B5 and X1R5G5B5 map to Bink
  surface types 5, 3, 10 and 9. All four are handled.
- **Sound system:** `AudioManager::Open` passes the address of
  `BinkOpenDirectSound`, read out of the import table rather than called, into
  `BinkSetSoundSystem`. That address is the replacement's own, so it is
  swapped for the real one on the way through.
- **Mode patch sites:** `push 32/480/640` and the 16-bit fallback, both in
  `PlayIntroMovie`. Each signature occurs once in the file. The bare 640/480
  push pair also occurs a third time elsewhere, and is left alone.

## Testing without the game

    cutscenes/binkproxy/run-test.sh <dir> Intro.bik 900 3440 1440

`binktest.exe` drives the DLL the way `PlayMovie` does. `run-test.sh` sets up
the GStreamer environment that Proton's own launcher would. Without it, Media
Foundation has no decoders and every movie falls back to its `.bik`.

Measured on the installed intro: 900 frames through the fit path at
3440x1440, 2 late, 12 ms average decode. A frame compared against ffmpeg's
decode of the same file gave RMSE 0.015. `Activision.bik` through the
pass-through path came out pillarboxed to exactly 1920x1440.

## What depends on the setup

The intro plays full screen in game here with `MSAA.asi` installed, so on this setup
both of these hold. On another, `BinkProxy.log` says which failed:

- **The back buffer has to lock with MSAA on.** `msaa` clears
  `LOCKABLE_BACKBUFFER` on the grounds that "the engine never locks it", but
  this path does lock it. A failed lock is caught (`IsBadWritePtr` on the first
  and last row) and logged as `back buffer not writable; frame skipped`. If
  that line appears, the reels play black, and the fix belongs in `msaa`,
  not here.
- **The raised mode has to be found.** `FindDisplayMode` has to list the
  desktop size. Menus.asi depends on the same thing for the shell.

## Not yet tried

- **In-game cinematics** (`BinkThread`) can take replacements too. The code
  path is generic, but only the intro has been tried.
