# Fragment: Fleet Operations: Roots, from launch to a Federation skirmish with render3d's hulls claimed

Included by the `fo-render3d-*` scenarios (`a2test --game fo-roots ... --install .`): the
sides are picked by `_fo-sides` (Federation against Klingon), and the wait after LAUNCH is
for what render3d logs when the first hull of the map is drawn,
`(claimed at the instance draw)`, written since the click on LAUNCH. The old wait on the
resource counter ("6000") depended on the race, which Instant Action deals at random.

1. Launch the game.
2. Wait for the main menu.
3. Include "_fo-sides".
4. Click "LAUNCH".
5. Wait up to 240 seconds for "Render3D.log" to say "(claimed at the instance draw)".
6. Wait 5 seconds.
