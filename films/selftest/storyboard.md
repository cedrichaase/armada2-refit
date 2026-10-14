# Bench self-test
Film: selftest

The smallest film there is: a card and one scene shot in two variants.
`./a2film probe films/selftest` checks the filming end to end; `./a2film animatic
films/selftest` the cut.

## Test

### t00 · Card
Length: 2

```card
[accent] A2FILM
SELF-TEST
```

### t01 · Galaxy at the planet
Shows: lighting
Scene: planet
Variants: stock, refit
Layout: wipe
Title: Planet
Sub: A wipe from stock to refit
In: 0.5
Length: 4

```setup
scene orbit planet 120 -6 1600
```
```action
wait 1
scene glide 4 150 -4 1500
wait 4
```
