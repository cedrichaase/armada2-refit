# Operating the bench: objects under the fog of war (issue #3)

Tested on 2026-10-21, `SCENE=fog-blackhole`, 1280x720, `-nointro a2_borg01`.

## Staging a ghost

    SCENE=fog-blackhole ./a2test session start --res 1280x720 --install <copy> \
        --install testbench/scene --args '-nointro a2_borg01'
    ./a2test drive --session ID shot seen
    ./a2test drive --session ID scene "goto scout 6500 0 6500"
    # 35-40 s later the scout is out of sensor range and the fog has closed
    ./a2test drive --session ID shot fogged

- An object the player has seen and no longer sees is a *ghost* (a spirit of
  `Ghost::ghostList`), not the object: the seen object stops drawing and the ghost
  takes its place. `Render3D.log` names the first ghost of each class
  (`overlays: ghost of <odf> at x,y,z, N additive mesh(es)`).
- The black hole stands 70 below the grid (its ODF's `distanceBelowGrid`): the scene
  puts it at y 0 and the log says -70. Its ghost is small at the scene's camera (about
  80 px across at 1280x720 from 2600 away); its disc's outline is where to measure.
- The scene's quantum singularity sits at 3500,0,3450 and shows lower left of the
  black hole from the scene's camera.

## A stock reference in the same scene

`--install` cannot go with `--mod stock`. Start with `--no-launch`, set `Enable=0`
under `[Render3D]` in the clone's `Render3D.ini` (only that line: a blanket `sed` on
`^Enable=1` also hits `[Specials]`, harmless with the renderer off but untidy), then
`drive launch -nointro a2_borg01`. **The launch needs the same `A2_PROTON`/`A2_UMU`
environment as `session start`**: without it the launcher exits (2) before the game
starts.

The `[Overlays]` keys (`Ghosts=`, `Fog=`) are read once at start, not on `Reload=`, so
an A/B of the ghosts alone takes two sessions.

## Measuring

Mean luminance of small patches, through ImageMagick:

    magick shot.png -crop 12x12+X+Y +repage -colorspace gray -format '%[fx:mean*255]' info:

At 1280x720 with this scene, the disc's rim at 605,368 and 655,390 against the fog
outside at 570,368 and 700,395; the swirl's centre at 620,355 (40x40).
