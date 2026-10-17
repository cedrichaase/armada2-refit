# Marks self-test
Film: marks

A board for the event marks (films/README.md, "Marks"), not a film: each shot makes the
events Scene.asi marks happen and states them as `Expect:`, so `./a2film probe
films/marks --jobs 1` checks the marks end to end (Scene.asi's clock against the take's,
the runner's, the checks), and `./a2film marks films/marks` lists what each take saw.
The stock variant only: the marks do not depend on the refit, and it installs fastest.

## Marks

### m01 · Selection, a glide and a cloak
Shows: marks
Scene: cloak
Variants: stock
Length: 10
Cut: from glide.end to decloak.done:warbird +0.5
Expect: selected >= 2 by 3s; glide.end before cloak.start; cloak.done:warbird before 12s; decloak.done:warbird; action.press count == 1

Two ships are selected by command, the camera glides for 3 s, the warbird cloaks and
decloaks. `Cut:` takes the shot from the end of the glide to half a second after the
warbird is seen again.

```setup
scene orbit warbird 230 12 260
```
```action
wait 1
scene select warbird bop
press ctrl+1
wait 1
scene glide 3 300 6 300
wait 3.5
scene cloak warbird on
wait 6
scene cloak warbird off
wait 6
```

### m02 · Fire and an explosion
Shows: marks
Scene: explosion
Variants: stock
Length: 8
Cut: from fire.start:galaxy -0.5 to explosion:big +3
Expect: fire.start:galaxy before 4s; no explosion before 5s; explosion:big after 5s; destroyed:big; fire.end:galaxy

The Galaxy is given its weapons and ordered at the battleship, which is blown up after
a few seconds. The first probe (2026-10-17) showed what only the marks would have: every
burst's `target=small`, the destroyer nearest it, not the battleship it was ordered at.
So `Expect:` asks for fire, not for its target; `fire.start:big` would ask for that.

```action
wait 1
scene weapons galaxy on; attack galaxy big
wait 5
scene kill big
wait 6
```
