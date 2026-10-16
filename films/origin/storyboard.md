# I never wrote a line of this
Film: origin
Final: 3840x2160
Pointer: 1920x1080
Variant refit: --assets none --install . --install testbench/scene

The game footage for the origin video: how the refit came to be, told by its maker, who
wrote none of its code and directed all of it by sentences. The video itself is mostly
screen recordings, screenshots and cards, and is cut elsewhere (the maker-promo board
`yt-origin.board.md`); this board films only the in-game shots it needs that
`films/promo` does not have. Every other game shot in that video is a promo take, used
by path.

Each shot here answers one of the maker's prompts: the prompt is typed on screen over
it in the cut, and the shot is the result. So each shot shows exactly what that prompt
changed, and nothing on screen may claim more.

For the editor:

- The house style holds (`films/README.md`): close-ups, the camera a little below the
  subject looking up, a wipe for one centred subject, no game music in any take.
- **"Refit" is the game as a release installs it** (`--assets none`), as in
  `films/promo`: every code layer and none of `$A2_DATA`'s assets, so no remastered hull
  textures. What changes between stock and refit here is the renderer, not the art.
- **No titles or captions in the takes or the layouts**: the video's own text track
  (the typed prompts) goes over them. The `Title:` fields are left out on purpose.
- Every shot leaves at least a second of take before `In:` and after its `Length:`, as
  handles for the cut.

## Taste

The video's fourth part: the maker's prompts were judgement, not "fix it", and what the
agents did with them. Each shot is one prompt's result, held long enough to read the
prompt over it.

### ak13 · The Akira class
Shows: lighting: hulls
Scene: akira.ini
Variants: stock, refit
Layout: wipe
Transition: cut
In: 1.0
Length: 12
Status: draft

The prompt: "the akira class hulls look too dark". One Akira class, close and a little
below, turning slowly: stock's hull against refit's, lit per pixel by the scene's key
light. The eye lands on the saucer and the weapons pod above it, where stock is flat and
dark and refit shows the plating. Stock fades a hull that comes within about 150 of the
camera (`films/promo` h01), so the camera stays outside that.

```setup
scene hover off; shieldfx off; tooltips off
scene orbit akira 205 -6 175
```
```action
wait 1
scene glide 16 245 -4 165
wait 14.5
```

### mt14 · The Metreon nebula
Shows: nebulae
Scene: metreon.ini
Variants: refit
Layout: full refit
Transition: cut
In: 1.0
Length: 10
Status: draft

The prompt: "make the metreon nebula look more like actual thunderstorm clouds". A storm
front of three Metreon clouds from outside, the camera drifting in: crimson billows,
lit and shadowed sides, and lightning flickering inside them. Refit alone: the prompt is
about the result, and stock's billboards answer a different question. Lightning falls at
random: if a take has none inside the window, move `In:` or re-probe.

```setup
scene hover off; shieldfx off; tooltips off
scene orbit 2250 0 2300 200 -5 1000
```
```action
wait 1
scene glide 13 225 -3 820
wait 12.5
```

## Bigger prompts

The fifth part: the renderer rewritten in Rust, shown at its best.

### as17 · The assimilation beam
Shows: specials
Scene: specials
Variants: stock, refit
Layout: wipe
Transition: cut
In: 2.0
Length: 10
Status: draft

A Borg cube fires its technology assimilation beam (`gTechAssimBeam`) at the Sovereign:
in refit a green beam with two strands twisting about it, drawing data back to the cube
(`render3d/ordnance.ini`, `techassimbeam`). The cube large in frame, the camera on its
side and a little below, so the eye goes from the cube along the beam to the target.
The beam needs its target within 300 (`TechAssimBeam.odf`): the cube is spawned 220
away, as in `films/promo` b05. `special` must answer `fired`.

The user's direction for this beam (`ART-DIRECTION.md`, "Tractor and assimilation
beams": a fluctuating cone of light in vapour, its wide end taking in most of the
target, washing its hull in the beam's colour) is not on `main` yet: it is on the
unmerged branch `worktree-cloak-art-direction`. Filmed from `main`, this shot shows the
beam as `main` draws it, and must not be cut as the result of that direction.

```setup
scene hover off; tooltips off
scene spawn holder bbattle1 3000 0 3220 180 1
wait 3
scene engines holder off; energy holder on
scene orbit 3000 0 3120 75 -6 320
```
```action
wait 1
scene special holder gTechAssimBeam tgt; glide 13 95 -4 290
wait 13
```
